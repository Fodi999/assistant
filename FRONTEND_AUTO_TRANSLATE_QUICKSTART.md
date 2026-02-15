# 🚀 Быстрый старт: Auto-Translate для фронтенда

## Структура файлов

```
src/
├── components/
│   ├── ProductForm.tsx                # Форма создания/редактирования с auto-translate
│   ├── ProductForm.css
│   ├── TranslationStats.tsx           # Статистика переводов
│   └── TranslationStats.css
├── services/
│   └── translationService.ts          # Сервис перевода (API интеграция)
├── hooks/
│   └── useDictionaryCache.ts          # Hook для кеша переводов
├── pages/
│   └── AdminProductsPage.tsx          # Главная страница со списком
└── ...
```

## 1️⃣ Скопируйте утилиты

### services/translationService.ts

```typescript
import { AppError } from '../types/errors';

interface TranslationResponse {
  pl: string;
  ru: string;
  uk: string;
  source: 'dictionary' | 'groq' | 'fallback';
  cost: number;
}

class TranslationService {
  constructor(
    private apiUrl: string,
    private token: string
  ) {}

  /**
   * Получить переводы для английского названия
   */
  async getTranslations(name_en: string): Promise<TranslationResponse> {
    if (!name_en.trim()) {
      throw new Error('English name is required');
    }

    const response = await fetch(`${this.apiUrl}/api/admin/products`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        name_en,
        name_pl: '',
        name_ru: '',
        name_uk: '',
        category_id: 'temp',
        unit: 'kilogram',
        auto_translate: true
      })
    });

    if (!response.ok) {
      throw new Error('Translation failed');
    }

    const product = await response.json();

    return {
      pl: product.name_pl,
      ru: product.name_ru,
      uk: product.name_uk,
      source: this.detectSource(name_en, product),
      cost: this.detectSource(name_en, product) === 'dictionary' ? 0 : 0.01
    };
  }

  private detectSource(englishName: string, product: any): TranslationResponse['source'] {
    const allSame =
      product.name_pl === englishName &&
      product.name_ru === englishName &&
      product.name_uk === englishName;

    return allSame ? 'fallback' : 'groq';
  }
}

export default TranslationService;
```

### hooks/useCategories.ts

```typescript
import { useState, useEffect } from 'react';

interface Category {
  id: string;
  name: string;
}

export const useCategories = () => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const token = localStorage.getItem('admin_token');
        const apiUrl = process.env.REACT_APP_API_URL || 
          'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

        const response = await fetch(`${apiUrl}/api/admin/categories`, {
          headers: { 'Authorization': `Bearer ${token}` }
        });

        const data = await response.json();
        setCategories(data.categories || []);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load categories');
      } finally {
        setLoading(false);
      }
    };

    fetchCategories();
  }, []);

  return { categories, loading, error };
};
```

## 2️⃣ Создайте ProductForm компонент

### components/ProductForm.tsx (SIMPLIFIED VERSION)

