# 🍳 Recipe System Implementation Plan

**Date**: 15 февраля 2026  
**Status**: Planning  
**Priority**: P1 - After inventory bug fix

---

## 📋 Overview

Двухслойная система рецептов:
1. **Private Recipes** - личные рецепты пользователя с расчетом себестоимости
2. **Public Recipes** - опубликованные рецепты с AI-переводами на все языки

---

## 🗄️ Database Schema

### ✅ Уже существует:
- `recipes` - базовая таблица (возможно нужно дополнить)
- `recipe_ingredients` - ингредиенты рецептов
- Domain модели в `src/domain/recipe.rs`

### ❓ Нужно проверить/добавить:
- [ ] Таблица `recipe_translations` для AI-переводов
- [ ] Поля `is_public`, `published_at` в `recipes`
- [ ] Поля для default language (`name_default`, `language_default`)
- [ ] Cost calculation fields (`total_cost_cents`, `cost_per_serving_cents`)

---

## 🎯 Implementation Steps

### STEP 1: Database Migration ✅
**File**: `migrations/20260215_create_recipes.sql`

**Задачи**:
- [ ] Проверить существующую схему `recipes`
- [ ] Добавить недостающие поля если нужно
- [ ] Создать `recipe_translations` если не существует
- [ ] Добавить индексы для производительности
- [ ] Запустить миграцию

**SQL**:
```sql
-- Проверить текущую схему
\d recipes
\d recipe_translations

-- Если нет recipe_translations - создать
CREATE TABLE recipe_translations (
    id UUID PRIMARY KEY,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    language VARCHAR(5) NOT NULL CHECK (language IN ('ru', 'en', 'pl', 'uk')),
    name TEXT NOT NULL,
    instructions TEXT NOT NULL,
    translated_by VARCHAR(20) DEFAULT 'ai',
    translated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(recipe_id, language)
);
```

---

### STEP 2: Domain Models
**Files**: 
- `src/domain/recipe.rs` ✅ (существует)
- `src/domain/recipe_translation.rs` (новый)

**Задачи**:
- [ ] Дополнить `Recipe` struct полями для publishing
- [ ] Создать `RecipeTranslation` struct
- [ ] Добавить методы `publish()`, `unpublish()`
- [ ] Добавить метод `calculate_costs()`

**Пример**:
```rust
impl Recipe {
    pub fn publish(&mut self) {
        self.is_public = true;
        self.published_at = Some(chrono::Utc::now().naive_utc());
    }

    pub fn calculate_costs(&mut self, ingredients_total: i64) {
        self.total_cost_cents = ingredients_total;
        self.cost_per_serving_cents = ingredients_total / self.servings as i64;
    }
}
```

---

### STEP 3: Translation Service
**File**: `src/application/recipe_translation_service.rs` (новый)

**Задачи**:
- [ ] Создать `RecipeTranslationService`
- [ ] Метод `translate_recipe()` - вызывает Groq API
- [ ] Переводит сразу на все 3 языка (ru/en/pl/uk кроме default)
- [ ] Сохраняет в `recipe_translations`

**Пример**:
```rust
pub struct RecipeTranslationService {
    groq_service: Arc<GroqService>,
    translation_repo: Arc<RecipeTranslationRepository>,
}

impl RecipeTranslationService {
    pub async fn translate_recipe(
        &self,
        recipe_id: RecipeId,
        name: &str,
        instructions: &str,
        from_lang: Language,
    ) -> AppResult<Vec<RecipeTranslation>> {
        let target_langs = vec!["ru", "en", "pl", "uk"]
            .into_iter()
            .filter(|&lang| lang != from_lang.code())
            .collect::<Vec<_>>();

        let mut translations = Vec::new();
        for target_lang in target_langs {
            let translated_name = self.groq_service
                .translate(name, from_lang.code(), target_lang)
                .await?;
            
            let translated_instructions = self.groq_service
                .translate(instructions, from_lang.code(), target_lang)
                .await?;

            let translation = RecipeTranslation::new(
                recipe_id,
                Language::from_code(target_lang)?,
                translated_name,
                translated_instructions,
                TranslationSource::AI,
            );

            self.translation_repo.save(&translation).await?;
            translations.push(translation);
        }

        Ok(translations)
    }
}
```

---

### STEP 4: Recipe Service (Create + Cost)
**File**: `src/application/recipe_service.rs`

