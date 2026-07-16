-- Icon ordering: per-icon order settings, a small catalog of add-on options
-- (candle, stand, packaging, ...), and the orders customers submit from the
-- public icon page.

ALTER TABLE church_icons
    ADD COLUMN IF NOT EXISTS order_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS order_block_text TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS production_time TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS price_cents BIGINT,
    ADD COLUMN IF NOT EXISTS currency TEXT NOT NULL DEFAULT 'UAH',
    ADD COLUMN IF NOT EXISTS consecration_available BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE IF NOT EXISTS icon_order_options (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    slug TEXT NOT NULL,
    name_uk TEXT NOT NULL DEFAULT '',
    name_ru TEXT NOT NULL DEFAULT '',
    name_en TEXT NOT NULL DEFAULT '',
    photo_url TEXT NOT NULL DEFAULT '',
    price_cents BIGINT NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'UAH',
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT icon_order_options_site_slug_unique UNIQUE (site_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_icon_order_options_site_active
    ON icon_order_options(site_id, is_active, sort_order);

DROP TRIGGER IF EXISTS icon_order_options_set_updated_at ON icon_order_options;
CREATE TRIGGER icon_order_options_set_updated_at
    BEFORE UPDATE ON icon_order_options
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE SEQUENCE IF NOT EXISTS icon_order_number_seq START 1;

CREATE TABLE IF NOT EXISTS icon_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    is_global BOOLEAN NOT NULL DEFAULT false,
    order_number TEXT NOT NULL UNIQUE,
    icon_id UUID REFERENCES church_icons(id) ON DELETE SET NULL,
    icon_title_snapshot TEXT NOT NULL DEFAULT '',
    icon_slug_snapshot TEXT NOT NULL DEFAULT '',
    customer_name TEXT NOT NULL,
    contact_method TEXT NOT NULL,
    contact_value TEXT NOT NULL,
    preferred_contact_channel TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '',
    consecration_requested BOOLEAN NOT NULL DEFAULT false,
    comment TEXT NOT NULL DEFAULT '',
    consent_given BOOLEAN NOT NULL DEFAULT false,
    status TEXT NOT NULL DEFAULT 'new',
    admin_note TEXT NOT NULL DEFAULT '',
    total_price_cents BIGINT NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'UAH',
    is_read BOOLEAN NOT NULL DEFAULT false,
    client_ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT icon_orders_contact_method_check CHECK (contact_method IN ('phone', 'email')),
    CONSTRAINT icon_orders_status_check CHECK (status IN
        ('new', 'contacted', 'confirmed', 'in_production', 'ready', 'shipped', 'completed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_icon_orders_site_status
    ON icon_orders(site_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_icon_orders_site_unread
    ON icon_orders(site_id, is_read) WHERE is_read = false;

DROP TRIGGER IF EXISTS icon_orders_set_updated_at ON icon_orders;
CREATE TRIGGER icon_orders_set_updated_at
    BEFORE UPDATE ON icon_orders
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE IF NOT EXISTS icon_order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES icon_orders(id) ON DELETE CASCADE,
    option_id UUID REFERENCES icon_order_options(id) ON DELETE SET NULL,
    option_name_snapshot TEXT NOT NULL,
    price_cents_snapshot BIGINT NOT NULL DEFAULT 0,
    quantity INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_icon_order_items_order ON icon_order_items(order_id);
