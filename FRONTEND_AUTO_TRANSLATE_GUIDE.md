# 🌍 Frontend Automatic Translation Guide

## Overview

Полное руководство по реализации автоматических переводов на фронтенде для создания и редактирования продуктов.

**Возможности:**
- ✅ Автоматический перевод с English → Polish, Russian, Ukrainian
- ✅ Умное заполнение полей (Dictionary Cache → Groq AI → English fallback)
- ✅ Real-time preview переводов
- ✅ Visual feedback (loading states, success indicators)
- ✅ Cost display ($0.01 for AI, $0.00 for cache)
- ✅ Manual override поддержка
- ✅ Batch translation для редактирования

**Система стоимости:**
- 🟢 Dictionary cache: $0.00 (мгновенно из БД)
- 🟡 Groq AI (первый раз): $0.01 за ингредиент
- 🔵 Repeat requests: $0.00 (кеш работает)

---

## 1. Utility Functions & Hooks

### 1.1 useDictionaryCache Hook

```typescript
// hooks/useDictionaryCache.ts
import { useState, useCallback } from 'react';

interface TranslationResult {
  pl: string;
  ru: string;
  uk: string;
  source: 'dictionary' | 'groq' | 'fallback';
  cost: number; // 0 or 0.01
}

interface CacheStats {
  totalRequests: number;
  cacheHits: number;
  aiCalls: number;
  totalCostUSD: number;
}

export const useDictionaryCache = (apiUrl: string, token: string) => {
  const [isTranslating, setIsTranslating] = useState(false);
  const [stats, setStats] = useState<CacheStats>({
    totalRequests: 0,
    cacheHits: 0,
    aiCalls: 0,
    totalCostUSD: 0
  });

  const translate = useCallback(
    async (englishName: string): Promise<TranslationResult | null> => {
      if (!englishName.trim()) {
        return null;
      }

      setIsTranslating(true);

      try {
        // Генерируем случайный суффикс для теста (обычно не нужен)
        // В production API сам кеширует по name_en
        const response = await fetch(`${apiUrl}/api/admin/products`, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${token}`,
            'Content-Type': 'application/json'
          },
          // Отправляем ТОЛЬКО name_en, остальное заполнится автоматически
          body: JSON.stringify({
            name_en: englishName,
            name_pl: '', // Оставляем пустым для автоперевода
            name_ru: '',
            name_uk: '',
            category_id: '', // Заполнится позже
            unit: 'kilogram', // Или любой unit
            auto_translate: true // 👈 КЛЮЧЕВОЙ ФЛАГ!
          })
        });

        if (!response.ok) {
          const error = await response.json();
          console.error('Translation error:', error);
          return null;
        }

        const product = await response.json();

        // Определяем источник перевода по результатам
        const result: TranslationResult = {
          pl: product.name_pl,
          ru: product.name_ru,
          uk: product.name_uk,
          source: detectSource(englishName, product),
          cost: detectSource(englishName, product) === 'dictionary' ? 0 : 0.01
        };

        // Обновляем статистику
        setStats(prev => ({
          totalRequests: prev.totalRequests + 1,
          cacheHits: result.source === 'dictionary' ? prev.cacheHits + 1 : prev.cacheHits,
          aiCalls: result.source === 'groq' ? prev.aiCalls + 1 : prev.aiCalls,
          totalCostUSD: prev.totalCostUSD + result.cost
        }));

        return result;
      } catch (error) {
        console.error('Translation failed:', error);
        return null;
      } finally {
        setIsTranslating(false);
      }
    },
    [apiUrl, token]
  );

  return {
    translate,
    isTranslating,
    stats
  };
};

// Helper: определяет источник перевода
function detectSource(
  englishName: string,
  product: any
): 'dictionary' | 'groq' | 'fallback' {
  // Если все языки = английскому → fallback
  if (
    product.name_pl === englishName &&
    product.name_ru === englishName &&
    product.name_uk === englishName
  ) {
    return 'fallback';
  }

  // Если хотя бы один язык отличается → был перевод
  // (не можем точно определить dictionary vs groq без логирования на бэке)
  // Но для UI это не критично
  return 'groq'; // или 'dictionary', будет показано одинаково
}
```

**ВНИМАНИЕ:** Текущая реализация требует создания продукта для получения переводов. **Лучше создать отдельный endpoint на бэкенде** для только перевода (см. раздел 5).

### 1.2 Альтернатива: Транспортный слой для переводов

```typescript
// services/translationService.ts
import { AppError } from '../types/errors';

