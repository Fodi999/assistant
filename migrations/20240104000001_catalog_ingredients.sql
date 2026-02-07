-- Enable trigram extension for fuzzy search (must be first)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create ENUM types for type safety
CREATE TYPE unit_type AS ENUM (
    'gram',
    'kilogram',
    'liter',
    'milliliter',
    'piece',
    'bunch',
    'can',
    'bottle',
    'package'
);

CREATE TYPE allergen_type AS ENUM (
    'Milk',
    'Eggs',
    'Fish',
    'Shellfish',
    'TreeNuts',
    'Peanuts',
    'Wheat',
    'Soybeans',
    'Sesame',
    'Celery',
    'Mustard',
    'Sulfites',
    'Lupin',
    'Molluscs'
);

CREATE TYPE season_type AS ENUM (
    'Spring',
    'Summer',
    'Autumn',
    'Winter',
    'AllYear'
);

-- Create catalog_ingredients table
CREATE TABLE catalog_ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Multilingual names
    name_pl TEXT NOT NULL,
    name_en TEXT NOT NULL,
    name_uk TEXT NOT NULL,
    name_ru TEXT NOT NULL,
    
    -- Core properties with ENUM types
    default_unit unit_type NOT NULL,
    default_shelf_life_days INTEGER CHECK (default_shelf_life_days > 0),
    
    -- Metadata with ENUM arrays
    allergens allergen_type[] NOT NULL DEFAULT '{}',
    calories_per_100g INTEGER CHECK (calories_per_100g >= 0),
    seasons season_type[] NOT NULL DEFAULT '{}',
    
    -- UX
    image_url TEXT,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create function for auto-updating updated_at
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for auto-updating updated_at
CREATE TRIGGER trg_catalog_ingredients_updated
BEFORE UPDATE ON catalog_ingredients
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- Create indexes for fast search by language
CREATE INDEX idx_catalog_ingredients_name_pl ON catalog_ingredients USING gin (name_pl gin_trgm_ops);
CREATE INDEX idx_catalog_ingredients_name_en ON catalog_ingredients USING gin (name_en gin_trgm_ops);
CREATE INDEX idx_catalog_ingredients_name_uk ON catalog_ingredients USING gin (name_uk gin_trgm_ops);
CREATE INDEX idx_catalog_ingredients_name_ru ON catalog_ingredients USING gin (name_ru gin_trgm_ops);

-- Insert 100 common restaurant ingredients with multilingual names
INSERT INTO catalog_ingredients (name_pl, name_en, name_uk, name_ru, default_unit, default_shelf_life_days, allergens, calories_per_100g, seasons) VALUES

-- Dairy & Eggs
('Mleko pasteryzowane', 'Pasteurized milk', 'Пастеризоване молоко', 'Пастеризованное молоко', 'liter', 7, ARRAY['Milk']::allergen_type[], 42, ARRAY['AllYear']::season_type[]),
('Śmietana 18%', 'Cream 18%', 'Вершки 18%', 'Сливки 18%', 'liter', 14, ARRAY['Milk']::allergen_type[], 180, ARRAY['AllYear']::season_type[]),
('Masło', 'Butter', 'Масло вершкове', 'Сливочное масло', 'kilogram', 30, ARRAY['Milk']::allergen_type[], 717, ARRAY['AllYear']::season_type[]),
('Jaja kurze', 'Chicken eggs', 'Курячі яйця', 'Куриные яйца', 'piece', 21, ARRAY['Eggs']::allergen_type[], 155, ARRAY['AllYear']::season_type[]),
('Ser żółty', 'Hard cheese', 'Твердий сир', 'Твёрдый сыр', 'kilogram', 60, ARRAY['Milk']::allergen_type[], 402, ARRAY['AllYear']::season_type[]),
('Ser mozzarella', 'Mozzarella cheese', 'Сир моцарелла', 'Сыр моцарелла', 'kilogram', 14, ARRAY['Milk']::allergen_type[], 280, ARRAY['AllYear']::season_type[]),
('Jogurt naturalny', 'Plain yogurt', 'Натуральний йогурт', 'Натуральный йогурт', 'kilogram', 21, ARRAY['Milk']::allergen_type[], 59, ARRAY['AllYear']::season_type[]),
('Ser twarogowy', 'Cottage cheese', 'Сир кисломолочний', 'Творог', 'kilogram', 14, ARRAY['Milk']::allergen_type[], 98, ARRAY['AllYear']::season_type[]),

