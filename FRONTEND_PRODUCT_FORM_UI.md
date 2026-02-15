# 🎨 UI Form Layout для создания продукта

Вот правильная структура формы с правильными текстами и расположением:

---

## Полный HTML Layout

```html
<div class="product-creation-page">
  <!-- HEADER -->
  <header class="page-header">
    <nav class="breadcrumb">
      <a href="/admin">Товары</a>
      <span>/</span>
      <span>Новый продукт</span>
    </nav>
    
    <h1>Создать новый продукт</h1>
    <p class="subtitle">Добавьте новый продукт в каталог</p>
  </header>

  <!-- HINT BOX -->
  <div class="hint-box">
    <span class="hint-icon">💡</span>
    <div class="hint-content">
      <strong>Совет:</strong> Введите только английское название — система автоматически переведёт его на все языки (Polski, Українська, Русский).
    </div>
  </div>

  <!-- FORM -->
  <form class="product-form">
    <!-- SECTION 1: BASIC INFO -->
    <section class="form-section">
      <h2>Основная информация</h2>
      <p class="section-description">Заполните обязательные поля для создания продукта</p>

      <!-- NAME EN -->
      <div class="form-group">
        <label for="name_en">
          Название (English) <span class="required">*</span>
        </label>
        <div class="input-wrapper">
          <input
            id="name_en"
            type="text"
            placeholder="напр. Tomato, Apple, Milk"
            class="input-lg"
          />
          <button type="button" class="btn-translate">
            🌍 Перевести
          </button>
        </div>
        <p class="helper-text">Будет автоматически переведено на польский, украинский и русский</p>
      </div>

      <!-- AUTO-TRANSLATE TOGGLE -->
      <div class="form-group checkbox-group">
        <label class="checkbox-label">
          <input type="checkbox" id="auto_translate" checked />
          <span>🤖 Автоматический перевод на другие языки</span>
        </label>
        <p class="helper-text">При включении система автоматически переведёт названия на Polski, Українська и Русский</p>
      </div>

      <!-- TRANSLATION PREVIEW -->
      <div class="translation-preview" id="translation_preview" style="display: none;">
        <div class="preview-header">
          <h3>📝 Предпросмотр переводов</h3>
          <span class="source-badge source-groq">🤖 AI (Groq)</span>
          <span class="cost-badge">$0.01</span>
        </div>
        <div class="translation-grid">
          <div class="translation-item">
            <span class="lang-flag">🇵🇱</span>
            <span class="lang-name">Polski</span>
            <span class="translation-text" id="trans_pl">Pomidor</span>
          </div>
          <div class="translation-item">
            <span class="lang-flag">🇷🇺</span>
            <span class="lang-name">Русский</span>
            <span class="translation-text" id="trans_ru">Помидор</span>
          </div>
          <div class="translation-item">
            <span class="lang-flag">🇺🇦</span>
            <span class="lang-name">Українська</span>
            <span class="translation-text" id="trans_uk">Помідор</span>
          </div>
        </div>
      </div>
    </section>

    <!-- SECTION 2: CATEGORY & UNIT -->
    <section class="form-section">
      <h2>Характеристики</h2>

      <!-- CATEGORY -->
      <div class="form-group">
        <label for="category_id">
          Категория <span class="required">*</span>
        </label>
        <select id="category_id" class="input-lg">
          <option value="">Выберите категорию...</option>
          <option value="1">Молочные продукты и яйца</option>
          <option value="2">Мясо и птица</option>
          <option value="3">Рыба и морепродукты</option>
          <option value="4">Овощи</option>
          <option value="5">Фрукты</option>
        </select>
      </div>

      <!-- UNIT -->
      <div class="form-group">
        <label for="unit">
          Единица измерения <span class="required">*</span>
        </label>
        <select id="unit" class="input-lg">
          <option value="kilogram">килограмм (кг)</option>
          <option value="gram">грамм (г)</option>
          <option value="liter">литр (л)</option>
          <option value="milliliter">миллилитр (мл)</option>
          <option value="piece">штука</option>
          <option value="bunch">пучок</option>
          <option value="can">банка</option>
          <option value="package">упаковка</option>
        </select>
      </div>
    </section>

    <!-- SECTION 3: DESCRIPTION -->
    <section class="form-section">
      <h2>Описание</h2>

      <!-- DESCRIPTION -->
      <div class="form-group">
        <label for="description">Опишите продукт (необязательно)</label>
        <textarea
          id="description"
          class="input-lg"
          rows="4"
          placeholder="Например: Свежие помидоры, выращенные без пестицидов..."
        ></textarea>
        <p class="helper-text char-count">0/500 символов</p>
      </div>
    </section>

    <!-- SECTION 4: TRANSLATIONS -->
    <section class="form-section">
      <h2>Переводы</h2>
      <p class="section-description">Оставьте пустым для автоматического AI-перевода</p>

      <!-- POLISH -->
      <div class="form-group">
        <label for="name_pl">
          <span class="lang-flag">🇵🇱</span> Polski (польский)
        </label>
        <input
          id="name_pl"
          type="text"
          placeholder="Оставьте пустым — AI переведёт автоматически"
          class="input-lg"
        />
        <p class="helper-text">Заполнится автоматически AI-переводом</p>
      </div>

      <!-- RUSSIAN -->
      <div class="form-group">
        <label for="name_ru">
          <span class="lang-flag">🇷🇺</span> Русский
        </label>
        <input
          id="name_ru"
          type="text"
          placeholder="Оставьте пустым — AI переведёт автоматически"
          class="input-lg"
        />
        <p class="helper-text">Заполнится автоматически AI-переводом</p>
      </div>

      <!-- UKRAINIAN -->
      <div class="form-group">
        <label for="name_uk">
          <span class="lang-flag">🇺🇦</span> Українська (украинский)
        </label>
        <input
          id="name_uk"
          type="text"
          placeholder="Оставьте пустым — AI переведёт автоматически"
          class="input-lg"
        />
        <p class="helper-text">Заполнится автоматически AI-переводом</p>
      </div>
    </section>

    <!-- FORM ACTIONS -->
    <div class="form-actions">
      <button type="button" class="btn btn-secondary">
        Отмена
      </button>
      <button type="submit" class="btn btn-primary">
        ➕ Создать продукт
      </button>
    </div>
  </form>
</div>
```

