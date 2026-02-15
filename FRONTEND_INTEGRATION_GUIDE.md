# 🔗 Интеграция с существующим фронтендом

Это руководство показывает, как интегрировать автоматические переводы в ваш текущий Next.js/React фронтенд.

---

## Структура вашего текущего фронтенда

На основе `FRONTEND_ADMIN_GUIDE.md`, у вас есть:

```
admin-panel/
├── components/
│   ├── ProductForm.tsx       ← СУЩЕСТВУЕТ (нужно обновить)
│   └── ProductImageUpload.tsx
├── pages/
│   └── admin/
│       └── products.tsx      ← СУЩЕСТВУЕТ (главная страница)
└── ...
```

---

## Шаг 1: Добавить сервисы

### Создать `services/translationService.ts`

```typescript
// services/translationService.ts

interface TranslationResponse {
  pl: string;
  ru: string;
  uk: string;
  source: 'dictionary' | 'groq' | 'fallback';
  cost: number;
}

class TranslationService {
  private apiUrl: string;
  private token: string;

  constructor(apiUrl: string, token: string) {
    this.apiUrl = apiUrl;
    this.token = token;
  }

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

### Создать `services/categoryService.ts`

```typescript
// services/categoryService.ts

class CategoryService {
  private apiUrl: string;
  private token: string;

  constructor(apiUrl: string, token: string) {
    this.apiUrl = apiUrl;
    this.token = token;
  }

  async getCategories() {
    const response = await fetch(`${this.apiUrl}/api/admin/categories`, {
      headers: { 'Authorization': `Bearer ${this.token}` }
    });

    if (!response.ok) throw new Error('Failed to load categories');
    
    const data = await response.json();
    return data.categories || [];
  }
}

export default CategoryService;
```

---

## Шаг 2: Обновить существующий ProductForm

### ВАРИАНТ А: Минимальное обновление (добавить auto-translate)

```typescript
// components/ProductForm.tsx - ОБНОВЛЁННАЯ ВЕРСИЯ

import React, { useState, useEffect } from 'react';
import TranslationService from '../services/translationService';
import CategoryService from '../services/categoryService';

interface ProductFormData {
  name_en: string;
  name_pl?: string;
  name_uk?: string;
  name_ru?: string;
  category_id: string;
  unit: string;
  description?: string;
  auto_translate?: boolean; // 🔑 NEW
}