**Задачи**:
- [ ] Метод `create_recipe()` - создает рецепт + переводы
- [ ] Расчет себестоимости из inventory
- [ ] Связь с `inventory_products`
- [ ] Cost snapshot в `recipe_ingredients`

**Flow**:
```
1. User creates recipe
   ↓
2. Backend validates ingredients
   ↓
3. Calculate total cost from inventory prices
   ↓
4. Save recipe
   ↓
5. Generate AI translations (async)
   ↓
6. Return DTO
```

**Пример**:
```rust
pub async fn create_recipe(
    &self,
    user_id: UserId,
    tenant_id: TenantId,
    payload: CreateRecipePayload,
) -> AppResult<RecipeResponse> {
    // 1. Validate ingredients exist in user's inventory
    let mut total_cost = 0i64;
    let mut ingredients = Vec::new();

    for ing in payload.ingredients {
        let inventory_product = self.inventory_service
            .get_product(tenant_id, ing.inventory_product_id)
            .await?;

        let cost = (ing.quantity * inventory_product.price_per_unit_cents as f64) as i64;
        total_cost += cost;

        ingredients.push(RecipeIngredient::new(
            recipe_id,
            ing.inventory_product_id,
            ing.quantity,
            inventory_product.price_per_unit_cents,
            ing.display_order.unwrap_or(0),
        ));
    }

    // 2. Create recipe
    let mut recipe = Recipe::new(
        user_id.as_uuid(),
        tenant_id.as_uuid(),
        payload.name,
        payload.instructions,
        payload.language,
        payload.servings,
        payload.prep_time_minutes,
        payload.cook_time_minutes,
    );
    recipe.calculate_costs(total_cost);

    // 3. Save recipe
    self.recipe_repo.save(&recipe).await?;

    // 4. Save ingredients
    for ingredient in ingredients {
        self.recipe_ingredient_repo.save(&ingredient).await?;
    }

    // 5. Generate translations (async - don't wait)
    let translation_service = self.translation_service.clone();
    let recipe_id = recipe.id;
    let name = recipe.name_default.clone();
    let instructions = recipe.instructions_default.clone();
    let language = recipe.language_default;
    
    tokio::spawn(async move {
        let _ = translation_service
            .translate_recipe(recipe_id, &name, &instructions, language)
            .await;
    });

    // 6. Return response
    Ok(RecipeResponse::from_recipe(recipe))
}
```

---

### STEP 5: Publish/Unpublish
**File**: `src/application/recipe_service.rs`

**Задачи**:
- [ ] Метод `publish_recipe()`
- [ ] Метод `unpublish_recipe()`
- [ ] Проверка прав доступа (только владелец)

**Пример**:
```rust
pub async fn publish_recipe(
    &self,
    user_id: UserId,
    recipe_id: RecipeId,
) -> AppResult<()> {
    let mut recipe = self.recipe_repo
        .find_by_id(recipe_id)
        .await?
        .ok_or_else(|| AppError::not_found("Recipe not found"))?;

    // Check ownership
    if recipe.user_id != user_id.as_uuid() {
        return Err(AppError::forbidden("You don't own this recipe"));
    }

    recipe.publish();
    self.recipe_repo.update(&recipe).await?;

    Ok(())
}
```

---

### STEP 6: Public Feed
**File**: `src/application/recipe_service.rs`

**Задачи**:
- [ ] Метод `get_public_recipes()` - с пагинацией
- [ ] SQL с LEFT JOIN на `recipe_translations`
- [ ] Fallback на default language
- [ ] Фильтры (категория, язык, сортировка)

**SQL Example**:
```sql
SELECT 
    r.id,
    COALESCE(rt.name, r.name_default) as name,
    COALESCE(rt.instructions, r.instructions_default) as instructions,
    r.servings,
    r.total_cost_cents,
    r.cost_per_serving_cents,
    r.published_at,
    u.display_name as author_name
FROM recipes r
LEFT JOIN recipe_translations rt 
    ON rt.recipe_id = r.id 
   AND rt.language = $1
INNER JOIN users u ON u.id = r.user_id
WHERE r.is_public = true
ORDER BY r.published_at DESC
LIMIT $2 OFFSET $3
```

---

### STEP 7: API Endpoints
**File**: `src/interfaces/http/recipes.rs` (новый)

