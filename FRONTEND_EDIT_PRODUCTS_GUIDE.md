# ✏️ Редактирование продуктов с опциональными переводами

## Сценарий

При редактировании продукта администратор может:
1. **Только изменить English название** → остальное не трогать
2. **Изменить English + автоматически перевести** → новые переводы заменят старые
3. **Вручную отредактировать каждый язык** → полный контроль

---

## Компонент EditProductForm

```typescript
// components/EditProductForm.tsx
import React, { useState, useEffect } from 'react';
import TranslationService from '../services/translationService';
import { useCategories } from '../hooks/useCategories';
import './ProductForm.css';

interface EditProductFormProps {
  productId: string;
  initialData: {
    id: string;
    name_en: string;
    name_pl: string;
    name_uk: string;
    name_ru: string;
    category_id: string;
    unit: string;
    description: string;
    image_url?: string;
  };
  onSuccess: () => void;
  onCancel: () => void;
}

const EditProductForm: React.FC<EditProductFormProps> = ({
  productId,
  initialData,
  onSuccess,
  onCancel
}) => {
  const [formData, setFormData] = useState(initialData);
  const [originalData, setOriginalData] = useState(initialData);
  
  const [showTranslateOptions, setShowTranslateOptions] = useState(false);
  const [autoTranslateNewName, setAutoTranslateNewName] = useState(false);
  const [translations, setTranslations] = useState({
    pl: initialData.name_pl,
    ru: initialData.name_ru,
    uk: initialData.name_uk,
    source: 'none' as 'dictionary' | 'groq' | 'fallback' | 'none',
    cost: 0
  });

  const [isTranslating, setIsTranslating] = useState(false);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  const { categories, loading: categoriesLoading } = useCategories();

  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.REACT_APP_API_URL || 
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';
  const translationService = new TranslationService(apiUrl, token);

  // Проверяем изменения
  useEffect(() => {
    const changed = JSON.stringify(formData) !== JSON.stringify(originalData);
    setHasChanges(changed);
  }, [formData]);

  // Если name_en изменился, показываем опцию перевести
  useEffect(() => {
    if (formData.name_en !== originalData.name_en) {
      setShowTranslateOptions(true);
    } else {
      setShowTranslateOptions(false);
      setAutoTranslateNewName(false);
    }
  }, [formData.name_en]);

  // Auto-translate если выбрано
  const handleAutoTranslateNewName = async () => {
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

      // Предлагаем пользователю применить новые переводы
      const shouldApply = window.confirm(
        `New translations found:\n\n` +
        `🇵🇱 Polish: ${result.pl}\n` +
        `🇷🇺 Russian: ${result.ru}\n` +
        `🇺🇦 Ukrainian: ${result.uk}\n\n` +
        `Apply these translations?`
      );

      if (shouldApply) {
        setFormData(prev => ({
          ...prev,
          name_pl: result.pl,
          name_ru: result.ru,
          name_uk: result.uk
        }));
      }
    } catch (err) {
      setError('Translation failed. Try again or enter manually.');
    } finally {
      setIsTranslating(false);
    }
  };

  // Submit
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!formData.name_en.trim() || !formData.category_id) {
      setError('Please fill in required fields');
      return;
    }

    if (!hasChanges) {
      onCancel();
      return;
    }

    setIsSubmitting(true);
    setError('');

    try {
      const response = await fetch(`${apiUrl}/api/admin/products/${productId}`, {
        method: 'PUT',
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
          auto_translate: autoTranslateNewName
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

  // Reset to original
  const handleReset = () => {
    if (window.confirm('Discard all changes?')) {
      setFormData(originalData);
      setShowTranslateOptions(false);
    }
  };

  // ======= RENDER =======

  return (
    <form onSubmit={handleSubmit} className="product-form edit-form">
      {/* HEADER */}
      <div className="form-header">
        <h3>✏️ Edit Product</h3>
        {hasChanges && <span className="badge-changed">• You have unsaved changes</span>}
      </div>

      {error && <div className="alert alert-error">{error}</div>}

      {/* ENGLISH NAME WITH TRANSLATE OPTION */}
      <div className="form-group">
        <label>Name (English) *</label>
        <div className="input-with-button">
          <input
            type="text"
            value={formData.name_en}
            onChange={e => setFormData({ ...formData, name_en: e.target.value })}
            disabled={isSubmitting}
          />
        </div>
        <small>Original: {originalData.name_en}</small>
      </div>

      {/* TRANSLATE OPTIONS (если название изменилось) */}
      {showTranslateOptions && (
        <div className="translate-options">
          <label>
            <input
              type="checkbox"
              checked={autoTranslateNewName}
              onChange={e => setAutoTranslateNewName(e.target.checked)}
              disabled={isSubmitting || isTranslating}
            />
            {' '}🤖 Auto-translate the new name to other languages?
          </label>
          <small>
            This will replace the current translations with AI translations.
            You can manually adjust them below.
          </small>

          {autoTranslateNewName && (
            <button
              type="button"
              onClick={handleAutoTranslateNewName}
              disabled={isTranslating || !formData.name_en.trim()}
              className="btn-small"
            >
              {isTranslating ? '🔄 Translating...' : '🌍 Preview Translations'}
            </button>
          )}
        </div>
      )}

      {/* TRANSLATION PREVIEW */}
      {autoTranslateNewName && translations.source !== 'none' && (
        <div className={`translation-preview source-${translations.source}`}>
          <h4>📝 New Translations</h4>
          <div className="translation-grid">
            <div className="translation-item">
              <span className="lang-flag">🇵🇱</span>
              <div className="translation-row">
                <span className="label">Polish</span>
                <span className="old">{originalData.name_pl}</span>
                <span className="arrow">→</span>
                <span className="new">{translations.pl}</span>
              </div>
            </div>
            <div className="translation-item">
              <span className="lang-flag">🇷🇺</span>
              <div className="translation-row">
                <span className="label">Russian</span>
                <span className="old">{originalData.name_ru}</span>
                <span className="arrow">→</span>
                <span className="new">{translations.ru}</span>
              </div>
            </div>
            <div className="translation-item">
              <span className="lang-flag">🇺🇦</span>
              <div className="translation-row">
                <span className="label">Ukrainian</span>
                <span className="old">{originalData.name_uk}</span>
                <span className="arrow">→</span>
                <span className="new">{translations.uk}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* MANUAL TRANSLATIONS */}
      <div className="translation-fields">
        <h4>Translations (edit manually)</h4>

        <div className="form-group">
          <label>🇵🇱 Polish</label>
          <div className="translation-input">
            <input
              type="text"
              value={formData.name_pl}
              onChange={e => setFormData({ ...formData, name_pl: e.target.value })}
              disabled={isSubmitting}
            />
            {formData.name_pl !== originalData.name_pl && (
              <span className="badge-changed">Modified</span>
            )}
          </div>
          <small>Original: {originalData.name_pl}</small>
        </div>

        <div className="form-group">
          <label>🇷🇺 Russian</label>
          <div className="translation-input">
            <input
              type="text"
              value={formData.name_ru}
              onChange={e => setFormData({ ...formData, name_ru: e.target.value })}
              disabled={isSubmitting}
            />
            {formData.name_ru !== originalData.name_ru && (
              <span className="badge-changed">Modified</span>
            )}
          </div>
          <small>Original: {originalData.name_ru}</small>
        </div>

        <div className="form-group">
          <label>🇺🇦 Ukrainian</label>
          <div className="translation-input">
            <input
              type="text"
              value={formData.name_uk}
              onChange={e => setFormData({ ...formData, name_uk: e.target.value })}
              disabled={isSubmitting}
            />
            {formData.name_uk !== originalData.name_uk && (
              <span className="badge-changed">Modified</span>
            )}
          </div>
          <small>Original: {originalData.name_uk}</small>
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
          <option value="">Select category...</option>
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

      {/* ACTIONS */}
      <div className="form-actions">
        <button 
          type="submit" 
          disabled={!hasChanges || isSubmitting || isTranslating}
          className="btn btn-primary"
        >
          {isSubmitting ? '💾 Saving...' : '✏️ Save Changes'}
        </button>
        <button 
          type="button" 
          onClick={handleReset}
          disabled={!hasChanges || isSubmitting}
          className="btn btn-secondary"
        >
          Reset
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
  );
};

export default EditProductForm;
```

