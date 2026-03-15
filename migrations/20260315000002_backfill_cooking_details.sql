-- ============================================================
-- Migration: Backfill cooking detail columns for existing states
-- Fill weight_change_percent, state_type, oil_absorption_g, water_loss_percent
-- based on state value for all existing records
-- ============================================================

UPDATE ingredient_states SET
    weight_change_percent = 0.0,
    state_type = 'raw',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'raw' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 5.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'boiled' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -25.0,
    state_type = 'heat',
    oil_absorption_g = 10.0,
    water_loss_percent = 30.0
WHERE state = 'fried' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -15.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 15.0
WHERE state = 'baked' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -20.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 18.0
WHERE state = 'grilled' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -3.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 3.0
WHERE state = 'steamed' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -30.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 25.0
WHERE state = 'smoked' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 0.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'frozen' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -70.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 70.0
WHERE state = 'dried' AND state_type IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 5.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'pickled' AND state_type IS NULL;