---

## CSS Стили

```css
/* ============ PAGE HEADER ============ */
.page-header {
  margin-bottom: 32px;
}

.breadcrumb {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 14px;
  color: #666;
  margin-bottom: 16px;
}

.breadcrumb a {
  color: #0066cc;
  text-decoration: none;
  cursor: pointer;
}

.breadcrumb a:hover {
  text-decoration: underline;
}

.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 32px;
  font-weight: 700;
  color: #333;
}

.page-header .subtitle {
  margin: 0;
  font-size: 16px;
  color: #666;
}

/* ============ HINT BOX ============ */
.hint-box {
  background: #f0f7ff;
  border: 1px solid #cce5ff;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 32px;
  display: flex;
  gap: 12px;
}

.hint-icon {
  font-size: 24px;
  flex-shrink: 0;
}

.hint-content {
  flex: 1;
}

.hint-content strong {
  color: #0066cc;
  display: block;
  margin-bottom: 4px;
}

.hint-content {
  color: #333;
  font-size: 14px;
  line-height: 1.5;
}

/* ============ FORM ============ */
.product-form {
  max-width: 700px;
}

.form-section {
  background: white;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  padding: 24px;
  margin-bottom: 20px;
}

.form-section h2 {
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 600;
  color: #333;
}

.section-description {
  margin: 0 0 20px 0;
  font-size: 14px;
  color: #666;
}

/* ============ FORM GROUPS ============ */
.form-group {
  margin-bottom: 20px;
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-weight: 600;
  color: #333;
  font-size: 14px;
}

.required {
  color: #c33;
  margin-left: 2px;
}

/* ============ INPUTS ============ */
.input-wrapper {
  display: flex;
  gap: 8px;
  align-items: center;
}

.input-lg {
  width: 100%;
  padding: 12px 14px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  font-family: inherit;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.input-lg:focus {
  outline: none;
  border-color: #0066cc;
  box-shadow: 0 0 0 3px rgba(0, 102, 204, 0.1);
}

textarea.input-lg {
  resize: vertical;
  min-height: 100px;
}

/* ============ HELPER TEXT ============ */
.helper-text {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: #666;
  line-height: 1.4;
}

.char-count {
  text-align: right;
  color: #999;
}

/* ============ CHECKBOX ============ */
.checkbox-group {
  margin-bottom: 24px;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  font-weight: 500;
  font-size: 14px;
  margin-bottom: 12px;
}

.checkbox-label input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
  margin: 0;
  flex-shrink: 0;
}

.checkbox-label span {
  color: #333;
}

/* ============ TRANSLATION PREVIEW ============ */
.translation-preview {
  background: #f0f7ff;
  border: 2px solid #0066cc;
  border-radius: 8px;
  padding: 16px;
  margin: 20px 0;
  animation: slideIn 0.3s ease-out;
}

.preview-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.preview-header h3 {
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

.source-badge.source-groq {
  background: #cce5ff;
  color: #004085;
}

.source-badge.source-dictionary {
  background: #d4edda;
  color: #155724;
}

.cost-badge {
  display: inline-block;
  background: #fff3cd;
  color: #856404;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  margin-left: auto;
}

.translation-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
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
  line-height: 1;
}

.lang-name {
  font-size: 11px;
  font-weight: 600;
  color: #666;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.translation-text {
  font-size: 14px;
  font-weight: 600;
  color: #333;
  text-align: center;
  word-break: break-word;
}

/* ============ BUTTONS ============ */
.btn-translate {
  padding: 10px 16px;
  background: #0066cc;
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  font-size: 14px;
  transition: background 0.2s, transform 0.1s;
}

.btn-translate:hover {
  background: #0052a3;
  transform: translateY(-1px);
}

.btn-translate:disabled {
  background: #ccc;
  cursor: not-allowed;
  transform: none;
}

/* ============ FORM ACTIONS ============ */
.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 32px;
  justify-content: flex-end;
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
  min-width: 180px;
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

/* ============ ANIMATIONS ============ */
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

/* ============ RESPONSIVE ============ */
@media (max-width: 768px) {
  .page-header h1 {
    font-size: 24px;
  }

  .form-section {
    padding: 16px;
  }

  .translation-grid {
    grid-template-columns: 1fr;
  }

  .form-actions {
    flex-direction: column;
  }

  .btn-primary {
    flex: none;
    width: 100%;
  }

  .input-wrapper {
    flex-direction: column;
  }

  .btn-translate {
    width: 100%;
  }
}
```

