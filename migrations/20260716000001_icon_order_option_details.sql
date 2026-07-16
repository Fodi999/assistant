-- Adds short description and category fields to icon order add-on options,
-- so the admin form can show a bit more merchandising detail per item.

ALTER TABLE icon_order_options
    ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT '';