---

## Дополнительные стили для редактирования

```css
/* Добавить в ProductForm.css */

.edit-form {
  /* Дополнительные стили для редактирования */
}

.form-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 2px solid #e0e0e0;
}

.form-header h3 {
  margin: 0;
  font-size: 18px;
  color: #333;
}

.badge-changed {
  display: inline-block;
  background: #fff3cd;
  color: #856404;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
}

/* Translate options */
.translate-options {
  background: #f0f7ff;
  border: 1px solid #cce5ff;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}

.translate-options label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  cursor: pointer;
  font-weight: 500;
}

.translate-options input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.translate-options small {
  display: block;
  color: #666;
  margin-bottom: 12px;
}

.btn-small {
  padding: 8px 12px;
  background: #0066cc;
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.btn-small:hover:not(:disabled) {
  background: #0052a3;
}

.btn-small:disabled {
  background: #ccc;
  cursor: not-allowed;
}

/* Translation input with badge */
.translation-input {
  display: flex;
  gap: 8px;
  align-items: center;
}

.translation-input input {
  flex: 1;
}

.translation-input .badge-changed {
  flex-shrink: 0;
}

/* Translation preview for edits */
.translation-preview .translation-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.translation-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.translation-row .label {
  font-weight: 600;
  color: #666;
  min-width: 70px;
}

.translation-row .old {
  color: #999;
  text-decoration: line-through;
  font-size: 11px;
}

.translation-row .arrow {
  color: #999;
  font-weight: bold;
}

.translation-row .new {
  color: #28a745;
  font-weight: 600;
}

/* Отключить кнопку если нет изменений */
button[type="submit"]:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Больше действий для редактирования */
.form-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.form-actions .btn {
  flex: 1;
  min-width: 120px;
}

@media (max-width: 768px) {
  .form-actions {
    flex-direction: column;
  }

  .form-actions .btn {
    width: 100%;
  }

  .form-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
```