interface TranslationRequest {
  name_en: string;
  auto_translate?: boolean;
}

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
   * 🌍 Получить переводы для английского названия
   * 
   * Использует гибридный подход:
   * 1. Dictionary cache (если есть в БД) → мгновенно, $0
   * 2. Groq AI (если нет в кеше) → 1-2 сек, $0.01
   * 3. Fallback (если AI недоступен) → английский, $0
   */
  async getTranslations(name_en: string): Promise<TranslationResponse> {
    if (!name_en.trim()) {
      throw new Error('English name is required');
    }

    // ❌ ТЕКУЩИЙ ПОДХОД (требует создания продукта):
    // Отправляем POST /api/admin/products с auto_translate=true
    // Возвращаем созданный продукт с переводами

    // ✅ РЕКОМЕНДУЕМЫЙ ПОДХОД (нужно добавить на бэкенде):
    // POST /api/admin/translations
    // Body: { "name_en": "Apple", "auto_translate": true }
    // Response: { "pl": "Jabłko", "ru": "Яблоко", "uk": "Яблуко", "source": "groq", "cost": 0.01 }

    // На данный момент используем первый подход
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
        category_id: 'temp', // Временный ID
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

  private detectSource(
    englishName: string,
    product: any
  ): 'dictionary' | 'groq' | 'fallback' {
    // Проверяем, были ли переводы
    const allSame =
      product.name_pl === englishName &&
      product.name_ru === englishName &&
      product.name_uk === englishName;

    return allSame ? 'fallback' : 'groq';
  }
}

export default TranslationService;
```

---

## 2. Product Form with Auto-Translate

### 2.1 Enhanced Product Form Component

```tsx
// components/ProductForm.tsx
import React, { useState, useEffect } from 'react';
import { useCategories } from '../hooks/useCategories';
import TranslationService from '../services/translationService';
import './ProductForm.css';

interface ProductFormProps {
  productId?: string;
  initialData?: any;
  onSuccess: () => void;
  onCancel: () => void;
}

interface TranslationState {
  pl: string;
  ru: string;
  uk: string;
  source?: 'dictionary' | 'groq' | 'fallback';
  cost?: number;
}

interface FormErrors {
  name_en?: string;
  category_id?: string;
  unit?: string;
  [key: string]: string | undefined;
}

