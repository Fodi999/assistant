```sql
-- Add image_url to dishes table

-- +goose Up
-- +goose StatementBegin
ALTER TABLE dishes
ADD COLUMN IF NOT EXISTS image_url TEXT;

COMMENT ON COLUMN dishes.image_url IS 'URL of the dish image (optional)';
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
ALTER TABLE dishes
DROP COLUMN IF EXISTS image_url;
-- +goose StatementEnd
```