---

## Использование в ProductList

```typescript
// components/ProductList.tsx
import React, { useState } from 'react';
import EditProductForm from './EditProductForm';

interface ProductListProps {
  products: any[];
  onProductUpdated: () => void;
}

const ProductList: React.FC<ProductListProps> = ({
  products,
  onProductUpdated
}) => {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingData, setEditingData] = useState<any>(null);

  const handleEditClick = (product: any) => {
    setEditingId(product.id);
    setEditingData(product);
  };

  const handleEditSuccess = () => {
    setEditingId(null);
    setEditingData(null);
    onProductUpdated();
  };

  if (editingId && editingData) {
    return (
      <EditProductForm
        productId={editingId}
        initialData={editingData}
        onSuccess={handleEditSuccess}
        onCancel={() => {
          setEditingId(null);
          setEditingData(null);
        }}
      />
    );
  }

  return (
    <div className="products-grid">
      {products.map(product => (
        <div key={product.id} className="product-card">
          <h3>{product.name_en}</h3>
          <div className="translations">
            <span>🇵🇱 {product.name_pl}</span>
            <span>🇷🇺 {product.name_ru}</span>
            <span>🇺🇦 {product.name_uk}</span>
          </div>
          <button
            onClick={() => handleEditClick(product)}
            className="btn-edit"
          >
            ✏️ Edit
          </button>
        </div>
      ))}
    </div>
  );
};

export default ProductList;
```

---

## 🧪 Тестирование редактирования

### Сценарий 1: Изменить только English имя
```
1. Открыть редактирование продукта
2. Изменить name_en с "Tomato" на "Red Tomato"
3. Не выбирать auto-translate
4. Нажать Save Changes
   ✅ Должен сохраниться с новым английским названием
   ✅ Остальные языки не меняются
```

### Сценарий 2: Изменить + перевести
```
1. Открыть редактирование продукта
2. Изменить name_en с "Tomato" на "Cherry Tomato"
3. Выбрать чекбокс "Auto-translate"
4. Нажать "🌍 Preview Translations"
5. Увидеть новые переводы (старые → новые)
6. Нажать Save Changes
   ✅ Должны примениться новые переводы
   ✅ Если переводы из кеша - должно быть $0.00
   ✅ Если новый перевод - $0.01
```

### Сценарий 3: Вручную отредактировать
```
1. Открыть редактирование
2. Изменить name_pl с "Pomidor" на "Pomidorek"
3. Не менять другие языки
4. Нажать Save Changes
   ✅ Только Polish должен измениться
   ✅ English и остальные остаются старыми
```

---

## ✅ Checklist для фронтенда

- [ ] Скопировал `EditProductForm.tsx` в `src/components/`
- [ ] Добавил стили в `ProductForm.css`
- [ ] Обновил `ProductList.tsx` для использования `EditProductForm`
- [ ] Протестировал все три сценария редактирования
- [ ] Убедился, что "Unsaved changes" badge появляется/исчезает правильно
- [ ] Проверил, что Reset кнопка отключена когда нет изменений
- [ ] Проверил graceful degradation (если перевод не удался)

