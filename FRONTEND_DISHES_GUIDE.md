# Руководство по реализации раздела "Блюда" (Dishes) на фронтенде

Раздел "Блюда" (Меню) — это место, где рецепты превращаются в коммерческие продукты. Здесь пользователь устанавливает цену продажи, а система автоматически рассчитывает себестоимость (Food Cost) и маржинальность (Profit Margin).

## 1. Архитектура раздела

Раздел должен состоять из двух основных экранов:
1. **Список блюд (Меню)** — таблица или карточки со всеми блюдами, их ценами, себестоимостью и маржой.
2. **Модалка/Страница создания блюда** — форма, где пользователь выбирает рецепт и задает цену.

---

## 2. API Клиент (TypeScript)

Добавьте эти функции в ваш API-клиент (например, `src/api/dishes.ts`):

```typescript
// Типы данных
export interface Dish {
  id: string;
  recipe_id: string;
  name: string;
  description: string | null;
  selling_price_cents: number;
  recipe_cost_cents: number | null;
  food_cost_percent: number | null;
  profit_margin_percent: number | null;
  active: boolean;
}

export interface CreateDishPayload {
  recipe_id: string;
  name: string;
  description?: string;
  selling_price_cents: number;
}

// API вызовы
export const dishesApi = {
  // Получить список всех блюд
  list: async (): Promise<Dish[]> => {
    const response = await apiFetch('/api/dishes');
    return response.json();
  },

  // Создать новое блюдо из рецепта
  create: async (payload: CreateDishPayload): Promise<Dish> => {
    const response = await apiFetch('/api/dishes', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    return response.json();
  },

  // Пересчитать себестоимость всех блюд (если цены на складе изменились)
  recalculateAll: async (): Promise<void> => {
    await apiFetch('/api/dishes/recalculate-all', { method: 'POST' });
  }
};
```

---

## 3. Компонент: Форма создания блюда (React / Next.js)

Когда пользователь нажимает "Добавить в меню" из карточки рецепта или на странице блюд:

```tsx
import { useState, useEffect } from 'react';
import { dishesApi } from '@/api/dishes';
import { recipesApi } from '@/api/recipes'; // Предполагается, что у вас есть API для рецептов

export function CreateDishModal({ isOpen, onClose, initialRecipeId = null }) {
  const [recipes, setRecipes] = useState([]);
  const [selectedRecipeId, setSelectedRecipeId] = useState(initialRecipeId);
  const [name, setName] = useState('');
  const [price, setPrice] = useState(''); // В рублях/долларах для UI
  const [isLoading, setIsLoading] = useState(false);

  // Загружаем список рецептов для выпадающего списка
  useEffect(() => {
    if (isOpen) {
      recipesApi.list().then(setRecipes);
    }
  }, [isOpen]);

  // Автозаполнение имени блюда при выборе рецепта
  const handleRecipeChange = (recipeId) => {
    setSelectedRecipeId(recipeId);
    const recipe = recipes.find(r => r.id === recipeId);
    if (recipe && !name) {
      setName(recipe.name); // По умолчанию имя блюда = имя рецепта
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setIsLoading(true);

    try {
      // Конвертируем рубли в копейки (cents) перед отправкой на бэкенд
      const priceCents = Math.round(parseFloat(price) * 100);

      await dishesApi.create({
        recipe_id: selectedRecipeId,
        name: name,
        selling_price_cents: priceCents,
      });
      
      onClose(); // Закрываем модалку
      // TODO: Обновить список блюд (например, через mutate в SWR/React Query)
    } catch (error) {
      console.error("Ошибка создания блюда:", error);
      alert("Не удалось создать блюдо");
    } finally {
      setIsLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal">
      <h2>Добавить блюдо в меню</h2>
      <form onSubmit={handleSubmit}>
        
        <label>Выберите рецепт:</label>
        <select 
          value={selectedRecipeId || ''} 
          onChange={(e) => handleRecipeChange(e.target.value)}
          required
        >
          <option value="" disabled>-- Выберите рецепт --</option>
          {recipes.map(r => (
            <option key={r.id} value={r.id}>{r.name}</option>
          ))}
        </select>

        <label>Название в меню:</label>
        <input 
          type="text" 
          value={name} 
          onChange={(e) => setName(e.target.value)} 
          required 
        />

        <label>Цена продажи (₽):</label>
        <input 
          type="number" 
          step="0.01" 
          min="0"
          value={price} 
          onChange={(e) => setPrice(e.target.value)} 
          required 
        />

        <button type="submit" disabled={isLoading}>
          {isLoading ? 'Сохранение...' : 'Создать блюдо'}
        </button>
      </form>
    </div>
  );
}
```