---

## React компонент с правильными текстами

```tsx
import React, { useState, useEffect } from 'react';
import TranslationService from '../services/translationService';
import { useCategories } from '../hooks/useCategories';
import './ProductForm.css';

interface ProductFormData {
  name_en: string;
  name_pl: string;
  name_ru: string;
  name_uk: string;
  category_id: string;
  unit: string;
  description: string;
  auto_translate: boolean;
}

const CreateProductForm: React.FC<{ onSuccess: () => void }> = ({ onSuccess }) => {
  const [formData, setFormData] = useState<ProductFormData>({
    name_en: '',
    name_pl: '',
    name_ru: '',
    name_uk: '',
    category_id: '',
    unit: 'kilogram',
    description: '',
    auto_translate: true
  });

  const [translations, setTranslations] = useState({
    pl: '',
    ru: '',
    uk: '',
    source: 'none' as 'dictionary' | 'groq' | 'fallback' | 'none'
  });

  const [isTranslating, setIsTranslating] = useState(false);
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { categories, loading: categoriesLoading } = useCategories();
  const token = localStorage.getItem('admin_token') || '';
  const apiUrl = process.env.REACT_APP_API_URL || 
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';
  const translationService = new TranslationService(apiUrl, token);

  // Debounce auto-translate
  useEffect(() => {
    if (!formData.auto_translate || !formData.name_en.trim()) return;

    const timer = setTimeout(() => {
      handleAutoTranslate();
    }, 800);

    return () => clearTimeout(timer);
  }, [formData.auto_translate, formData.name_en]);

  const handleAutoTranslate = async () => {
    if (!formData.name_en.trim()) return;

    setIsTranslating(true);

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
      setError('Ошибка при переводе. Попробуйте позже или введите вручную.');
    } finally {
      setIsTranslating(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (!formData.name_en.trim() || !formData.category_id) {
      setError('Заполните все обязательные поля');
      return;
    }

    setIsSubmitting(true);

    try {
      const response = await fetch(`${apiUrl}/api/admin/products`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
      });

      if (!response.ok) {
        const data = await response.json();
        setError(data.details || 'Ошибка при сохранении');
        return;
      }

      onSuccess();
    } catch (err) {
      setError('Ошибка сети');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="product-creation-page">
      <header className="page-header">
        <nav className="breadcrumb">
          <a href="/admin">Товары</a>
          <span>/</span>
          <span>Новый продукт</span>
        </nav>
        <h1>Создать новый продукт</h1>
        <p className="subtitle">Добавьте новый продукт в каталог</p>
      </header>

      <div className="hint-box">
        <span className="hint-icon">💡</span>
        <div className="hint-content">
          <strong>Совет:</strong> Введите только английское название — система автоматически переведёт его на все языки (Polski, Українська, Русский).
        </div>
      </div>

      <form className="product-form" onSubmit={handleSubmit}>
        {error && <div className="alert alert-error">{error}</div>}

        {/* SECTION 1: BASIC INFO */}
        <section className="form-section">
          <h2>Основная информация</h2>
          <p className="section-description">Заполните обязательные поля для создания продукта</p>

          {/* NAME EN */}
          <div className="form-group">
            <label htmlFor="name_en">
              Название (English) <span className="required">*</span>
            </label>
            <div className="input-wrapper">
              <input
                id="name_en"
                type="text"
                value={formData.name_en}
                onChange={e => setFormData({ ...formData, name_en: e.target.value })}
                placeholder="напр. Tomato, Apple, Milk"
                className="input-lg"
              />
              <button
                type="button"
                onClick={handleAutoTranslate}
                disabled={isTranslating || !formData.name_en.trim()}
                className="btn-translate"
              >
                {isTranslating ? '🔄 Перевод...' : '🌍 Перевести'}
              </button>
            </div>
            <p className="helper-text">Будет автоматически переведено на польский, украинский и русский</p>
          </div>

          {/* AUTO-TRANSLATE TOGGLE */}
          <div className="form-group checkbox-group">
            <label className="checkbox-label">
              <input
                type="checkbox"
                id="auto_translate"
                checked={formData.auto_translate}
                onChange={e =>
                  setFormData({ ...formData, auto_translate: e.target.checked })
                }
                disabled={isSubmitting}
              />
              <span>🤖 Автоматический перевод на другие языки</span>
            </label>
            <p className="helper-text">При включении система автоматически переведёт названия на Polski, Українська и Русский</p>
          </div>

          {/* TRANSLATION PREVIEW */}
          {formData.auto_translate && translations.source !== 'none' && (
            <div className="translation-preview">
              <div className="preview-header">
                <h3>📝 Предпросмотр переводов</h3>
                <span className={`source-badge source-${translations.source}`}>
                  {translations.source === 'dictionary' ? '💾 Кеш' : '🤖 AI'}
                </span>
              </div>
              <div className="translation-grid">
                <div className="translation-item">
                  <span className="lang-flag">🇵🇱</span>
                  <span className="lang-name">Polski</span>
                  <span className="translation-text">{translations.pl}</span>
                </div>
                <div className="translation-item">
                  <span className="lang-flag">🇷🇺</span>
                  <span className="lang-name">Русский</span>
                  <span className="translation-text">{translations.ru}</span>
                </div>
                <div className="translation-item">
                  <span className="lang-flag">🇺🇦</span>
                  <span className="lang-name">Українська</span>
                  <span className="translation-text">{translations.uk}</span>
                </div>
              </div>
            </div>
          )}
        </section>

        {/* SECTION 2: CATEGORY & UNIT */}
        <section className="form-section">
          <h2>Характеристики</h2>

          <div className="form-group">
            <label htmlFor="category_id">
              Категория <span className="required">*</span>
            </label>
            <select
              id="category_id"
              value={formData.category_id}
              onChange={e =>
                setFormData({ ...formData, category_id: e.target.value })
              }
              className="input-lg"
              disabled={categoriesLoading || isSubmitting}
            >
              <option value="">Выберите категорию...</option>
              {categories.map(cat => (
                <option key={cat.id} value={cat.id}>{cat.name}</option>
              ))}
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="unit">
              Единица измерения <span className="required">*</span>
            </label>
            <select
              id="unit"
              value={formData.unit}
              onChange={e => setFormData({ ...formData, unit: e.target.value })}
              className="input-lg"
              disabled={isSubmitting}
            >
              <option value="kilogram">килограмм (кг)</option>
              <option value="gram">грамм (г)</option>
              <option value="liter">литр (л)</option>
              <option value="milliliter">миллилитр (мл)</option>
              <option value="piece">штука</option>
              <option value="bunch">пучок</option>
              <option value="can">банка</option>
              <option value="package">упаковка</option>
            </select>
          </div>
        </section>

        {/* SECTION 3: DESCRIPTION */}
        <section className="form-section">
          <h2>Описание</h2>

          <div className="form-group">
            <label htmlFor="description">Опишите продукт (необязательно)</label>
            <textarea
              id="description"
              value={formData.description}
              onChange={e =>
                setFormData({ ...formData, description: e.target.value })
              }
              className="input-lg"
              rows={4}
              placeholder="Например: Свежие помидоры, выращенные без пестицидов..."
            />
            <p className="helper-text char-count">{formData.description.length}/500 символов</p>
          </div>
        </section>

        {/* SECTION 4: TRANSLATIONS */}
        <section className="form-section">
          <h2>Переводы</h2>
          <p className="section-description">Оставьте пустым для автоматического AI-перевода</p>

          <div className="form-group">
            <label htmlFor="name_pl">
              <span className="lang-flag">🇵🇱</span> Polski (польский)
            </label>
            <input
              id="name_pl"
              type="text"
              value={formData.name_pl}
              onChange={e => setFormData({ ...formData, name_pl: e.target.value })}
              placeholder="Оставьте пустым — AI переведёт автоматически"
              className="input-lg"
              disabled={isSubmitting}
            />
            <p className="helper-text">Заполнится автоматически AI-переводом</p>
          </div>

          <div className="form-group">
            <label htmlFor="name_ru">
              <span className="lang-flag">🇷🇺</span> Русский
            </label>
            <input
              id="name_ru"
              type="text"
              value={formData.name_ru}
              onChange={e => setFormData({ ...formData, name_ru: e.target.value })}
              placeholder="Оставьте пустым — AI переведёт автоматически"
              className="input-lg"
              disabled={isSubmitting}
            />
            <p className="helper-text">Заполнится автоматически AI-переводом</p>
          </div>

          <div className="form-group">
            <label htmlFor="name_uk">
              <span className="lang-flag">🇺🇦</span> Українська (украинский)
            </label>
            <input
              id="name_uk"
              type="text"
              value={formData.name_uk}
              onChange={e => setFormData({ ...formData, name_uk: e.target.value })}
              placeholder="Оставьте пустым — AI переведёт автоматически"
              className="input-lg"
              disabled={isSubmitting}
            />
            <p className="helper-text">Заполнится автоматически AI-переводом</p>
          </div>
        </section>

        {/* FORM ACTIONS */}
        <div className="form-actions">
          <button type="button" className="btn btn-secondary">
            Отмена
          </button>
          <button type="submit" disabled={isSubmitting} className="btn btn-primary">
            {isSubmitting ? '💾 Сохраняю...' : '➕ Создать продукт'}
          </button>
        </div>
      </form>
    </div>
  );
};

export default CreateProductForm;
```

---

## 📋 Структура по разделам

| Раздел | Назначение | Поля |
|--------|-----------|------|
| **Основная информация** | Главные параметры | Название (EN), Auto-translate toggle, Preview |
| **Характеристики** | Категория и единица | Category, Unit |
| **Описание** | Опциональное описание | Description |
| **Переводы** | Вручную (опционально) | Polski, Russian, Ukrainian |

---

## 🎯 Правильный UX поток

```
1. Admin вводит "Tomato"
         ↓
2. Автоматически (через 800ms):
   - Система переводит
   - Показывает preview
         ↓
3. Admin видит:
   🇵🇱 Pomidor
   🇷🇺 Помидор
   🇺🇦 Помідор
         ↓
4. Admin выбирает категорию и unit
         ↓
5. Нажимает "➕ Создать продукт"
         ↓
6. ✅ Продукт создан с переводами
```

Это правильная и удобная форма! 🎉
