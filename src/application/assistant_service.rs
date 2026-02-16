use crate::application::{InventoryAlertService, InventoryService, DishService};
use crate::domain::assistant::{
    command::AssistantCommand,
    response::AssistantResponse,
    rules::next_step,
};
use crate::domain::{CatalogIngredientId, RecipeId, DishName, Money};
use crate::infrastructure::persistence::{
    AssistantStateRepository, AssistantStateRepositoryTrait, UserRepository,
    UserRepositoryTrait,
};
use crate::shared::{result::AppResult, types::{TenantId, UserId}};

#[derive(Clone)]
pub struct AssistantService {
    state_repo: AssistantStateRepository,
    user_repo: UserRepository,
    inventory_service: InventoryService,
    inventory_alert_service: InventoryAlertService,
    dish_service: DishService,
}

impl AssistantService {
    pub fn new(
        state_repo: AssistantStateRepository,
        user_repo: UserRepository,
        inventory_service: InventoryService,
        inventory_alert_service: InventoryAlertService,
        dish_service: DishService,
    ) -> Self {
        Self {
            state_repo,
            user_repo,
            inventory_service,
            inventory_alert_service,
            dish_service,
        }
    }

    /// Получить текущее состояние пользователя
    pub async fn get_state(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<AssistantResponse> {
        let state = self.state_repo.get_or_create(user_id, tenant_id).await?;
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| crate::shared::AppError::not_found("User not found"))?;
        
        let mut response = state.current_step.to_response(user.language);
        
        // Обогащаем response проверкой просрочки (только на экранах с inventory)
        if matches!(state.current_step, crate::domain::assistant::step::AssistantStep::InventorySetup | crate::domain::assistant::step::AssistantStep::RecipeSetup) {
            self.enrich_with_inventory_warnings(&mut response, user_id, tenant_id, user.language).await?;
        }
        
        Ok(response)
    }

    /// Выполнить команду и получить новое состояние
    pub async fn handle_command(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        command: AssistantCommand,
    ) -> AppResult<AssistantResponse> {
        // Получаем текущий state
        let state = self.state_repo.get_or_create(user_id, tenant_id).await?;
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| crate::shared::AppError::not_found("User not found"))?;

        // Обрабатываем команду AddProduct - сохраняем продукт в inventory
        if let AssistantCommand::AddProduct(payload) = &command {
            let catalog_id = CatalogIngredientId::from_uuid(payload.catalog_ingredient_id);
            self.inventory_service
                .add_product(
                    user_id,
                    tenant_id,
                    catalog_id,
                    payload.price_per_unit_cents,
                    payload.quantity,
                    payload.received_at.unwrap_or_else(|| time::OffsetDateTime::now_utc()),
                    payload.expires_at,
                )
                .await?;
        }

        // Обрабатываем команду CreateDish - создаем блюдо с финансовым анализом
        let mut dish_financials = None;
        if let AssistantCommand::CreateDish(payload) = &command {
            let recipe_id = RecipeId::from_uuid(payload.recipe_id);
            let dish_name = DishName::new(payload.name.clone())?;
            let selling_price = Money::from_cents(payload.selling_price_cents as i64)?;
            
            // Создаем блюдо
            let dish = self.dish_service
                .create_dish(
                    tenant_id,
                    recipe_id,
                    dish_name,
                    payload.description.clone(),
                    selling_price,
                )
                .await?;

            // Сразу рассчитываем финансы для "момент вау"
            let financials = self.dish_service
                .calculate_financials(dish.id(), user_id, tenant_id)
                .await?;

            tracing::info!(
                "Dish created: {} | Selling: {} PLN | Cost: {} PLN | Profit: {} PLN | Margin: {:.1}%",
                financials.dish_name,
                financials.selling_price_cents as f64 / 100.0,
                financials.recipe_cost_cents as f64 / 100.0,
                financials.profit_cents as f64 / 100.0,
                financials.profit_margin_percent
            );

            // Сохраняем для добавления в response
            dish_financials = Some(financials);
        }

        // Проверяем FinishInventory - должны быть продукты
        if matches!(command, AssistantCommand::FinishInventory) {
            let has_products = self.inventory_service.has_products(user_id, tenant_id).await?;
            if !has_products {
                return Err(crate::shared::AppError::validation(
                    "Cannot finish inventory: no products added yet",
                ));
            }
        }

        // Применяем правило перехода
        let next = next_step(state.current_step, &command);

        // Сохраняем новый step (только если изменился)
        if next != state.current_step {
            self.state_repo.update_step(user_id, tenant_id, next).await?;
        }

        // Получаем базовый response
        let mut response = next.to_response(user.language);