-- Meat & Poultry
('Pierś z kurczaka', 'Chicken breast', 'Куряче філе', 'Куриное филе', 'kilogram', 3, ARRAY[]::allergen_type[], 165, ARRAY['AllYear']::season_type[]),
('Udka z kurczaka', 'Chicken thighs', 'Курячі стегна', 'Куриные бёдра', 'kilogram', 3, ARRAY[]::allergen_type[], 209, ARRAY['AllYear']::season_type[]),
('Wołowina', 'Beef', 'Яловичина', 'Говядина', 'kilogram', 5, ARRAY[]::allergen_type[], 250, ARRAY['AllYear']::season_type[]),
('Wieprzowina', 'Pork', 'Свинина', 'Свинина', 'kilogram', 5, ARRAY[]::allergen_type[], 242, ARRAY['AllYear']::season_type[]),
('Mięso mielone', 'Ground meat', 'Фарш', 'Фарш', 'kilogram', 2, ARRAY[]::allergen_type[], 332, ARRAY['AllYear']::season_type[]),
('Szynka', 'Ham', 'Шинка', 'Ветчина', 'kilogram', 14, ARRAY[]::allergen_type[], 145, ARRAY['AllYear']::season_type[]),
('Boczek', 'Bacon', 'Бекон', 'Бекон', 'kilogram', 30, ARRAY[]::allergen_type[], 541, ARRAY['AllYear']::season_type[]),
('Kiełbasa', 'Sausage', 'Ковбаса', 'Колбаса', 'kilogram', 21, ARRAY[]::allergen_type[], 301, ARRAY['AllYear']::season_type[]),

-- Fish & Seafood
('Łosoś', 'Salmon', 'Лосось', 'Лосось', 'kilogram', 3, ARRAY['Fish']::allergen_type[], 208, ARRAY['AllYear']::season_type[]),
('Dorsz', 'Cod', 'Тріска', 'Треска', 'kilogram', 3, ARRAY['Fish']::allergen_type[], 82, ARRAY['AllYear']::season_type[]),
('Krewetki', 'Shrimp', 'Креветки', 'Креветки', 'kilogram', 2, ARRAY['Shellfish']::allergen_type[], 99, ARRAY['AllYear']::season_type[]),
('Tuńczyk w puszce', 'Canned tuna', 'Тунець консервований', 'Тунец консервированный', 'can', 730, ARRAY['Fish']::allergen_type[], 116, ARRAY['AllYear']::season_type[]),

