use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleClassification {
    pub category_slug: String,
    pub unit: String,
}

/// Static rules for classification to bypass AI for well-known ingredients.
pub struct ClassificationRules;

impl ClassificationRules {
    pub fn try_classify(name: &str) -> Option<RuleClassification> {
        let name_lower = name.to_lowercase();
        
        // --- Dairy ---
        if name_lower.contains("milk") || name_lower.contains("mleko") || name_lower.contains("молоко") || name_lower.contains("молоко") {
            return Some(RuleClassification {
                category_slug: "dairy_and_eggs".to_string(),
                unit: "liter".to_string(),
            });
        }
        if name_lower.contains("egg") || name_lower.contains("jajk") || name_lower.contains("яйц") {
            return Some(RuleClassification {
                category_slug: "dairy_and_eggs".to_string(),
                unit: "piece".to_string(),
            });
        }
        if name_lower.contains("butter") || name_lower.contains("masło") || name_lower.contains("масло") {
             return Some(RuleClassification {
                category_slug: "dairy_and_eggs".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Meat & Poultry ---
        if name_lower.contains("beef") || name_lower.contains("wołowin") || name_lower.contains("говяж") || name_lower.contains("ялович") {
            return Some(RuleClassification {
                category_slug: "meat".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("chicken") || name_lower.contains("kurczak") || name_lower.contains("курица") || name_lower.contains("курк") {
            return Some(RuleClassification {
                category_slug: "meat".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Vegetables ---
        if name_lower.contains("tomato") || name_lower.contains("pomidor") || name_lower.contains("томат") || name_lower.contains("помідор") {
            return Some(RuleClassification {
                category_slug: "vegetables".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("potato") || name_lower.contains("ziemniak") || name_lower.contains("картоф") {
            return Some(RuleClassification {
                category_slug: "vegetables".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("onion") || name_lower.contains("cebul") || name_lower.contains("лук") || name_lower.contains("цибул") {
            return Some(RuleClassification {
                category_slug: "vegetables".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Fruits ---
        if name_lower.contains("apple") || name_lower.contains("jabłk") || name_lower.contains("яблок") {
            return Some(RuleClassification {
                category_slug: "fruits".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Seafood ---
        if name_lower.contains("fish") || name_lower.contains("ryba") || name_lower.contains("рыба") {
            return Some(RuleClassification {
                category_slug: "seafood".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("salmon") || name_lower.contains("łosoś") || name_lower.contains("лосос") {
            return Some(RuleClassification {
                category_slug: "seafood".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Grains & Flour ---
        if name_lower.contains("flour") || name_lower.contains("mąka") || name_lower.contains("мук") {
            return Some(RuleClassification {
                category_slug: "grains".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("sugar") || name_lower.contains("cukier") || name_lower.contains("сахар") || name_lower.contains("цукор") {
            return Some(RuleClassification {
                category_slug: "grains".to_string(),
                unit: "kilogram".to_string(),
            });
        }
        if name_lower.contains("salt") || name_lower.contains("sól") || name_lower.contains("соль") || name_lower.contains("сіль") {
             return Some(RuleClassification {
                category_slug: "grains".to_string(),
                unit: "kilogram".to_string(),
            });
        }

        // --- Drinks ---
        if name_lower.contains("water") || name_lower.contains("woda") || name_lower.contains("вода") {
            return Some(RuleClassification {
                category_slug: "beverages".to_string(),
                unit: "liter".to_string(),
            });
        }
        if name_lower.contains("coffee") || name_lower.contains("kawa") || name_lower.contains("кофе") || name_lower.contains("кав") {
             return Some(RuleClassification {
                category_slug: "beverages".to_string(),
                unit: "kilogram".to_string(), // usually beans/ground
            });
        }

        None
    }
}