        // Добавляем финансовый анализ, если создавали блюдо
        if let Some(ref financials) = dish_financials {
            response.dish_financials = Some(financials.clone());

            // Генерируем предупреждения о рентабельности
            if !financials.is_healthy_margin() {
                let message = match user.language {
                    crate::shared::Language::Pl => format!(
                        "⚠️ Niska marża zysku ({:.1}%)! Rozważ zwięksenie ceny lub obniżenie kosztów.",
                        financials.profit_margin_percent
                    ),
                    crate::shared::Language::En => format!(
                        "⚠️ Low profit margin ({:.1}%)! Consider increasing the price or reducing costs.",
                        financials.profit_margin_percent
                    ),
                    crate::shared::Language::Uk => format!(
                        "⚠️ Низька маржа прибутку ({:.1}%)! Розгляньте збільшення ціни або зниження витрат.",
                        financials.profit_margin_percent
                    ),
                    crate::shared::Language::Ru => format!(
                        "⚠️ Низкая маржа прибыли ({:.1}%)! Рассмотрите увеличение цены или снижение затрат.",
                        financials.profit_margin_percent
                    ),
                };
                response.warnings.push(crate::domain::assistant::response::AssistantWarning {
                    level: crate::domain::assistant::response::WarningLevel::Financial,
                    message,
                });
            }
            
            if !financials.is_acceptable_food_cost() {
                let message = match user.language {
                    crate::shared::Language::Pl => format!(
                        "⚠️ Wysoki koszt produktów ({:.1}%)! Przepis może бути нерентабельний.",
                        financials.food_cost_percent
                    ),
                    crate::shared::Language::En => format!(
                        "⚠️ High food cost ({:.1}%)! Recipe may not be profitable.",
                        financials.food_cost_percent
                    ),
                    crate::shared::Language::Uk => format!(
                        "⚠️ Висока вартість продуктів ({:.1}%)! Рецепт може бути нерентабельним.",
                        financials.food_cost_percent
                    ),
                    crate::shared::Language::Ru => format!(
                        "⚠️ Высокая стоимость продуктов ({:.1}%)! Рецепт может быть нерентабельным.",
                        financials.food_cost_percent
                    ),
                };
                response.warnings.push(crate::domain::assistant::response::AssistantWarning {
                    level: crate::domain::assistant::response::WarningLevel::Financial,
                    message,
                });
            }
        }

        // Обогащаем response проверкой просрочки (только на экранах с inventory)
        if matches!(next, crate::domain::assistant::step::AssistantStep::InventorySetup | crate::domain::assistant::step::AssistantStep::RecipeSetup) {
            self.enrich_with_inventory_warnings(&mut response, user_id, tenant_id, user.language).await?;
        }

        Ok(response)
    }

    /// Enrich response with inventory expiration warnings
    async fn enrich_with_inventory_warnings(
        &self,
        response: &mut AssistantResponse,
        _user_id: UserId,
        tenant_id: TenantId,
        language: crate::shared::Language,
    ) -> AppResult<()> {
        use crate::domain::assistant::response::{AssistantWarning, WarningLevel};
        use crate::shared::Language;

        let status = self.inventory_alert_service.get_inventory_status(tenant_id).await?;

        // ❌ Critical: Expired products
        if status.expired > 0 {
            let message = match language {
                Language::Pl => format!("⚠️ Masz {} przeterminowanych produktów", status.expired),
                Language::En => format!("⚠️ There are {} expired products in inventory", status.expired),
                Language::Uk => format!("⚠️ У вас є {} прострочених продуктів", status.expired),
                Language::Ru => format!("⚠️ У вас {} просроченных продуктов", status.expired),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Critical,
                message,
            });
        }

        // ⏰ Warning: Critical (0-1 days)
        if status.critical > 0 {
            let message = match language {
                Language::Pl => format!("⏰ {} produktów wymaga pilnej uwagi (0-1 dni)", status.critical),
                Language::En => format!("⏰ {} products need urgent attention (0-1 days left)", status.critical),
                Language::Uk => format!("⏰ {} продуктів потребують термінової уваги (0-1 днів)", status.critical),
                Language::Ru => format!("⏰ {} продуктов требуют срочного внимания (0-1 дней)", status.critical),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Warning,
                message,
            });
        }

        // 📦 Warning: Low Stock
        if status.low_stock > 0 {
            let message = match language {
                Language::Pl => format!("📦 {} produktów ma niski stan magazynowy", status.low_stock),
                Language::En => format!("📦 {} products are low on stock", status.low_stock),
                Language::Uk => format!("📦 {} продуктів мають низький рівень запасу", status.low_stock),
                Language::Ru => format!("📦 {} продуктов заканчиваются на складе", status.low_stock),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Warning,
                message,
            });
        }

        Ok(())
    }
}