-- Vegetables
('Pomidor', 'Tomato', 'Помідор', 'Помидор', 'kilogram', 7, ARRAY[]::allergen_type[], 18, ARRAY['Summer', 'Autumn']::season_type[]),
('Ogórek', 'Cucumber', 'Огірок', 'Огурец', 'kilogram', 7, ARRAY[]::allergen_type[], 15, ARRAY['Summer']::season_type[]),
('Cebula', 'Onion', 'Цибуля', 'Лук', 'kilogram', 30, ARRAY[]::allergen_type[], 40, ARRAY['AllYear']::season_type[]),
('Czosnek', 'Garlic', 'Часник', 'Чеснок', 'kilogram', 90, ARRAY[]::allergen_type[], 149, ARRAY['AllYear']::season_type[]),
('Marchew', 'Carrot', 'Морква', 'Морковь', 'kilogram', 21, ARRAY[]::allergen_type[], 41, ARRAY['AllYear']::season_type[]),
('Ziemniak', 'Potato', 'Картопля', 'Картофель', 'kilogram', 30, ARRAY[]::allergen_type[], 77, ARRAY['AllYear']::season_type[]),
('Papryka', 'Bell pepper', 'Перець болгарський', 'Болгарский перец', 'kilogram', 7, ARRAY[]::allergen_type[], 31, ARRAY['Summer', 'Autumn']::season_type[]),
('Sałata', 'Lettuce', 'Салат', 'Салат', 'piece', 5, ARRAY[]::allergen_type[], 15, ARRAY['Spring', 'Summer']::season_type[]),
('Brokuł', 'Broccoli', 'Брокколі', 'Брокколи', 'kilogram', 7, ARRAY[]::allergen_type[], 34, ARRAY['AllYear']::season_type[]),
('Kalafior', 'Cauliflower', 'Цвітна капуста', 'Цветная капуста', 'piece', 7, ARRAY[]::allergen_type[], 25, ARRAY['AllYear']::season_type[]),
('Kapusta', 'Cabbage', 'Капуста', 'Капуста', 'piece', 14, ARRAY[]::allergen_type[], 25, ARRAY['Autumn', 'Winter']::season_type[]),
('Szpinak', 'Spinach', 'Шпинат', 'Шпинат', 'kilogram', 5, ARRAY[]::allergen_type[], 23, ARRAY['Spring', 'Autumn']::season_type[]),
('Cukinia', 'Zucchini', 'Кабачок', 'Кабачок', 'kilogram', 7, ARRAY[]::allergen_type[], 17, ARRAY['Summer']::season_type[]),
('Bakłażan', 'Eggplant', 'Баклажан', 'Баклажан', 'kilogram', 7, ARRAY[]::allergen_type[], 25, ARRAY['Summer']::season_type[]),
('Groszek zielony', 'Green peas', 'Зелений горошок', 'Зелёный горошек', 'kilogram', 3, ARRAY[]::allergen_type[], 81, ARRAY['Spring', 'Summer']::season_type[]),
('Kukurydza', 'Corn', 'Кукурудза', 'Кукуруза', 'piece', 3, ARRAY[]::allergen_type[], 86, ARRAY['Summer', 'Autumn']::season_type[]),
('Pieczarka', 'Button mushroom', 'Печериця', 'Шампиньон', 'kilogram', 5, ARRAY[]::allergen_type[], 22, ARRAY['AllYear']::season_type[]),
('Borowik', 'Porcini mushroom', 'Білий гриб', 'Белый гриб', 'kilogram', 3, ARRAY[]::allergen_type[], 22, ARRAY['Autumn']::season_type[]),

-- Fruits
('Jabłko', 'Apple', 'Яблуко', 'Яблоко', 'kilogram', 30, ARRAY[]::allergen_type[], 52, ARRAY['Autumn', 'Winter']::season_type[]),
('Banan', 'Banana', 'Банан', 'Банан', 'kilogram', 7, ARRAY[]::allergen_type[], 89, ARRAY['AllYear']::season_type[]),
('Pomarańcza', 'Orange', 'Апельсин', 'Апельсин', 'kilogram', 14, ARRAY[]::allergen_type[], 47, ARRAY['Winter']::season_type[]),
('Cytryna', 'Lemon', 'Лимон', 'Лимон', 'kilogram', 21, ARRAY[]::allergen_type[], 29, ARRAY['AllYear']::season_type[]),
('Truskawka', 'Strawberry', 'Полуниця', 'Клубника', 'kilogram', 3, ARRAY[]::allergen_type[], 32, ARRAY['Spring', 'Summer']::season_type[]),
('Malina', 'Raspberry', 'Малина', 'Малина', 'kilogram', 2, ARRAY[]::allergen_type[], 52, ARRAY['Summer']::season_type[]),
('Borówka', 'Blueberry', 'Чорниця', 'Черника', 'kilogram', 7, ARRAY[]::allergen_type[], 57, ARRAY['Summer']::season_type[]),
('Winogrono', 'Grape', 'Виноград', 'Виноград', 'kilogram', 7, ARRAY[]::allergen_type[], 69, ARRAY['Autumn']::season_type[]),
('Arbuz', 'Watermelon', 'Кавун', 'Арбуз', 'piece', 14, ARRAY[]::allergen_type[], 30, ARRAY['Summer']::season_type[]),
('Awokado', 'Avocado', 'Авокадо', 'Авокадо', 'piece', 5, ARRAY[]::allergen_type[], 160, ARRAY['AllYear']::season_type[]),

