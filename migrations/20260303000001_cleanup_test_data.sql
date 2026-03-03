-- Cleanup test/junk data from catalog
-- FK tables referencing catalog_ingredients (all ON DELETE RESTRICT):
--   1. inventory_batches.catalog_ingredient_id
--   2. tenant_ingredients.catalog_ingredient_id
--   3. recipe_ingredients.catalog_ingredient_id
--   4. catalog_translations.ingredient_id (ON DELETE CASCADE — auto-cleaned)
--
-- Strategy: 
--   - For junk categories: delete all FK refs, then hard-delete ingredients, then categories
--   - For remaining junk by name: soft-delete (is_active=false) to preserve FK integrity
--   - Only hard-delete ingredients that have ZERO FK references

DO $$
DECLARE
    junk_cat_ids UUID[] := ARRAY[
        '5e562930-fd54-4e82-b303-a9f95bce822c'::UUID,
        'c03e20d6-9fc2-4c09-ab55-03e39cd865a1'::UUID,
        '75d83b9a-3442-486c-afc9-6e41f9d54d28'::UUID,
        'dcffd533-e17c-41b5-a1b6-3b72ffcaed67'::UUID,
        '0fff9786-6352-41e1-ac2b-ea7dfa99a4e0'::UUID,
        '87727a2d-1419-48dc-a18e-4f721a5eda8d'::UUID,
        '061d8060-9652-4628-86d1-67c9149f0918'::UUID,
        'dd60f586-3f2f-4207-99ad-8f1e1def0ef2'::UUID,
        'b19dcd6a-36ed-4aba-8dbf-7a4a359fae9d'::UUID,
        '008f77f1-1d70-4061-a068-c7ddff49c571'::UUID,
        '3b3cf500-b839-4092-9dc0-f64fc41c9415'::UUID,
        'bc2cd293-eb73-44e7-bf85-580a42e15133'::UUID
    ];
    ingredient_ids UUID[];
BEGIN
    -- Step 1: Collect ingredient IDs in junk categories
    SELECT ARRAY(
        SELECT id FROM catalog_ingredients WHERE category_id = ANY(junk_cat_ids)
    ) INTO ingredient_ids;

    -- Step 2: Clean ALL FK references to these ingredients
    IF ingredient_ids IS NOT NULL AND array_length(ingredient_ids, 1) > 0 THEN
        DELETE FROM inventory_batches WHERE catalog_ingredient_id = ANY(ingredient_ids);
        DELETE FROM tenant_ingredients WHERE catalog_ingredient_id = ANY(ingredient_ids);
        DELETE FROM recipe_ingredients WHERE catalog_ingredient_id = ANY(ingredient_ids);
        -- catalog_translations has ON DELETE CASCADE, auto-cleaned
    END IF;

    -- Step 3: Hard-delete ingredients in junk categories (FKs already cleared)
    DELETE FROM catalog_ingredients WHERE category_id = ANY(junk_cat_ids);

    -- Step 4: Hard-delete junk categories
    DELETE FROM catalog_categories WHERE id = ANY(junk_cat_ids);

    -- Step 5: Soft-delete remaining junk by name (safe, no FK violations)
    UPDATE catalog_ingredients
    SET is_active = false
    WHERE is_active = true
      AND (name_en LIKE 'Test %'
           OR name_en LIKE 'Expiring Fish%'
           OR name_en LIKE 'Low Milk%');

    -- Step 6: Hard-delete soft-deleted ingredients that have ZERO FK references
    DELETE FROM catalog_ingredients ci
    WHERE ci.is_active = false
      AND NOT EXISTS (SELECT 1 FROM inventory_batches ib WHERE ib.catalog_ingredient_id = ci.id)
      AND NOT EXISTS (SELECT 1 FROM tenant_ingredients ti WHERE ti.catalog_ingredient_id = ci.id)
      AND NOT EXISTS (SELECT 1 FROM recipe_ingredients ri WHERE ri.catalog_ingredient_id = ci.id);
END $$;
