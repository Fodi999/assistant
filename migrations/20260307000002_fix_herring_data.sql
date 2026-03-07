-- Migration: fix herring data — correct translations, add to Fish & Seafood, fill missing fields
UPDATE catalog_ingredients SET
    name_pl         = 'Śledź',
    name_uk         = 'Оселедець',
    description_en  = 'Herring is a small, oily saltwater fish rich in omega-3 fatty acids, widely used in Northern and Eastern European cuisine.',
    description_ru  = 'Сельдь — небольшая жирная морская рыба, богатая омега-3 жирными кислотами, широко используемая в кухнях Северной и Восточной Европы.',
    description_pl  = 'Śledź to mała, tłusta ryba morska bogata w kwasy tłuszczowe omega-3, szeroko stosowana w kuchni północno- i wschodnioeuropejskiej.',
    description_uk  = 'Оселедець — невелика жирна морська риба, багата на омега-3 жирні кислоти, широко використовувана в кухнях Північної та Східної Європи.',
    density_g_per_ml = 1.05,
    seasons         = ARRAY['Autumn', 'Winter']::season_type[],
    allergens       = ARRAY['Fish']::allergen_type[],
    category_id     = (SELECT id FROM catalog_categories WHERE name_en = 'Fish & Seafood')
WHERE slug = 'herring';