-- Grains & Pasta
('Mąka pszenna', 'Wheat flour', 'Пшеничне борошно', 'Пшеничная мука', 'kilogram', 365, ARRAY['Wheat']::allergen_type[], 364, ARRAY['AllYear']::season_type[]),
('Ryż', 'Rice', 'Рис', 'Рис', 'kilogram', 730, ARRAY[]::allergen_type[], 130, ARRAY['AllYear']::season_type[]),
('Makaron', 'Pasta', 'Макарони', 'Макароны', 'kilogram', 730, ARRAY['Wheat']::allergen_type[], 371, ARRAY['AllYear']::season_type[]),
('Kasza gryczana', 'Buckwheat', 'Гречка', 'Гречка', 'kilogram', 365, ARRAY[]::allergen_type[], 343, ARRAY['AllYear']::season_type[]),
('Płatki owsiane', 'Oatmeal', 'Вівсянка', 'Овсянка', 'kilogram', 365, ARRAY[]::allergen_type[], 389, ARRAY['AllYear']::season_type[]),
('Chleb', 'Bread', 'Хліб', 'Хлеб', 'piece', 3, ARRAY['Wheat']::allergen_type[], 265, ARRAY['AllYear']::season_type[]),
('Bułka tarta', 'Breadcrumbs', 'Панірувальні сухарі', 'Панировочные сухари', 'kilogram', 180, ARRAY['Wheat']::allergen_type[], 395, ARRAY['AllYear']::season_type[]),

-- Oils & Fats
('Olej rzepakowy', 'Rapeseed oil', 'Ріпакова олія', 'Рапсовое масло', 'liter', 365, ARRAY[]::allergen_type[], 884, ARRAY['AllYear']::season_type[]),
('Oliwa z oliwek', 'Olive oil', 'Оливкова олія', 'Оливковое масло', 'liter', 730, ARRAY[]::allergen_type[], 884, ARRAY['AllYear']::season_type[]),
('Olej słonecznikowy', 'Sunflower oil', 'Соняшникова олія', 'Подсолнечное масло', 'liter', 365, ARRAY[]::allergen_type[], 884, ARRAY['AllYear']::season_type[]),

-- Spices & Herbs
('Sól', 'Salt', 'Сіль', 'Соль', 'kilogram', 3650, ARRAY[]::allergen_type[], 0, ARRAY['AllYear']::season_type[]),
('Pieprz czarny', 'Black pepper', 'Чорний перець', 'Чёрный перец', 'kilogram', 730, ARRAY[]::allergen_type[], 251, ARRAY['AllYear']::season_type[]),
('Papryka słodka', 'Sweet paprika', 'Солодка паприка', 'Сладкая паприка', 'kilogram', 365, ARRAY[]::allergen_type[], 282, ARRAY['AllYear']::season_type[]),
('Bazylia', 'Basil', 'Базилік', 'Базилик', 'bunch', 7, ARRAY[]::allergen_type[], 23, ARRAY['Summer']::season_type[]),
('Oregano', 'Oregano', 'Орегано', 'Орегано', 'bunch', 7, ARRAY[]::allergen_type[], 265, ARRAY['Summer']::season_type[]),
('Tymianek', 'Thyme', 'Чебрець', 'Тимьян', 'bunch', 7, ARRAY[]::allergen_type[], 101, ARRAY['AllYear']::season_type[]),
('Rozmaryn', 'Rosemary', 'Розмарин', 'Розмарин', 'bunch', 14, ARRAY[]::allergen_type[], 131, ARRAY['AllYear']::season_type[]),
('Koperek', 'Dill', 'Кріп', 'Укроп', 'bunch', 7, ARRAY[]::allergen_type[], 43, ARRAY['Summer']::season_type[]),
('Pietruszka', 'Parsley', 'Петрушка', 'Петрушка', 'bunch', 7, ARRAY[]::allergen_type[], 36, ARRAY['AllYear']::season_type[]),
('Cynamon', 'Cinnamon', 'Кориця', 'Корица', 'kilogram', 730, ARRAY[]::allergen_type[], 247, ARRAY['AllYear']::season_type[]),
('Imbir', 'Ginger', 'Імбир', 'Имбирь', 'kilogram', 30, ARRAY[]::allergen_type[], 80, ARRAY['AllYear']::season_type[]),
('Kurkuma', 'Turmeric', 'Куркума', 'Куркума', 'kilogram', 730, ARRAY[]::allergen_type[], 354, ARRAY['AllYear']::season_type[]),

