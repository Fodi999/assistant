-- ══════════════════════════════════════════════════════════════════════════════
-- SEED food_pairing — full coverage for all 111 products
-- Migration: 20260312000003_seed_food_pairing_full.sql
--
-- Strategy: INSERT OR IGNORE via ON CONFLICT DO NOTHING
-- Scores: flavor_score / nutrition_score / culinary_score / pair_score (0–10)
-- ══════════════════════════════════════════════════════════════════════════════

INSERT INTO food_pairing (ingredient_a, ingredient_b, flavor_score, nutrition_score, culinary_score, pair_score)
SELECT a.id, b.id, t.fs, t.ns, t.cs, t.ps
FROM (VALUES

-- ════════════════════════════════════════
-- SEAFOOD pairings
-- ════════════════════════════════════════

-- salmon
('salmon','lemon',       9.2, 7.0, 9.5, 9.0),
('salmon','dill',        9.5, 6.5, 9.5, 9.2),
('salmon','garlic',      8.5, 8.0, 8.8, 8.5),
('salmon','avocado',     8.8, 9.5, 8.5, 9.0),
('salmon','capers',      8.0, 6.0, 8.5, 8.0),
('salmon','cream-cheese',8.5, 6.5, 9.0, 8.5),
('salmon','soy-sauce',   8.8, 7.0, 9.0, 8.7),
('salmon','ginger',      8.5, 8.0, 8.8, 8.5),
('salmon','white-wine',  8.0, 5.0, 9.0, 8.2),
('salmon','olive-oil',   8.0, 9.0, 8.5, 8.5),
('salmon','black-pepper',7.5, 6.0, 8.0, 7.7),
('salmon','onion',       7.0, 7.0, 7.5, 7.2),
('salmon','spinach',     7.5, 9.5, 8.0, 8.2),
('salmon','potato',      7.0, 7.5, 8.0, 7.5),
('salmon','broccoli',    7.5, 9.5, 7.8, 8.1),

-- tuna
('tuna','lemon',         8.5, 7.0, 9.0, 8.5),
('tuna','avocado',       9.0, 9.5, 9.2, 9.2),
('tuna','soy-sauce',     9.2, 7.0, 9.5, 9.2),
('tuna','sesame-seeds',  8.5, 7.5, 9.0, 8.7),
('tuna','ginger',        8.8, 8.0, 9.0, 8.8),
('tuna','cucumber',      8.0, 7.5, 8.5, 8.1),
('tuna','rice',          8.5, 7.0, 9.5, 8.7),
('tuna','mayonnaise',    7.5, 5.0, 8.5, 7.5),
('tuna','onion',         7.5, 7.0, 7.8, 7.5),
('tuna','olive-oil',     8.0, 9.0, 8.5, 8.5),
('tuna','black-pepper',  7.0, 6.0, 7.5, 7.2),
('tuna','corn',          6.5, 6.5, 7.0, 6.7),

-- canned-tuna
('canned-tuna','mayonnaise',  8.0, 5.0, 8.5, 7.8),
('canned-tuna','onion',       7.5, 7.0, 8.0, 7.6),
('canned-tuna','lemon',       7.5, 7.0, 8.0, 7.5),
('canned-tuna','corn',        7.0, 6.5, 7.5, 7.0),
('canned-tuna','pasta',       7.5, 6.5, 8.5, 7.7),
('canned-tuna','olive-oil',   7.5, 9.0, 8.0, 8.0),
('canned-tuna','tomato',      7.0, 8.0, 7.8, 7.5),
('canned-tuna','black-pepper',6.5, 6.0, 7.0, 6.7),
('canned-tuna','cucumber',    7.0, 7.5, 7.5, 7.2),
('canned-tuna','lettuce',     7.5, 8.0, 8.0, 7.8),

-- cod
('cod','lemon',          9.0, 7.0, 9.2, 9.0),
('cod','garlic',         8.5, 8.0, 8.8, 8.5),
('cod','potato',         8.0, 7.5, 9.0, 8.3),
('cod','butter',         8.5, 5.5, 9.0, 8.3),
('cod','dill',           8.5, 6.5, 9.0, 8.5),
('cod','olive-oil',      8.0, 9.0, 8.5, 8.5),
('cod','parsley',        8.0, 7.0, 8.5, 8.1),
('cod','tomato',         7.5, 8.0, 8.0, 7.8),
('cod','onion',          7.5, 7.0, 8.0, 7.6),
('cod','black-pepper',   7.0, 6.0, 7.5, 7.0),
('cod','capers',         7.5, 5.5, 8.0, 7.5),

-- mackerel
('mackerel','lemon',     9.0, 7.0, 9.2, 9.0),
('mackerel','mustard',   8.5, 5.5, 8.8, 8.2),
('mackerel','garlic',    8.0, 8.0, 8.5, 8.2),
('mackerel','dill',      8.5, 6.5, 9.0, 8.5),
('mackerel','potato',    7.5, 7.5, 8.5, 7.9),
('mackerel','onion',     8.0, 7.0, 8.5, 8.0),
('mackerel','tomato',    7.5, 8.0, 8.0, 7.8),
('mackerel','olive-oil', 7.5, 9.0, 8.0, 8.0),
('mackerel','parsley',   8.0, 7.0, 8.5, 8.0),
('mackerel','vinegar',   7.5, 5.0, 8.0, 7.5),

-- herring
('herring','onion',      9.0, 7.0, 9.2, 9.0),
('herring','potato',     8.5, 7.5, 9.0, 8.5),
('herring','mustard',    8.0, 5.5, 8.5, 8.0),
('herring','dill',       8.5, 6.5, 9.0, 8.5),
('herring','vinegar',    7.5, 5.0, 8.0, 7.5),
('herring','sour-cream', 8.0, 5.5, 8.5, 8.0),
('herring','beet',       7.5, 8.5, 8.0, 8.0),
('herring','lemon',      7.5, 7.0, 8.0, 7.6),
('herring','black-pepper',7.0, 6.0, 7.5, 7.0),

-- shrimp
('shrimp','garlic',      9.5, 8.0, 9.5, 9.3),
('shrimp','lemon',       9.0, 7.0, 9.2, 9.0),
('shrimp','butter',      9.0, 5.5, 9.2, 8.8),
('shrimp','avocado',     8.5, 9.5, 8.8, 8.9),
('shrimp','pasta',       8.5, 6.5, 9.0, 8.5),
('shrimp','olive-oil',   8.5, 9.0, 9.0, 8.8),
('shrimp','black-pepper',7.5, 6.0, 8.0, 7.7),
('shrimp','parsley',     8.0, 7.0, 8.5, 8.0),
('shrimp','tomato',      7.5, 8.0, 8.0, 7.8),
('shrimp','rice',        8.0, 7.0, 8.5, 8.0),
('shrimp','ginger',      8.0, 8.0, 8.5, 8.2),
('shrimp','soy-sauce',   8.5, 7.0, 9.0, 8.5),
('shrimp','chili',       7.5, 7.0, 8.0, 7.8),

-- sea-bass
('sea-bass','lemon',     9.2, 7.0, 9.5, 9.2),
('sea-bass','garlic',    8.5, 8.0, 8.8, 8.5),
('sea-bass','olive-oil', 8.5, 9.0, 9.0, 8.8),
('sea-bass','tomato',    7.5, 8.0, 8.0, 7.8),
('sea-bass','white-wine',8.0, 5.0, 9.0, 8.2),
('sea-bass','dill',      8.0, 6.5, 8.5, 8.1),
('sea-bass','capers',    8.0, 5.5, 8.5, 8.0),
('sea-bass','parsley',   8.0, 7.0, 8.5, 8.0),
('sea-bass','butter',    8.5, 5.5, 9.0, 8.3),
('sea-bass','potato',    7.5, 7.5, 8.5, 7.9),
('sea-bass','zucchini',  7.5, 8.0, 8.0, 7.9),

-- trout
('trout','lemon',        9.0, 7.0, 9.2, 9.0),
('trout','dill',         9.0, 6.5, 9.2, 9.0),
('trout','almonds',      8.5, 8.5, 8.8, 8.6),
('trout','butter',       8.5, 5.5, 9.0, 8.3),
('trout','garlic',       8.0, 8.0, 8.5, 8.2),
('trout','olive-oil',    8.0, 9.0, 8.5, 8.5),
('trout','potato',       7.5, 7.5, 8.5, 7.9),
('trout','parsley',      8.0, 7.0, 8.5, 8.0),
('trout','white-wine',   7.5, 5.0, 8.5, 7.8),
('trout','spinach',      7.5, 9.5, 8.0, 8.2),

-- pike
('pike','onion',         8.5, 7.0, 8.8, 8.4),
('pike','lemon',         8.0, 7.0, 8.5, 8.0),
('pike','dill',          8.5, 6.5, 9.0, 8.5),
('pike','parsley',       8.0, 7.0, 8.5, 8.0),
('pike','garlic',        7.5, 8.0, 8.0, 7.8),
('pike','potato',        7.5, 7.5, 8.5, 7.9),
('pike','black-pepper',  7.0, 6.0, 7.5, 7.0),
('pike','butter',        8.0, 5.5, 8.5, 8.0),

-- carp
('carp','onion',         8.5, 7.0, 8.8, 8.4),
('carp','lemon',         8.0, 7.0, 8.5, 8.0),
('carp','garlic',        7.5, 8.0, 8.0, 7.8),
('carp','dill',          8.0, 6.5, 8.5, 8.0),
('carp','parsley',       7.5, 7.0, 8.0, 7.6),
('carp','potato',        7.5, 7.5, 8.5, 7.9),
('carp','butter',        8.0, 5.5, 8.5, 8.0),
('carp','black-pepper',  6.5, 6.0, 7.0, 6.7),
('carp','vinegar',       7.0, 5.0, 7.5, 7.0),

-- ════════════════════════════════════════
-- MEAT pairings
-- ════════════════════════════════════════

-- beef
('beef','garlic',        9.5, 8.0, 9.5, 9.3),
('beef','rosemary',      9.2, 6.5, 9.5, 9.2),
('beef','black-pepper',  9.0, 6.0, 9.2, 9.0),
('beef','onion',         8.5, 7.0, 9.0, 8.5),
('beef','potato',        8.5, 7.5, 9.0, 8.5),
('beef','tomato',        8.0, 8.0, 8.5, 8.2),
('beef','red-wine',      9.0, 5.5, 9.5, 9.0),
('beef','thyme',         8.5, 6.5, 9.0, 8.5),
('beef','mushrooms',     8.5, 8.5, 9.0, 8.7),
('beef','butter',        8.5, 5.5, 9.0, 8.3),
('beef','mustard',       8.0, 5.5, 8.5, 8.0),
('beef','soy-sauce',     7.5, 7.0, 8.0, 7.8),

-- chicken-breast
('chicken-breast','garlic',   9.0, 8.0, 9.2, 9.0),
('chicken-breast','lemon',    9.0, 7.0, 9.2, 9.0),
('chicken-breast','rosemary', 8.8, 6.5, 9.0, 8.8),
('chicken-breast','thyme',    8.5, 6.5, 9.0, 8.5),
('chicken-breast','olive-oil',8.5, 9.0, 9.0, 8.8),
('chicken-breast','onion',    8.0, 7.0, 8.5, 8.0),
('chicken-breast','paprika',  8.0, 7.0, 8.5, 8.0),
('chicken-breast','tomato',   7.5, 8.0, 8.0, 7.8),
('chicken-breast','honey',    7.5, 6.5, 8.0, 7.7),
('chicken-breast','mustard',  7.5, 5.5, 8.0, 7.5),
('chicken-breast','avocado',  7.5, 9.5, 7.8, 8.1),
('chicken-breast','broccoli', 7.5, 9.5, 8.0, 8.2),
('chicken-breast','spinach',  7.5, 9.5, 8.0, 8.2),

-- chicken-thighs
('chicken-thighs','garlic',   9.0, 8.0, 9.2, 9.0),
('chicken-thighs','rosemary', 8.8, 6.5, 9.0, 8.8),
('chicken-thighs','thyme',    8.5, 6.5, 9.0, 8.5),
('chicken-thighs','onion',    8.5, 7.0, 9.0, 8.5),
('chicken-thighs','paprika',  8.5, 7.0, 9.0, 8.5),
('chicken-thighs','lemon',    8.5, 7.0, 9.0, 8.5),
('chicken-thighs','potato',   8.0, 7.5, 8.8, 8.2),
('chicken-thighs','olive-oil',8.0, 9.0, 8.5, 8.5),
('chicken-thighs','honey',    8.0, 6.5, 8.5, 8.0),
('chicken-thighs','mustard',  7.5, 5.5, 8.0, 7.5),
('chicken-thighs','butter',   8.0, 5.5, 8.5, 8.0),

-- pork
('pork','apple',          9.0, 7.5, 9.2, 9.0),
('pork','garlic',         9.0, 8.0, 9.2, 9.0),
('pork','rosemary',       8.5, 6.5, 9.0, 8.5),
('pork','onion',          8.5, 7.0, 9.0, 8.5),
('pork','mustard',        8.5, 5.5, 9.0, 8.5),
('pork','honey',          8.5, 6.5, 9.0, 8.5),
('pork','soy-sauce',      8.0, 7.0, 8.5, 8.2),
('pork','ginger',         8.0, 8.0, 8.5, 8.2),
('pork','thyme',          8.0, 6.5, 8.5, 8.0),
('pork','potato',         8.0, 7.5, 8.8, 8.2),
('pork','cabbage',        8.0, 8.0, 8.5, 8.2),
('pork','black-pepper',   7.5, 6.0, 8.0, 7.7),

-- bacon
('bacon','eggs',          9.5, 7.0, 9.5, 9.3),
('bacon','potato',        9.0, 7.5, 9.2, 9.0),
('bacon','onion',         8.5, 7.0, 9.0, 8.5),
('bacon','tomato',        8.5, 8.0, 9.0, 8.7),
('bacon','black-pepper',  8.0, 6.0, 8.5, 8.0),
('bacon','avocado',       8.0, 9.5, 8.5, 8.5),
('bacon','lettuce',       8.5, 8.0, 9.0, 8.5),
('bacon','garlic',        7.5, 8.0, 8.0, 7.8),
('bacon','mushrooms',     8.0, 8.5, 8.5, 8.3),

-- ham
('ham','hard-cheese',     9.0, 7.0, 9.2, 9.0),
('ham','mustard',         8.5, 5.5, 9.0, 8.5),
('ham','bread',           9.0, 6.0, 9.2, 9.0),
('ham','tomato',          8.0, 8.0, 8.5, 8.2),
('ham','lettuce',         8.0, 8.0, 8.5, 8.2),
('ham','eggs',            8.0, 7.0, 8.5, 8.0),
('ham','onion',           7.5, 7.0, 8.0, 7.6),
('ham','pineapple',       7.5, 7.5, 8.0, 7.7),
('ham','black-pepper',    7.0, 6.0, 7.5, 7.0),

-- sausage
('sausage','mustard',     9.0, 5.5, 9.2, 9.0),
('sausage','onion',       8.5, 7.0, 9.0, 8.5),
('sausage','bread',       8.5, 6.0, 9.0, 8.5),
('sausage','ketchup',     8.5, 5.0, 9.0, 8.5),
('sausage','potato',      8.0, 7.5, 8.5, 8.1),
('sausage','cabbage',     8.0, 8.0, 8.5, 8.2),
('sausage','black-pepper',7.0, 6.0, 7.5, 7.0),

-- sausages
('sausages','mustard',    9.0, 5.5, 9.2, 9.0),
('sausages','onion',      8.5, 7.0, 9.0, 8.5),
('sausages','bread',      8.5, 6.0, 9.0, 8.5),
('sausages','ketchup',    8.5, 5.0, 9.0, 8.5),
('sausages','potato',     8.0, 7.5, 8.5, 8.1),
('sausages','black-pepper',7.0, 6.0, 7.5, 7.0),

-- ground-meat
('ground-meat','onion',   9.0, 7.0, 9.2, 9.0),
('ground-meat','garlic',  9.0, 8.0, 9.2, 9.0),
('ground-meat','tomato',  8.5, 8.0, 9.0, 8.7),
('ground-meat','black-pepper',8.0, 6.0, 8.5, 8.0),
('ground-meat','pasta',   8.5, 6.5, 9.0, 8.5),
('ground-meat','potato',  8.0, 7.5, 8.5, 8.1),
('ground-meat','paprika', 8.0, 7.0, 8.5, 8.0),
('ground-meat','thyme',   7.5, 6.5, 8.0, 7.7),
('ground-meat','eggs',    7.5, 7.0, 8.0, 7.6),
('ground-meat','breadcrumbs',7.5, 5.5, 8.5, 7.7),
('ground-meat','rice',    7.5, 7.0, 8.0, 7.6),

-- ════════════════════════════════════════
-- DAIRY pairings
-- ════════════════════════════════════════

-- butter
('butter','garlic',       9.0, 6.0, 9.2, 9.0),
('butter','bread',        9.0, 5.5, 9.2, 9.0),
('butter','lemon',        8.5, 6.5, 9.0, 8.5),
('butter','parsley',      8.5, 6.5, 9.0, 8.5),
('butter','honey',        8.5, 6.5, 9.0, 8.5),
('butter','sugar',        8.5, 4.0, 9.0, 8.5),
('butter','rosemary',     8.0, 6.0, 8.5, 8.0),
('butter','thyme',        8.0, 6.0, 8.5, 8.0),
('butter','chocolate',    9.0, 6.0, 9.5, 9.0),

-- milk
('milk','honey',          8.5, 7.0, 8.5, 8.3),
('milk','cinnamon',       8.5, 7.5, 8.5, 8.5),
('milk','vanilla',        8.5, 6.0, 9.0, 8.5),
('milk','oatmeal',        8.5, 9.0, 9.0, 8.8),
('milk','chocolate',      9.0, 6.0, 9.5, 9.0),
('milk','banana',         8.0, 8.5, 8.5, 8.3),
('milk','sugar',          7.5, 4.0, 8.0, 7.5),
('milk','cocoa',          8.5, 7.0, 9.0, 8.5),
('milk','coffee',         9.0, 5.5, 9.5, 9.0),
('milk','wheat-flour',    7.5, 6.5, 8.5, 7.7),

-- hard-cheese
('hard-cheese','bread',   9.0, 6.0, 9.2, 9.0),
('hard-cheese','wine',    9.0, 5.5, 9.5, 9.0),
('hard-cheese','apple',   8.5, 7.5, 8.8, 8.6),
('hard-cheese','grape',   8.5, 7.5, 9.0, 8.7),
('hard-cheese','ham',     9.0, 7.0, 9.2, 9.0),
('hard-cheese','tomato',  8.0, 8.0, 8.5, 8.2),
('hard-cheese','pasta',   8.5, 6.5, 9.0, 8.5),
('hard-cheese','walnuts', 8.5, 8.5, 8.8, 8.6),
('hard-cheese','honey',   8.5, 6.5, 9.0, 8.5),
('hard-cheese','basil',   7.5, 6.5, 8.0, 7.7),
('hard-cheese','black-pepper',7.5, 6.0, 8.0, 7.7),

-- mozzarella-cheese
('mozzarella-cheese','tomato', 9.5, 8.0, 9.8, 9.5),
('mozzarella-cheese','basil',  9.5, 6.5, 9.8, 9.5),
('mozzarella-cheese','olive-oil',9.0, 9.0, 9.5, 9.2),
('mozzarella-cheese','olives', 8.5, 7.0, 9.0, 8.7),
('mozzarella-cheese','bread',  8.5, 6.0, 9.0, 8.5),
('mozzarella-cheese','grape',  8.0, 7.5, 8.5, 8.1),
('mozzarella-cheese','black-pepper',7.5, 6.0, 8.0, 7.7),

-- cottage-cheese
('cottage-cheese','honey',     8.5, 7.0, 8.8, 8.4),
('cottage-cheese','berries',   8.5, 9.0, 8.8, 8.8),
('cottage-cheese','dill',      8.0, 6.5, 8.5, 8.0),
('cottage-cheese','garlic',    7.5, 8.0, 8.0, 7.8),
('cottage-cheese','onion',     7.5, 7.0, 8.0, 7.6),
('cottage-cheese','radish',    7.5, 8.0, 8.0, 7.8),
('cottage-cheese','cucumber',  7.5, 7.5, 8.0, 7.7),
('cottage-cheese','tomato',    7.5, 8.0, 8.0, 7.8),
('cottage-cheese','vanilla',   8.0, 6.0, 8.5, 8.0),
('cottage-cheese','banana',    8.0, 8.5, 8.5, 8.3),

-- chicken-eggs
('chicken-eggs','bacon',       9.5, 7.0, 9.5, 9.3),
('chicken-eggs','tomato',      8.5, 8.0, 8.8, 8.5),
('chicken-eggs','onion',       8.0, 7.0, 8.5, 8.0),
('chicken-eggs','butter',      9.0, 5.5, 9.2, 8.8),
('chicken-eggs','spinach',     8.5, 9.5, 9.0, 8.8),
('chicken-eggs','hard-cheese', 8.5, 7.0, 9.0, 8.5),
('chicken-eggs','avocado',     8.5, 9.5, 8.8, 8.9),
('chicken-eggs','salmon',      8.5, 9.0, 9.0, 8.8),
('chicken-eggs','mushrooms',   8.0, 8.5, 8.5, 8.3),
('chicken-eggs','bread',       8.5, 6.0, 9.0, 8.5),
('chicken-eggs','dill',        8.0, 6.5, 8.5, 8.0),
('chicken-eggs','black-pepper',7.5, 6.0, 8.0, 7.7),
('chicken-eggs','wheat-flour', 8.0, 6.5, 8.5, 8.0),

-- ════════════════════════════════════════
-- VEGETABLES pairings
-- ════════════════════════════════════════

-- tomato
('tomato','basil',        9.5, 6.5, 9.8, 9.5),
('tomato','garlic',       9.2, 8.0, 9.5, 9.2),
('tomato','olive-oil',    9.0, 9.0, 9.2, 9.0),
('tomato','mozzarella-cheese',9.5, 8.0, 9.8, 9.5),
('tomato','onion',        8.5, 7.0, 9.0, 8.5),
('tomato','black-pepper', 8.0, 6.0, 8.5, 8.0),
('tomato','oregano',      8.5, 6.5, 9.0, 8.5),
('tomato','balsamic',     8.5, 6.0, 9.0, 8.5),
('tomato','cucumber',     8.0, 7.5, 8.5, 8.1),
('tomato','avocado',      8.5, 9.5, 8.8, 8.9),
('tomato','lemon',        7.5, 7.0, 8.0, 7.6),
('tomato','sugar',        7.0, 4.0, 7.5, 7.0),

-- garlic
('garlic','olive-oil',    9.5, 9.0, 9.5, 9.3),
('garlic','lemon',        8.5, 7.5, 9.0, 8.7),
('garlic','rosemary',     9.0, 6.5, 9.2, 9.0),
('garlic','thyme',        9.0, 6.5, 9.2, 9.0),
('garlic','parsley',      8.5, 7.0, 9.0, 8.5),
('garlic','butter',       9.0, 6.0, 9.2, 9.0),
('garlic','black-pepper', 8.5, 6.0, 9.0, 8.5),
('garlic','onion',        8.5, 7.0, 9.0, 8.5),
('garlic','chili',        8.5, 7.0, 9.0, 8.5),
('garlic','tomato',       9.2, 8.0, 9.5, 9.2),

-- onion
('onion','garlic',        9.0, 8.0, 9.2, 9.0),
('onion','tomato',        8.5, 8.0, 9.0, 8.7),
('onion','olive-oil',     8.5, 9.0, 9.0, 8.8),
('onion','thyme',         8.5, 6.5, 9.0, 8.5),
('onion','butter',        8.5, 5.5, 9.0, 8.5),
('onion','black-pepper',  7.5, 6.0, 8.0, 7.7),
('onion','vinegar',       7.5, 5.0, 8.0, 7.5),
('onion','sugar',         7.5, 4.0, 8.5, 7.8),
('onion','balsamic',      8.5, 6.0, 9.0, 8.5),
('onion','parsley',       8.0, 7.0, 8.5, 8.0),

-- potato
('potato','garlic',       9.0, 8.0, 9.2, 9.0),
('potato','butter',       9.0, 5.5, 9.2, 9.0),
('potato','rosemary',     8.5, 6.5, 9.0, 8.5),
('potato','onion',        8.5, 7.0, 9.0, 8.5),
('potato','hard-cheese',  8.5, 7.0, 9.0, 8.5),
('potato','black-pepper', 7.5, 6.0, 8.0, 7.7),
('potato','dill',         8.5, 6.5, 9.0, 8.5),
('potato','bacon',        9.0, 7.5, 9.2, 9.0),
('potato','olive-oil',    8.0, 9.0, 8.5, 8.5),
('potato','sour-cream',   8.5, 5.5, 9.0, 8.5),

-- spinach
('spinach','garlic',      9.0, 8.5, 9.0, 8.8),
('spinach','lemon',       8.5, 8.0, 8.8, 8.5),
('spinach','olive-oil',   8.5, 9.5, 9.0, 8.8),
('spinach','eggs',        8.5, 9.5, 9.0, 8.8),
('spinach','hard-cheese', 8.5, 8.5, 9.0, 8.7),
('spinach','pasta',       8.0, 8.0, 8.5, 8.2),
('spinach','nutmeg',      8.0, 7.0, 8.5, 8.0),
('spinach','salmon',      8.0, 9.5, 8.5, 8.5),
('spinach','almonds',     8.0, 9.5, 8.5, 8.5),

-- broccoli
('broccoli','garlic',     9.0, 8.5, 9.0, 8.8),
('broccoli','olive-oil',  8.5, 9.5, 9.0, 8.8),
('broccoli','lemon',      8.5, 8.0, 8.8, 8.5),
('broccoli','hard-cheese',8.5, 8.5, 9.0, 8.7),
('broccoli','sesame-seeds',8.0, 8.5, 8.5, 8.3),
('broccoli','soy-sauce',  8.0, 7.5, 8.5, 8.1),
('broccoli','almonds',    8.0, 9.5, 8.5, 8.5),
('broccoli','chicken-breast',7.5, 9.5, 8.0, 8.2),

-- carrot
('carrot','ginger',       9.0, 8.5, 9.0, 8.8),
('carrot','honey',        8.5, 7.5, 8.8, 8.6),
('carrot','olive-oil',    8.0, 9.0, 8.5, 8.5),
('carrot','lemon',        8.0, 8.0, 8.5, 8.2),
('carrot','cinnamon',     8.0, 7.5, 8.5, 8.1),
('carrot','orange',       8.5, 8.5, 8.8, 8.6),
('carrot','butter',       8.0, 5.5, 8.5, 8.0),
('carrot','parsley',      8.0, 7.0, 8.5, 8.0),
('carrot','thyme',        7.5, 6.5, 8.0, 7.7),
('carrot','garlic',       7.5, 8.0, 8.0, 7.8),

-- bell-pepper
('bell-pepper','olive-oil',   8.5, 9.0, 9.0, 8.8),
('bell-pepper','garlic',      8.5, 8.0, 9.0, 8.5),
('bell-pepper','onion',       8.5, 7.0, 9.0, 8.5),
('bell-pepper','tomato',      8.5, 8.0, 9.0, 8.7),
('bell-pepper','basil',       8.0, 6.5, 8.5, 8.0),
('bell-pepper','oregano',     8.0, 6.5, 8.5, 8.0),
('bell-pepper','cheese',      8.0, 7.0, 8.5, 8.0),
('bell-pepper','eggs',        7.5, 7.0, 8.0, 7.6),
('bell-pepper','black-pepper',7.0, 6.0, 7.5, 7.0),

-- cucumber
('cucumber','dill',           9.0, 6.5, 9.0, 8.8),
('cucumber','garlic',         8.5, 8.0, 8.8, 8.5),
('cucumber','lemon',          8.0, 7.5, 8.5, 8.1),
('cucumber','vinegar',        8.0, 5.0, 8.5, 8.0),
('cucumber','onion',          8.0, 7.0, 8.5, 8.1),
('cucumber','sour-cream',     8.5, 5.5, 9.0, 8.5),
('cucumber','tomato',         8.0, 7.5, 8.5, 8.1),
('cucumber','olive-oil',      7.5, 9.0, 8.0, 8.0),
('cucumber','sesame-seeds',   7.5, 7.5, 8.0, 7.7),
('cucumber','black-pepper',   7.0, 6.0, 7.5, 7.0),

-- eggplant
('eggplant','garlic',         9.0, 8.0, 9.0, 8.8),
('eggplant','olive-oil',      9.0, 9.0, 9.2, 9.0),
('eggplant','tomato',         8.5, 8.0, 9.0, 8.7),
('eggplant','basil',          8.5, 6.5, 9.0, 8.5),
('eggplant','onion',          8.0, 7.0, 8.5, 8.0),
('eggplant','hard-cheese',    8.0, 7.0, 8.5, 8.0),
('eggplant','sesame-seeds',   8.0, 7.5, 8.5, 8.1),
('eggplant','lemon',          7.5, 7.5, 8.0, 7.7),
('eggplant','oregano',        8.0, 6.5, 8.5, 8.0),
('eggplant','black-pepper',   7.0, 6.0, 7.5, 7.0),

-- zucchini
('zucchini','garlic',         8.5, 8.0, 9.0, 8.5),
('zucchini','olive-oil',      8.5, 9.0, 9.0, 8.8),
('zucchini','lemon',          8.0, 7.5, 8.5, 8.1),
('zucchini','basil',          8.0, 6.5, 8.5, 8.0),
('zucchini','tomato',         8.0, 8.0, 8.5, 8.2),
('zucchini','hard-cheese',    8.0, 7.0, 8.5, 8.0),
('zucchini','onion',          7.5, 7.0, 8.0, 7.6),
('zucchini','thyme',          7.5, 6.5, 8.0, 7.7),
('zucchini','eggs',           7.5, 7.0, 8.0, 7.6),

-- cabbage
('cabbage','carrot',          8.5, 8.5, 8.8, 8.6),
('cabbage','vinegar',         8.5, 5.0, 9.0, 8.5),
('cabbage','onion',           8.5, 7.0, 9.0, 8.5),
('cabbage','dill',            8.5, 6.5, 9.0, 8.5),
('cabbage','olive-oil',       8.0, 9.0, 8.5, 8.5),
('cabbage','black-pepper',    7.5, 6.0, 8.0, 7.7),
('cabbage','garlic',          7.5, 8.0, 8.0, 7.8),
('cabbage','lemon',           7.5, 7.5, 8.0, 7.7),
('cabbage','apple',           8.0, 7.5, 8.5, 8.1),

-- lettuce
('lettuce','olive-oil',       8.5, 9.0, 8.8, 8.8),
('lettuce','lemon',           8.5, 7.5, 8.8, 8.6),
('lettuce','avocado',         8.5, 9.5, 8.8, 8.9),
('lettuce','tomato',          8.5, 8.0, 9.0, 8.7),
('lettuce','cucumber',        8.5, 7.5, 9.0, 8.7),
('lettuce','hard-cheese',     7.5, 7.0, 8.0, 7.7),
('lettuce','onion',           7.5, 7.0, 8.0, 7.6),
('lettuce','black-pepper',    7.0, 6.0, 7.5, 7.0),
('lettuce','vinegar',         7.5, 5.0, 8.0, 7.5),

-- cauliflower
('cauliflower','garlic',      8.5, 8.0, 9.0, 8.5),
('cauliflower','butter',      8.5, 5.5, 9.0, 8.5),
('cauliflower','hard-cheese', 8.5, 7.0, 9.0, 8.5),
('cauliflower','olive-oil',   8.5, 9.0, 9.0, 8.8),
('cauliflower','lemon',       8.0, 7.5, 8.5, 8.1),
('cauliflower','turmeric',    8.5, 8.5, 9.0, 8.7),
('cauliflower','black-pepper',7.5, 6.0, 8.0, 7.7),
('cauliflower','onion',       7.5, 7.0, 8.0, 7.6),

-- corn
('corn','butter',             8.5, 5.5, 9.0, 8.5),
('corn','black-pepper',       7.5, 6.0, 8.0, 7.7),
('corn','hard-cheese',        8.0, 7.0, 8.5, 8.0),
('corn','avocado',            8.0, 9.5, 8.5, 8.5),
('corn','tomato',             7.5, 8.0, 8.0, 7.8),
('corn','lime',               8.0, 7.5, 8.5, 8.1),
('corn','olive-oil',          7.5, 9.0, 8.0, 8.0),
('corn','onion',              7.5, 7.0, 8.0, 7.6),

-- green-peas
('green-peas','mint',         8.5, 7.5, 8.8, 8.6),
('green-peas','butter',       8.5, 5.5, 9.0, 8.5),
('green-peas','lemon',        8.0, 7.5, 8.5, 8.1),
('green-peas','garlic',       7.5, 8.0, 8.0, 7.8),
('green-peas','onion',        7.5, 7.0, 8.0, 7.6),
('green-peas','pasta',        8.0, 7.5, 8.5, 8.1),
('green-peas','ham',          8.0, 7.5, 8.5, 8.1),
('green-peas','black-pepper', 7.0, 6.0, 7.5, 7.0),

-- artichoke
('artichoke','lemon',         8.5, 7.5, 8.8, 8.6),
('artichoke','garlic',        8.5, 8.0, 8.8, 8.5),
('artichoke','olive-oil',     8.5, 9.0, 9.0, 8.8),
('artichoke','butter',        8.0, 5.5, 8.5, 8.0),
('artichoke','parsley',       8.0, 7.0, 8.5, 8.0),
('artichoke','hard-cheese',   7.5, 7.0, 8.0, 7.7),
('artichoke','white-wine',    7.5, 5.0, 8.5, 7.8),
('artichoke','black-pepper',  7.0, 6.0, 7.5, 7.0),

-- button-mushroom
('button-mushroom','garlic',      9.0, 8.0, 9.2, 9.0),
('button-mushroom','butter',      9.0, 5.5, 9.2, 9.0),
('button-mushroom','onion',       8.5, 7.0, 9.0, 8.5),
('button-mushroom','parsley',     8.5, 7.0, 9.0, 8.5),
('button-mushroom','thyme',       8.5, 6.5, 9.0, 8.5),
('button-mushroom','olive-oil',   8.0, 9.0, 8.5, 8.5),
('button-mushroom','hard-cheese', 8.0, 7.0, 8.5, 8.0),
('button-mushroom','eggs',        8.0, 7.0, 8.5, 8.0),
('button-mushroom','sour-cream',  8.0, 5.5, 8.5, 8.0),
('button-mushroom','black-pepper',7.5, 6.0, 8.0, 7.7),
('button-mushroom','lemon',       7.5, 7.5, 8.0, 7.7),

-- porcini-mushroom
('porcini-mushroom','garlic',     9.0, 8.0, 9.2, 9.0),
('porcini-mushroom','butter',     9.0, 5.5, 9.2, 9.0),
('porcini-mushroom','onion',      8.5, 7.0, 9.0, 8.5),
('porcini-mushroom','parsley',    8.5, 7.0, 9.0, 8.5),
('porcini-mushroom','thyme',      8.5, 6.5, 9.0, 8.5),
('porcini-mushroom','pasta',      9.0, 6.5, 9.5, 9.0),
('porcini-mushroom','risotto',    9.0, 6.5, 9.5, 9.0),
('porcini-mushroom','hard-cheese',8.5, 7.0, 9.0, 8.7),
('porcini-mushroom','olive-oil',  8.5, 9.0, 9.0, 8.8),
('porcini-mushroom','lemon',      7.5, 7.5, 8.0, 7.7),

-- ════════════════════════════════════════
-- FRUITS pairings
-- ════════════════════════════════════════

-- apple
('apple','cinnamon',      9.5, 7.5, 9.5, 9.3),
('apple','honey',         8.5, 7.5, 8.8, 8.6),
('apple','walnuts',       8.5, 9.0, 8.8, 8.7),
('apple','hard-cheese',   8.5, 7.5, 8.8, 8.6),
('apple','pork',          9.0, 7.5, 9.2, 9.0),
('apple','butter',        8.5, 5.5, 9.0, 8.5),
('apple','vanilla',       8.0, 6.0, 8.5, 8.0),
('apple','lemon',         7.5, 7.5, 8.0, 7.7),
('apple','cardamom',      8.0, 7.5, 8.5, 8.1),
('apple','ginger',        8.0, 8.0, 8.5, 8.2),

-- lemon
('lemon','honey',         9.0, 7.5, 9.2, 9.0),
('lemon','ginger',        9.0, 8.5, 9.2, 9.0),
('lemon','garlic',        8.5, 8.0, 9.0, 8.7),
('lemon','olive-oil',     8.5, 9.0, 9.0, 8.8),
('lemon','dill',          8.5, 6.5, 9.0, 8.5),
('lemon','parsley',       8.5, 7.0, 9.0, 8.5),
('lemon','thyme',         8.5, 6.5, 9.0, 8.5),
('lemon','butter',        8.5, 6.5, 9.0, 8.5),
('lemon','vanilla',       7.5, 6.0, 8.0, 7.7),
('lemon','sugar',         8.0, 4.0, 8.5, 8.0),

-- orange
('orange','chocolate',    9.0, 7.5, 9.2, 9.0),
('orange','honey',        8.5, 7.5, 8.8, 8.6),
('orange','cinnamon',     8.5, 7.5, 8.8, 8.6),
('orange','ginger',       8.5, 8.5, 8.8, 8.6),
('orange','vanilla',      8.0, 6.0, 8.5, 8.0),
('orange','carrot',       8.5, 8.5, 8.8, 8.6),
('orange','duck',         9.0, 7.0, 9.5, 9.0),
('orange','pork',         8.0, 7.5, 8.5, 8.1),
('orange','almonds',      8.0, 8.5, 8.5, 8.3),
('orange','raspberry',    8.5, 8.5, 8.8, 8.6),

-- avocado
('avocado','lime',        9.0, 8.0, 9.2, 9.0),
('avocado','tomato',      9.0, 8.5, 9.2, 9.0),
('avocado','onion',       8.5, 7.5, 8.8, 8.6),
('avocado','garlic',      8.5, 8.5, 8.8, 8.6),
('avocado','chili',       8.5, 7.5, 9.0, 8.7),
('avocado','cilantro',    9.0, 7.0, 9.2, 9.0),
('avocado','eggs',        8.5, 9.5, 8.8, 8.9),
('avocado','sesame-seeds',8.0, 8.5, 8.5, 8.3),
('avocado','soy-sauce',   8.0, 7.5, 8.5, 8.1),
('avocado','olive-oil',   8.0, 9.5, 8.5, 8.5),

-- banana
('banana','honey',        8.5, 7.5, 8.8, 8.6),
('banana','chocolate',    9.0, 7.0, 9.2, 9.0),
('banana','cinnamon',     8.5, 7.5, 8.8, 8.6),
('banana','walnuts',      8.5, 9.0, 8.8, 8.7),
('banana','peanut-butter',9.0, 8.5, 9.2, 9.0),
('banana','vanilla',      8.5, 6.0, 9.0, 8.5),
('banana','oatmeal',      8.5, 9.5, 9.0, 8.8),
('banana','strawberry',   8.5, 9.0, 8.8, 8.8),
('banana','milk',         8.0, 8.5, 8.5, 8.3),

-- strawberry
('strawberry','chocolate',9.5, 7.5, 9.5, 9.3),
('strawberry','cream',    9.0, 5.5, 9.2, 9.0),
('strawberry','vanilla',  8.5, 6.0, 9.0, 8.5),
('strawberry','lemon',    8.0, 8.0, 8.5, 8.2),
('strawberry','basil',    8.5, 6.5, 8.8, 8.6),
('strawberry','honey',    8.0, 7.5, 8.5, 8.1),
('strawberry','banana',   8.5, 9.0, 8.8, 8.8),
('strawberry','mint',     8.5, 7.5, 8.8, 8.6),

-- raspberry
('raspberry','chocolate', 9.5, 7.5, 9.5, 9.3),
('raspberry','cream',     8.5, 5.5, 9.0, 8.5),
('raspberry','lemon',     8.5, 8.0, 8.8, 8.6),
('raspberry','vanilla',   8.5, 6.0, 9.0, 8.5),
('raspberry','mint',      8.5, 7.5, 8.8, 8.6),
('raspberry','orange',    8.5, 8.5, 8.8, 8.6),
('raspberry','almonds',   8.0, 9.0, 8.5, 8.5),
('raspberry','honey',     8.0, 7.5, 8.5, 8.1),

-- blueberry
('blueberry','lemon',     8.5, 8.0, 8.8, 8.6),
('blueberry','vanilla',   8.5, 6.0, 9.0, 8.5),
('blueberry','honey',     8.0, 7.5, 8.5, 8.1),
('blueberry','oatmeal',   8.5, 9.5, 9.0, 8.8),
('blueberry','cream',     8.5, 5.5, 9.0, 8.5),
('blueberry','cinnamon',  8.0, 7.5, 8.5, 8.1),
('blueberry','almonds',   8.0, 9.0, 8.5, 8.5),
('blueberry','mint',      8.0, 7.5, 8.5, 8.1),

-- pineapple
('pineapple','coconut',   9.0, 7.5, 9.2, 9.0),
('pineapple','lime',      8.5, 7.5, 8.8, 8.6),
('pineapple','ginger',    8.5, 8.5, 8.8, 8.6),
('pineapple','vanilla',   8.0, 6.0, 8.5, 8.0),
('pineapple','ham',       8.0, 7.5, 8.5, 8.1),
('pineapple','chicken-breast',7.5, 9.0, 8.0, 8.1),
('pineapple','honey',     8.0, 7.5, 8.5, 8.1),
('pineapple','mint',      8.0, 7.5, 8.5, 8.1),

-- peach
('peach','honey',         8.5, 7.5, 8.8, 8.6),
('peach','vanilla',       8.5, 6.0, 9.0, 8.5),
('peach','cinnamon',      8.5, 7.5, 8.8, 8.6),
('peach','raspberry',     8.5, 8.5, 8.8, 8.6),
('peach','almonds',       8.5, 8.5, 8.8, 8.6),
('peach','cream',         8.0, 5.5, 8.5, 8.0),
('peach','ginger',        7.5, 8.5, 8.0, 8.0),

-- pear
('pear','hard-cheese',    9.0, 7.5, 9.2, 9.0),
('pear','walnuts',        8.5, 9.0, 8.8, 8.7),
('pear','honey',          8.5, 7.5, 8.8, 8.6),
('pear','cinnamon',       8.5, 7.5, 8.8, 8.6),
('pear','ginger',         8.0, 8.5, 8.5, 8.3),
('pear','vanilla',        8.0, 6.0, 8.5, 8.0),
('pear','chocolate',      8.5, 7.5, 8.8, 8.6),

-- apricot
('apricot','honey',       8.5, 7.5, 8.8, 8.6),
('apricot','vanilla',     8.5, 6.0, 9.0, 8.5),
('apricot','almonds',     8.5, 8.5, 8.8, 8.6),
('apricot','cinnamon',    8.5, 7.5, 8.8, 8.6),
('apricot','ginger',      8.0, 8.5, 8.5, 8.3),
('apricot','cream',       8.0, 5.5, 8.5, 8.0),
('apricot','chicken-breast',7.5, 9.0, 8.0, 8.1),

-- cherry
('cherry','chocolate',    9.5, 7.5, 9.5, 9.3),
('cherry','vanilla',      8.5, 6.0, 9.0, 8.5),
('cherry','almonds',      8.5, 8.5, 8.8, 8.6),
('cherry','cream',        8.5, 5.5, 9.0, 8.5),
('cherry','cinnamon',     8.0, 7.5, 8.5, 8.1),
('cherry','honey',        8.0, 7.5, 8.5, 8.1),
('cherry','red-wine',     8.5, 6.0, 9.0, 8.5),

-- grape
('grape','hard-cheese',   9.0, 7.5, 9.2, 9.0),
('grape','walnuts',       8.5, 9.0, 8.8, 8.7),
('grape','honey',         8.0, 7.5, 8.5, 8.1),
('grape','rosemary',      7.5, 6.5, 8.0, 7.7),
('grape','balsamic',      8.5, 6.0, 9.0, 8.5),
('grape','almonds',       8.0, 8.5, 8.5, 8.3),
('grape','cream-cheese',  8.0, 6.5, 8.5, 8.1),

-- watermelon
('watermelon','feta',     9.0, 8.0, 9.2, 9.0),
('watermelon','mint',     9.0, 7.5, 9.2, 9.0),
('watermelon','lime',     8.5, 7.5, 8.8, 8.6),
('watermelon','basil',    8.0, 6.5, 8.5, 8.0),
('watermelon','ginger',   7.5, 8.5, 8.0, 8.0),
('watermelon','honey',    8.0, 7.5, 8.5, 8.1),
('watermelon','cucumber', 8.0, 7.5, 8.5, 8.1),

-- plum
('plum','cinnamon',       8.5, 7.5, 8.8, 8.6),
('plum','vanilla',        8.0, 6.0, 8.5, 8.0),
('plum','honey',          8.0, 7.5, 8.5, 8.1),
('plum','almonds',        8.0, 8.5, 8.5, 8.3),
('plum','ginger',         8.0, 8.5, 8.5, 8.3),
('plum','pork',           7.5, 7.5, 8.0, 7.7),
('plum','red-wine',       8.5, 6.0, 9.0, 8.5),

-- ════════════════════════════════════════
-- NUTS pairings
-- ════════════════════════════════════════

-- almonds
('almonds','honey',       8.5, 8.5, 8.8, 8.6),
('almonds','chocolate',   9.0, 8.5, 9.2, 9.0),
('almonds','lemon',       7.5, 8.5, 7.8, 7.9),
('almonds','cinnamon',    8.0, 8.5, 8.5, 8.3),
('almonds','vanilla',     8.0, 8.5, 8.5, 8.3),
('almonds','raspberry',   8.0, 9.0, 8.5, 8.5),
('almonds','orange',      8.0, 8.5, 8.5, 8.3),
('almonds','trout',       8.5, 9.5, 8.8, 8.9),
('almonds','spinach',     8.0, 9.5, 8.5, 8.5),

-- walnuts
('walnuts','honey',       8.5, 8.5, 8.8, 8.6),
('walnuts','apple',       8.5, 9.0, 8.8, 8.7),
('walnuts','hard-cheese', 8.5, 8.5, 8.8, 8.6),
('walnuts','chocolate',   9.0, 8.5, 9.2, 9.0),
('walnuts','banana',      8.5, 9.0, 8.8, 8.7),
('walnuts','pear',        8.5, 9.0, 8.8, 8.7),
('walnuts','cinnamon',    8.0, 8.5, 8.5, 8.3),
('walnuts','olive-oil',   7.5, 9.5, 8.0, 8.1),
('walnuts','garlic',      7.5, 8.5, 8.0, 8.0),

-- hazelnuts
('hazelnuts','chocolate', 9.5, 8.5, 9.5, 9.3),
('hazelnuts','coffee',    8.5, 8.0, 9.0, 8.5),
('hazelnuts','honey',     8.5, 8.5, 8.8, 8.6),
('hazelnuts','vanilla',   8.5, 8.0, 9.0, 8.5),
('hazelnuts','banana',    8.0, 9.0, 8.5, 8.5),
('hazelnuts','raspberry', 8.5, 9.0, 8.8, 8.7),
('hazelnuts','caramel',   8.5, 7.0, 9.0, 8.5),

-- sesame-seeds
('sesame-seeds','soy-sauce',  9.0, 7.5, 9.2, 9.0),
('sesame-seeds','ginger',     8.5, 8.5, 8.8, 8.6),
('sesame-seeds','honey',      8.5, 8.5, 8.8, 8.6),
('sesame-seeds','garlic',     8.5, 8.5, 8.8, 8.6),
('sesame-seeds','avocado',    8.0, 9.5, 8.5, 8.5),
('sesame-seeds','tuna',       8.5, 8.5, 9.0, 8.7),
('sesame-seeds','cucumber',   8.0, 8.0, 8.5, 8.2),
('sesame-seeds','olive-oil',  7.5, 9.5, 8.0, 8.1),

-- sunflower-seeds
('sunflower-seeds','honey',       8.0, 8.5, 8.5, 8.3),
('sunflower-seeds','lemon',       7.5, 8.5, 8.0, 8.0),
('sunflower-seeds','garlic',      7.5, 8.5, 8.0, 8.0),
('sunflower-seeds','spinach',     7.5, 9.5, 8.0, 8.2),
('sunflower-seeds','olive-oil',   7.5, 9.5, 8.0, 8.1),
('sunflower-seeds','pumpkin',     8.0, 9.0, 8.5, 8.5),

-- ════════════════════════════════════════
-- LEGUMES pairings
-- ════════════════════════════════════════

-- beans
('beans','garlic',        8.5, 8.5, 9.0, 8.7),
('beans','tomato',        8.5, 8.5, 9.0, 8.7),
('beans','onion',         8.5, 7.5, 9.0, 8.5),
('beans','olive-oil',     8.0, 9.5, 8.5, 8.5),
('beans','rosemary',      8.0, 7.0, 8.5, 8.0),
('beans','thyme',         8.0, 7.0, 8.5, 8.0),
('beans','lemon',         7.5, 8.0, 8.0, 7.8),
('beans','black-pepper',  7.0, 6.5, 7.5, 7.0),
('beans','cumin',         8.5, 7.5, 9.0, 8.7),

-- chickpeas
('chickpeas','garlic',    9.0, 8.5, 9.2, 9.0),
('chickpeas','lemon',     8.5, 8.0, 8.8, 8.6),
('chickpeas','olive-oil', 8.5, 9.5, 9.0, 8.8),
('chickpeas','cumin',     9.0, 7.5, 9.2, 9.0),
('chickpeas','paprika',   8.5, 7.5, 9.0, 8.7),
('chickpeas','onion',     8.0, 7.5, 8.5, 8.1),
('chickpeas','tomato',    8.0, 8.5, 8.5, 8.3),
('chickpeas','spinach',   8.0, 9.5, 8.5, 8.5),
('chickpeas','sesame-seeds',8.5, 8.5, 9.0, 8.7),
('chickpeas','parsley',   8.0, 7.5, 8.5, 8.1),

-- lentils
('lentils','garlic',      8.5, 8.5, 9.0, 8.7),
('lentils','onion',       8.5, 7.5, 9.0, 8.5),
('lentils','cumin',       9.0, 7.5, 9.2, 9.0),
('lentils','lemon',       8.5, 8.0, 8.8, 8.6),
('lentils','tomato',      8.0, 8.5, 8.5, 8.3),
('lentils','spinach',     8.0, 9.5, 8.5, 8.5),
('lentils','olive-oil',   8.0, 9.5, 8.5, 8.5),
('lentils','turmeric',    8.5, 8.5, 9.0, 8.7),
('lentils','ginger',      8.0, 8.5, 8.5, 8.3),
('lentils','carrot',      8.0, 8.5, 8.5, 8.3),

-- ════════════════════════════════════════
-- GRAINS pairings
-- ════════════════════════════════════════

-- rice
('rice','garlic',         8.5, 8.0, 8.8, 8.5),
('rice','soy-sauce',      9.0, 7.0, 9.2, 9.0),
('rice','ginger',         8.5, 8.0, 8.8, 8.5),
('rice','onion',          8.0, 7.0, 8.5, 8.0),
('rice','butter',         8.5, 5.5, 9.0, 8.5),
('rice','sesame-seeds',   8.0, 8.0, 8.5, 8.2),
('rice','lemon',          7.5, 7.5, 8.0, 7.7),
('rice','turmeric',       8.0, 8.5, 8.5, 8.3),
('rice','coconut',        8.5, 7.5, 9.0, 8.5),

-- pasta
('pasta','garlic',        9.0, 8.0, 9.2, 9.0),
('pasta','tomato',        9.0, 8.0, 9.5, 9.0),
('pasta','olive-oil',     8.5, 9.0, 9.0, 8.8),
('pasta','hard-cheese',   8.5, 7.0, 9.0, 8.5),
('pasta','basil',         8.5, 6.5, 9.0, 8.5),
('pasta','onion',         8.0, 7.0, 8.5, 8.0),
('pasta','black-pepper',  7.5, 6.0, 8.0, 7.7),
('pasta','lemon',         7.5, 7.5, 8.0, 7.7),
('pasta','spinach',       8.0, 9.5, 8.5, 8.5),
('pasta','salmon',        8.5, 9.0, 9.0, 8.8),

-- oatmeal
('oatmeal','honey',       9.0, 8.5, 9.0, 8.8),
('oatmeal','banana',      8.5, 9.5, 9.0, 8.8),
('oatmeal','blueberry',   8.5, 9.5, 9.0, 8.8),
('oatmeal','cinnamon',    8.5, 8.0, 9.0, 8.5),
('oatmeal','milk',        8.5, 9.0, 9.0, 8.8),
('oatmeal','apple',       8.5, 9.0, 9.0, 8.8),
('oatmeal','walnuts',     8.5, 9.5, 9.0, 8.9),
('oatmeal','strawberry',  8.5, 9.0, 9.0, 8.8),

-- buckwheat
('buckwheat','onion',     8.5, 7.5, 9.0, 8.5),
('buckwheat','mushrooms', 8.5, 8.5, 9.0, 8.7),
('buckwheat','butter',    8.5, 5.5, 9.0, 8.5),
('buckwheat','garlic',    7.5, 8.0, 8.0, 7.8),
('buckwheat','black-pepper',7.0, 6.0, 7.5, 7.0),
('buckwheat','milk',      8.0, 8.5, 8.5, 8.3),
('buckwheat','honey',     7.5, 7.5, 8.0, 7.7),

-- wheat-flour
('wheat-flour','eggs',    8.5, 7.5, 9.0, 8.5),
('wheat-flour','butter',  8.5, 5.5, 9.0, 8.5),
('wheat-flour','milk',    8.5, 8.0, 9.0, 8.5),
('wheat-flour','sugar',   8.5, 4.0, 9.0, 8.5),
('wheat-flour','vanilla', 8.0, 5.5, 8.5, 8.0),
('wheat-flour','yeast',   8.0, 5.5, 9.0, 8.0),
('wheat-flour','salt',    8.0, 5.0, 8.5, 8.0),

-- bread
('bread','butter',        9.0, 5.5, 9.2, 9.0),
('bread','garlic',        9.0, 7.5, 9.2, 9.0),
('bread','olive-oil',     8.5, 9.0, 9.0, 8.8),
('bread','hard-cheese',   8.5, 7.0, 9.0, 8.5),
('bread','honey',         8.5, 6.5, 9.0, 8.5),
('bread','tomato',        8.0, 8.0, 8.5, 8.2),
('bread','avocado',       8.5, 9.5, 9.0, 8.9),
('bread','eggs',          8.5, 7.5, 9.0, 8.5),

-- breadcrumbs
('breadcrumbs','eggs',    8.0, 7.0, 8.5, 8.0),
('breadcrumbs','garlic',  7.5, 7.5, 8.0, 7.7),
('breadcrumbs','parsley', 7.5, 7.0, 8.0, 7.6),
('breadcrumbs','lemon',   7.5, 7.5, 8.0, 7.7),
('breadcrumbs','hard-cheese',7.5, 7.0, 8.0, 7.7),
('breadcrumbs','olive-oil',7.0, 8.5, 7.5, 7.7),

-- ════════════════════════════════════════
-- SPICES pairings
-- ════════════════════════════════════════

-- basil
('basil','tomato',        9.5, 7.5, 9.8, 9.5),
('basil','garlic',        9.0, 8.0, 9.2, 9.0),
('basil','olive-oil',     9.0, 9.0, 9.2, 9.0),
('basil','mozzarella-cheese',9.5, 7.5, 9.8, 9.5),
('basil','lemon',         8.0, 7.5, 8.5, 8.1),
('basil','pine-nuts',     8.5, 8.5, 9.0, 8.7),
('basil','strawberry',    8.5, 7.5, 8.8, 8.6),
('basil','peach',         8.0, 7.5, 8.5, 8.1),
('basil','pasta',         8.5, 7.0, 9.0, 8.5),

-- dill
('dill','cucumber',       9.0, 7.0, 9.2, 9.0),
('dill','garlic',         8.5, 8.0, 8.8, 8.5),
('dill','lemon',          8.5, 7.5, 8.8, 8.6),
('dill','potato',         8.5, 7.5, 9.0, 8.5),
('dill','salmon',         9.5, 8.0, 9.5, 9.2),
('dill','sour-cream',     8.5, 5.5, 9.0, 8.5),
('dill','butter',         8.0, 5.5, 8.5, 8.0),
('dill','eggs',           8.0, 7.0, 8.5, 8.0),
('dill','cottage-cheese', 8.0, 7.5, 8.5, 8.0),

-- rosemary
('rosemary','garlic',     9.5, 8.0, 9.5, 9.3),
('rosemary','olive-oil',  9.0, 9.0, 9.2, 9.0),
('rosemary','lemon',      8.5, 7.5, 9.0, 8.7),
('rosemary','potato',     8.5, 7.5, 9.0, 8.5),
('rosemary','lamb',       9.5, 7.5, 9.5, 9.3),
('rosemary','bread',      8.5, 6.5, 9.0, 8.5),
('rosemary','honey',      8.0, 7.5, 8.5, 8.1),
('rosemary','sea-salt',   8.0, 5.5, 8.5, 8.0),

-- thyme
('thyme','garlic',        9.0, 8.0, 9.2, 9.0),
('thyme','lemon',         8.5, 7.5, 9.0, 8.7),
('thyme','olive-oil',     8.5, 9.0, 9.0, 8.8),
('thyme','tomato',        8.5, 8.0, 9.0, 8.7),
('thyme','mushrooms',     8.5, 8.5, 9.0, 8.7),
('thyme','onion',         8.5, 7.0, 9.0, 8.5),
('thyme','honey',         8.0, 7.5, 8.5, 8.1),

-- oregano
('oregano','tomato',      9.5, 8.0, 9.5, 9.3),
('oregano','garlic',      9.0, 8.0, 9.2, 9.0),
('oregano','olive-oil',   8.5, 9.0, 9.0, 8.8),
('oregano','lemon',       8.0, 7.5, 8.5, 8.1),
('oregano','hard-cheese', 8.0, 7.0, 8.5, 8.0),
('oregano','eggplant',    8.5, 8.0, 9.0, 8.7),
('oregano','bell-pepper', 8.0, 7.5, 8.5, 8.1),

-- parsley
('parsley','garlic',      8.5, 8.0, 9.0, 8.5),
('parsley','lemon',       8.5, 7.5, 9.0, 8.7),
('parsley','olive-oil',   8.5, 9.0, 9.0, 8.8),
('parsley','butter',      8.5, 6.0, 9.0, 8.5),
('parsley','tomato',      8.0, 8.0, 8.5, 8.2),
('parsley','onion',       8.0, 7.0, 8.5, 8.0),
('parsley','black-pepper',7.5, 6.0, 8.0, 7.7),

-- black-pepper
('black-pepper','garlic', 8.5, 8.0, 9.0, 8.5),
('black-pepper','olive-oil',8.0, 9.0, 8.5, 8.5),
('black-pepper','lemon',  7.5, 7.5, 8.0, 7.7),
('black-pepper','butter', 8.0, 5.5, 8.5, 8.0),
('black-pepper','onion',  7.5, 7.0, 8.0, 7.6),
('black-pepper','tomato', 8.0, 8.0, 8.5, 8.2),
('black-pepper','cream',  7.5, 5.0, 8.0, 7.5),

-- ginger
('ginger','garlic',       9.0, 8.5, 9.2, 9.0),
('ginger','lemon',        9.0, 8.5, 9.2, 9.0),
('ginger','honey',        8.5, 8.0, 8.8, 8.6),
('ginger','soy-sauce',    9.0, 7.5, 9.2, 9.0),
('ginger','sesame-seeds', 8.5, 8.5, 8.8, 8.6),
('ginger','carrot',       9.0, 8.5, 9.2, 9.0),
('ginger','coconut',      8.5, 7.5, 9.0, 8.5),
('ginger','lime',         8.5, 8.0, 8.8, 8.6),
('ginger','turmeric',     8.5, 9.0, 9.0, 8.8),

-- cinnamon
('cinnamon','apple',      9.5, 7.5, 9.5, 9.3),
('cinnamon','honey',      8.5, 7.5, 8.8, 8.6),
('cinnamon','vanilla',    8.5, 6.0, 9.0, 8.5),
('cinnamon','chocolate',  8.5, 7.0, 9.0, 8.5),
('cinnamon','banana',     8.5, 8.5, 8.8, 8.6),
('cinnamon','orange',     8.5, 8.5, 8.8, 8.6),
('cinnamon','oatmeal',    8.5, 8.0, 9.0, 8.5),
('cinnamon','milk',       8.5, 8.5, 8.8, 8.6),
('cinnamon','cardamom',   8.0, 7.5, 8.5, 8.1),

-- turmeric
('turmeric','ginger',     8.5, 9.0, 9.0, 8.8),
('turmeric','garlic',     8.5, 8.5, 9.0, 8.7),
('turmeric','black-pepper',9.0, 8.0, 9.0, 8.8),
('turmeric','coconut',    8.5, 7.5, 9.0, 8.5),
('turmeric','lemon',      7.5, 8.5, 8.0, 8.0),
('turmeric','rice',       8.0, 8.5, 8.5, 8.3),
('turmeric','chickpeas',  8.5, 9.0, 9.0, 8.8),
('turmeric','lentils',    8.5, 9.0, 9.0, 8.8),

-- sweet-paprika
('sweet-paprika','garlic',    8.5, 8.0, 9.0, 8.5),
('sweet-paprika','onion',     8.5, 7.0, 9.0, 8.5),
('sweet-paprika','olive-oil', 8.0, 9.0, 8.5, 8.5),
('sweet-paprika','tomato',    8.5, 8.0, 9.0, 8.7),
('sweet-paprika','chicken-breast',8.5, 9.0, 9.0, 8.8),
('sweet-paprika','chickpeas', 8.5, 9.0, 9.0, 8.8),
('sweet-paprika','lemon',     7.5, 7.5, 8.0, 7.7),
('sweet-paprika','sour-cream',8.0, 5.5, 8.5, 8.0),

-- ════════════════════════════════════════
-- OTHER pairings
-- ════════════════════════════════════════

-- olive-oil
('olive-oil','garlic',    9.5, 9.0, 9.5, 9.3),
('olive-oil','lemon',     8.5, 9.0, 9.0, 8.8),
('olive-oil','rosemary',  9.0, 9.0, 9.2, 9.0),
('olive-oil','thyme',     8.5, 9.0, 9.0, 8.8),
('olive-oil','basil',     9.0, 9.0, 9.2, 9.0),
('olive-oil','black-pepper',8.0, 9.0, 8.5, 8.5),
('olive-oil','tomato',    9.0, 9.0, 9.2, 9.0),
('olive-oil','vinegar',   8.5, 8.5, 9.0, 8.7),

-- honey
('honey','lemon',         9.0, 7.5, 9.2, 9.0),
('honey','ginger',        8.5, 8.0, 8.8, 8.6),
('honey','cinnamon',      8.5, 7.5, 8.8, 8.6),
('honey','mustard',       8.5, 5.5, 9.0, 8.5),
('honey','garlic',        8.5, 8.0, 9.0, 8.7),
('honey','rosemary',      8.0, 7.5, 8.5, 8.1),
('honey','walnuts',       8.5, 8.5, 8.8, 8.6),
('honey','vanilla',       8.5, 6.0, 9.0, 8.5),
('honey','butter',        8.5, 6.5, 9.0, 8.5),

-- mustard
('mustard','honey',       9.0, 6.0, 9.2, 9.0),
('mustard','garlic',      8.5, 8.0, 9.0, 8.7),
('mustard','lemon',       8.0, 7.5, 8.5, 8.1),
('mustard','vinegar',     8.0, 5.0, 8.5, 8.0),
('mustard','dill',        8.0, 6.5, 8.5, 8.0),
('mustard','black-pepper',7.5, 6.0, 8.0, 7.7),

-- soy-sauce
('soy-sauce','garlic',    9.0, 7.5, 9.2, 9.0),
('soy-sauce','ginger',    9.0, 7.5, 9.2, 9.0),
('soy-sauce','sesame-seeds',8.5, 8.0, 9.0, 8.7),
('soy-sauce','honey',     8.5, 6.5, 9.0, 8.5),
('soy-sauce','lime',      8.5, 7.5, 9.0, 8.7),
('soy-sauce','rice',      9.0, 7.0, 9.2, 9.0),
('soy-sauce','avocado',   8.0, 9.5, 8.5, 8.5),

-- vinegar
('vinegar','garlic',      8.0, 7.5, 8.5, 8.0),
('vinegar','onion',       7.5, 7.0, 8.0, 7.6),
('vinegar','mustard',     8.0, 5.0, 8.5, 8.0),
('vinegar','olive-oil',   8.5, 8.5, 9.0, 8.7),
('vinegar','honey',       7.5, 6.5, 8.0, 7.7),
('vinegar','black-pepper',7.0, 6.0, 7.5, 7.0),

-- chocolate
('chocolate','orange',    9.0, 7.5, 9.2, 9.0),
('chocolate','raspberry', 9.5, 7.5, 9.5, 9.3),
('chocolate','cherry',    9.5, 7.5, 9.5, 9.3),
('chocolate','coffee',    9.5, 6.5, 9.5, 9.3),
('chocolate','caramel',   9.0, 6.0, 9.2, 9.0),
('chocolate','almonds',   9.0, 8.5, 9.2, 9.0),
('chocolate','hazelnuts', 9.5, 8.5, 9.5, 9.3),
('chocolate','banana',    9.0, 7.5, 9.2, 9.0),
('chocolate','strawberry',9.5, 7.5, 9.5, 9.3),
('chocolate','vanilla',   9.0, 6.0, 9.2, 9.0),
('chocolate','mint',      8.5, 7.0, 9.0, 8.5),
('chocolate','chili',     7.5, 7.0, 8.0, 7.8),

-- red-wine
('red-wine','beef',       9.0, 5.5, 9.5, 9.0),
('red-wine','hard-cheese',8.5, 6.0, 9.0, 8.5),
('red-wine','chocolate',  8.5, 5.5, 9.0, 8.5),
('red-wine','rosemary',   8.0, 6.0, 8.5, 8.0),
('red-wine','garlic',     8.0, 7.0, 8.5, 8.0),
('red-wine','cherry',     8.5, 7.5, 9.0, 8.5),
('red-wine','plum',       8.5, 6.5, 9.0, 8.5),

-- white-wine
('white-wine','garlic',   8.5, 7.0, 9.0, 8.5),
('white-wine','lemon',    8.5, 7.0, 9.0, 8.5),
('white-wine','butter',   8.5, 5.5, 9.0, 8.5),
('white-wine','shallot',  8.5, 6.5, 9.0, 8.5),
('white-wine','thyme',    8.0, 6.0, 8.5, 8.0),
('white-wine','parsley',  8.0, 6.5, 8.5, 8.0),
('white-wine','seafood',  9.0, 6.0, 9.5, 9.0),

-- olives
('olives','tomato',       8.5, 8.0, 9.0, 8.7),
('olives','garlic',       8.5, 8.0, 9.0, 8.5),
('olives','olive-oil',    8.0, 9.5, 8.5, 8.5),
('olives','hard-cheese',  8.5, 7.0, 9.0, 8.5),
('olives','oregano',      8.5, 6.5, 9.0, 8.5),
('olives','lemon',        7.5, 7.5, 8.0, 7.7),
('olives','capers',       8.0, 5.5, 8.5, 8.0),
('olives','anchovies',    8.5, 7.5, 9.0, 8.7),

-- canned-tomatoes
('canned-tomatoes','garlic',    9.0, 8.0, 9.2, 9.0),
('canned-tomatoes','onion',     8.5, 7.0, 9.0, 8.5),
('canned-tomatoes','basil',     9.0, 6.5, 9.2, 9.0),
('canned-tomatoes','oregano',   8.5, 6.5, 9.0, 8.5),
('canned-tomatoes','olive-oil', 8.5, 9.0, 9.0, 8.8),
('canned-tomatoes','pasta',     9.0, 7.0, 9.5, 9.0),
('canned-tomatoes','ground-meat',8.5, 7.5, 9.0, 8.7),
('canned-tomatoes','black-pepper',7.5, 6.0, 8.0, 7.7),

-- sugar
('sugar','vanilla',       8.5, 4.0, 9.0, 8.5),
('sugar','butter',        8.5, 4.0, 9.0, 8.5),
('sugar','cinnamon',      8.5, 5.0, 9.0, 8.5),
('sugar','lemon',         8.0, 5.0, 8.5, 8.0),
('sugar','eggs',          8.0, 5.5, 8.5, 8.0),
('sugar','wheat-flour',   8.5, 4.5, 9.0, 8.5),

-- vanilla
('vanilla','chocolate',   9.0, 6.0, 9.2, 9.0),
('vanilla','strawberry',  8.5, 7.0, 9.0, 8.5),
('vanilla','banana',      8.5, 7.0, 9.0, 8.5),
('vanilla','peach',       8.5, 7.0, 9.0, 8.5),
('vanilla','cinnamon',    8.5, 6.0, 9.0, 8.5),
('vanilla','honey',       8.5, 6.5, 9.0, 8.5),
('vanilla','milk',        8.5, 7.5, 9.0, 8.5),
('vanilla','butter',      8.5, 5.5, 9.0, 8.5),

-- cocoa
('cocoa','banana',        8.5, 7.5, 9.0, 8.5),
('cocoa','honey',         8.5, 7.5, 9.0, 8.5),
('cocoa','vanilla',       9.0, 6.0, 9.2, 9.0),
('cocoa','cinnamon',      8.5, 7.5, 9.0, 8.5),
('cocoa','milk',          8.5, 7.5, 9.0, 8.5),
('cocoa','almonds',       8.5, 9.0, 9.0, 8.8),
('cocoa','chili',         7.5, 7.0, 8.0, 7.8),

-- mayonnaise
('mayonnaise','garlic',   8.5, 5.5, 9.0, 8.5),
('mayonnaise','lemon',    8.0, 5.5, 8.5, 8.0),
('mayonnaise','mustard',  8.5, 5.0, 9.0, 8.5),
('mayonnaise','dill',     8.0, 5.5, 8.5, 8.0),
('mayonnaise','black-pepper',7.0, 5.5, 7.5, 7.0),
('mayonnaise','onion',    7.5, 6.0, 8.0, 7.6),

-- ketchup
('ketchup','garlic',      8.0, 6.0, 8.5, 8.0),
('ketchup','onion',       7.5, 6.0, 8.0, 7.6),
('ketchup','black-pepper',7.0, 5.5, 7.5, 7.0),
('ketchup','basil',       7.5, 6.0, 8.0, 7.6),
('ketchup','oregano',     7.5, 6.0, 8.0, 7.6),

-- pickles
('pickles','dill',        9.0, 6.5, 9.2, 9.0),
('pickles','garlic',      8.5, 7.5, 9.0, 8.5),
('pickles','onion',       8.0, 7.0, 8.5, 8.0),
('pickles','black-pepper',7.5, 6.0, 8.0, 7.7),
('pickles','mustard',     7.5, 5.5, 8.0, 7.5),

-- beer
('beer','garlic',         7.5, 5.0, 8.0, 7.5),
('beer','mustard',        8.0, 4.5, 8.5, 8.0),
('beer','sausage',        9.0, 4.5, 9.2, 9.0),
('beer','cheese',         8.5, 6.0, 9.0, 8.5),
('beer','onion',          7.5, 6.0, 8.0, 7.6),
('beer','lemon',          7.5, 6.0, 8.0, 7.5),
('beer','black-pepper',   7.0, 5.5, 7.5, 7.0),

-- orange-juice
('orange-juice','ginger', 8.5, 8.0, 8.8, 8.6),
('orange-juice','honey',  8.5, 7.5, 8.8, 8.6),
('orange-juice','lemon',  7.5, 8.0, 8.0, 7.8),
('orange-juice','carrot', 8.5, 8.5, 8.8, 8.6),
('orange-juice','vanilla',7.5, 6.0, 8.0, 7.7),

-- vanilla-ice-cream
('vanilla-ice-cream','strawberry', 9.0, 6.5, 9.2, 9.0),
('vanilla-ice-cream','chocolate',  9.0, 6.0, 9.2, 9.0),
('vanilla-ice-cream','raspberry',  8.5, 7.0, 9.0, 8.5),
('vanilla-ice-cream','honey',      8.0, 6.5, 8.5, 8.0),
('vanilla-ice-cream','cinnamon',   8.0, 7.0, 8.5, 8.0),
('vanilla-ice-cream','banana',     8.5, 7.5, 9.0, 8.5),
('vanilla-ice-cream','walnuts',    8.5, 8.0, 9.0, 8.5),
('vanilla-ice-cream','caramel',    9.0, 5.5, 9.2, 9.0),

-- sunflower-oil
('sunflower-oil','garlic',    8.0, 7.5, 8.5, 8.0),
('sunflower-oil','onion',     8.0, 7.0, 8.5, 8.0),
('sunflower-oil','black-pepper',7.0, 6.0, 7.5, 7.0),
('sunflower-oil','paprika',   7.5, 7.0, 8.0, 7.6),

-- rapeseed-oil
('rapeseed-oil','garlic',     8.0, 7.5, 8.5, 8.0),
('rapeseed-oil','mustard',    7.5, 5.5, 8.0, 7.5),
('rapeseed-oil','lemon',      7.5, 8.0, 8.0, 7.8),
('rapeseed-oil','onion',      7.5, 7.0, 8.0, 7.6),

-- baking-powder
('baking-powder','wheat-flour',8.5, 5.0, 9.0, 8.5),
('baking-powder','eggs',      8.5, 6.5, 9.0, 8.5),
('baking-powder','butter',    8.0, 5.0, 8.5, 8.0),
('baking-powder','sugar',     8.0, 4.0, 8.5, 8.0),
('baking-powder','vanilla',   7.5, 5.0, 8.0, 7.5),

-- mineral-water
('mineral-water','lemon',     7.5, 7.5, 7.5, 7.5),
('mineral-water','mint',      7.5, 7.5, 7.5, 7.5),
('mineral-water','cucumber',  7.5, 7.5, 7.5, 7.5),
('mineral-water','ginger',    7.5, 8.0, 7.5, 7.7),

-- frozen-vegetables
('frozen-vegetables','garlic',    7.5, 8.0, 8.0, 7.8),
('frozen-vegetables','olive-oil', 7.5, 9.0, 8.0, 8.0),
('frozen-vegetables','black-pepper',7.0, 6.5, 7.5, 7.0),
('frozen-vegetables','lemon',     7.5, 7.5, 8.0, 7.7),
('frozen-vegetables','butter',    7.5, 5.5, 8.0, 7.5)

) AS t(sa, sb, fs, ns, cs, ps)
JOIN products a ON a.slug = t.sa
JOIN products b ON b.slug = t.sb
ON CONFLICT (ingredient_a, ingredient_b) DO NOTHING;

-- Verify results
SELECT COUNT(*) as total_pairings FROM food_pairing;
SELECT COUNT(DISTINCT ingredient_a) as products_with_pairings FROM food_pairing;
