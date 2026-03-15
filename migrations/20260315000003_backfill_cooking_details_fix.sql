-- ============================================================
-- Migration: Fix backfill — previous migration used WHERE state_type IS NULL
-- but state_type had DEFAULT 'raw', so condition never matched.
-- Now using weight_change_percent IS NULL as the guard.
-- ============================================================

UPDATE ingredient_states SET
    weight_change_percent = 0.0,
    state_type = 'raw',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'raw' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 5.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'boiled' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -25.0,
    state_type = 'heat',
    oil_absorption_g = 10.0,
    water_loss_percent = 30.0
WHERE state = 'fried' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -15.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 15.0
WHERE state = 'baked' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -20.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 18.0
WHERE state = 'grilled' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -3.0,
    state_type = 'heat',
    oil_absorption_g = 0.0,
    water_loss_percent = 3.0
WHERE state = 'steamed' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -30.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 25.0
WHERE state = 'smoked' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 0.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'frozen' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = -70.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 70.0
WHERE state = 'dried' AND weight_change_percent IS NULL;

UPDATE ingredient_states SET
    weight_change_percent = 5.0,
    state_type = 'preserved',
    oil_absorption_g = 0.0,
    water_loss_percent = 0.0
WHERE state = 'pickled' AND weight_change_percent IS NULL;
