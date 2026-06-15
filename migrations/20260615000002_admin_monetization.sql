CREATE TABLE IF NOT EXISTS admin_affiliate_products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site TEXT NOT NULL CHECK (site IN ('culinary', 'construction')),
    title JSONB NOT NULL DEFAULT '{}'::jsonb,
    slug TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT '',
    network TEXT NOT NULL DEFAULT 'custom',
    merchant TEXT NOT NULL DEFAULT '',
    affiliate_url TEXT NOT NULL DEFAULT '',
    image_url TEXT,
    detail_image_url TEXT,
    price NUMERIC,
    currency TEXT NOT NULL DEFAULT 'PLN',
    commission_percent NUMERIC,
    cookie_days INTEGER,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'published', 'archived')),
    languages TEXT[] NOT NULL DEFAULT '{}',
    seo_title JSONB,
    seo_description JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (site, slug)
);

CREATE TABLE IF NOT EXISTS admin_affiliate_offers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES admin_affiliate_products(id) ON DELETE CASCADE,
    network TEXT NOT NULL DEFAULT 'custom',
    merchant TEXT NOT NULL DEFAULT '',
    affiliate_url TEXT NOT NULL DEFAULT '',
    price NUMERIC,
    currency TEXT NOT NULL DEFAULT 'PLN',
    commission_percent NUMERIC,
    cookie_days INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_admin_affiliate_products_site
    ON admin_affiliate_products(site, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_admin_affiliate_offers_product
    ON admin_affiliate_offers(product_id);

CREATE TABLE IF NOT EXISTS admin_suppliers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    country TEXT NOT NULL DEFAULT '',
    city TEXT,
    categories TEXT[] NOT NULL DEFAULT '{}',
    contact TEXT NOT NULL DEFAULT '',
    website TEXT,
    commission_terms TEXT,
    supplier_type TEXT NOT NULL DEFAULT 'local_supplier'
        CHECK (supplier_type IN ('marketplace', 'local_supplier', 'manufacturer', 'affiliate_merchant')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS admin_construction_materials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title JSONB NOT NULL DEFAULT '{}'::jsonb,
    slug TEXT UNIQUE NOT NULL,
    category TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT 'Алматы',
    supplier_ids UUID[] NOT NULL DEFAULT '{}',
    unit TEXT NOT NULL DEFAULT 'm2' CHECK (unit IN ('m2', 'm3', 'piece', 'kg', 'bag', 'hour')),
    material_price NUMERIC,
    work_price NUMERIC,
    currency TEXT NOT NULL DEFAULT 'KZT',
    margin_percent NUMERIC,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'published', 'archived')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS admin_construction_bundles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title JSONB NOT NULL DEFAULT '{}'::jsonb,
    slug TEXT UNIQUE NOT NULL,
    city TEXT NOT NULL DEFAULT 'Алматы',
    materials TEXT[] NOT NULL DEFAULT '{}',
    works TEXT[] NOT NULL DEFAULT '{}',
    area_m2 NUMERIC,
    material_cost NUMERIC,
    work_cost NUMERIC,
    total_price NUMERIC,
    currency TEXT NOT NULL DEFAULT 'KZT',
    supplier_ids UUID[] NOT NULL DEFAULT '{}',
    lead_form_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'published', 'archived')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE site_leads
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'new'
        CHECK (status IN ('new', 'contacted', 'quoted', 'won', 'lost')),
    ADD COLUMN IF NOT EXISTS potential_value NUMERIC,
    ADD COLUMN IF NOT EXISTS currency TEXT NOT NULL DEFAULT 'KZT',
    ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS city TEXT,
    ADD COLUMN IF NOT EXISTS contact TEXT;

INSERT INTO admin_suppliers (name, country, city, categories, contact, website, commission_terms, supplier_type)
VALUES
    ('Almaty Kerama', 'Kazakhstan', 'Алматы', ARRAY['Плитка', 'Керамогранит'], '+7 700 000 00 01', 'https://supplier.example', '8% от подтвержденной заявки', 'local_supplier'),
    ('Kitchen Pro EU', 'Germany', 'Berlin', ARRAY['Оборудование', 'Миксеры'], 'partners@kitchen.example', 'https://kitchen.example', 'Awin CPA 6%', 'affiliate_merchant'),
    ('Allegro', 'Poland', NULL, ARRAY['Кухонные товары'], 'affiliate@allegro.example', 'https://allegro.pl', 'marketplace rate', 'marketplace')
