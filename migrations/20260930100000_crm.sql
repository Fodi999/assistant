-- CRM: the business's own client cards.
--
--  * `client.note`: free text of the owner/manager/reception. Not for health data.
--  * One staff-made card per (business, phone). The phone is unique only inside a
--    business, and only among cards made by staff: the phone of an app customer is
--    typed by the customer and not verified, so it never merges with anything.
--  * Old manual appointments that carry a phone get a card (see the dry run in
--    scripts/crm_backfill_dry_run.sql). Appointments without a phone stay without one.
--
-- Fails loudly (instead of silently touching zero rows) when the migration role
-- is filtered by the forced row-level security: it needs BYPASSRLS or must own
-- the tables' policies.
SET LOCAL row_security = off;

ALTER TABLE client
    ADD COLUMN note text CHECK (note IS NULL OR length(note) <= 1000);

-- The backfill links history; it must not look like an edit of the visits.
ALTER TABLE appointment DISABLE TRIGGER appointment_bump_version;
ALTER TABLE appointment DISABLE TRIGGER appointment_set_updated_at;

-- 1. One staff card per (business, phone). The name comes from the latest visit.
--    Phones are already E.164 (CHECK on appointment.client_phone); stripping
--    separators is only a safety net.
WITH manual AS (
    SELECT DISTINCT ON (business_id, phone)
           business_id, phone, left(btrim(client_name), 120) AS client_name
    FROM (
        SELECT business_id, client_name, start_at,
               regexp_replace(client_phone, '[ ()-]', '', 'g') AS phone
        FROM appointment
        WHERE client_id IS NULL AND client_phone IS NOT NULL AND client_name IS NOT NULL
    ) a
    WHERE phone ~ '^\+[1-9][0-9]{6,14}$' AND btrim(client_name) <> ''
    ORDER BY business_id, phone, start_at DESC
)
INSERT INTO client (business_id, source, full_name, phone_e164)
SELECT business_id, 'staff', client_name, phone
FROM manual m
WHERE NOT EXISTS (
    SELECT 1 FROM client c
    WHERE c.business_id = m.business_id AND c.source = 'staff' AND c.phone_e164 = m.phone
);

-- 2. Link the visits to those cards.
UPDATE appointment a
SET client_id = c.id
FROM client c
WHERE a.client_id IS NULL AND a.client_phone IS NOT NULL
  AND c.business_id = a.business_id AND c.source = 'staff'
  AND c.phone_e164 = regexp_replace(a.client_phone, '[ ()-]', '', 'g');

ALTER TABLE appointment ENABLE TRIGGER appointment_bump_version;
ALTER TABLE appointment ENABLE TRIGGER appointment_set_updated_at;

-- 3. Uniqueness, after the backfill.
CREATE UNIQUE INDEX client_staff_phone_key ON client (business_id, phone_e164)
    WHERE source = 'staff' AND phone_e164 IS NOT NULL;