```typescript
import React, { useState, useEffect } from 'react';
import TranslationService from '../services/translationService';
import { useCategories } from '../hooks/useCategories';
import './ProductForm.css';

interface ProductFormProps {
  productId?: string;
  initialData?: any;
  onSuccess: () => void;
  onCancel: () => void;
}

const ProductForm: React.FC<ProductFormProps> = ({
  productId,
  initialData,
  onSuccess,
  onCancel
}) => {
  const [formData, setFormData] = useState({
    name_en: initialData?.name_en || '',
    name_pl: initialData?.name_pl || '',
    name_ru: initialData?.name_ru || '',
    name_uk: initialData?.name_uk || '',
    category_id: initialData?.category_id || '',
    unit: initialData?.unit || 'kilogram',
    description: initialData?.description || '',
    auto_translate: !productId
  });

  const [translations, setTranslations] = useState({
    pl: initialData?.name_pl || '',
    ru: initialData?.name_ru || '',
    uk: initialData?.name_uk || '',
    source: 'none' as 'dictionary' | 'groq' | 'fallback' | 'none',
    cost: 0
  });

  const [isTranslating, setIsTranslating] = useState(false);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { categories, loading: categoriesLoading } = useCategories();

  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.REACT_APP_API_URL || 
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';
  const translationService = new TranslationService(apiUrl, token);

  // Auto-translate
  const handleAutoTranslate = async () => {
    if (!formData.name_en.trim()) {
      setError('English name is required');
      return;
    }

    setIsTranslating(true);
    setError('');

    try {
      const result = await translationService.getTranslations(formData.name_en);
      
      setTranslations({
        pl: result.pl,
        ru: result.ru,
        uk: result.uk,
        source: result.source,
        cost: result.cost
      });

      setFormData(prev => ({
        ...prev,
        name_pl: result.pl,
        name_ru: result.ru,
        name_uk: result.uk
      }));
    } catch (err) {
      setError('Translation failed. Try again or enter manually.');
    } finally {
      setIsTranslating(false);
    }
  };

  // Debounce auto-translate
  useEffect(() => {
    if (!formData.auto_translate || !formData.name_en.trim()) return;

    const timer = setTimeout(() => {
      handleAutoTranslate();
    }, 800);

    return () => clearTimeout(timer);
  }, [formData.auto_translate, formData.name_en]);

  // Submit
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!formData.name_en.trim() || !formData.category_id) {
      setError('Please fill in required fields');
      return;
    }

    setIsSubmitting(true);
    setError('');

    try {
      const url = productId
        ? `${apiUrl}/api/admin/products/${productId}`
        : `${apiUrl}/api/admin/products`;

      const response = await fetch(url, {
        method: productId ? 'PUT' : 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          name_en: formData.name_en,
          name_pl: formData.name_pl,
          name_ru: formData.name_ru,
          name_uk: formData.name_uk,
          category_id: formData.category_id,
          unit: formData.unit,
          description: formData.description,
          auto_translate: formData.auto_translate
        })
      });

      if (!response.ok) {
        const data = await response.json();
        setError(data.details || 'Failed to save');
        return;
      }

      onSuccess();
    } catch (err) {
      setError('Network error');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="product-form">
      {error && <div className="alert alert-error">{error}</div>}

      {/* NAME EN */}
      <div className="form-group">
        <label>Name (English) *</label>
        <div className="input-with-button">
          <input
            type="text"
            value={formData.name_en}
            onChange={e => setFormData({ ...formData, name_en: e.target.value })}
            placeholder="e.g., Tomato, Apple..."
            required
            disabled={isSubmitting}
          />
          {!productId && (
            <button
              type="button"
              onClick={handleAutoTranslate}
              disabled={isTranslating || !formData.name_en.trim()}
              className="btn-translate"
            >
              {isTranslating ? '🔄' : '🌍'} Translate
            </button>
          )}
        </div>
      </div>

      {/* AUTO-TRANSLATE TOGGLE */}
      {!productId && (
        <div className="form-group">
          <label>
            <input
              type="checkbox"
              checked={formData.auto_translate}
              onChange={e =>
                setFormData({ ...formData, auto_translate: e.target.checked })
              }
              disabled={isSubmitting}
            />
            {' '}🤖 Auto-translate other languages
            {translations.cost > 0 && (
              <span className="cost-badge">${translations.cost.toFixed(2)}</span>
            )}
          </label>
        </div>
      )}

      {/* TRANSLATION PREVIEW */}
      {!productId && formData.auto_translate && translations.source !== 'none' && (
        <div className={`translation-preview source-${translations.source}`}>
          <h4>📝 Translations</h4>
          <div className="translation-grid">
            <div className="translation-item">
              <span className="lang-flag">🇵🇱</span>
              <span>{translations.pl}</span>
            </div>
            <div className="translation-item">
              <span className="lang-flag">🇷🇺</span>
              <span>{translations.ru}</span>
            </div>
            <div className="translation-item">
              <span className="lang-flag">🇺🇦</span>
              <span>{translations.uk}</span>
            </div>
          </div>
          <small>
            Source: {translations.source === 'dictionary' ? '💾 Cache' : 
                    translations.source === 'groq' ? '🤖 AI' : '⚪ Fallback'}
            {translations.cost > 0 && ` · Cost: $${translations.cost.toFixed(2)}`}
          </small>
        </div>
      )}

      {/* MANUAL TRANSLATIONS */}
      <div className="translation-fields">
        <h4>Translations (optional)</h4>

        <div className="form-group">
          <label>🇵🇱 Polish</label>
          <input
            type="text"
            value={formData.name_pl}
            onChange={e => setFormData({ ...formData, name_pl: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label>🇷🇺 Russian</label>
          <input
            type="text"
            value={formData.name_ru}
            onChange={e => setFormData({ ...formData, name_ru: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label>🇺🇦 Ukrainian</label>
          <input
            type="text"
            value={formData.name_uk}
            onChange={e => setFormData({ ...formData, name_uk: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>
      </div>

      {/* CATEGORY */}
      <div className="form-group">
        <label>Category *</label>
        <select
          value={formData.category_id}
          onChange={e => setFormData({ ...formData, category_id: e.target.value })}
          required
          disabled={categoriesLoading || isSubmitting}
        >
          <option value="">
            {categoriesLoading ? 'Loading...' : 'Select category...'}
          </option>
          {categories.map(cat => (
            <option key={cat.id} value={cat.id}>{cat.name}</option>
          ))}
        </select>
      </div>

      {/* UNIT */}
      <div className="form-group">
        <label>Unit *</label>
        <select
          value={formData.unit}
          onChange={e => setFormData({ ...formData, unit: e.target.value })}
          required
          disabled={isSubmitting}
        >
          <option value="kilogram">Kilogram</option>
          <option value="gram">Gram</option>
          <option value="liter">Liter</option>
          <option value="milliliter">Milliliter</option>
          <option value="piece">Piece</option>
          <option value="bunch">Bunch</option>
          <option value="can">Can</option>
          <option value="package">Package</option>
        </select>
      </div>

      {/* DESCRIPTION */}
      <div className="form-group">
        <label>Description</label>
        <textarea
          value={formData.description}
          onChange={e =>
            setFormData({ ...formData, description: e.target.value })
          }
          placeholder="Optional..."
          rows={3}
          disabled={isSubmitting}
        />
      </div>

      {/* ACTIONS */}
      <div className="form-actions">
        <button type="submit" disabled={isSubmitting || isTranslating}>
          {isSubmitting ? '💾 Saving...' : productId ? '✏️ Update' : '➕ Create'}
        </button>
        <button type="button" onClick={onCancel} disabled={isSubmitting}>
          Cancel
        </button>
      </div>
    </form>
  );
};

export default ProductForm;
```

