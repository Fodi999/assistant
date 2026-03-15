-- ============================================================
-- Migration: Force-backfill cooking details (unconditional)
-- Previous migrations used conditional WHERE guards that may
-- not have matched. This one overwrites unconditionally.
-- ============================================================

UPDATE ingredient_states SET weight_change_percent = 0.0,   state_type = 'raw',       oil_absorption_g = 0.0,  water_loss_percent = 0.0  WHERE state = 'raw';
UPDATE ingredient_states SET weight_change_percent = 5.0,   state_type = 'heat',      oil_absorption_g = 0.0,  water_loss_percent = 0.0  WHERE state = 'boiled';
UPDATE ingredient_states SET weight_change_percent = -25.0, state_type = 'heat',      oil_absorption_g = 10.0, water_loss_percent = 30.0 WHERE state = 'fried';
UPDATE ingredient_states SET weight_change_percent = -15.0, state_type = 'heat',      oil_absorption_g = 0.0,  water_loss_percent = 15.0 WHERE state = 'baked';
UPDATE ingredient_states SET weight_change_percent = -20.0, state_type = 'heat',      oil_absorption_g = 0.0,  water_loss_percent = 18.0 WHERE state = 'grilled';
UPDATE ingredient_states SET weight_change_percent = -3.0,  state_type = 'heat',      oil_absorption_g = 0.0,  water_loss_percent = 3.0  WHERE state = 'steamed';
UPDATE ingredient_states SET weight_change_percent = -30.0, state_type = 'preserved', oil_absorption_g = 0.0,  water_loss_percent = 25.0 WHERE state = 'smoked';
UPDATE ingredient_states SET weight_change_percent = 0.0,   state_type = 'preserved', oil_absorption_g = 0.0,  water_loss_percent = 0.0  WHERE state = 'frozen';
UPDATE ingredient_states SET weight_change_percent = -70.0, state_type = 'preserved', oil_absorption_g = 0.0,  water_loss_percent = 70.0 WHERE state = 'dried';
UPDATE ingredient_states SET weight_change_percent = 5.0,   state_type = 'preserved', oil_absorption_g = 0.0,  water_loss_percent = 0.0  WHERE state = 'pickled';
