-- Dry run for migrations/20260930100000_crm.sql. READ ONLY: changes nothing.
--
-- Run in the Neon SQL editor as the schema OWNER (the role that runs
-- migrations). Row-level security is FORCED on these tables, so a role
-- without BYPASSRLS would see zero rows: check the first block; if
-- "appointments_total" is 0 while the app has visits, the role is filtered.
--
-- Phones are masked (last 3 digits only).

BEGIN READ ONLY;

-- 0. Can we see the data at all?
SELECT (SELECT count(*) FROM appointment) AS appointments_total,
       (SELECT count(*) FROM client)      AS clients_total,
       (SELECT count(*) FROM client WHERE source = 'staff') AS staff_clients_total;

-- 1. Are stored phones already normalised E.164? The column has a CHECK; is it validated?
SELECT conname, convalidated, pg_get_constraintdef(oid) AS definition
FROM pg_constraint
WHERE conrelid = 'appointment'::regclass AND contype = 'c'
  AND pg_get_constraintdef(oid) LIKE '%client_phone%';

-- 1b. Rows whose phone is NOT already canonical E.164 (should be 0), and how many
--     the backfill's own normalisation (strip spaces, dashes, brackets) would rescue.
SELECT count(*) FILTER (WHERE client_phone IS NOT NULL
                         AND client_phone !~ '^\+[1-9][0-9]{6,14}$')                 AS not_canonical,
       count(*) FILTER (WHERE client_phone IS NOT NULL
                         AND client_phone !~ '^\+[1-9][0-9]{6,14}$'
                         AND regexp_replace(client_phone, '[ ()-]', '', 'g')
                             ~ '^\+[1-9][0-9]{6,14}$')                               AS rescued_by_normalising,
       count(*) FILTER (WHERE client_phone IS NOT NULL
                         AND regexp_replace(client_phone, '[ ()-]', '', 'g')
                             !~ '^\+[1-9][0-9]{6,14}$')                              AS unusable
FROM appointment;

-- 2. Possible duplicates among existing staff clients: (business, phone) seen twice.
--    These would make the unique index fail. Expect no rows.
SELECT business_id, '…' || right(phone_e164, 3) AS phone_tail, count(*) AS cards
FROM client
WHERE source = 'staff' AND phone_e164 IS NOT NULL
GROUP BY business_id, phone_e164
HAVING count(*) > 1;

-- 3. Manual appointments (no client card yet) that carry a phone, and how many are usable.
WITH manual AS (
    SELECT business_id, id, client_name,
           regexp_replace(client_phone, '[ ()-]', '', 'g') AS phone
    FROM appointment
    WHERE client_id IS NULL AND client_phone IS NOT NULL
)
SELECT count(*)                                                            AS manual_with_phone,
       count(*) FILTER (WHERE phone ~ '^\+[1-9][0-9]{6,14}$'
                          AND client_name IS NOT NULL)                     AS valid_and_usable,
       count(*) FILTER (WHERE phone !~ '^\+[1-9][0-9]{6,14}$')             AS invalid_phone,
       count(*) FILTER (WHERE client_name IS NULL)                         AS no_name
FROM manual;

-- 3b. Manual appointments WITHOUT a phone: they stay without a card (for information).
SELECT count(*) AS manual_without_phone
FROM appointment
WHERE client_id IS NULL AND client_phone IS NULL AND client_name IS NOT NULL;

-- 4. Cards the backfill would create, and appointments it would link.
WITH usable AS (
    SELECT business_id, id,
           regexp_replace(client_phone, '[ ()-]', '', 'g') AS phone
    FROM appointment
    WHERE client_id IS NULL AND client_phone IS NOT NULL AND client_name IS NOT NULL
), groups AS (
    SELECT business_id, phone, count(*) AS visits
    FROM usable
    WHERE phone ~ '^\+[1-9][0-9]{6,14}$'
    GROUP BY business_id, phone
)
SELECT count(*)                                                     AS groups_total,
       count(*) FILTER (WHERE NOT EXISTS (
            SELECT 1 FROM client c
            WHERE c.business_id = groups.business_id AND c.source = 'staff'
              AND c.phone_e164 = groups.phone))                     AS cards_to_create,
       count(*) FILTER (WHERE EXISTS (
            SELECT 1 FROM client c
            WHERE c.business_id = groups.business_id AND c.source = 'staff'
              AND c.phone_e164 = groups.phone))                     AS already_have_a_card,
       coalesce(sum(visits), 0)                                     AS appointments_to_link
FROM groups;

-- 5. Same phone written with different names (the latest visit's name wins). For information.
SELECT business_id, '…' || right(phone, 3) AS phone_tail,
       count(DISTINCT lower(btrim(client_name))) AS distinct_names, count(*) AS visits
FROM (SELECT business_id, client_name, regexp_replace(client_phone, '[ ()-]', '', 'g') AS phone
      FROM appointment
      WHERE client_id IS NULL AND client_phone IS NOT NULL AND client_name IS NOT NULL) t
GROUP BY business_id, phone
HAVING count(DISTINCT lower(btrim(client_name))) > 1
ORDER BY visits DESC
LIMIT 20;

-- 6. Per business (which tenants are touched).
SELECT b.name, count(*) AS manual_with_phone
FROM appointment a JOIN business b ON b.id = a.business_id
WHERE a.client_id IS NULL AND a.client_phone IS NOT NULL
GROUP BY b.name ORDER BY manual_with_phone DESC LIMIT 20;

ROLLBACK;
