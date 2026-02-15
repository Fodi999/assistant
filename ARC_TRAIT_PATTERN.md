# 🦀 Rust Arc<dyn Trait> Pattern Guide

**Правильные паттерны для trait objects в Axum/Tokio приложениях**

---

## ❌ АНТИ-ПАТТЕРНЫ (не делать так!)

### 1. Cast через `as` (почти всегда ошибка)
```rust
// ❌ НЕПРАВИЛЬНО - компилятор может неправильно вывести типы
let repo = Arc::new(ConcreteRepo::new(pool)) as Arc<dyn RepoTrait>;
```

### 2. Повторное оборачивание в Arc
```rust
// ❌ НЕПРАВИЛЬНО - создаёт Arc<Arc<T>>
let repo_impl = Arc::new(ConcreteRepo::new(pool));
let repo: Arc<dyn RepoTrait> = Arc::new(repo_impl); // Двойной Arc!
```

### 3. Использование `.as_ref()` для Arc
```rust
// ❌ НЕПРАВИЛЬНО - as_ref() возвращает &T, а не Arc<dyn Trait>
let repo = Arc::new(ConcreteRepo::new(pool));
let repo_trait = repo.as_ref() as &dyn RepoTrait; // Это ссылка, не Arc!
```

---

## ✅ ПРАВИЛЬНЫЕ ПАТТЕРНЫ

### Pattern 1: Явная переменная с типом (РЕКОМЕНДУЕТСЯ)

```rust
// 1️⃣ Создаём конкретную реализацию
let repo_impl = Arc::new(RecipeRepositoryV2::new(pool.clone()));

// 2️⃣ Поднимаем в trait object ЯВНОЙ переменной с типом
let repo: Arc<dyn RecipeV2RepositoryTrait> = repo_impl;

// 3️⃣ Передаём в сервис
let service = RecipeService::new(repo);
```

**Почему это работает:**
- Компилятор точно знает целевой тип (`Arc<dyn RecipeV2RepositoryTrait>`)
- Автоматический upcast без `as`
- Нет двойного оборачивания
- Читаемо и понятно

---

### Pattern 2: Inline с явной аннотацией типа

```rust
let repo: Arc<dyn RecipeV2RepositoryTrait> = 
    Arc::new(RecipeRepositoryV2::new(pool.clone()));

let service = RecipeService::new(repo);
```

**Когда использовать:**
- Когда переменная используется сразу
- Короткие имена типов
- Локальная область видимости

---

### Pattern 3: Turbofish для Arc::new (реже)

```rust
let repo = Arc::<dyn RecipeV2RepositoryTrait>::new(
    RecipeRepositoryV2::new(pool.clone())
);
```

**Когда использовать:**
- Когда нужен trait object сразу
- Типы короткие
- Для generic функций

---

## 🏗️ ПОЛНЫЙ ПРИМЕР (main.rs)

### Плохо (старый код)
```rust
// ❌ Проблемный код
let recipe_v2_repo = Arc::new(RecipeRepositoryV2::new(pool.clone()));
let recipe_ingredient_repo = Arc::new(RecipeIngredientRepository::new(pool.clone()));

// Потом пытаемся передать в RecipeV2Service
// который ожидает Arc<dyn Trait>, но получает Arc<ConcreteType>
let service = RecipeV2Service::new(
    recipe_v2_repo,  // ❌ Type mismatch!
    recipe_ingredient_repo,  // ❌ Type mismatch!
    // ...
);
```

### Хорошо (правильный код)
```rust
// ✅ Правильный подход
// 1️⃣ Создаём конкретные реализации
let recipe_v2_repo_impl = Arc::new(RecipeRepositoryV2::new(pool.clone()));
let recipe_ingredient_repo_impl = Arc::new(RecipeIngredientRepository::new(pool.clone()));
let recipe_translation_repo_impl = Arc::new(RecipeTranslationRepository::new(pool.clone()));
let catalog_repo_impl = Arc::new(repositories.catalog_ingredient.clone());

// 2️⃣ Поднимаем в trait objects с явными типами
let recipe_v2_repo: Arc<dyn RecipeV2RepositoryTrait> = recipe_v2_repo_impl;
let recipe_ingredient_repo: Arc<dyn RecipeIngredientRepositoryTrait> = recipe_ingredient_repo_impl;
let recipe_translation_repo: Arc<dyn RecipeTranslationRepositoryTrait> = recipe_translation_repo_impl;
let catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait> = catalog_repo_impl;

// 3️⃣ Создаём сервисы с trait objects
let recipe_translation_service = Arc::new(RecipeTranslationService::new(
    recipe_translation_repo,
    recipe_v2_repo.clone(),
    groq_service_v2,
));

let recipe_v2_service = Arc::new(RecipeV2Service::new(
    recipe_v2_repo,
    recipe_ingredient_repo,
    catalog_repo,
    recipe_translation_service,
));
```

---

## 🔍 ДИАГНОСТИКА ПРОБЛЕМ

### Ошибка: "cannot cast ... to Arc<dyn Trait>"
```
error[E0605]: non-primitive cast: `Arc<ConcreteRepo>` as `Arc<dyn RepoTrait>`
```

**Решение:** Используй Pattern 1 (явная переменная с типом)

---

### Ошибка: "expected Arc<dyn Trait>, found Arc<ConcreteType>"
```
error[E0308]: mismatched types
  expected struct `Arc<dyn RecipeV2RepositoryTrait>`
     found struct `Arc<RecipeRepositoryV2>`
```

**Решение:** Добавь явную аннотацию типа:
```rust
let repo: Arc<dyn RecipeV2RepositoryTrait> = repo_impl;
```

---

### Ошибка: "the trait `Clone` is not implemented for `dyn RepoTrait`"

**Проблема:** Trait object не реализует Clone автоматически.

**Решение 1:** Оборачивай сервис в Arc (рекомендуется для Axum)
```rust
#[derive(Clone)]
pub struct AppState {
    pub recipe_service: Arc<RecipeService>, // Не требует RecipeService: Clone
}
```

**Решение 2:** Добавь Clone в суперtrаit (реже)
```rust
pub trait RecipeV2RepositoryTrait: Send + Sync + Clone {
    // ...
}
```

---

## 🎯 ПРАВИЛА БОЛЬШОГО ПАЛЬЦА

### DO ✅
1. **Всегда используй явные переменные с типами** для trait objects
2. **Создавай `_impl` переменные** перед upcast
3. **Оборачивай сервисы в Arc** для Axum State
4. **Проверяй `cargo check`** перед коммитом

### DON'T ❌
1. **Не используй `as Arc<dyn Trait>`** - почти всегда ошибка
2. **Не оборачивай Arc дважды** - проверь где создаётся Arc
3. **Не используй `.as_ref()`** для преобразования Arc
4. **Не передавай concrete types** в функции ожидающие trait objects

---

## 📚 ССЫЛКИ

- [Rust Book: Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html)
- [Arc<dyn Trait> patterns](https://www.rustnote.com/blog/arc-dyn-trait/)
- [Axum State management](https://docs.rs/axum/latest/axum/extract/struct.State.html)

---

**Последнее обновление:** 2026-02-15  
**Применено в:** feature/recipes-v2 (commit 39833e6)