-- Condiments & Sauces
('Ketchup', 'Ketchup', 'Кетчуп', 'Кетчуп', 'bottle', 365, ARRAY[]::allergen_type[], 112, ARRAY['AllYear']::season_type[]),
('Musztarda', 'Mustard', 'Гірчиця', 'Горчица', 'bottle', 365, ARRAY['Mustard']::allergen_type[], 66, ARRAY['AllYear']::season_type[]),
('Majonez', 'Mayonnaise', 'Майонез', 'Майонез', 'bottle', 180, ARRAY['Eggs']::allergen_type[], 680, ARRAY['AllYear']::season_type[]),
('Sos sojowy', 'Soy sauce', 'Соєвий соус', 'Соевый соус', 'bottle', 730, ARRAY['Soybeans']::allergen_type[], 53, ARRAY['AllYear']::season_type[]),
('Ocet', 'Vinegar', 'Оцет', 'Уксус', 'liter', 3650, ARRAY[]::allergen_type[], 21, ARRAY['AllYear']::season_type[]),
('Miód', 'Honey', 'Мед', 'Мёд', 'kilogram', 730, ARRAY[]::allergen_type[], 304, ARRAY['AllYear']::season_type[]),

-- Beverages
('Woda mineralna', 'Mineral water', 'Мінеральна вода', 'Минеральная вода', 'liter', 365, ARRAY[]::allergen_type[], 0, ARRAY['AllYear']::season_type[]),
('Sok pomarańczowy', 'Orange juice', 'Апельсиновий сік', 'Апельсиновый сок', 'liter', 7, ARRAY[]::allergen_type[], 45, ARRAY['AllYear']::season_type[]),
('Wino białe', 'White wine', 'Біле вино', 'Белое вино', 'bottle', 730, ARRAY['Sulfites']::allergen_type[], 82, ARRAY['AllYear']::season_type[]),
('Wino czerwone', 'Red wine', 'Червоне вино', 'Красное вино', 'bottle', 730, ARRAY['Sulfites']::allergen_type[], 85, ARRAY['AllYear']::season_type[]),
('Piwo', 'Beer', 'Пиво', 'Пиво', 'bottle', 180, ARRAY['Wheat']::allergen_type[], 43, ARRAY['AllYear']::season_type[]),

-- Nuts & Seeds
('Orzechy włoskie', 'Walnuts', 'Волоські горіхи', 'Грецкие орехи', 'kilogram', 180, ARRAY['TreeNuts']::allergen_type[], 654, ARRAY['AllYear']::season_type[]),
('Migdały', 'Almonds', 'Мигдаль', 'Миндаль', 'kilogram', 180, ARRAY['TreeNuts']::allergen_type[], 579, ARRAY['AllYear']::season_type[]),
('Orzechy laskowe', 'Hazelnuts', 'Ліщина', 'Фундук', 'kilogram', 180, ARRAY['TreeNuts']::allergen_type[], 628, ARRAY['AllYear']::season_type[]),
('Nasiona słonecznika', 'Sunflower seeds', 'Насіння соняшника', 'Семена подсолнечника', 'kilogram', 365, ARRAY[]::allergen_type[], 584, ARRAY['AllYear']::season_type[]),
('Nasiona sezamu', 'Sesame seeds', 'Насіння кунжуту', 'Семена кунжута', 'kilogram', 365, ARRAY['Sesame']::allergen_type[], 573, ARRAY['AllYear']::season_type[]),