**Endpoints**:
```
POST   /api/recipes              - Create recipe
GET    /api/recipes              - List user's recipes
GET    /api/recipes/{id}         - Get recipe details
PUT    /api/recipes/{id}         - Update recipe
DELETE /api/recipes/{id}         - Delete recipe

POST   /api/recipes/{id}/publish  - Publish to public feed
POST   /api/recipes/{id}/unpublish - Remove from public feed

GET    /api/recipes/public        - Get public recipes (paginated)
GET    /api/recipes/public/{id}   - Get public recipe details
```

---

### STEP 8: Frontend Integration
**Files**:
- `frontend/src/services/recipes.ts`
- `frontend/src/pages/recipes/index.tsx`
- `frontend/src/pages/recipes/[id].tsx`
- `frontend/src/pages/recipes/create.tsx`
- `frontend/src/pages/recipes/public.tsx`

**Components**:
- `RecipeCard` - preview card
- `RecipeForm` - create/edit form
- `RecipeIngredientSelector` - search from inventory
- `RecipeCostDisplay` - show costs
- `PublicRecipeFeed` - browse public recipes

---

## 🎨 UI/UX Features

### Private Recipes Page
- List of user's recipes
- Cost breakdown per serving
- Profit margin calculator
- Edit/Delete actions
- "Publish" button

### Create Recipe Form
1. Recipe name & instructions
2. Select ingredients from inventory
   - Search by name
   - Auto-fill price
   - Calculate total cost
3. Set servings count
4. Auto-calculate cost per serving
5. Preview translations (after save)

### Public Feed
- Browse published recipes
- Filter by category/language
- Search
- View recipe with translations
- "Copy to my recipes" button

---

## 🚀 Deployment Plan

### Phase 1: Backend (1-2 days)
1. ✅ Database migration
2. ✅ Domain models
3. ✅ Services (create, cost calculation)
4. ✅ API endpoints
5. ✅ Tests

### Phase 2: Translations (1 day)
1. ✅ Translation service
2. ✅ Groq integration
3. ✅ Async translation job
4. ✅ Error handling

### Phase 3: Publishing (1 day)
1. ✅ Publish/unpublish logic
2. ✅ Public feed endpoint
3. ✅ Access control
4. ✅ Tests

### Phase 4: Frontend (2-3 days)
1. ✅ Recipe list page
2. ✅ Create recipe form
3. ✅ Recipe details page
4. ✅ Public feed page
5. ✅ Responsive design

### Phase 5: Polish & Launch (1 day)
1. ✅ Error messages
2. ✅ Loading states
3. ✅ Empty states
4. ✅ Production deploy

---

## ✅ Testing Checklist

### Backend
- [ ] Create recipe with ingredients
- [ ] Cost calculation correct
- [ ] Translations generated (all 3 languages)
- [ ] Publish/unpublish works
- [ ] Public feed returns correct data
- [ ] Tenant isolation (can't see other's private recipes)
- [ ] User can only publish own recipes

### Frontend
- [ ] Recipe form validation
- [ ] Ingredient selector works
- [ ] Cost display updates
- [ ] Translations display correctly
- [ ] Public feed loads
- [ ] Mobile responsive

---

## 💰 Cost Estimation

### Groq API Cost
- 3 translations per recipe (ru/en/pl/uk minus default)
- ~500 tokens per translation (name + instructions)
- Cost: ~$0.003 per recipe
- 1000 recipes = $3

**Very affordable!**

---

## 🔮 Future Enhancements

- [ ] Recipe ratings & reviews
- [ ] Favorite recipes
- [ ] Shopping list generation
- [ ] Nutrition calculator
- [ ] Recipe categories/tags
- [ ] Image upload for recipes
- [ ] Video instructions
- [ ] Share recipes via link
- [ ] Export to PDF
- [ ] Print-friendly format

---

## 📝 Notes

**Translation Strategy**: 
✅ Translate immediately on create (all 3 languages)
- Pros: No latency, predictable cost, better UX
- Cons: Upfront API call (but async, doesn't block response)

**Alternative** (not recommended):
❌ Translate on-demand when user switches language
- Pros: Lower upfront cost
- Cons: Latency, complexity, worse UX

**Decision**: Go with immediate translation.

---

*Ready to implement!*  
*Start with Step 1: Verify existing schema*