## 3️⃣ Добавьте стили

### components/ProductForm.css (MINIMAL VERSION)

```css
.product-form {
  max-width: 600px;
  background: #fff;
  border-radius: 8px;
  padding: 24px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-weight: 600;
  color: #333;
  font-size: 14px;
}

.form-group input,
.form-group textarea,
.form-group select {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  font-family: inherit;
}

.form-group input:focus,
.form-group textarea:focus,
.form-group select:focus {
  outline: none;
  border-color: #0066cc;
  box-shadow: 0 0 0 3px rgba(0,102,204,0.1);
}

.form-group input:disabled,
.form-group textarea:disabled,
.form-group select:disabled {
  background: #f5f5f5;
  color: #999;
}

/* Input with button */
.input-with-button {
  display: flex;
  gap: 8px;
}

.input-with-button input {
  flex: 1;
}

.btn-translate {
  padding: 10px 16px;
  background: #0066cc;
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}

.btn-translate:hover:not(:disabled) {
  background: #0052a3;
}

.btn-translate:disabled {
  background: #ccc;
  cursor: not-allowed;
}

/* Alerts */
.alert {
  padding: 12px 16px;
  border-radius: 6px;
  margin-bottom: 16px;
  font-size: 14px;
}

.alert-error {
  background: #fee;
  color: #c33;
  border-left: 4px solid #c33;
}

/* Translation preview */
.translation-preview {
  background: #f0f7ff;
  border: 2px solid #0066cc;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}

.translation-preview h4 {
  margin: 0 0 12px 0;
  font-size: 14px;
}

.translation-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 12px;
}

.translation-item {
  background: white;
  padding: 12px;
  border-radius: 6px;
  text-align: center;
  border: 1px solid #ddd;
}

.lang-flag {
  font-size: 24px;
  display: block;
  margin-bottom: 4px;
}

.cost-badge {
  background: #fff3cd;
  color: #856404;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  margin-left: 8px;
}

/* Translation fields */
.translation-fields {
  background: #f9f9f9;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}

.translation-fields h4 {
  margin: 0 0 16px 0;
  font-size: 14px;
}

/* Form actions */
.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}

button {
  padding: 12px 24px;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
}

button[type="submit"] {
  background: #0066cc;
  color: white;
  flex: 1;
}

button[type="submit"]:hover:not(:disabled) {
  background: #0052a3;
}

button[type="submit"]:disabled {
  background: #ccc;
  cursor: not-allowed;
}

button[type="button"] {
  background: #f0f0f0;
  color: #333;
}

button[type="button"]:hover:not(:disabled) {
  background: #e0e0e0;
}
```

