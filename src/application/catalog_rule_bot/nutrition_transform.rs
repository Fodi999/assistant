use crate::domain::ProcessingState;

/// Base nutrition values from the raw ingredient (per 100g)
#[derive(Debug, Clone)]
pub struct BaseNutrition {
    pub calories: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbs: f64,
    pub fiber: f64,
    pub water_percent: f64,
}

/// Transformed nutrition for a specific processing state
#[derive(Debug, Clone)]
pub struct TransformedNutrition {
    pub calories_per_100g: f64,
    pub protein_per_100g: f64,
    pub fat_per_100g: f64,
    pub carbs_per_100g: f64,
    pub fiber_per_100g: f64,
    pub water_percent: f64,
}

/// Transformation coefficients for each processing state.
/// All values are multipliers applied to the base (raw) nutrition.
///
/// Based on general food science principles:
/// - Boiling: water absorption → slight concentration of nutrients per 100g
/// - Frying: fat absorption + water loss → significant calorie increase
/// - Baking: moderate water loss → moderate concentration
/// - Grilling: fat drip + water loss → slight concentration
/// - Steaming: minimal nutrient change
/// - Smoking: significant water loss → strong concentration
/// - Freezing: negligible change
/// - Drying: extreme water loss → extreme concentration
/// - Pickling: minor changes, slight calorie reduction
#[derive(Debug, Clone)]
struct TransformCoefficients {
    calories: f64,
    protein: f64,
    fat: f64,
    carbs: f64,
    fiber: f64,
    water_delta: f64, // absolute change in water % (negative = loss)
}

fn get_coefficients(state: ProcessingState) -> TransformCoefficients {
    match state {
        ProcessingState::Raw => TransformCoefficients {
            calories: 1.0,
            protein: 1.0,
            fat: 1.0,
            carbs: 1.0,
            fiber: 1.0,
            water_delta: 0.0,
        },
        ProcessingState::Boiled => TransformCoefficients {
            // Water absorption → slight dilution, but per-100g values increase
            // due to mass loss from dissolved nutrients
            calories: 1.05,
            protein: 0.95, // slight protein loss to water
            fat: 0.95,
            carbs: 1.0,
            fiber: 0.95,
            water_delta: 5.0, // absorbs water
        },
        ProcessingState::Fried => TransformCoefficients {
            // Significant fat absorption + water loss
            calories: 1.40,
            protein: 1.0,
            fat: 1.80, // major fat increase from oil
            carbs: 1.05,
            fiber: 0.95,
            water_delta: -20.0, // significant water loss
        },
        ProcessingState::Baked => TransformCoefficients {
            // Moderate water loss → concentration
            calories: 1.15,
            protein: 1.05,
            fat: 1.05,
            carbs: 1.10,
            fiber: 1.0,
            water_delta: -12.0,
        },
        ProcessingState::Grilled => TransformCoefficients {
            // Fat drips off + water loss
            calories: 1.10,
            protein: 1.10,
            fat: 0.85, // fat drips off
            carbs: 1.05,
            fiber: 1.0,
            water_delta: -15.0,
        },
        ProcessingState::Steamed => TransformCoefficients {
            // Minimal change — gentlest cooking method
            calories: 1.02,
            protein: 0.98,
            fat: 1.0,
            carbs: 1.0,
            fiber: 0.98,
            water_delta: 2.0, // slight water absorption
        },
        ProcessingState::Smoked => TransformCoefficients {
            // Significant water loss → concentration
            calories: 1.25,
            protein: 1.20,
            fat: 1.15,
            carbs: 1.10,
            fiber: 1.0,
            water_delta: -25.0,
        },
        ProcessingState::Frozen => TransformCoefficients {
            // Negligible nutritional change
            calories: 1.0,
            protein: 1.0,
            fat: 1.0,
            carbs: 1.0,
            fiber: 1.0,
            water_delta: 0.0,
        },
        ProcessingState::Dried => TransformCoefficients {
            // Extreme water loss → extreme concentration
            // Typically 3-4x concentration for most nutrients
            calories: 3.0,
            protein: 3.2,
            fat: 2.8,
            carbs: 3.5,
            fiber: 3.0,
            water_delta: -70.0, // most water removed
        },
        ProcessingState::Pickled => TransformCoefficients {
            // Minor changes, slight calorie adjustment
            calories: 0.95,
            protein: 0.90,
            fat: 1.0,
            carbs: 1.10, // sugar from brine
            fiber: 0.90,
            water_delta: 5.0,
        },
    }
}

/// Transform base nutrition values according to processing state rules.
/// Returns None if base nutrition is missing critical values.
pub fn transform_nutrition(
    base: &BaseNutrition,
    state: ProcessingState,
) -> TransformedNutrition {
    let coeff = get_coefficients(state);

    let water = (base.water_percent + coeff.water_delta).clamp(0.0, 99.0);

    TransformedNutrition {
        calories_per_100g: round2(base.calories * coeff.calories),
        protein_per_100g: round2(base.protein * coeff.protein),
        fat_per_100g: round2(base.fat * coeff.fat),
        carbs_per_100g: round2(base.carbs * coeff.carbs),
        fiber_per_100g: round2(base.fiber * coeff.fiber),
        water_percent: round2(water),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_potato() -> BaseNutrition {
        BaseNutrition {
            calories: 77.0,
            protein: 2.0,
            fat: 0.1,
            carbs: 17.0,
            fiber: 2.2,
            water_percent: 79.0,
        }
    }

    #[test]
    fn raw_is_unchanged() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProcessingState::Raw);
        assert_eq!(result.calories_per_100g, 77.0);
        assert_eq!(result.protein_per_100g, 2.0);
    }

    #[test]
    fn fried_increases_fat() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProcessingState::Fried);
        assert!(result.fat_per_100g > base.fat * 1.5);
        assert!(result.calories_per_100g > base.calories * 1.2);
    }

    #[test]
    fn frozen_is_mostly_unchanged() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProcessingState::Frozen);
        assert_eq!(result.calories_per_100g, 77.0);
        assert_eq!(result.protein_per_100g, 2.0);
    }

    #[test]
    fn dried_concentrates() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProcessingState::Dried);
        assert!(result.calories_per_100g > base.calories * 2.5);
        assert!(result.water_percent < 15.0);
    }
}
