-- Create catalog categories table
CREATE TABLE catalog_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Multilingual names
    name_pl TEXT NOT NULL,
    name_en TEXT NOT NULL,
    name_uk TEXT NOT NULL,
    name_ru TEXT NOT NULL,
    
    -- Sort order for display
    sort_order INTEGER NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert common restaurant categories
INSERT INTO catalog_categories (name_pl, name_en, name_uk, name_ru, sort_order) VALUES
('Nabiał i jaja', 'Dairy & Eggs', 'Молочні продукти та яйця', 'Молочные продукты и яйця', 1),
('Mięso i drób', 'Meat & Poultry', 'М''ясо та птиця', 'Мясо и птица', 2),
('Ryby i owoce morza', 'Fish & Seafood', 'Риба та морепродукти', 'Рыба и морепродукты', 3),
('Warzywa', 'Vegetables', 'Овочі', 'Овощи', 4),
('Owoce', 'Fruits', 'Фрукти', 'Фрукты', 5),
('Zboża i makarony', 'Grains & Pasta', 'Крупи та макарони', 'Крупы и макароны', 6),
('Oleje i tłuszcze', 'Oils & Fats', 'Олії та жири', 'Масла и жиры', 7),
('Przyprawy i zioła', 'Spices & Herbs', 'Спеції та трави', 'Специи и травы', 8),
('Sosy i przyprawy', 'Condiments & Sauces', 'Соуси та приправи', 'Соусы и приправы', 9),
('Napoje', 'Beverages', 'Напої', 'Напитки', 10),
('Orzechy i nasiona', 'Nuts & Seeds', 'Горіхи та насіння', 'Орехи и семена', 11),
('Rośliny strączkowe', 'Legumes', 'Бобові', 'Бобовые', 12),
('Słodycze i pieczywo', 'Sweets & Baking', 'Солодощі та випічка', 'Сладости и выпечка', 13),
('Konserwy', 'Canned & Preserved', 'Консерви', 'Консервы', 14),
('Mrożonki', 'Frozen', 'Заморожені продукти', 'Замороженные продукты', 15);

-- Add category_id to catalog_ingredients (nullable first, then we'll make it NOT NULL after updates)
ALTER TABLE catalog_ingredients
ADD COLUMN category_id UUID REFERENCES catalog_categories(id);

-- Create index for category filtering
CREATE INDEX idx_catalog_ingredients_category ON catalog_ingredients(category_id);

-- Update existing ingredients with category_id (using subqueries to find category by name)
-- Dairy & Eggs
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Dairy & Eggs')
WHERE name_en IN ('Pasteurized milk', 'Cream 18%', 'Butter', 'Chicken eggs', 'Hard cheese', 'Mozzarella cheese', 'Plain yogurt', 'Cottage cheese');

-- Meat & Poultry
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Meat & Poultry')
WHERE name_en IN ('Chicken breast', 'Chicken thighs', 'Beef', 'Pork', 'Ground meat', 'Ham', 'Bacon', 'Sausage');

-- Fish & Seafood
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Fish & Seafood')
WHERE name_en IN ('Salmon', 'Cod', 'Shrimp', 'Canned tuna');

-- Vegetables
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Vegetables')
WHERE name_en IN ('Tomato', 'Cucumber', 'Onion', 'Garlic', 'Carrot', 'Potato', 'Bell pepper', 'Lettuce', 'Broccoli', 'Cauliflower', 'Cabbage', 'Spinach', 'Zucchini', 'Eggplant', 'Green peas', 'Corn', 'Button mushroom', 'Porcini mushroom');

-- Fruits
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Fruits')
WHERE name_en IN ('Apple', 'Banana', 'Orange', 'Lemon', 'Strawberry', 'Raspberry', 'Blueberry', 'Grape', 'Watermelon', 'Avocado');

-- Grains & Pasta
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Grains & Pasta')
WHERE name_en IN ('Wheat flour', 'Rice', 'Pasta', 'Buckwheat', 'Oatmeal', 'Bread', 'Breadcrumbs');

-- Oils & Fats
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Oils & Fats')
WHERE name_en IN ('Rapeseed oil', 'Olive oil', 'Sunflower oil');

-- Spices & Herbs
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Spices & Herbs')
WHERE name_en IN ('Salt', 'Black pepper', 'Sweet paprika', 'Basil', 'Oregano', 'Thyme', 'Rosemary', 'Dill', 'Parsley', 'Cinnamon', 'Ginger', 'Turmeric');

-- Condiments & Sauces
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Condiments & Sauces')
WHERE name_en IN ('Ketchup', 'Mustard', 'Mayonnaise', 'Soy sauce', 'Vinegar', 'Honey');

-- Beverages
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Beverages')
WHERE name_en IN ('Mineral water', 'Orange juice', 'White wine', 'Red wine', 'Beer');

-- Nuts & Seeds
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Nuts & Seeds')
WHERE name_en IN ('Walnuts', 'Almonds', 'Hazelnuts', 'Sunflower seeds', 'Sesame seeds');

-- Legumes
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Legumes')
WHERE name_en IN ('Beans', 'Lentils', 'Chickpeas');

-- Sweets & Baking
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Sweets & Baking')
WHERE name_en IN ('Sugar', 'Chocolate', 'Cocoa', 'Baking powder', 'Vanilla');

-- Canned & Preserved
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Canned & Preserved')
WHERE name_en IN ('Canned tomatoes', 'Pickles', 'Olives');

-- Frozen
UPDATE catalog_ingredients SET category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Frozen')
WHERE name_en IN ('Frozen vegetables', 'Vanilla ice cream');

-- Now make category_id NOT NULL after all updates
ALTER TABLE catalog_ingredients
ALTER COLUMN category_id SET NOT NULL;

COMMENT ON TABLE catalog_categories IS 'Product categories for organizing catalog ingredients';
COMMENT ON COLUMN catalog_categories.sort_order IS 'Display order for categories in UI';
