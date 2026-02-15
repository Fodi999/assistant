use crate::domain::recipe_v2::Recipe;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rule-based validator that runs BEFORE AI
/// Проверяет логику, совместимость, безопасность
#[derive(Debug, Clone)]
pub struct RecipeValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub dish_type: Option<DishType>,
    pub missing_critical_ingredients: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Critical,  // Блокирует публикацию
    High,      // Требует исправления
    Medium,    // Рекомендация
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DishType {
    Cake,
    Pie,
    Bread,
    Dessert,
    Soup,
    Salad,
    MainCourse,
    Appetizer,
    Beverage,
    Unknown,
}

impl RecipeValidator {
    pub fn new() -> Self {
        Self
    }

    /// Главная функция валидации
    pub fn validate(&self, recipe: &Recipe) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Detect dish type from name
        let dish_type = self.detect_dish_type(&recipe.name_default);

        // 2. Basic checks
        if recipe.name_default.trim().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_NAME".to_string(),
                message: "Название рецепта не может быть пустым".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if recipe.instructions_default.trim().is_empty() {
            errors.push(ValidationError {
                code: "EMPTY_INSTRUCTIONS".to_string(),
                message: "Инструкции приготовления не могут быть пустыми".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if recipe.servings < 1 {
            errors.push(ValidationError {
                code: "INVALID_SERVINGS".to_string(),
                message: "Количество порций должно быть минимум 1".to_string(),
                severity: ErrorSeverity::High,
            });
        }

        // 3. Check instruction length
        if recipe.instructions_default.len() < 20 {
            warnings.push(ValidationWarning {
                code: "SHORT_INSTRUCTIONS".to_string(),
                message: "Инструкции слишком короткие (менее 20 символов)".to_string(),
            });
        }

        if recipe.instructions_default.len() > 10000 {
            errors.push(ValidationError {
                code: "INSTRUCTIONS_TOO_LONG".to_string(),
                message: "Инструкции слишком длинные (более 10000 символов)".to_string(),
                severity: ErrorSeverity::Medium,
            });
        }

        // 4. Check for dangerous instructions
        self.check_food_safety(&recipe.instructions_default, &mut errors, &mut warnings);

        // 5. Check ingredients compatibility with dish type
        if let Some(ref dtype) = dish_type {
            self.check_ingredient_compatibility(recipe, dtype, &mut errors, &mut warnings);
        }

        // 6. Detect missing critical ingredients for dish type
        let missing_critical = if let Some(ref dtype) = dish_type {
            self.detect_missing_critical_ingredients(recipe, dtype)
        } else {
            Vec::new()
        };

        let is_valid = errors.iter().all(|e| e.severity != ErrorSeverity::Critical);

        ValidationResult {
            is_valid,
            errors,
            warnings,
            dish_type,
            missing_critical_ingredients: missing_critical,
        }
    }

    /// Определение типа блюда по названию
    fn detect_dish_type(&self, name: &str) -> Option<DishType> {
        let name_lower = name.to_lowercase();

        // Выпечка
        if name_lower.contains("торт") || name_lower.contains("cake") {
            return Some(DishType::Cake);
        }
        if name_lower.contains("пирог") || name_lower.contains("pie") || name_lower.contains("пирожное") {
            return Some(DishType::Pie);
        }
        if name_lower.contains("хлеб") || name_lower.contains("булка") || name_lower.contains("bread") {
            return Some(DishType::Bread);
        }

        // Десерты
        if name_lower.contains("десерт") || name_lower.contains("мороженое") || name_lower.contains("желе") {
            return Some(DishType::Dessert);
        }

        // Супы
        if name_lower.contains("суп") || name_lower.contains("борщ") || name_lower.contains("щи") 
            || name_lower.contains("рассольник") || name_lower.contains("soup") {
            return Some(DishType::Soup);
        }

        // Салаты
        if name_lower.contains("салат") || name_lower.contains("salad") {
            return Some(DishType::Salad);
        }

        // Напитки
        if name_lower.contains("напиток") || name_lower.contains("сок") || name_lower.contains("компот") 
            || name_lower.contains("коктейль") || name_lower.contains("смузи") {
            return Some(DishType::Beverage);
        }

        // По умолчанию - основное блюдо
        Some(DishType::MainCourse)
    }

    /// Проверка безопасности продуктов
    fn check_food_safety(&self, instructions: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) {
        let instructions_lower = instructions.to_lowercase();

        // Опасные паттерны
        if instructions_lower.contains("сыр") && (instructions_lower.contains("подать") || instructions_lower.contains("не готовить")) {
            // Это нормально для сырых салатов
            // Но проверим на мясо
        }

        if (instructions_lower.contains("мясо") || instructions_lower.contains("курица") || instructions_lower.contains("свинина"))
            && (instructions_lower.contains("сыр") || instructions_lower.contains("не готовить") || instructions_lower.contains("подать свеж")) {
            errors.push(ValidationError {
                code: "RAW_MEAT_DANGER".to_string(),
                message: "⚠️ ОПАСНО: Мясо должно быть термически обработано".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if instructions_lower.contains("яйц") && instructions_lower.contains("сыр") && !instructions_lower.contains("вар") && !instructions_lower.contains("жар") {
            warnings.push(ValidationWarning {
                code: "RAW_EGG_WARNING".to_string(),
                message: "Сырые яйца могут содержать сальмонеллу. Рекомендуется термическая обработка".to_string(),
            });
        }

        // Проверка на нереалистичное время
        if instructions_lower.contains("1 минут") && (instructions_lower.contains("мясо") || instructions_lower.contains("картофель")) {
            warnings.push(ValidationWarning {
                code: "UNREALISTIC_COOKING_TIME".to_string(),
                message: "Время приготовления кажется слишком коротким".to_string(),
            });
        }

        // Проверка на отсутствие термообработки для блюд, которые её требуют
        if !instructions_lower.contains("вар") && !instructions_lower.contains("жар") && !instructions_lower.contains("печь") 
            && !instructions_lower.contains("туш") && !instructions_lower.contains("готов") {
            
            if instructions_lower.contains("мясо") || instructions_lower.contains("рыба") || instructions_lower.contains("курица") {
                errors.push(ValidationError {
                    code: "NO_THERMAL_PROCESSING".to_string(),
                    message: "Не указана термическая обработка для продуктов животного происхождения".to_string(),
                    severity: ErrorSeverity::Critical,
                });
            }
        }
    }

    /// Проверка совместимости ингредиентов с типом блюда
    fn check_ingredient_compatibility(&self, recipe: &Recipe, dish_type: &DishType, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) {
        let instructions_lower = recipe.instructions_default.to_lowercase();
        let name_lower = recipe.name_default.to_lowercase();

        match dish_type {
            DishType::Cake | DishType::Pie | DishType::Bread => {
                // Для выпечки нужна мука (или альтернатива)
                if !instructions_lower.contains("мук") && !instructions_lower.contains("миндальн") 
                    && !instructions_lower.contains("крахмал") {
                    errors.push(ValidationError {
                        code: "MISSING_FLOUR_IN_BAKING".to_string(),
                        message: format!("{:?} обычно требует муку или её альтернативу", dish_type),
                        severity: ErrorSeverity::High,
                    });
                }

                // Проверка на несовместимые ингредиенты
                if instructions_lower.contains("мясо") || instructions_lower.contains("рыба") {
                    warnings.push(ValidationWarning {
                        code: "UNUSUAL_INGREDIENT_FOR_DESSERT".to_string(),
                        message: "Необычно использовать мясо/рыбу в выпечке/десерте".to_string(),
                    });
                }
            }

            DishType::Soup => {
                // Суп должен содержать жидкость
                if !instructions_lower.contains("вод") && !instructions_lower.contains("бульон") 
                    && !instructions_lower.contains("молоко") && !instructions_lower.contains("сливк") {
                    warnings.push(ValidationWarning {
                        code: "SOUP_WITHOUT_LIQUID".to_string(),
                        message: "В супе обычно используется вода, бульон или молоко".to_string(),
                    });
                }

                // Суп должен вариться
                if !instructions_lower.contains("вар") {
                    warnings.push(ValidationWarning {
                        code: "SOUP_WITHOUT_BOILING".to_string(),
                        message: "Суп обычно требует варки".to_string(),
                    });
                }
            }

            DishType::Salad => {
                // Салат обычно не варится
                if instructions_lower.contains("варить 2 часа") || instructions_lower.contains("печь") {
                    warnings.push(ValidationWarning {
                        code: "SALAD_WITH_LONG_COOKING".to_string(),
                        message: "Салаты обычно не требуют длительной термической обработки".to_string(),
                    });
                }
            }

            _ => {}
        }

        // Проверка логики: торт из овощей
        if name_lower.contains("торт") && (instructions_lower.contains("свекла") || instructions_lower.contains("капуста")) 
            && !instructions_lower.contains("морков") {  // Морковный торт - это нормально
            errors.push(ValidationError {
                code: "ILLOGICAL_INGREDIENT_COMBINATION".to_string(),
                message: "Невозможно приготовить торт из указанных овощей (свекла, капуста)".to_string(),
                severity: ErrorSeverity::High,
            });
        }
    }

    /// Определение отсутствующих критических ингредиентов
    fn detect_missing_critical_ingredients(&self, recipe: &Recipe, dish_type: &DishType) -> Vec<String> {
        let instructions_lower = recipe.instructions_default.to_lowercase();
        let mut missing = Vec::new();

        match dish_type {
            DishType::Cake | DishType::Pie => {
                if !instructions_lower.contains("мук") && !instructions_lower.contains("миндальн") {
                    missing.push("мука или миндальная мука".to_string());
                }
                if !instructions_lower.contains("сахар") && !instructions_lower.contains("мёд") && !instructions_lower.contains("подсластител") {
                    missing.push("сахар или подсластитель".to_string());
                }
                if !instructions_lower.contains("яйц") && !instructions_lower.contains("яйцо") {
                    missing.push("яйца".to_string());
                }
                if !instructions_lower.contains("масло") && !instructions_lower.contains("сливк") && !instructions_lower.contains("молоко") {
                    missing.push("жир (масло, сливки или молоко)".to_string());
                }
            }

            DishType::Bread => {
                if !instructions_lower.contains("дрожж") && !instructions_lower.contains("закваск") {
                    missing.push("дрожжи или закваска".to_string());
                }
            }

            DishType::Soup => {
                if !instructions_lower.contains("вод") && !instructions_lower.contains("бульон") {
                    missing.push("вода или бульон".to_string());
                }
            }

            _ => {}
        }

        missing
    }
}

impl Default for RecipeValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_recipe(name: &str, instructions: &str) -> Recipe {
        Recipe {
            id: crate::domain::recipe_v2::RecipeId(Uuid::new_v4()),
            tenant_id: crate::domain::TenantId(Uuid::new_v4()),
            name_default: name.to_string(),
            instructions_default: instructions.to_string(),
            servings: 4,
            language_default: "ru".to_string(),
            created_by: crate::domain::UserId(Uuid::new_v4()),
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn test_detect_cake() {
        let validator = RecipeValidator::new();
        let dtype = validator.detect_dish_type("Шоколадный торт");
        assert_eq!(dtype, Some(DishType::Cake));
    }

    #[test]
    fn test_detect_soup() {
        let validator = RecipeValidator::new();
        let dtype = validator.detect_dish_type("Борщ украинский");
        assert_eq!(dtype, Some(DishType::Soup));
    }

    #[test]
    fn test_raw_meat_danger() {
        let validator = RecipeValidator::new();
        let recipe = create_test_recipe(
            "Опасное блюдо",
            "Нарезать мясо и подать сырым"
        );
        
        let result = validator.validate(&recipe);
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "RAW_MEAT_DANGER"));
    }

    #[test]
    fn test_cake_without_flour() {
        let validator = RecipeValidator::new();
        let recipe = create_test_recipe(
            "Невозможный торт",
            "Смешать свеклу и капусту. Запечь 30 минут."
        );
        
        let result = validator.validate(&recipe);
        
        // Должна быть ошибка про муку И про логику
        assert!(result.errors.iter().any(|e| e.code == "MISSING_FLOUR_IN_BAKING" || e.code == "ILLOGICAL_INGREDIENT_COMBINATION"));
    }

    #[test]
    fn test_valid_soup() {
        let validator = RecipeValidator::new();
        let recipe = create_test_recipe(
            "Борщ классический",
            "Сварить свеклу в воде. Добавить капусту. Варить 2 часа."
        );
        
        let result = validator.validate(&recipe);
        
        assert!(result.is_valid);
        assert_eq!(result.dish_type, Some(DishType::Soup));
    }

    #[test]
    fn test_missing_critical_ingredients_for_cake() {
        let validator = RecipeValidator::new();
        let recipe = create_test_recipe(
            "Торт без ингредиентов",
            "Смешать всё и запечь"
        );
        
        let result = validator.validate(&recipe);
        
        assert!(!result.missing_critical_ingredients.is_empty());
        assert!(result.missing_critical_ingredients.iter().any(|m| m.contains("мука")));
    }
}
