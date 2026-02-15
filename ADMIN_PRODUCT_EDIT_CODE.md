# 📝 Редактирование продукта в админ каталоге

## API Эндпоинт

```
PUT /api/admin/products/:id
Authorization: Bearer <admin_token>
Content-Type: application/json
```

## Структуры данных

### Request (UpdateProductRequest)

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name_en: Option<String>,      // Английское название
    pub name_pl: Option<String>,      // Польское название
    pub name_uk: Option<String>,      // Украинское название
    pub name_ru: Option<String>,      // Русское название
    pub category_id: Option<Uuid>,    // ID категории
    pub unit: Option<UnitType>,       // Единица измерения (штука, кг, литр и т.д.)
    pub description: Option<String>,  // Описание
}
```

**Все поля опциональны** - при обновлении отправляются только те поля, которые нужно изменить.

### Response (ProductResponse)

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name_en: String,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Uuid,
    pub unit: UnitType,
    pub description: Option<String>,
    pub image_url: Option<String>,
}
```

## Backend код

### HTTP Handler (src/interfaces/http/admin_catalog.rs)

```rust
/// Update product
pub async fn update_product(
    _claims: AdminClaims,                           // Проверка, что это админ
    Path(id): Path<Uuid>,                           // ID продукта из URL
    State(service): State<AdminCatalogService>,     // Сервис каталога
    Json(req): Json<UpdateProductRequest>,          // Данные для обновления
) -> Result<Json<ProductResponse>, AppError> {
    let product = service.update_product(id, req).await?;
    Ok(Json(product))
}
```

### Service Logic (src/application/admin_catalog.rs)

```rust
/// Update product
pub async fn update_product(
    &self,
    id: Uuid,
    req: UpdateProductRequest,
) -> AppResult<ProductResponse> {
    // 1️⃣ Проверяем, что продукт существует
    let existing = self.get_product_by_id(id).await?;

    // 2️⃣ Если обновляется name_en, проверяем на дубликаты
    if let Some(ref new_name_en) = req.name_en {
        let name_en = new_name_en.trim();
        if name_en.is_empty() {
            return Err(AppError::validation("name_en cannot be empty"));
        }

        // Ищем другой продукт с таким же именем
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM catalog_ingredients 
             WHERE LOWER(name_en) = LOWER($1) AND id != $2 
             AND COALESCE(is_active, true) = true)"
        )
        .bind(name_en)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Err(AppError::conflict(&format!(
                "Product '{}' already exists",
                name_en
            )));
        }
    }

    // 3️⃣ Нормализуем переводы (если пусто, используем English)
    let name_en = req.name_en.as_deref().map(|s| s.trim().to_string());
    let final_name_en = name_en.as_deref().unwrap_or(&existing.name_en);
    
    let name_pl = req.name_pl.as_deref()
        .map(|s| normalize_translation(s, final_name_en));
    let name_uk = req.name_uk.as_deref()
        .map(|s| normalize_translation(s, final_name_en));
    let name_ru = req.name_ru.as_deref()
        .map(|s| normalize_translation(s, final_name_en));

    // 4️⃣ Выполняем UPDATE запрос
    let product = sqlx::query_as::<_, ProductResponse>(
        r#"
        UPDATE catalog_ingredients
        SET
            name_en = COALESCE($2, name_en),
            name_pl = COALESCE($3, name_pl),
            name_uk = COALESCE($4, name_uk),
            name_ru = COALESCE($5, name_ru),
            category_id = COALESCE($6, category_id),
            default_unit = COALESCE($7, default_unit),
            description = COALESCE($8, description)
        WHERE id = $1 AND COALESCE(is_active, true) = true
        RETURNING
            id, name_en, name_pl, name_uk, name_ru,
            category_id,
            default_unit as unit,
            description,
            image_url
        "#
    )
    .bind(id)
    .bind(&name_en)
    .bind(&name_pl)
    .bind(&name_uk)
    .bind(&name_ru)
    .bind(req.category_id)
    .bind(&req.unit)
    .bind(&req.description)
    .fetch_one(&self.pool)
    .await?;

    Ok(product)
}
```