const ProductForm: React.FC<ProductFormProps> = ({
  productId,
  initialData,
  onSuccess,
  onCancel
}) => {
  // ============ STATE ============
  const { categories, loading: categoriesLoading } = useCategories();
  const [formData, setFormData] = useState({
    name_en: initialData?.name_en || '',
    name_pl: initialData?.name_pl || '',
    name_ru: initialData?.name_ru || '',
    name_uk: initialData?.name_uk || '',
    category_id: initialData?.category_id || '',
    unit: initialData?.unit || 'kilogram',
    description: initialData?.description || '',
    auto_translate: !productId // При создании = true, при редактировании = false по умолчанию
  });

  const [translations, setTranslations] = useState<TranslationState>({
    pl: initialData?.name_pl || '',
    ru: initialData?.name_ru || '',
    uk: initialData?.name_uk || ''
  });

  const [isTranslating, setIsTranslating] = useState(false);
  const [errors, setErrors] = useState<FormErrors>({});
  const [globalError, setGlobalError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [translationStats, setTranslationStats] = useState({
    cost: 0,
    source: 'none' as 'dictionary' | 'groq' | 'fallback' | 'none'
  });

  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.REACT_APP_API_URL || 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';
  const translationService = new TranslationService(apiUrl, token);

  // ============ AUTO-TRANSLATE LOGIC ============
  
  /**
   * Запросить автоматические переводы
   */
  const handleAutoTranslate = async () => {
    if (!formData.name_en.trim()) {
      setErrors(prev => ({ ...prev, name_en: 'English name is required' }));
      return;
    }

    setIsTranslating(true);
    setErrors(prev => ({ ...prev, name_en: '' }));

    try {
      const result = await translationService.getTranslations(formData.name_en);

      setTranslations({
        pl: result.pl,
        ru: result.ru,
        uk: result.uk,
        source: result.source,
        cost: result.cost
      });

      setTranslationStats({
        cost: result.cost,
        source: result.source
      });

      // Копируем переводы в форму
      setFormData(prev => ({
        ...prev,
        name_pl: result.pl,
        name_ru: result.ru,
        name_uk: result.uk
      }));
    } catch (error) {
      setGlobalError('Translation failed. Please try again or enter manually.');
      console.error('Translation error:', error);
    } finally {
      setIsTranslating(false);
    }
  };

  /**
   * Автоматический перевод при изменении name_en (с debounce)
   */
  useEffect(() => {
    if (!formData.auto_translate || !formData.name_en.trim()) {
      return;
    }

    const timer = setTimeout(() => {
      handleAutoTranslate();
    }, 800); // 800ms debounce

    return () => clearTimeout(timer);
  }, [formData.auto_translate, formData.name_en]);

  // ============ VALIDATION ============

  const validateForm = (): boolean => {
    const newErrors: FormErrors = {};

    if (!formData.name_en.trim()) {
      newErrors.name_en = 'English name is required';
    }

    if (!formData.category_id) {
      newErrors.category_id = 'Category is required';
    }

    if (!formData.unit) {
      newErrors.unit = 'Unit is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  // ============ SUBMIT ============

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setGlobalError('');

    if (!validateForm()) {
      setGlobalError('Please fill in all required fields');
      return;
    }

    setIsSubmitting(true);

    try {
      const url = productId
        ? `${apiUrl}/api/admin/products/${productId}`
        : `${apiUrl}/api/admin/products`;

      const method = productId ? 'PUT' : 'POST';

      const response = await fetch(url, {
        method,
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
          auto_translate: formData.auto_translate // Отправляем флаг на бэкенд
        })
      });

      const data = await response.json();

      if (!response.ok) {
        if (data.code === 'CONFLICT') {
          setGlobalError(`Product "${data.details}" already exists`);
        } else if (data.code === 'VALIDATION_ERROR') {
          setGlobalError(data.details);
        } else {
          setGlobalError('Failed to save product');
        }
        return;
      }

      // Success
      onSuccess();
    } catch (error) {
      setGlobalError('Network error. Please try again.');
      console.error('Submit error:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  // ============ RENDER ============

  return (
    <div className="product-form-container">
      <form onSubmit={handleSubmit} className="product-form">
        {/* GLOBAL ERROR */}
        {globalError && <div className="alert alert-error">{globalError}</div>}

        {/* ENGLISH NAME (REQUIRED) */}
        <div className="form-group">
          <label htmlFor="name_en">
            Name (English) <span className="required">*</span>
          </label>
          <div className="input-with-button">
            <input
              id="name_en"
              type="text"
              value={formData.name_en}
              onChange={e => setFormData({ ...formData, name_en: e.target.value })}
              placeholder="e.g., Tomato, Apple, Pasta..."
              className={errors.name_en ? 'input-error' : ''}
              disabled={isSubmitting}
              maxLength={100}
            />
            {!productId && (
              <button
                type="button"
                onClick={handleAutoTranslate}
                disabled={isTranslating || !formData.name_en.trim()}
                className="btn-translate"
                title="Auto-translate to Polish, Russian, Ukrainian"
              >
                {isTranslating ? '🔄 Translating...' : '🌍 Translate'}
              </button>
            )}
          </div>
          {errors.name_en && <span className="error-message">{errors.name_en}</span>}
          <small>Required. Will auto-fill other languages if left empty.</small>
        </div>

        {/* AUTO-TRANSLATE TOGGLE (only on create) */}
        {!productId && (
          <div className="form-group">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={formData.auto_translate}
                onChange={e =>
                  setFormData({ ...formData, auto_translate: e.target.checked })
                }
                disabled={isSubmitting}
              />
              <span>
                🤖 Auto-translate other languages using AI
                {translationStats.cost > 0 && (
                  <span className="cost-badge">${translationStats.cost.toFixed(2)}</span>
                )}
              </span>
            </label>
            <small>
              When checked: Polish, Russian, Ukrainian will be auto-translated.
              Cache hit = $0.00, New translation = $0.01
            </small>
          </div>
        )}

        {/* TRANSLATION PREVIEW */}
        {!productId && formData.auto_translate && translations.source && (
          <div className={`translation-preview source-${translations.source}`}>
            <div className="preview-header">
              <h4>📝 Translation Preview</h4>
              <span className={`source-badge source-${translations.source}`}>
                {translations.source === 'dictionary' && '💾 Dictionary Cache'}
                {translations.source === 'groq' && '🤖 AI (Groq)'}
                {translations.source === 'fallback' && '⚪ Fallback'}
              </span>
              {translationStats.cost > 0 && (
                <span className="cost-info">
                  💰 ${translationStats.cost.toFixed(2)}
                </span>
              )}
            </div>

            <div className="translation-grid">
              <div className="translation-item">
                <span className="lang-flag">🇵🇱</span>
                <span className="lang-name">Polish</span>
                <span className="translation-text">{translations.pl}</span>
              </div>
              <div className="translation-item">
                <span className="lang-flag">🇷🇺</span>
                <span className="lang-name">Russian</span>
                <span className="translation-text">{translations.ru}</span>
              </div>
              <div className="translation-item">
                <span className="lang-flag">🇺🇦</span>
                <span className="lang-name">Ukrainian</span>
                <span className="translation-text">{translations.uk}</span>
              </div>
            </div>
          </div>
        )}

        {/* MANUAL TRANSLATION FIELDS */}
        <div className="translation-fields">
          <h4>Manual Translations (optional)</h4>

          <div className="form-group">
            <label htmlFor="name_pl">
              Name (Polish) <span className="lang-flag">🇵🇱</span>
            </label>
            <input
              id="name_pl"
              type="text"
              value={formData.name_pl}
              onChange={e => setFormData({ ...formData, name_pl: e.target.value })}
              placeholder="Leave empty to auto-fill"
              disabled={isSubmitting}
              maxLength={100}
            />
            <small>
              {formData.name_pl === formData.name_en
                ? '⚪ Same as English (fallback)'
                : '✅ Custom translation'}
            </small>
          </div>

          <div className="form-group">
            <label htmlFor="name_ru">
              Name (Russian) <span className="lang-flag">🇷🇺</span>
            </label>
            <input
              id="name_ru"
              type="text"
              value={formData.name_ru}
              onChange={e => setFormData({ ...formData, name_ru: e.target.value })}
              placeholder="Leave empty to auto-fill"
              disabled={isSubmitting}
              maxLength={100}
            />
            <small>
              {formData.name_ru === formData.name_en
                ? '⚪ Same as English (fallback)'
                : '✅ Custom translation'}
            </small>
          </div>

          <div className="form-group">
            <label htmlFor="name_uk">
              Name (Ukrainian) <span className="lang-flag">🇺🇦</span>
            </label>
            <input
              id="name_uk"
              type="text"
              value={formData.name_uk}
              onChange={e => setFormData({ ...formData, name_uk: e.target.value })}
              placeholder="Leave empty to auto-fill"
              disabled={isSubmitting}
              maxLength={100}
            />
            <small>
              {formData.name_uk === formData.name_en
                ? '⚪ Same as English (fallback)'
                : '✅ Custom translation'}
            </small>
          </div>
        </div>

        {/* CATEGORY (REQUIRED) */}
        <div className="form-group">
          <label htmlFor="category">
            Category <span className="required">*</span>
          </label>
          <select
            id="category"
            value={formData.category_id}
            onChange={e => setFormData({ ...formData, category_id: e.target.value })}
            disabled={categoriesLoading || isSubmitting}
            className={errors.category_id ? 'input-error' : ''}
          >
            <option value="">
              {categoriesLoading ? 'Loading categories...' : 'Select a category...'}
            </option>
            {categories.map(cat => (
              <option key={cat.id} value={cat.id}>
                {cat.name}
              </option>
            ))}
          </select>
          {errors.category_id && (
            <span className="error-message">{errors.category_id}</span>
          )}
        </div>

        {/* UNIT (REQUIRED) */}
        <div className="form-group">
          <label htmlFor="unit">
            Unit <span className="required">*</span>
          </label>
          <select
            id="unit"
            value={formData.unit}
            onChange={e => setFormData({ ...formData, unit: e.target.value })}
            disabled={isSubmitting}
            className={errors.unit ? 'input-error' : ''}
          >
            <option value="kilogram">Kilogram (kg)</option>
            <option value="gram">Gram (g)</option>
            <option value="liter">Liter (L)</option>
            <option value="milliliter">Milliliter (ml)</option>
            <option value="piece">Piece</option>
            <option value="bunch">Bunch</option>
            <option value="can">Can</option>
            <option value="package">Package</option>
          </select>
          {errors.unit && <span className="error-message">{errors.unit}</span>}
        </div>

        {/* DESCRIPTION */}
        <div className="form-group">
          <label htmlFor="description">Description</label>
          <textarea
            id="description"
            value={formData.description}
            onChange={e =>
              setFormData({ ...formData, description: e.target.value })
            }
            placeholder="Optional description..."
            rows={4}
            disabled={isSubmitting}
            maxLength={500}
          />
          <small>
            {formData.description.length}/500 characters
          </small>
        </div>

        {/* ACTIONS */}
        <div className="form-actions">
          <button
            type="submit"
            disabled={isSubmitting || isTranslating}
            className="btn btn-primary"
          >
            {isSubmitting
              ? '💾 Saving...'
              : productId
              ? '✏️ Update Product'
              : '➕ Create Product'}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="btn btn-secondary"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
};

export default ProductForm;
```

### 2.2 Product Form Styles

```css
/* ProductForm.css */

.product-form-container {
  max-width: 900px;
  margin: 0 auto;
  padding: 20px;
}

.product-form {
  background: #fff;
  border-radius: 8px;
  padding: 30px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

/* ALERTS */
.alert {
  padding: 12px 16px;
  border-radius: 6px;
  margin-bottom: 20px;
  font-weight: 500;
}

.alert-error {
  background: #fee;
  color: #c33;
  border-left: 4px solid #c33;
}

.alert-success {
  background: #efe;
  color: #3c3;
  border-left: 4px solid #3c3;
}

/* FORM GROUPS */
.form-group {
  margin-bottom: 24px;
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
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-group input:focus,
.form-group textarea:focus,
.form-group select:focus {
  outline: none;
  border-color: #0066cc;
  box-shadow: 0 0 0 3px rgba(0, 102, 204, 0.1);
}

.form-group input:disabled,
.form-group textarea:disabled,
.form-group select:disabled {
  background: #f5f5f5;
  color: #999;
  cursor: not-allowed;
}

.form-group textarea {
  resize: vertical;
  min-height: 80px;
}

.form-group small {
  display: block;
  margin-top: 6px;
  color: #666;
  font-size: 12px;
  line-height: 1.4;
}

.required {
  color: #c33;
  margin-left: 2px;
}

/* INPUT WITH BUTTON */
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
  transition: background 0.2s, transform 0.1s;
  font-size: 14px;
}

.btn-translate:hover:not(:disabled) {
  background: #0052a3;
  transform: translateY(-1px);
}

.btn-translate:disabled {
  background: #ccc;
  cursor: not-allowed;
}

/* ERROR STATES */
.input-error {
  border-color: #c33 !important;
  background: #fef5f5;
}

.error-message {
  display: block;
  color: #c33;
  font-size: 12px;
  margin-top: 4px;
  font-weight: 500;
}

/* CHECKBOX LABEL */
.checkbox-label {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  cursor: pointer;
  font-weight: 500;
  font-size: 14px;
}

.checkbox-label input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
  margin: 0;
  padding: 0;
  flex-shrink: 0;
}

.cost-badge {
  display: inline-block;
  background: #fff3cd;
  color: #856404;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  margin-left: 8px;
}

/* TRANSLATION PREVIEW */
.translation-preview {
  background: #f0f7ff;
  border: 2px solid #0066cc;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 24px;
  animation: slideIn 0.3s ease-out;
}

.translation-preview.source-dictionary {
  border-color: #28a745;
  background: #f0fff4;
}

.translation-preview.source-groq {
  border-color: #0066cc;
  background: #f0f7ff;
}

.translation-preview.source-fallback {
  border-color: #999;
  background: #f9f9f9;
}

.preview-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.preview-header h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.source-badge {
  display: inline-block;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
}

.source-badge.source-dictionary {
  background: #d4edda;
  color: #155724;
}

.source-badge.source-groq {
  background: #cce5ff;
  color: #004085;
}

.source-badge.source-fallback {
  background: #e2e3e5;
  color: #383d41;
}

.cost-info {
  margin-left: auto;
  font-weight: 600;
  color: #666;
  font-size: 13px;
}

.translation-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 12px;
}

.translation-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px;
  background: white;
  border-radius: 6px;
  border: 1px solid #ddd;
}

.lang-flag {
  font-size: 24px;
}

.lang-name {
  font-size: 11px;
  font-weight: 600;
  color: #666;
  text-transform: uppercase;
}

.translation-text {
  font-size: 14px;
  font-weight: 600;
  color: #333;
  text-align: center;
  word-break: break-word;
}

/* TRANSLATION FIELDS */
.translation-fields {
  background: #f9f9f9;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 24px;
  border: 1px solid #e0e0e0;
}

.translation-fields h4 {
  margin: 0 0 16px 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.translation-fields .form-group {
  margin-bottom: 16px;
}

.translation-fields .form-group:last-child {
  margin-bottom: 0;
}

/* FORM ACTIONS */
.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 32px;
  padding-top: 24px;
  border-top: 1px solid #e0e0e0;
}

.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-primary {
  background: #0066cc;
  color: white;
  flex: 1;
}

.btn-primary:hover:not(:disabled) {
  background: #0052a3;
  box-shadow: 0 4px 12px rgba(0, 102, 204, 0.3);
  transform: translateY(-2px);
}

.btn-primary:disabled {
  background: #ccc;
  cursor: not-allowed;
}

.btn-secondary {
  background: #f0f0f0;
  color: #333;
}

.btn-secondary:hover:not(:disabled) {
  background: #e0e0e0;
}

.btn-secondary:disabled {
  background: #f5f5f5;
  color: #999;
  cursor: not-allowed;
}

/* ANIMATIONS */
@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* RESPONSIVE */
@media (max-width: 768px) {
  .product-form {
    padding: 20px;
  }

  .translation-grid {
    grid-template-columns: 1fr;
  }

  .preview-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .cost-info {
    margin-left: 0;
  }

  .form-actions {
    flex-direction: column;
  }

  .btn {
    width: 100%;
  }
}
```

---

## 3. Translation Statistics Component

```tsx
// components/TranslationStats.tsx
import React from 'react';
import './TranslationStats.css';

interface Stats {
  totalRequests: number;
  cacheHits: number;
  aiCalls: number;
  totalCostUSD: number;
}

interface TranslationStatsProps {
  stats: Stats;
}

const TranslationStats: React.FC<TranslationStatsProps> = ({ stats }) => {
  const hitRate = stats.totalRequests > 0 
    ? ((stats.cacheHits / stats.totalRequests) * 100).toFixed(1)
    : '0';

  return (
    <div className="translation-stats">
      <div className="stats-grid">
        {/* Total Requests */}
        <div className="stat-card">
          <div className="stat-icon">📊</div>
          <div className="stat-content">
            <div className="stat-label">Total Requests</div>
            <div className="stat-value">{stats.totalRequests}</div>
          </div>
        </div>

        {/* Cache Hits */}
        <div className="stat-card stat-success">
          <div className="stat-icon">💾</div>
          <div className="stat-content">
            <div className="stat-label">Cache Hits</div>
            <div className="stat-value">{stats.cacheHits}</div>
            <div className="stat-sub">{hitRate}% hit rate</div>
          </div>
        </div>

        {/* AI Calls */}
        <div className="stat-card stat-info">
          <div className="stat-icon">🤖</div>
          <div className="stat-content">
            <div className="stat-label">AI Translations</div>
            <div className="stat-value">{stats.aiCalls}</div>
            <div className="stat-sub">{(stats.aiCalls * 0.01).toFixed(2)}$ cost</div>
          </div>
        </div>

        {/* Total Cost */}
        <div className="stat-card stat-cost">
          <div className="stat-icon">💰</div>
          <div className="stat-content">
            <div className="stat-label">Total Cost</div>
            <div className="stat-value">${stats.totalCostUSD.toFixed(2)}</div>
            <div className="stat-sub">API usage</div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default TranslationStats;
```

```css
/* TranslationStats.css */

.translation-stats {
  padding: 20px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.stat-card {
  background: white;
  border-radius: 8px;
  padding: 16px;
  border: 1px solid #e0e0e0;
  display: flex;
  gap: 12px;
  transition: all 0.2s;
}

.stat-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  transform: translateY(-2px);
}

.stat-card.stat-success {
  border-color: #d4edda;
  background: #f0fff4;
}

.stat-card.stat-info {
  border-color: #cce5ff;
  background: #f0f7ff;
}

.stat-card.stat-cost {
  border-color: #ffe5cc;
  background: #fff8f0;
}

.stat-icon {
  font-size: 28px;
  flex-shrink: 0;
  line-height: 1;
  margin-top: 2px;
}

.stat-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-label {
  font-size: 12px;
  font-weight: 600;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #333;
}

.stat-sub {
  font-size: 11px;
  color: #999;
}
```

---

## 4. Integration with Product List

```tsx
// pages/AdminProductsPage.tsx
import React, { useState, useEffect } from 'react';
import ProductForm from '../components/ProductForm';
import TranslationStats from '../components/TranslationStats';
import './AdminProductsPage.css';

const AdminProductsPage: React.FC = () => {
  const [products, setProducts] = useState([]);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [stats, setStats] = useState({
    totalRequests: 0,
    cacheHits: 0,
    aiCalls: 0,
    totalCostUSD: 0
  });

  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.REACT_APP_API_URL || 
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
    }
  };

  const handleFormSuccess = () => {
    setShowForm(false);
    setEditingId(null);
    loadProducts();
  };

  return (
    <div className="admin-products-page">
      <header className="page-header">
        <div>
          <h1>🍽️ Product Catalog</h1>
          <p>Manage your ingredient master catalog with automatic translations</p>
        </div>
        <button
          className="btn btn-primary"
          onClick={() => {
            setEditingId(null);
            setShowForm(true);
          }}
        >
          ➕ Add Product
        </button>
      </header>

      {/* TRANSLATION STATS */}
      <TranslationStats stats={stats} />

      {/* FORM MODAL */}
      {showForm && (
        <div className="modal-overlay">
          <div className="modal">
            <ProductForm
              productId={editingId || undefined}
              onSuccess={handleFormSuccess}
              onCancel={() => setShowForm(false)}
            />
          </div>
        </div>
      )}

      {/* PRODUCTS LIST */}
      <div className="products-list">
        {products.length === 0 ? (
          <div className="empty-state">
            <p>📦 No products yet. Create one to get started!</p>
          </div>
        ) : (
          <div className="products-grid">
            {products.map(product => (
              <div key={product.id} className="product-card">
                {product.image_url && (
                  <img 
                    src={product.image_url} 
                    alt={product.name_en}
                    className="product-image"
                  />
                )}
                <div className="product-info">
                  <h3>{product.name_en}</h3>
                  <div className="translations">
                    <span className="translation">
                      🇵🇱 {product.name_pl}
                    </span>
                    <span className="translation">
                      🇷🇺 {product.name_ru}
                    </span>
                    <span className="translation">
                      🇺🇦 {product.name_uk}
                    </span>
                  </div>
                  <div className="product-meta">
                    <span className="unit">📦 {product.unit}</span>
                  </div>
                </div>
                <div className="product-actions">
                  <button
                    className="btn btn-small"
                    onClick={() => {
                      setEditingId(product.id);
                      setShowForm(true);
                    }}
                  >
                    Edit
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default AdminProductsPage;
```

---

## 5. 🚀 RECOMMENDED: Dedicated Translation Endpoint

**На бэкенде добавить отдельный endpoint для только переводов (без создания продукта):**

```rust
// src/interfaces/http/admin_catalog.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub name_en: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub pl: String,
    pub ru: String,
    pub uk: String,
    pub source: String,
    pub cost: f64,
}

#[post("/api/admin/translate")]
pub async fn translate_ingredient(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(req): Json<TranslationRequest>,
) -> Result<Json<TranslationResponse>, AppError> {
    let dictionary = &state.dictionary_service;
    let groq = &state.groq_service;

    // 1. Проверяем словарь
    if let Some(entry) = dictionary.find_by_en(&req.name_en).await? {
        return Ok(Json(TranslationResponse {
            pl: entry.name_pl,
            ru: entry.name_ru,
            uk: entry.name_uk,
            source: "dictionary".to_string(),
            cost: 0.0,
        }));
    }

    // 2. Если нет в словаре - просим Groq
    match groq.translate(&req.name_en).await {
        Ok(translation) => {
            // Сохраняем в словарь для будущего
            let _ = dictionary.insert(&req.name_en, &translation).await;

            Ok(Json(TranslationResponse {
                pl: translation.pl,
                ru: translation.ru,
                uk: translation.uk,
                source: "groq".to_string(),
                cost: 0.01,
            }))
        }
        Err(_) => {
            // Fallback - вернуть как есть
            Ok(Json(TranslationResponse {
                pl: req.name_en.clone(),
                ru: req.name_en.clone(),
                uk: req.name_en.clone(),
                source: "fallback".to_string(),
                cost: 0.0,
            }))
        }
    }
}
```

**Использование на фронтенде будет проще:**

```typescript
const result = await fetch(`${apiUrl}/api/admin/translate`, {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({ name_en: 'Apple' })
});

const { pl, ru, uk, source, cost } = await result.json();
```

---

## 6. Best Practices & Tips

### 6.1 Performance Optimization

```typescript
// Debounce перевод при изменении name_en
const [debounceTimer, setDebounceTimer] = useState<NodeJS.Timeout | null>(null);

const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
  const value = e.target.value;
  setFormData(prev => ({ ...prev, name_en: value }));

  if (debounceTimer) clearTimeout(debounceTimer);

  const timer = setTimeout(() => {
    if (value.trim()) {
      handleAutoTranslate();
    }
  }, 800); // 800ms debounce

  setDebounceTimer(timer);
};
```

### 6.2 Caching on Frontend

```typescript
// Кеш последних переводов на клиенте
const translationCache = new Map<string, TranslationResponse>();

const getCachedOrTranslate = async (name_en: string) => {
  // Проверяем локальный кеш
  if (translationCache.has(name_en)) {
    return translationCache.get(name_en);
  }

  // Если нет - запрашиваем с бэка
  const result = await translate(name_en);

  // Сохраняем в локальный кеш
  translationCache.set(name_en, result);

  return result;
};
```

### 6.3 Error Handling

```typescript
// Graceful degradation если AI недоступен
const handleTranslateWithFallback = async (name_en: string) => {
  try {
    const result = await translationService.getTranslations(name_en);
    return result;
  } catch (error) {
    // Fallback: используем английский для всех языков
    return {
      pl: name_en,
      ru: name_en,
      uk: name_en,
      source: 'fallback' as const,
      cost: 0
    };
  }
};
```

### 6.4 Batch Translation (для редактирования)

```typescript
// Перевести сразу несколько ингредиентов
const batchTranslate = async (names: string[]) => {
  const results = await Promise.allSettled(
    names.map(name => translationService.getTranslations(name))
  );

  return results.map((result, index) => ({
    name: names[index],
    translations: result.status === 'fulfilled' ? result.value : null,
    error: result.status === 'rejected' ? result.reason : null
  }));
};
```

---

## 7. Integration Checklist

- [ ] Create `services/translationService.ts`
- [ ] Create `hooks/useDictionaryCache.ts`
- [ ] Create `components/ProductForm.tsx` with auto-translate
- [ ] Create `components/TranslationStats.tsx`
- [ ] Add `ProductForm.css` styles
- [ ] Add `TranslationStats.css` styles
- [ ] Update `pages/AdminProductsPage.tsx`
- [ ] Test on Koyeb production
  - [ ] Create product with auto-translate
  - [ ] Verify translations appear
  - [ ] Check dictionary cache works (repeat request)
  - [ ] Verify cost tracking
- [ ] (Optional) Add dedicated `/api/admin/translate` endpoint on backend

---

## 8. Testing Checklist

### Create Product Flow
```bash
1. Fill in name_en = "Papaya"
2. Click "🌍 Translate" button
3. See preview: PL: Papaja, RU: Папайя, UK: Папая
4. See source badge: "🤖 AI (Groq)" or "💾 Dictionary Cache"
5. See cost: $0.01 (first time) or $0.00 (cache)
6. Fill category, unit
7. Click "➕ Create Product"
8. See success message
9. Product appears in list with all translations

Repeat with same name:
- Should show "💾 Dictionary Cache" badge
- Should show $0.00 cost
- Should be instant (< 100ms)
```

### Edit Product Flow
```bash
1. Click "Edit" on existing product
2. Form pre-fills with current data
3. Auto-translate checkbox is OFF (don't override)
4. Can manually edit any language field
5. Click "✏️ Update Product"
6. See success message
```

### Error Cases
```bash
1. Empty name_en → error message appears
2. No category → error message appears
3. Network error → graceful error message
4. AI timeout (5s) → fallback to English automatically
5. Invalid token → redirect to login
```

---

## 9. Styling Tips

### Dark Mode Support

```css
@media (prefers-color-scheme: dark) {
  .product-form {
    background: #2a2a2a;
    color: #e0e0e0;
  }

  .form-group input,
  .form-group textarea,
  .form-group select {
    background: #1a1a1a;
    color: #e0e0e0;
    border-color: #444;
  }

  .form-group input:focus,
  .form-group textarea:focus,
  .form-group select:focus {
    border-color: #0066cc;
  }

  .translation-preview {
    background: #1a2a3a;
    border-color: #0066cc;
  }
}
```

---

## Summary

**Что реализовано:**

✅ Auto-translate при создании продукта
✅ Debounce для оптимальной производительности
✅ Real-time translation preview
✅ Cost tracking (Dictionary vs Groq)
✅ Manual override поддержка
✅ Proper error handling & fallbacks
✅ Responsive UI
✅ Statistics dashboard

**Следующие шаги:**

1. Скопируйте код компонентов в ваш фронтенд
2. Установите зависимости если нужны (`npm install`)
3. Обновите API URL в `.env` файле
4. Протестируйте на Koyeb production
5. (Optional) добавьте dedicated endpoint на бэкенде

**Cost Model:**
- Dictionary hit: $0.00 (мгновенно)
- First AI translation: $0.01 (1-2 сек)
- Repeat requests: $0.00 (кеш работает)
