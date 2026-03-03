-- Cleanup test/junk data from catalog
-- Removes soft-deleted products in test categories, then removes test categories

-- Step 1: Hard-delete any soft-deleted ingredients in junk test categories
DELETE FROM catalog_ingredients
WHERE category_id IN (
    '5e562930-fd54-4e82-b303-a9f95bce822c',
    'c03e20d6-9fc2-4c09-ab55-03e39cd865a1',
    '75d83b9a-3442-486c-afc9-6e41f9d54d28',
    'dcffd533-e17c-41b5-a1b6-3b72ffcaed67',
    '0fff9786-6352-41e1-ac2b-ea7dfa99a4e0',
    '87727a2d-1419-48dc-a18e-4f721a5eda8d',
    '061d8060-9652-4628-86d1-67c9149f0918',
    'dd60f586-3f2f-4207-99ad-8f1e1def0ef2',
    'b19dcd6a-36ed-4aba-8dbf-7a4a359fae9d',
    '008f77f1-1d70-4061-a068-c7ddff49c571',
    '3b3cf500-b839-4092-9dc0-f64fc41c9415',
    'bc2cd293-eb73-44e7-bf85-580a42e15133'
);

-- Step 2: Delete junk test/alert categories
DELETE FROM catalog_categories
WHERE id IN (
    '5e562930-fd54-4e82-b303-a9f95bce822c',
    'c03e20d6-9fc2-4c09-ab55-03e39cd865a1',
    '75d83b9a-3442-486c-afc9-6e41f9d54d28',
    'dcffd533-e17c-41b5-a1b6-3b72ffcaed67',
    '0fff9786-6352-41e1-ac2b-ea7dfa99a4e0',
    '87727a2d-1419-48dc-a18e-4f721a5eda8d',
    '061d8060-9652-4628-86d1-67c9149f0918',
    'dd60f586-3f2f-4207-99ad-8f1e1def0ef2',
    'b19dcd6a-36ed-4aba-8dbf-7a4a359fae9d',
    '008f77f1-1d70-4061-a068-c7ddff49c571',
    '3b3cf500-b839-4092-9dc0-f64fc41c9415',
    'bc2cd293-eb73-44e7-bf85-580a42e15133'
);

-- Step 3: Hard-delete soft-deleted junk ingredients (Test, Expiring Fish, Low Milk, etc.)
DELETE FROM catalog_ingredients
WHERE deleted = true
   OR name_en LIKE 'Test %'
   OR name_en LIKE 'Expiring Fish%'
   OR name_en LIKE 'Low Milk%'
   OR name_en LIKE '%_964'
   OR name_en LIKE '%_133';