ON CONFLICT DO NOTHING;

INSERT INTO admin_affiliate_products (
    site, title, slug, category, network, merchant, affiliate_url, image_url, detail_image_url,
    price, currency, commission_percent, cookie_days, status, languages, seo_title, seo_description
)
VALUES
    ('culinary', '{"ru":"Поварской нож 20 см","pl":"Noz szefa 20 cm","en":"Chef knife 20 cm","kk":"Аспаз пышагы 20 см"}', 'chef-knife-20cm', 'Профессиональные инструменты', 'allegro', 'Allegro Smart', 'https://allegro.example/noz', 'https://images.unsplash.com/photo-1593618998160-e34014e67546?auto=format&fit=crop&w=640&q=80', 'https://images.unsplash.com/photo-1593618998160-e34014e67546?auto=format&fit=crop&w=1200&q=80', 349, 'PLN', 5, 7, 'published', ARRAY['ru','pl','en'], '{"ru":"Лучший поварской нож для дома и ресторана","pl":"Najlepszy noz szefa do domu i restauracji","en":"Best chef knife for home and restaurant","kk":"Уйге және мейрамханага аспаз пышагы"}', '{"ru":"Обзор ножа, плюсы, цена и affiliate-предложения.","pl":"Recenzja noza, zalety, cena i oferty partnerskie.","en":"Chef knife review with price and affiliate offers.","kk":"Пышак шолуы, бага және сериктестик усыныстар."}'),
    ('culinary', '{"ru":"Планетарный миксер","pl":"Mikser planetarny","en":"Stand mixer","kk":"Планетарлык миксер"}', 'stand-mixer-pro', 'Ресторанное оборудование', 'awin', 'Kitchen Pro', 'https://awin.example/mixer', NULL, NULL, 129, 'EUR', 6, 30, 'draft', ARRAY['ru','pl','en'], NULL, NULL),
    ('construction', '{"ru":"Керамогранит для ванной","pl":"Gres do lazienki","en":"Bathroom porcelain tiles","kk":"Жуынатын бөлмеге керамогранит"}', 'bathroom-porcelain-tiles-almaty', 'Отделочные материалы', 'custom', 'Almaty Kerama', 'https://supplier.example/tiles', NULL, NULL, 6900, 'KZT', 8, 30, 'active', ARRAY['ru','kk','en'], NULL, NULL)
ON CONFLICT (site, slug) DO NOTHING;

INSERT INTO admin_affiliate_offers (product_id, network, merchant, affiliate_url, price, currency, commission_percent, cookie_days, is_active)
SELECT id, network, merchant, affiliate_url, price, currency, commission_percent, cookie_days, TRUE
FROM admin_affiliate_products
ON CONFLICT DO NOTHING;

INSERT INTO admin_construction_materials (title, slug, category, city, unit, material_price, work_price, currency, margin_percent, status)
VALUES
    ('{"ru":"Керамогранит 60x60","pl":"Gres 60x60","en":"Porcelain tile 60x60","kk":"Керамогранит 60x60"}', 'porcelain-60x60', 'Плитка', 'Алматы', 'm2', 6900, 4500, 'KZT', 18, 'active'),
    ('{"ru":"Гидроизоляция санузла","pl":"Hydroizolacja lazienki","en":"Bathroom waterproofing","kk":"Санузел гидроизоляциясы"}', 'bathroom-waterproofing', 'Работы', 'Алматы', 'm2', 1200, 2800, 'KZT', 22, 'published')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO admin_construction_bundles (
    title, slug, city, materials, works, area_m2, material_cost, work_cost, total_price,
    currency, lead_form_enabled, status
)
VALUES
    ('{"ru":"Ванная под ключ","pl":"Lazienka pod klucz","en":"Turnkey bathroom","kk":"Дайын ванна"}', 'turnkey-bathroom-almaty', 'Алматы', ARRAY['Керамогранит', 'Клей', 'Гидроизоляция'], ARRAY['Демонтаж', 'Укладка', 'Затирка'], 42, 690000, 540000, 1451400, 'KZT', TRUE, 'active')
ON CONFLICT (slug) DO NOTHING;
