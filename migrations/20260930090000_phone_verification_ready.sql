-- Phone numbers: a place to record that a number was confirmed by a code.
--
-- Sign-up already stores an optional E.164 number in users.phone_e164 (unique
-- among live accounts). Nothing sends or checks codes yet: when SMS
-- verification is added, it sets phone_verified_at and everything else stays.
ALTER TABLE users
    ADD COLUMN phone_verified_at timestamptz,
    ADD CONSTRAINT users_phone_verified_needs_phone
        CHECK (phone_verified_at IS NULL OR phone_e164 IS NOT NULL);