### Helper function (нормализация переводов)

```rust
/// Если перевод пуст, используем английский текст как fallback
fn normalize_translation(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}
```

## Маршруты (src/interfaces/http/routes.rs)

```rust
let admin_catalog_routes = Router::new()
    .route("/products", get(admin_catalog::list_products))
    .route("/products/:id", get(admin_catalog::get_product))
    .route("/products", post(admin_catalog::create_product))
    .route("/products/:id", axum::routing::put(admin_catalog::update_product))  // 👈 UPDATE
    .route("/products/:id", axum::routing::delete(admin_catalog::delete_product))
    .route("/products/:id/image", post(admin_catalog::upload_product_image))
    .route("/products/:id/image", axum::routing::delete(admin_catalog::delete_product_image))
    .layer(admin_catalog_middleware)  // Проверка JWT токена админа
    .with_state(admin_catalog_service);
```

## Пример использования

### cURL

```bash
# Получить токен админа
TOKEN=$(curl -s -X POST "https://your-api.com/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' | jq -r '.token')

# Обновить только название на английском
curl -X PUT "https://your-api.com/api/admin/products/fb52875b-7947-4089-a84c-23d88cfbe2b5" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Pineapple Updated"
  }'

# Обновить всё сразу
curl -X PUT "https://your-api.com/api/admin/products/fb52875b-7947-4089-a84c-23d88cfbe2b5" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Pineapple",
    "name_ru": "Ананас",
    "name_pl": "Ananas",
    "name_uk": "Ананас",
    "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
    "unit": "штука",
    "description": "Tropical fruit"
  }'
```

### JavaScript/TypeScript

```typescript
async function updateProduct(
  productId: string,
  updates: {
    name_en?: string;
    name_ru?: string;
    name_pl?: string;
    name_uk?: string;
    category_id?: string;
    unit?: string;
    description?: string;
  }
) {
  const token = localStorage.getItem('admin_token');
  
  const response = await fetch(
    `https://your-api.com/api/admin/products/${productId}`,
    {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(updates),
    }
  );

  if (!response.ok) {
    throw new Error(`Failed to update product: ${response.statusText}`);
  }

  return await response.json();
}

// Использование
updateProduct('fb52875b-7947-4089-a84c-23d88cfbe2b5', {
  name_en: 'Pineapple',
  name_ru: 'Ананас',
  description: 'Tropical fruit',
}).then(product => {
  console.log('Updated:', product);
});
```

## Особенности

### 1. Опциональные поля
- Отправляете только те поля, которые нужно изменить
- Остальные сохраняют свои значения
- Используется `COALESCE` в SQL: `COALESCE($2, name_en)` - если $2 NULL, используется текущее значение

### 2. Проверка дубликатов
- При обновлении name_en проверяется что такого имени нет (кроме текущего продукта)
- Сравнение case-insensitive: `LOWER(name_en) = LOWER($1)`

### 3. Нормализация переводов
- Если перевод пуст, используется English текст
- `normalize_translation()` гарантирует что все языки заполнены

### 4. Soft Delete
- Продукты не удаляются, а деактивируются (`is_active = false`)
- Это сохраняет relationships с другими таблицами (inventory, recipes)

### 5. Безопасность
- Защищено middleware `require_admin`
- JWT токен проверяется в каждом запросе
- Только активные продукты редактируются

## Ошибки

```json
// Пустое имя на английском
{
  "error": "validation error",
  "message": "name_en cannot be empty"
}

// Дубликат имени
{
  "error": "conflict",
  "message": "Product 'Apple' already exists"
}

// Продукт не найден
{
  "error": "not found",
  "message": "Product not found"
}
```

---

**Статус:** ✅ Полная документация кода редактирования продукта