function ProductForm({ 
  productId, 
  onSuccess 
}: { 
  productId?: string
  onSuccess: () => void 
}) {
  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || 
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

  // ============ STATE ============
  const [formData, setFormData] = useState<ProductFormData>({
    name_en: '',
    name_pl: '',
    name_uk: '',
    name_ru: '',
    category_id: '',
    unit: 'kilogram',
    description: '',
    auto_translate: !productId // true при создании, false при редактировании
  });

  const [categories, setCategories] = useState<any[]>([]);
  const [categoriesLoading, setCategoriesLoading] = useState(true);
  const [translations, setTranslations] = useState({
    pl: '',
    ru: '',
    uk: '',
    source: 'none' as 'dictionary' | 'groq' | 'fallback' | 'none'
  });
  const [isTranslating, setIsTranslating] = useState(false);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const translationService = new TranslationService(apiUrl, token);
  const categoryService = new CategoryService(apiUrl, token);

  // ============ LOAD CATEGORIES ============
  useEffect(() => {
    const loadCategories = async () => {
      try {
        const data = await categoryService.getCategories();
        setCategories(data);
      } catch (err) {
        console.error('Failed to load categories:', err);
      } finally {
        setCategoriesLoading(false);
      }
    };

    loadCategories();
  }, []);

  // ============ AUTO-TRANSLATE ============
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
        source: result.source
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

  // ============ SUBMIT ============
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (!formData.name_en.trim()) {
      setError('English name is required');
      return;
    }

    setIsSubmitting(true);

    const url = productId
      ? `${apiUrl}/api/admin/products/${productId}`
      : `${apiUrl}/api/admin/products`;

    const method = productId ? 'PUT' : 'POST';

    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          name_en: formData.name_en,
          name_pl: formData.name_pl,
          name_uk: formData.name_uk,
          name_ru: formData.name_ru,
          category_id: formData.category_id,
          unit: formData.unit,
          description: formData.description,
          auto_translate: formData.auto_translate
        })
      });

      const data = await response.json();

      if (!response.ok) {
        if (data.code === 'CONFLICT') {
          setError(`Product '${data.details}' already exists`);
        } else if (data.code === 'VALIDATION_ERROR') {
          setError(data.details);
        } else {
          setError('Failed to save product');
        }
        return;
      }

      onSuccess();
    } catch (err) {
      setError('Network error');
    } finally {
      setIsSubmitting(false);
    }
  };

  // ============ RENDER ============
  return (
    <form onSubmit={handleSubmit}>
      {error && <div className="alert alert-error">{error}</div>}

      {/* NAME EN WITH TRANSLATE */}
      <div className="form-group">
        <label>Name (English) *</label>
        <div style={{ display: 'flex', gap: '8px' }}>
          <input
            type="text"
            value={formData.name_en}
            onChange={e => setFormData({ ...formData, name_en: e.target.value })}
            placeholder="e.g., Tomato, Apple..."
            required
            disabled={isSubmitting}
            style={{ flex: 1 }}
          />
          {!productId && (
            <button
              type="button"
              onClick={handleAutoTranslate}
              disabled={isTranslating || !formData.name_en.trim()}
              style={{
                padding: '10px 16px',
                background: '#0066cc',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer',
                whiteSpace: 'nowrap'
              }}
            >
              {isTranslating ? '🔄 Translating...' : '🌍 Translate'}
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
          </label>
          <small>When checked, Polish, Russian, Ukrainian will be auto-translated</small>
        </div>
      )}

      {/* TRANSLATION PREVIEW */}
      {!productId && formData.auto_translate && translations.source !== 'none' && (
        <div style={{
          background: '#f0f7ff',
          border: '2px solid #0066cc',
          borderRadius: '8px',
          padding: '16px',
          marginBottom: '16px'
        }}>
          <h4>📝 Translations</h4>
          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, 1fr)',
            gap: '12px'
          }}>
            <div style={{
              background: 'white',
              padding: '12px',
              borderRadius: '6px',
              textAlign: 'center',
              border: '1px solid #ddd'
            }}>
              <span style={{ fontSize: '24px' }}>🇵🇱</span>
              <div>{translations.pl}</div>
            </div>
            <div style={{
              background: 'white',
              padding: '12px',
              borderRadius: '6px',
              textAlign: 'center',
              border: '1px solid #ddd'
            }}>
              <span style={{ fontSize: '24px' }}>🇷🇺</span>
              <div>{translations.ru}</div>
            </div>
            <div style={{
              background: 'white',
              padding: '12px',
              borderRadius: '6px',
              textAlign: 'center',
              border: '1px solid #ddd'
            }}>
              <span style={{ fontSize: '24px' }}>🇺🇦</span>
              <div>{translations.uk}</div>
            </div>
          </div>
          <small style={{ marginTop: '12px', display: 'block' }}>
            Source: {translations.source === 'dictionary' ? '💾 Cache' : 
                    translations.source === 'groq' ? '🤖 AI' : '⚪ Fallback'}
          </small>
        </div>
      )}

      {/* MANUAL TRANSLATIONS */}
      <fieldset>
        <legend>Translations (optional)</legend>

        <div className="form-group">
          <label>Name (Polish)</label>
          <input
            type="text"
            value={formData.name_pl}
            onChange={e => setFormData({ ...formData, name_pl: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label>Name (Russian)</label>
          <input
            type="text"
            value={formData.name_ru}
            onChange={e => setFormData({ ...formData, name_ru: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label>Name (Ukrainian)</label>
          <input
            type="text"
            value={formData.name_uk}
            onChange={e => setFormData({ ...formData, name_uk: e.target.value })}
            placeholder="Leave empty to auto-fill"
            disabled={isSubmitting}
          />
        </div>
      </fieldset>

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
          rows={3}
          disabled={isSubmitting}
        />
      </div>

      {/* SUBMIT */}
      <button type="submit" disabled={isSubmitting || isTranslating}>
        {isSubmitting
          ? '💾 Saving...'
          : productId
          ? '✏️ Update Product'
          : '➕ Create Product'}
      </button>
    </form>
  );
}

export default ProductForm;
```

---

## Шаг 3: Обновить страницу со списком продуктов

### `pages/admin/products.tsx` (или похожий путь)

```typescript
// pages/admin/products.tsx

import React, { useState, useEffect } from 'react';
import ProductForm from '../../components/ProductForm';

interface Product {
  id: string;
  name_en: string;
  name_pl: string;
  name_ru: string;
  name_uk: string;
  unit: string;
  image_url?: string;
}

export default function ProductsPage() {
  const [products, setProducts] = useState<Product[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || 
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

  // Load products
  useEffect(() => {
    loadProducts();
  }, []);

  const loadProducts = async () => {
    try {
      const response = await fetch(`${apiUrl}/api/admin/products`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      const data = await response.json();
      setProducts(data || []);
    } catch (error) {
      console.error('Failed to load products:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <header style={{ marginBottom: '32px' }}>
        <h1>🍽️ Product Catalog</h1>
        <p>Manage your ingredient master catalog with automatic translations</p>
        <button onClick={() => setShowForm(true)}>
          ➕ Add Product
        </button>
      </header>

      {/* FORM MODAL */}
      {showForm && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'rgba(0,0,0,0.5)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 1000
        }}>
          <div style={{
            background: 'white',
            padding: '32px',
            borderRadius: '8px',
            maxWidth: '600px',
            width: '90%',
            maxHeight: '90vh',
            overflow: 'auto'
          }}>
            <ProductForm
              productId={editingId || undefined}
              onSuccess={() => {
                setShowForm(false);
                setEditingId(null);
                loadProducts();
              }}
            />
            <button
              onClick={() => {
                setShowForm(false);
                setEditingId(null);
              }}
              style={{
                marginTop: '16px',
                width: '100%',
                padding: '10px',
                background: '#f0f0f0',
                border: 'none',
                borderRadius: '6px',
                cursor: 'pointer'
              }}
            >
              Close
            </button>
          </div>
        </div>
      )}

      {/* PRODUCTS LIST */}
      {loading ? (
        <p>Loading products...</p>
      ) : products.length === 0 ? (
        <p>No products yet. Create one to get started!</p>
      ) : (
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))',
          gap: '16px'
        }}>
          {products.map(product => (
            <div
              key={product.id}
              style={{
                border: '1px solid #ddd',
                borderRadius: '8px',
                padding: '16px',
                background: 'white'
              }}
            >
              {product.image_url && (
                <img
                  src={product.image_url}
                  alt={product.name_en}
                  style={{
                    width: '100%',
                    height: '150px',
                    objectFit: 'cover',
                    borderRadius: '6px',
                    marginBottom: '12px'
                  }}
                />
              )}
              <h3>{product.name_en}</h3>
              <div style={{ fontSize: '12px', color: '#666', marginBottom: '12px' }}>
                <p>🇵🇱 {product.name_pl}</p>
                <p>🇷🇺 {product.name_ru}</p>
                <p>🇺🇦 {product.name_uk}</p>
              </div>
              <button
                onClick={() => {
                  setEditingId(product.id);
                  setShowForm(true);
                }}
                style={{
                  width: '100%',
                  padding: '8px',
                  background: '#0066cc',
                  color: 'white',
                  border: 'none',
                  borderRadius: '6px',
                  cursor: 'pointer'
                }}
              >
                ✏️ Edit
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

---

## Шаг 4: Окружение (.env.local)

```bash
# .env.local или .env.local.development

NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
```

---

## 🧪 Тестирование интеграции

### 1. Создать продукт с auto-translate

```
1. Открыть admin/products
2. Нажать "➕ Add Product"
3. Ввести "Banana" в English name
4. Нажать "🌍 Translate"
5. Должны появиться переводы:
   🇵🇱 Banan
   🇷🇺 Банан
   🇺🇦 Банан
6. Выбрать категорию
7. Нажать "➕ Create Product"
8. ✅ Продукт создан с переводами
```

### 2. Проверить кеш

```
1. Создать ещё один продукт с тем же "Banana"
2. Переводы должны появиться мгновенно (< 100ms)
3. Должен быть badge "💾 Cache" вместо "🤖 AI"
4. Стоимость $0.00 вместо $0.01
```

### 3. Редактировать

```
1. Нажать "✏️ Edit" на существующем продукте
2. Изменить name_en на "Ripe Banana"
3. Выбрать checkbox "Auto-translate"
4. Нажать "🌍 Preview Translations"
5. Увидеть новые переводы
6. Нажать "✏️ Save Changes"
7. ✅ Продукт обновлен с новыми переводами
```

---

## 🐛 Troubleshooting

### Переводы не появляются

1. Проверьте, что `auto_translate=true` отправляется на бэкенд
2. Проверьте логи бэкенда на ошибки Groq API
3. Проверьте, что `GROQ_API_KEY` установлен в Koyeb

### Network ошибки

1. Проверьте `NEXT_PUBLIC_API_URL` правильный
2. Проверьте, что CORS включён на бэкенде
3. Проверьте network tab в DevTools

### Переводы медленные

1. Нормально - первый перевод ~ 1 сек (Groq API)
2. Повторные должны быть < 100ms (из SQL кеша)
3. Если всё медленно - может быть проблема с сетью

---

## 📋 Финальный Checklist

- [ ] Скопировал `TranslationService` в `services/`
- [ ] Скопировал `CategoryService` в `services/`
- [ ] Обновил `ProductForm.tsx`
- [ ] Обновил `pages/admin/products.tsx` (или похожий путь)
- [ ] Добавил `NEXT_PUBLIC_API_URL` в `.env.local`
- [ ] Протестировал создание продукта с auto-translate
- [ ] Протестировал кеш (повторный перевод должен быть быстрым)
- [ ] Протестировал редактирование с опциональным переводом
- [ ] Проверил ошибки и graceful degradation

---

## 🎉 Готово!

Теперь ваш фронтенд полностью интегрирован с гибридной системой переводов. Администраторы могут:

✅ Создавать продукты только с English названием
✅ Автоматически переводить на PL, RU, UK
✅ Видеть источник перевода (Dictionary, Groq AI, или Fallback)
✅ Видеть стоимость ($0.00 для кеша, $0.01 для AI)
✅ Вручную редактировать любой перевод
✅ Переиспользовать переводы из кеша (бесплатно)

**Экономия на переводах:**
- Первый "Tomato" → $0.01 (Groq AI)
- Второй "Tomato" → $0.00 (из кеша)
- Третий "Tomato" → $0.00 (из кеша)
- ... (бесконечно бесплатно после первого перевода)
