-- What the business does and where it is, for the owner's profile.
--
-- business_type is a fixed list (the app shows it as a picker); address_line
-- is one free-text line. No coordinates or geocoding: that is a later stage.
ALTER TABLE business
    ADD COLUMN business_type text CHECK (
        business_type IS NULL OR business_type IN
            ('lashes', 'brows', 'nails', 'hair', 'beauty_studio', 'other')
    ),
    ADD COLUMN address_line text CHECK (
        address_line IS NULL OR length(btrim(address_line)) BETWEEN 1 AND 200
    );
