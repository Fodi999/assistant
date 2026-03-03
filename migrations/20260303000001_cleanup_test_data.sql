-- Cleanup test/junk data from catalog
-- FK chain: inventory_batches (was inventory_products) → catalog_ingredients → catalog_categories

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

    -- Step 2: Delete from inventory_batches (the only FK referencing catalog_ingredients)
    -- Note: inventory_products was RENAMED to inventory_batches in migration 20260216000003
    IF ingredient_ids IS NOT NULL AND array_length(ingredient_ids, 1) > 0 THEN
        DELETE FROM inventory_batches WHERE catalog_ingredient_id = ANY(ingredient_ids);
    END IF;

    -- Step 3: Delete ingredients in junk categories
    DELETE FROM catalog_ingredients WHERE category_id = ANY(junk_cat_ids);

    -- Step 4: Delete junk categories
    DELETE FROM catalog_categories WHERE id = ANY(junk_cat_ids);

    -- Step 5: Clean up remaining junk by name
    -- Note: soft-delete column is "is_active" (false = deleted), NOT "deleted"
    DELETE FROM inventory_batches WHERE catalog_ingredient_id IN (
        SELECT id FROM catalog_ingredients
        WHERE is_active = false OR name_en LIKE 'Test %'
           OR name_en LIKE 'Expiring Fish%' OR name_en LIKE 'Low Milk%'
    );
    DELETE FROM catalog_ingredients
    WHERE is_active = false OR name_en LIKE 'Test %'
       OR name_en LIKE 'Expiring Fish%' OR name_en LIKE 'Low Milk%';
END $$;