-- Legumes
('Fasola', 'Beans', 'Квасоля', 'Фасоль', 'kilogram', 730, ARRAY[]::allergen_type[], 127, ARRAY['AllYear']::season_type[]),
('Soczewica', 'Lentils', 'Сочевиця', 'Чечевица', 'kilogram', 730, ARRAY[]::allergen_type[], 116, ARRAY['AllYear']::season_type[]),
('Ciecierzyca', 'Chickpeas', 'Нут', 'Нут', 'kilogram', 730, ARRAY[]::allergen_type[], 164, ARRAY['AllYear']::season_type[]),

-- Sweets & Baking
('Cukier', 'Sugar', 'Цукор', 'Сахар', 'kilogram', 3650, ARRAY[]::allergen_type[], 387, ARRAY['AllYear']::season_type[]),
('Czekolada', 'Chocolate', 'Шоколад', 'Шоколад', 'kilogram', 365, ARRAY['Milk']::allergen_type[], 546, ARRAY['AllYear']::season_type[]),
('Kakao', 'Cocoa', 'Какао', 'Какао', 'kilogram', 730, ARRAY[]::allergen_type[], 228, ARRAY['AllYear']::season_type[]),
('Proszek do pieczenia', 'Baking powder', 'Розпушувач', 'Разрыхлитель', 'package', 730, ARRAY[]::allergen_type[], 53, ARRAY['AllYear']::season_type[]),
('Wanilia', 'Vanilla', 'Ваніль', 'Ваниль', 'package', 730, ARRAY[]::allergen_type[], 288, ARRAY['AllYear']::season_type[]),

-- Canned & Preserved
('Pomidory z puszki', 'Canned tomatoes', 'Помідори консервовані', 'Консервированные помидоры', 'can', 730, ARRAY[]::allergen_type[], 32, ARRAY['AllYear']::season_type[]),
('Ogórki kiszone', 'Pickles', 'Мариновані огірки', 'Маринованные огурцы', 'can', 365, ARRAY[]::allergen_type[], 11, ARRAY['AllYear']::season_type[]),
('Oliwki', 'Olives', 'Оливки', 'Оливки', 'can', 365, ARRAY[]::allergen_type[], 115, ARRAY['AllYear']::season_type[]),

-- Frozen
('Warzywa mrożone', 'Frozen vegetables', 'Заморожені овочі', 'Замороженные овощи', 'package', 365, ARRAY[]::allergen_type[], 65, ARRAY['AllYear']::season_type[]),
('Lody waniliowe', 'Vanilla ice cream', 'Ванільне морозиво', 'Ванильное мороженое', 'liter', 90, ARRAY['Milk', 'Eggs']::allergen_type[], 207, ARRAY['AllYear']::season_type[]);

COMMENT ON TABLE catalog_ingredients IS 'Master catalog of ingredients available across all restaurants';
COMMENT ON COLUMN catalog_ingredients.name_pl IS 'Polish name for search and display';
COMMENT ON COLUMN catalog_ingredients.name_en IS 'English name for search and display';
COMMENT ON COLUMN catalog_ingredients.name_uk IS 'Ukrainian name for search and display';
COMMENT ON COLUMN catalog_ingredients.name_ru IS 'Russian name for search and display';
COMMENT ON COLUMN catalog_ingredients.allergens IS 'Array of allergen types present in this ingredient';
COMMENT ON COLUMN catalog_ingredients.seasons IS 'Seasons when this ingredient is typically available';
