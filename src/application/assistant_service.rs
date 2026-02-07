use crate::application::{InventoryService, RecipeService, DishService};
use crate::domain::assistant::{
    command::{AssistantCommand, AddProductPayload, CreateDishPayload},
    response::AssistantResponse,
    rules::next_step,
    step::AssistantStep,
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
    recipe_service: RecipeService,
    dish_service: DishService,
}

impl AssistantService {
    pub fn new(
        state_repo: AssistantStateRepository,
        user_repo: UserRepository,
        inventory_service: InventoryService,
        recipe_service: RecipeService,
        dish_service: DishService,
    ) -> Self {
        Self {
            state_repo,
            user_repo,
            inventory_service,
            recipe_service,
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
                    payload.expires_at,
                )
                .await?;
        }

        // Обрабатываем команду CreateDish - создаем блюдо с финансовым анализом
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

            // Сразу рассчитываем финансы для "вау"-эффекта
            let financials = self.dish_service
                .calculate_financials(dish.id(), user_id, tenant_id)
                .await?;

            // TODO: Add financials to response (need to extend AssistantResponse)
            // For now, just log it
            tracing::info!(
                "Dish created: {} | Selling: {} PLN | Cost: {} PLN | Profit: {} PLN | Margin: {:.1}%",
                financials.dish_name,
                financials.selling_price_cents as f64 / 100.0,
                financials.recipe_cost_cents as f64 / 100.0,
                financials.profit_cents as f64 / 100.0,
                financials.profit_margin_percent
            );
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
            self.state_repo.update_step(user_id, next).await?;
        }

        // Получаем базовый response
        let mut response = next.to_response(user.language);

        // Обогащаем response проверкой просрочки (только на экранах с inventory)
        if matches!(next, crate::domain::assistant::step::AssistantStep::InventorySetup | crate::domain::assistant::step::AssistantStep::RecipeSetup) {
            self.enrich_with_inventory_warnings(&mut response, user_id, tenant_id, user.language).await?;
        }

        Ok(response)
    }

    /// Enrich response with inventory expiration warnings
    async fn enrich_with_inventory_warnings(
        &self,
        response: &mut crate::domain::assistant::response::AssistantResponse,
        user_id: UserId,
        tenant_id: TenantId,
        language: crate::shared::Language,
    ) -> AppResult<()> {
        use crate::domain::assistant::response::{AssistantWarning, WarningLevel};
        use crate::shared::Language;

        let status = self.inventory_service.get_status(user_id, tenant_id).await?;

        // ❌ Critical: Expired products
        if status.expired > 0 {
            let message = match language {
                Language::Pl => format!("⚠️ W magazynie {} przeterminowany produkt", if status.expired == 1 { "jest" } else { "są" }),
                Language::En => format!("⚠️ There {} {} expired product{} in inventory", 
                    if status.expired == 1 { "is" } else { "are" },
                    status.expired,
                    if status.expired == 1 { "" } else { "s" }
                ),
                Language::Uk => format!("⚠️ У складі {} прострочений продукт", if status.expired == 1 { "є" } else { "є" }),
                Language::Ru => format!("⚠️ На складе {} просроченный продукт", if status.expired == 1 { "есть" } else { "есть" }),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Critical,
                message,
            });
        }

        // ⚠️ Warning: Expiring today
        if status.expiring_today > 0 {
            let message = match language {
                Language::Pl => format!("⏰ {} produkt{} wygasa dziś", status.expiring_today, if status.expiring_today == 1 { "" } else { "y" }),
                Language::En => format!("⏰ {} product{} expire{} today", status.expiring_today, if status.expiring_today == 1 { "" } else { "s" }, if status.expiring_today == 1 { "s" } else { "" }),
                Language::Uk => format!("⏰ {} продукт{} закінчується сьогодні", status.expiring_today, if status.expiring_today == 1 { "" } else { "ів" }),
                Language::Ru => format!("⏰ {} продукт{} истекает сегодня", status.expiring_today, if status.expiring_today == 1 { "" } else { "ов" }),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Warning,
                message,
            });
        }

        // ⚠️ Info: Expiring soon
        if status.expiring_soon > 0 {
            let message = match language {
                Language::Pl => format!("ℹ️ {} produkt{} wkrótce się przeterminuje (2 dni)", status.expiring_soon, if status.expiring_soon == 1 { "" } else { "y" }),
                Language::En => format!("ℹ️ {} product{} will expire soon (2 days)", status.expiring_soon, if status.expiring_soon == 1 { "" } else { "s" }),
                Language::Uk => format!("ℹ️ {} продукт{} скоро закінчиться (2 дні)", status.expiring_soon, if status.expiring_soon == 1 { "" } else { "ів" }),
                Language::Ru => format!("ℹ️ {} продукт{} скоро истечет (2 дня)", status.expiring_soon, if status.expiring_soon == 1 { "" } else { "ов" }),
            };
            response.warnings.push(AssistantWarning {
                level: WarningLevel::Info,
                message,
            });
        }

        Ok(())
    }
}