---

## 4. Компонент: Таблица блюд с индикаторами маржи

На странице со списком блюд важно красиво показать экономику. Если маржа слишком низкая (Food Cost высокий), нужно подсветить это красным.

```tsx
import { useEffect, useState } from 'react';
import { dishesApi, Dish } from '@/api/dishes';

export function DishesList() {
  const [dishes, setDishes] = useState<Dish[]>([]);

  useEffect(() => {
    dishesApi.list().then(setDishes);
  }, []);

  // Функция для форматирования денег (из копеек в рубли)
  const formatMoney = (cents: number | null) => {
    if (cents === null) return '—';
    return (cents / 100).toLocaleString('ru-RU', { style: 'currency', currency: 'RUB' });
  };

  // Функция для цветовой индикации Food Cost
  // Идеальный Food Cost в ресторане: 25-35%
  const getFoodCostColor = (percent: number | null) => {
    if (percent === null) return 'text-gray-500';
    if (percent > 40) return 'text-red-500 font-bold'; // Слишком дорого в производстве
    if (percent < 20) return 'text-blue-500'; // Сверхприбыль
    return 'text-green-500'; // Норма
  };

  return (
    <div className="dishes-table-container">
      <div className="header-actions">
        <h1>Меню ресторана</h1>
        <button onClick={() => dishesApi.recalculateAll().then(() => dishesApi.list().then(setDishes))}>
          🔄 Обновить себестоимость
        </button>
      </div>

      <table className="w-full text-left">
        <thead>
          <tr>
            <th>Название</th>
            <th>Цена продажи</th>
            <th>Себестоимость</th>
            <th>Food Cost</th>
            <th>Маржа</th>
          </tr>
        </thead>
        <tbody>
          {dishes.map(dish => (
            <tr key={dish.id}>
              <td className="font-medium">{dish.name}</td>
              <td>{formatMoney(dish.selling_price_cents)}</td>
              <td>{formatMoney(dish.recipe_cost_cents)}</td>
              
              {/* Food Cost с цветовой индикацией */}
              <td className={getFoodCostColor(dish.food_cost_percent)}>
                {dish.food_cost_percent ? `${dish.food_cost_percent.toFixed(1)}%` : '—'}
              </td>
              
              {/* Маржа */}
              <td className="font-bold">
                {dish.profit_margin_percent ? `${dish.profit_margin_percent.toFixed(1)}%` : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

## 5. Ключевые моменты для фронтенда:
1. **Деньги в копейках**: Бэкенд ожидает и возвращает деньги в `cents` (копейках/центах). На фронтенде при вводе цены умножайте на 100, при выводе — делите на 100.
2. **Кнопка "Обновить себестоимость"**: Обязательно добавьте кнопку, которая вызывает `POST /api/dishes/recalculate-all`. Пользователь должен нажимать её, если он изменил цены на складе (в инвентаре), чтобы маржа блюд пересчиталась по новым ценам продуктов.
3. **Цветовая индикация**: В ресторанном бизнесе Food Cost > 40% считается критичным (блюдо работает в убыток с учетом аренды и зарплат). Подсвечивайте такие блюда красным, чтобы владелец обратил на них внимание.