## 4️⃣ Используйте в вашей странице

```typescript
// pages/AdminProductsPage.tsx
import ProductForm from '../components/ProductForm';
import { useState } from 'react';

export default function AdminProductsPage() {
  const [showForm, setShowForm] = useState(false);

  return (
    <div>
      <h1>🍽️ Products</h1>
      <button onClick={() => setShowForm(true)}>➕ Add Product</button>

      {showForm && (
        <ProductForm
          onSuccess={() => {
            setShowForm(false);
            // Reload products
          }}
          onCancel={() => setShowForm(false)}
        />
      )}
    </div>
  );
}
```

## 📋 Checklist

- [ ] Скопировал `translationService.ts` в `src/services/`
- [ ] Скопировал `useCategories.ts` в `src/hooks/`
- [ ] Скопировал `ProductForm.tsx` в `src/components/`
- [ ] Добавил `ProductForm.css`
- [ ] Обновил страницу со списком продуктов
- [ ] Установил переменные окружения:
  ```
  REACT_APP_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
  ```
- [ ] Протестировал создание продукта с auto-translate
- [ ] Проверил, что переводы появляются в реальном времени

## 🧪 Тестирование

1. **Создать продукт:**
   - Введите "Tomato"
   - Нажмите "🌍 Translate"
   - Должны появиться: PL: Pomidor, RU: Помидор, UK: Помідор
   - Выберите категорию, unit
   - Нажмите "➕ Create"
   - Продукт должен создаться с переводами

2. **Повторный перевод (кеш):**
   - Создайте ещё один продукт с тем же "Tomato"
   - На этот раз должно быть быстрее (< 1 сек из кеша)
   - Должен показать "💾 Cache" вместо "🤖 AI"

3. **Ошибки:**
   - Попробуйте отправить пустой name_en → ошибка
   - Попробуйте отправить без категории → ошибка

## 📚 Полная документация

Смотрите `FRONTEND_AUTO_TRANSLATE_GUIDE.md` для:
- Полного кода всех компонентов
- Статистики переводов
- Advanced паттернов (batch translate, caching)
- Best practices
