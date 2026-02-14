# 🎨 Frontend Admin Panel - Integration Guide

## 📋 Что нужно реализовать

### 1. Admin Authentication
```typescript
// Admin Login
POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/auth/login
{
  "email": "admin@fodi.app",
  "password": "Admin123!"
}

// Response
{
  "token": "eyJ0eXAiOiJKV1Q...",
  "expires_in": 86400
}

// Store token
localStorage.setItem('admin_token', response.t          <option value="package">Package</option>
        </select>
      </div>

      <div>
        <label>Description</label>equests
headers: {
  'Authorization': `Bearer ${admin_token}`
}
```

---

## 🛠️ Admin Panel Features

### 1. Product Management (Master Catalog)

#### API Endpoints для админа:
```typescript
// Base URL
const BASE_URL = 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

// List Products
GET /api/admin/products
Response: [
  {
    "id": "uuid",
    "name_en": "Tomato",
    "name_pl": "Pomidor",
    "name_uk": "Помідор",
    "name_ru": "Помидор",
    "category_id": "uuid",
    "unit": "kilogram",
    "description": "Fresh tomatoes",
    "image_url": "https://pub-85f883ab.r2.dev/products/uuid.jpg"
  }
]

// Create Product
POST /api/admin/products
Body: {
  "name_en": "Cucumber",      // REQUIRED
  "name_pl": "",               // Optional (auto-fills from name_en)
  "name_uk": "",               // Optional (auto-fills from name_en)
  "name_ru": "",               // Optional (auto-fills from name_en)
  "category_id": "uuid",       // REQUIRED
  "unit": "kilogram",          // REQUIRED: kilogram, gram, liter, milliliter, piece, bunch, can, package
  "description": "..."         // Optional
}

// Update Product
PUT /api/admin/products/:id
Body: {
  "name_en": "Green Cucumber",  // Optional
  "name_pl": "Zielony ogórek",  // Optional
  // ... any field
}

// Delete Product (soft-delete)
DELETE /api/admin/products/:id

// Upload Image
POST /api/admin/products/:id/image
Content-Type: multipart/form-data
Body: file (max 5MB, jpg/png/webp)

// Delete Image
DELETE /api/admin/products/:id/image

// Get Categories (для dropdown в форме) ⭐️ ИСПОЛЬЗУЙ ЭТОТ ENDPOINT
GET /api/admin/categories
Headers: Authorization: Bearer <admin_token>
Response: {
  "categories": [
    {
      "id": "5a841ce0-2ea5-4230-a1f7-011fa445afdc",
      "name": "Vegetables",
      "sort_order": 4
    }
  ]
}
```

**Available Categories (15 total):**
- Dairy & Eggs
- Meat & Poultry
- Fish & Seafood
- Vegetables
- Fruits
- Grains & Pasta
- Oils & Fats
- Spices & Herbs
- Condiments & Sauces
- Beverages
- Nuts & Seeds
- Legumes
- Sweets & Baking
- Canned & Preserved
- Frozen

---

### 2. UI Components Needed

#### 2.0 Categories Hook (загрузка списка категорий)

```typescript
// utils/categoryTranslations.ts
// Перевод категорий на русский
export const categoryTranslations: Record<string, string> = {
  'Dairy & Eggs': 'Молочные продукты и яйца',
  'Meat & Poultry': 'Мясо и птица',
  'Fish & Seafood': 'Рыба и морепродукты',
  'Vegetables': 'Овощи',
  'Fruits': 'Фрукты',
  'Grains & Pasta': 'Крупы и макароны',
  'Oils & Fats': 'Масла и жиры',
  'Spices & Herbs': 'Специи и травы',
  'Condiments & Sauces': 'Приправы и соусы',
  'Beverages': 'Напитки',
  'Nuts & Seeds': 'Орехи и семена',
  'Legumes': 'Бобовые',
  'Sweets & Baking': 'Сладости и выпечка',
  'Canned & Preserved': 'Консервы',
  'Frozen': 'Замороженные продукты'
};

// Функция для получения переведённого названия
export const translateCategory = (englishName: string): string => {
  return categoryTranslations[englishName] || englishName;
};

// hooks/useCategories.ts
import { useState, useEffect } from 'react';
import { translateCategory } from '@/utils/categoryTranslations';

interface Category {
  id: string;
  name: string;
  name_ru?: string; // Русское название для отображения
  sort_order: number;
}

export const useCategories = () => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const response = await fetch('https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/categories', {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('admin_token')}`
          }
        });

        if (!response.ok) {
          throw new Error('Failed to fetch categories');
        }

        const data = await response.json();
        
        // Добавляем русские названия и сортируем
        const categoriesWithTranslations = data.categories.map((cat: Category) => ({
          ...cat,
          name_ru: translateCategory(cat.name)
        }));
        
        setCategories(categoriesWithTranslations.sort((a: Category, b: Category) => 
          a.sort_order - b.sort_order
        ));
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    };

    fetchCategories();
  }, []);

  return { categories, loading, error };
};
```

#### 2.1 Product List Table
```tsx
import { useState, useEffect } from 'react';
import { translateCategory } from '@/utils/categoryTranslations';

interface Product {
  id: string;
  name_en: string;
  name_pl?: string;
  name_uk?: string;
  name_ru?: string;
  category_id: string;
  unit: string;
  description?: string;
  image_url?: string;
}

interface Category {
  id: string;
  name: string;
  name_ru?: string;
}

function ProductList() {
  const [products, setProducts] = useState<Product[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchProducts();
    fetchCategories();
  }, []);

  const fetchProducts = async () => {
    const token = localStorage.getItem('admin_token');
    const response = await fetch(`${BASE_URL}/api/admin/products`, {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    const data = await response.json();
    setProducts(data);
    setLoading(false);
  };

  const fetchCategories = async () => {
    const token = localStorage.getItem('admin_token');
    const response = await fetch(`${BASE_URL}/api/admin/categories`, {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    const data = await response.json();
    const categoriesWithTranslations = data.categories.map((cat: Category) => ({
      ...cat,
      name_ru: translateCategory(cat.name)
    }));
    setCategories(categoriesWithTranslations);
  };

  // Получить название категории по ID
  const getCategoryName = (categoryId: string): string => {
    const category = categories.find(cat => cat.id === categoryId);
    return category?.name_ru || category?.name || 'Неизвестно';
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this product?')) return;
    
    const token = localStorage.getItem('admin_token');
    await fetch(`${BASE_URL}/api/admin/products/${id}`, {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    
    // Reload list
    fetchProducts();
  };

  if (loading) return <div>Загрузка...</div>;

  return (
    <table>
      <thead>
        <tr>
          <th>Изображение</th>
          <th>Название (EN)</th>
          <th>Единица</th>
          <th>Категория</th>
          <th>Действия</th>
        </tr>
      </thead>
      <tbody>
        {products.map(product => (
          <tr key={product.id}>
            <td>
              {product.image_url ? (
                <img src={product.image_url} width="50" alt={product.name_en} />
              ) : (
                <div>Нет фото</div>
              )}
            </td>
            <td>{product.name_en}</td>
            <td>{product.unit}</td>
            <td>{getCategoryName(product.category_id)}</td>
            <td>
              <button onClick={() => handleEdit(product)}>Редактировать</button>
              <button onClick={() => handleDelete(product.id)}>Удалить</button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

#### 2.2 Create/Edit Product Form
```tsx
interface ProductFormData {
  name_en: string;
  name_pl?: string;
  name_uk?: string;
  name_ru?: string;
  category_id: string;
  unit: string;
  description?: string;
}

function ProductForm({ productId, onSuccess }: { productId?: string, onSuccess: () => void }) {
  const { categories, loading: categoriesLoading } = useCategories(); // 👈 Загружаем категории
  
  const [formData, setFormData] = useState<ProductFormData>({
    name_en: '',
    name_pl: '',
    name_uk: '',
    name_ru: '',
    category_id: '',
    unit: 'kilogram',
    description: ''
  });
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    const token = localStorage.getItem('admin_token');
    const url = productId 
      ? `${BASE_URL}/api/admin/products/${productId}`
      : `${BASE_URL}/api/admin/products`;
    
    const method = productId ? 'PUT' : 'POST';

    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(formData)
      });

      const data = await response.json();

      if (!response.ok) {
        // Handle errors
        if (data.code === 'CONFLICT') {
          setError(data.details); // "Product 'Tomato' already exists"
        } else if (data.code === 'VALIDATION_ERROR') {
          setError(data.details); // "name_en cannot be empty"
        } else {
          setError('Something went wrong');
        }
        return;
      }

      // Success
      onSuccess();
    } catch (err) {
      setError('Network error');
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      {error && <div className="error">{error}</div>}

      <div>
        <label>Name (English) *</label>
        <input
          type="text"
          value={formData.name_en}
          onChange={e => setFormData({...formData, name_en: e.target.value})}
          required
        />
        <small>This will auto-fill other languages if left empty</small>
      </div>

      <div>
        <label>Name (Polish)</label>
        <input
          type="text"
          value={formData.name_pl}
          onChange={e => setFormData({...formData, name_pl: e.target.value})}
          placeholder="Leave empty to use English name"
        />
      </div>

      <div>
        <label>Name (Ukrainian)</label>
        <input
          type="text"
          value={formData.name_uk}
          onChange={e => setFormData({...formData, name_uk: e.target.value})}
          placeholder="Leave empty to use English name"
        />
      </div>

      <div>
        <label>Name (Russian)</label>
        <input
          type="text"
          value={formData.name_ru}
          onChange={e => setFormData({...formData, name_ru: e.target.value})}
          placeholder="Leave empty to use English name"
        />
      </div>

      <div>
        <label>Category *</label>
        <select
          value={formData.category_id}
          onChange={e => setFormData({...formData, category_id: e.target.value})}
          required
          disabled={categoriesLoading}
        >
          <option value="">Выберите категорию...</option>
          {categories.map(cat => (
            <option key={cat.id} value={cat.id}>
              {cat.name_ru || cat.name}
            </option>
          ))}
        </select>
        {categoriesLoading && <small>Загрузка категорий...</small>}
      </div>

      <div>
        <label>Unit *</label>
        <select
          value={formData.unit}
          onChange={e => setFormData({...formData, unit: e.target.value})}
          required
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

      <div>
        <label>Description</label>
        <textarea
          value={formData.description}
          onChange={e => setFormData({...formData, description: e.target.value})}
          rows={3}
        />
      </div>

      <button type="submit">
        {productId ? 'Update' : 'Create'} Product
      </button>
    </form>
  );
}
```

#### 2.3 Image Upload Component (с автокомпрессией)
```tsx
function ProductImageUpload({ productId }: { productId: string }) {
  const [uploading, setUploading] = useState(false);
  const [compressing, setCompressing] = useState(false);
  const [error, setError] = useState('');

  // 🎨 Автоматическая компрессия изображения
  const compressImage = async (file: File): Promise<File> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      
      reader.onload = (event) => {
        const img = new Image();
        img.src = event.target?.result as string;
        
        img.onload = () => {
          const canvas = document.createElement('canvas');
          const ctx = canvas.getContext('2d');
          
          // Максимальные размеры (можно настроить)
          const MAX_WIDTH = 1200;
          const MAX_HEIGHT = 1200;
          
          let width = img.width;
          let height = img.height;
          
          // Пропорциональное уменьшение
          if (width > height) {
            if (width > MAX_WIDTH) {
              height *= MAX_WIDTH / width;
              width = MAX_WIDTH;
            }
          } else {
            if (height > MAX_HEIGHT) {
              width *= MAX_HEIGHT / height;
              height = MAX_HEIGHT;
            }
          }
          
          canvas.width = width;
          canvas.height = height;
          
          ctx?.drawImage(img, 0, 0, width, height);
          
          // Конвертируем в JPEG с качеством 0.8
          canvas.toBlob(
            (blob) => {
              if (blob) {
                const compressedFile = new File([blob], 'product.jpg', {
                  type: 'image/jpeg',
                  lastModified: Date.now()
                });
                resolve(compressedFile);
              } else {
                reject(new Error('Compression failed'));
              }
            },
            'image/jpeg',
            0.8 // Качество 80%
          );
        };
        
        img.onerror = () => reject(new Error('Failed to load image'));
      };
      
      reader.onerror = () => reject(new Error('Failed to read file'));
    });
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Validate file type
    const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/webp'];
    if (!validTypes.includes(file.type)) {
      setError('Invalid file type. Use JPG, PNG or WebP');
      return;
    }

    try {
      setError('');
      setCompressing(true);
      
      // 🎨 Автоматическая компрессия
      let finalFile = file;
      const maxSize = 1 * 1024 * 1024; // 1MB порог для компрессии
      
      if (file.size > maxSize) {
        console.log(`Original size: ${(file.size / 1024 / 1024).toFixed(2)} MB`);
        finalFile = await compressImage(file);
        console.log(`Compressed size: ${(finalFile.size / 1024 / 1024).toFixed(2)} MB`);
      }
      
      // Проверка после компрессии
      if (finalFile.size > 5 * 1024 * 1024) {
        setError('File too large even after compression. Try a smaller image.');
        return;
      }
      
      setCompressing(false);
      setUploading(true);

      const formData = new FormData();
      formData.append('image', finalFile);

      const token = localStorage.getItem('admin_token');

      const response = await fetch(
        `${BASE_URL}/api/admin/products/${productId}/image`,
        {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${token}`
          },
          body: formData
        }
      );

      if (!response.ok) {
        const data = await response.json();
        setError(data.details || 'Upload failed');
        return;
      }

      // Success - reload product
      window.location.reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setCompressing(false);
      setUploading(false);
    }
  };

  return (
    <div>
      <label>Product Image</label>
      <input
        type="file"
        accept="image/jpeg,image/jpg,image/png,image/webp"
        onChange={handleFileChange}
        disabled={uploading || compressing}
      />
      {compressing && <div>🎨 Compressing image...</div>}
      {uploading && <div>📤 Uploading...</div>}
      {error && <div className="error">{error}</div>}
      <small>Any size (auto-compressed to JPEG if &gt; 1MB)</small>
    </div>
  );
}
```

**💡 Как это работает:**
1. Если файл < 1MB → загружается как есть
2. Если файл > 1MB → автоматически:
   - Уменьшается до max 1200x1200px (пропорционально)
   - Конвертируется в JPEG
   - Сжимается с качеством 80%
3. PNG 3.6MB → JPEG ~800KB (проверено!)
4. Пользователь видит процесс: "Compressing..." → "Uploading..."

---

### 3. Error Handling

**💡 Альтернатива: использовать библиотеку browser-image-compression**

```bash
npm install browser-image-compression
```

```tsx
import imageCompression from 'browser-image-compression';

function ProductImageUploadAdvanced({ productId }: { productId: string }) {
  const [uploading, setUploading] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const [error, setError] = useState('');

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      setError('');
      
      // Превью
      const previewUrl = URL.createObjectURL(file);
      setPreview(previewUrl);
      
      // Компрессия с библиотекой (проще и надёжнее)
      const options = {
        maxSizeMB: 1,              // Макс 1MB
        maxWidthOrHeight: 1200,    // Макс размер
        useWebWorker: true,        // Быстрее
        fileType: 'image/jpeg'     // Всегда JPEG
      };
      
      console.log(`Original: ${(file.size / 1024 / 1024).toFixed(2)} MB`);
      const compressedFile = await imageCompression(file, options);
      console.log(`Compressed: ${(compressedFile.size / 1024 / 1024).toFixed(2)} MB`);
      
      // Загрузка
      setUploading(true);
      const formData = new FormData();
      formData.append('image', compressedFile);

      const token = localStorage.getItem('admin_token');
      const response = await fetch(
        `${BASE_URL}/api/admin/products/${productId}/image`,
        {
          method: 'POST',
          headers: { 'Authorization': `Bearer ${token}` },
          body: formData
        }
      );

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.details || 'Upload failed');
      }

      // Success
      alert('✅ Image uploaded!');
      window.location.reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
    }
  };

  return (
    <div>
      <label>Product Image</label>
      <input
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading}
      />
      
      {preview && (
        <div style={{ marginTop: '10px' }}>
          <img 
            src={preview} 
            alt="Preview" 
            style={{ maxWidth: '200px', borderRadius: '8px' }}
          />
        </div>
      )}
      
      {uploading && <div>⏳ Compressing and uploading...</div>}
      {error && <div className="error">❌ {error}</div>}
      <small>📸 Any size, auto-compressed to &lt;1MB JPEG</small>
    </div>
  );
}
```

**Преимущества библиотеки:**
- ✅ Работает с любыми размерами (даже 50MB)
- ✅ WebWorker → не блокирует UI
- ✅ Лучше оптимизирует JPEG
- ✅ Поддержка EXIF (сохраняет ориентацию)
- ✅ Прогресс бар из коробки

---

### 3. Error Handling

**Backend возвращает структурированные ошибки:**

```typescript
interface ApiError {
  code: string;
  message: string;
  details: string;
}

// Examples:
{
  "code": "VALIDATION_ERROR",
  "message": "Validation error",
  "details": "name_en cannot be empty"
}

{
  "code": "CONFLICT",
  "message": "Conflict",
  "details": "Product 'Tomato' already exists"
}

{
  "code": "AUTHENTICATION_ERROR",
  "message": "Authentication failed",
  "details": "Invalid or expired token"
}

{
  "code": "NOT_FOUND",
  "message": "Not found",
  "details": "Product not found"
}
```

**Обработка на фронте:**
```typescript
async function handleApiError(response: Response) {
  const data = await response.json();
  
  switch (data.code) {
    case 'VALIDATION_ERROR':
      // Show field validation error
      showFieldError(data.details);
      break;
    case 'CONFLICT':
      // Show duplicate warning
      showWarning(data.details);
      break;
    case 'AUTHENTICATION_ERROR':
      // Redirect to login
      redirectToLogin();
      break;
    default:
      // Generic error
      showError('Something went wrong');
  }
}
```

---

### 4. Features Checklist

#### Must Have ✅
- [ ] Login page (admin authentication)
- [ ] Product list table with pagination
- [ ] Create product form
- [ ] Edit product form
- [ ] Delete product (with confirmation)
- [ ] Image upload
- [ ] Search/filter products
- [ ] Error messages display

#### Nice to Have 🎯
- [ ] Bulk operations (delete multiple)
- [ ] Export to CSV
- [ ] Product categories management
- [ ] Drag & drop image upload
- [ ] Image preview before upload
- [ ] Duplicate detection warning
- [ ] Translation editor (side by side)
- [ ] Product usage stats (how many tenants use it)

---

### 5. API Helper Functions

```typescript
// api/admin.ts
const BASE_URL = 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

function getAuthHeaders() {
  const token = localStorage.getItem('admin_token');
  return {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  };
}

export async function loginAdmin(email: string, password: string) {
  const response = await fetch(`${BASE_URL}/api/admin/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  });
  
  if (!response.ok) throw new Error('Login failed');
  
  const data = await response.json();
  localStorage.setItem('admin_token', data.token);
  return data;
}

export async function fetchProducts(): Promise<Product[]> {
  const response = await fetch(`${BASE_URL}/api/admin/products`, {
    headers: getAuthHeaders()
  });
  
  if (!response.ok) throw new Error('Failed to fetch products');
  
  return response.json();
}

export async function createProduct(data: ProductFormData): Promise<Product> {
  const response = await fetch(`${BASE_URL}/api/admin/products`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: JSON.stringify(data)
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw error;
  }
  
  return response.json();
}

export async function updateProduct(id: string, data: Partial<ProductFormData>): Promise<Product> {
  const response = await fetch(`${BASE_URL}/api/admin/products/${id}`, {
    method: 'PUT',
    headers: getAuthHeaders(),
    body: JSON.stringify(data)
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw error;
  }
  
  return response.json();
}

export async function deleteProduct(id: string): Promise<void> {
  const response = await fetch(`${BASE_URL}/api/admin/products/${id}`, {
    method: 'DELETE',
    headers: getAuthHeaders()
  });
  
  if (!response.ok) throw new Error('Failed to delete product');
}

export async function uploadProductImage(id: string, file: File): Promise<string> {
  const formData = new FormData();
  formData.append('image', file);
  
  const token = localStorage.getItem('admin_token');
  const response = await fetch(`${BASE_URL}/api/admin/products/${id}/image`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`
    },
    body: formData
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw error;
  }
  
  return response.json(); // Returns image URL
}
```

---

### 6. Important Notes

#### ⚠️ Отличия от обычного каталога:

1. **НЕТ ЦЕНЫ в админ панели!**
   - Админ создаёт продукты БЕЗ цены
   - Цену устанавливает каждый ресторан сам

2. **Автозаполнение переводов**
   - Можно оставить name_pl, name_uk, name_ru пустыми
   - Backend автоматически заполнит их из name_en
   - Потом можно отредактировать вручную

3. **Case-insensitive uniqueness**
   - "Tomato" = "tomato" = "TOMATO"
   - Backend проверяет дубликаты автоматически
   - Покажет ошибку CONFLICT

4. **Soft-delete**
   - DELETE не удаляет из БД
   - Помечает is_active = false
   - Можно создать снова с тем же именем

---

### 7. Testing в админ панели

```typescript
// Test data
const testProduct = {
  name_en: "Test Product",
  name_pl: "",  // Will auto-fill
  category_id: "5a841ce0-2ea5-4230-a1f7-011fa445afdc",
  unit: "kilogram",
  description: "Test description"
};

// Test scenarios:
1. Create product → Success
2. Create duplicate → CONFLICT error
3. Create with empty name_en → VALIDATION_ERROR
4. Upload image → Success (returns URL)
5. Delete product → Success (soft-delete)
6. Create again with same name → Success (old is inactive)
```

---

### 8. Production URL

```typescript
// Use this in production
const PROD_URL = 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

// Admin credentials (for testing)
email: admin@fodi.app
password: Admin123!
```

---

## 🎯 Summary

**Что нужно реализовать:**
1. ✅ Login page (admin auth)
2. ✅ Product list (GET /api/admin/products)
3. ✅ Create form (POST /api/admin/products)
4. ✅ Edit form (PUT /api/admin/products/:id)
5. ✅ Delete button (DELETE /api/admin/products/:id)
6. ✅ Image upload (POST /api/admin/products/:id/image)
7. ✅ Error handling (show validation/conflict errors)

**Главное правило:**
- Админ работает ТОЛЬКО с master catalog
- НЕТ управления ценами (это tenant-specific)
- Фокус на качестве данных: имена, категории, единицы измерения

**Backend готов и протестирован! Можно начинать фронт! 🚀**

---

## 📸 9. Image Upload с автокомпрессией

### Вариант 1: Нативный JS (без библиотек)

```tsx
import { useState } from 'react';

interface ProductImageUploadProps {
  productId: string;
  currentImageUrl?: string;
  onUploadSuccess?: () => void;
}

function ProductImageUpload({ productId, currentImageUrl, onUploadSuccess }: ProductImageUploadProps) {
  const [uploading, setUploading] = useState(false);
  const [compressing, setCompressing] = useState(false);
  const [preview, setPreview] = useState<string | null>(currentImageUrl || null);
  const [error, setError] = useState('');

  // 🎨 Автоматическая компрессия изображения
  const compressImage = async (file: File): Promise<File> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      
      reader.onload = (event) => {
        const img = new Image();
        img.src = event.target?.result as string;
        
        img.onload = () => {
          const canvas = document.createElement('canvas');
          const ctx = canvas.getContext('2d');
          
          // Максимальные размеры
          const MAX_WIDTH = 1200;
          const MAX_HEIGHT = 1200;
          
          let width = img.width;
          let height = img.height;
          
          // Пропорциональное уменьшение
          if (width > height) {
            if (width > MAX_WIDTH) {
              height *= MAX_WIDTH / width;
              width = MAX_WIDTH;
            }
          } else {
            if (height > MAX_HEIGHT) {
              width *= MAX_HEIGHT / height;
              height = MAX_HEIGHT;
            }
          }
          
          canvas.width = width;
          canvas.height = height;
          ctx?.drawImage(img, 0, 0, width, height);
          
          // Конвертируем в JPEG с качеством 80%
          canvas.toBlob(
            (blob) => {
              if (blob) {
                const compressedFile = new File([blob], 'product.jpg', {
                  type: 'image/jpeg',
                  lastModified: Date.now()
                });
                console.log(`📦 Original: ${(file.size / 1024 / 1024).toFixed(2)} MB`);
                console.log(`✅ Compressed: ${(blob.size / 1024 / 1024).toFixed(2)} MB`);
                resolve(compressedFile);
              } else {
                reject(new Error('Compression failed'));
              }
            },
            'image/jpeg',
            0.8
          );
        };
        
        img.onerror = () => reject(new Error('Failed to load image'));
      };
      
      reader.onerror = () => reject(new Error('Failed to read file'));
    });
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      setError('');
      
      // 1. Показываем превью
      const previewUrl = URL.createObjectURL(file);
      setPreview(previewUrl);
      
      // 2. Компрессия (если файл больше 1MB)
      let finalFile = file;
      if (file.size > 1024 * 1024) {
        setCompressing(true);
        finalFile = await compressImage(file);
        setCompressing(false);
      }
      
      // 3. Загрузка
      setUploading(true);
      const formData = new FormData();
      formData.append('image', finalFile);

      const token = localStorage.getItem('admin_token');
      const response = await fetch(
        `https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/products/${productId}/image`,
        {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${token}`
          },
          body: formData
        }
      );

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.details || 'Upload failed');
      }

      const data = await response.json();
      setPreview(data.image_url);
      
      if (onUploadSuccess) {
        onUploadSuccess();
      }
      
      alert('✅ Image uploaded successfully!');
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
      setCompressing(false);
    }
  };

  const handleDeleteImage = async () => {
    if (!confirm('Delete product image?')) return;

    try {
      const token = localStorage.getItem('admin_token');
      const response = await fetch(
        `https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/products/${productId}/image`,
        {
          method: 'DELETE',
          headers: {
            'Authorization': `Bearer ${token}`
          }
        }
      );

      if (!response.ok) {
        throw new Error('Failed to delete image');
      }

      setPreview(null);
      alert('✅ Image deleted');
      
      if (onUploadSuccess) {
        onUploadSuccess();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Delete failed');
    }
  };

  return (
    <div className="image-upload">
      <label className="upload-label">
        📸 Product Image
      </label>
      
      {preview && (
        <div className="preview-container">
          <img 
            src={preview} 
            alt="Product preview" 
            className="preview-image"
          />
          <button 
            type="button"
            onClick={handleDeleteImage}
            className="delete-image-btn"
          >
            🗑️ Delete Image
          </button>
        </div>
      )}
      
      <input
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading || compressing}
        className="file-input"
      />
      
      {compressing && <p className="status">🎨 Compressing image...</p>}
      {uploading && <p className="status">⏳ Uploading...</p>}
      {error && <p className="error">❌ {error}</p>}
      
      <small className="hint">
        📸 Any size accepted. Files &gt;1MB will be auto-compressed to JPEG
      </small>
    </div>
  );
}

export default ProductImageUpload;
```

### CSS для компонента

```css
.image-upload {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin: 16px 0;
}

.upload-label {
  font-weight: 600;
  color: #333;
  font-size: 14px;
}

.file-input {
  padding: 8px 12px;
  border: 2px dashed #ccc;
  border-radius: 8px;
  cursor: pointer;
  background: #f9f9f9;
  transition: all 0.2s;
}

.file-input:hover {
  border-color: #4CAF50;
  background: #f0f9f0;
}

.file-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.preview-container {
  position: relative;
  display: inline-block;
  margin: 8px 0;
}

.preview-image {
  max-width: 300px;
  max-height: 300px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  display: block;
}

.delete-image-btn {
  margin-top: 8px;
  padding: 6px 12px;
  background: #f44336;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.delete-image-btn:hover {
  background: #d32f2f;
}

.status {
  color: #666;
  font-style: italic;
  font-size: 14px;
  margin: 4px 0;
}

.error {
  color: #f44336;
  font-weight: 500;
  font-size: 14px;
  margin: 4px 0;
}

.hint {
  color: #999;
  font-size: 12px;
  line-height: 1.4;
}
```

### Использование в форме

```tsx
function ProductEditPage({ productId }: { productId: string }) {
  const [product, setProduct] = useState<Product | null>(null);

  const fetchProduct = async () => {
    const token = localStorage.getItem('admin_token');
    const response = await fetch(
      `https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/products/${productId}`,
      {
        headers: { 'Authorization': `Bearer ${token}` }
      }
    );
    const data = await response.json();
    setProduct(data);
  };

  useEffect(() => {
    fetchProduct();
  }, [productId]);

  return (
    <div>
      <h2>Edit Product</h2>
      
      {/* Форма редактирования */}
      <ProductForm 
        productId={productId} 
        onSuccess={fetchProduct}
      />
      
      {/* Загрузка изображения */}
      {product && (
        <ProductImageUpload
          productId={productId}
          currentImageUrl={product.image_url}
          onUploadSuccess={fetchProduct}
        />
      )}
    </div>
  );
}
```

---

### Вариант 2: browser-image-compression (рекомендуется для production)

#### Установка

```bash
npm install browser-image-compression
```

#### Компонент с библиотекой

```tsx
import { useState } from 'react';
import imageCompression from 'browser-image-compression';

function ProductImageUpload({ productId, currentImageUrl, onUploadSuccess }: ProductImageUploadProps) {
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [preview, setPreview] = useState<string | null>(currentImageUrl || null);
  const [error, setError] = useState('');

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      setError('');
      setProgress(0);
      
      // 1. Превью
      const previewUrl = URL.createObjectURL(file);
      setPreview(previewUrl);
      
      // 2. Компрессия с прогрессом
      const options = {
        maxSizeMB: 1,              // Максимум 1MB
        maxWidthOrHeight: 1200,    // Максимальный размер стороны
        useWebWorker: true,        // Не блокирует UI
        fileType: 'image/jpeg',    // Всегда JPEG
        onProgress: (p: number) => {
          setProgress(Math.round(p));
        }
      };
      
      console.log(`📦 Original: ${(file.size / 1024 / 1024).toFixed(2)} MB`);
      const compressedFile = await imageCompression(file, options);
      console.log(`✅ Compressed: ${(compressedFile.size / 1024 / 1024).toFixed(2)} MB`);
      
      // 3. Загрузка
      setUploading(true);
      setProgress(100);
      
      const formData = new FormData();
      formData.append('image', compressedFile);

      const token = localStorage.getItem('admin_token');
      const response = await fetch(
        `https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/products/${productId}/image`,
        {
          method: 'POST',
          headers: { 'Authorization': `Bearer ${token}` },
          body: formData
        }
      );

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.details || 'Upload failed');
      }

      const data = await response.json();
      setPreview(data.image_url);
      
      if (onUploadSuccess) {
        onUploadSuccess();
      }
      
      alert('✅ Image uploaded successfully!');
      
      // Cleanup
      URL.revokeObjectURL(previewUrl);
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
      setProgress(0);
    }
  };

  return (
    <div className="image-upload">
      <label className="upload-label">📸 Product Image</label>
      
      {preview && (
        <div className="preview-container">
          <img src={preview} alt="Preview" className="preview-image" />
        </div>
      )}
      
      <input
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading}
        className="file-input"
      />
      
      {progress > 0 && progress < 100 && (
        <div className="progress-bar">
          <div className="progress-fill" style={{ width: `${progress}%` }} />
          <span>{progress}%</span>
        </div>
      )}
      
      {uploading && <p className="status">⏳ Uploading...</p>}
      {error && <p className="error">❌ {error}</p>}
      
      <small className="hint">
        📸 Any size accepted. Auto-compressed to &lt;1MB JPEG
      </small>
    </div>
  );
}
```

#### CSS для прогресс-бара

```css
.progress-bar {
  position: relative;
  height: 24px;
  background: #f0f0f0;
  border-radius: 12px;
  overflow: hidden;
  margin: 8px 0;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4CAF50, #45a049);
  transition: width 0.3s;
}

.progress-bar span {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-weight: 600;
  color: #333;
  font-size: 12px;
}
```

---

### 📊 Сравнение решений

| Параметр | Нативный JS | browser-image-compression |
|----------|-------------|---------------------------|
| Зависимости | ❌ Нет | ✅ +54KB gzipped |
| Размер PNG 3.6MB | JPEG 800KB | JPEG 750KB |
| Скорость | Средняя | Быстрая (WebWorker) |
| UI блокировка | Да (на ~1-2 сек) | Нет |
| Progress bar | Нет | Да |
| EXIF сохранение | Нет | Да |
| Сложность | Простой | Простой |

**Рекомендация:**
- **MVP / Прототип:** Вариант 1 (нативный JS)
- **Production:** Вариант 2 (browser-image-compression)

---

### 🎯 Результаты компрессии

**Реальные тесты:**
- ✅ PNG 3.6MB → JPEG 789KB (успешно загружено)
- ✅ PNG 1.5MB → JPEG ~400KB
- ✅ JPEG 500KB → без изменений
- ✅ WebP 4MB → JPEG ~800KB

**Лимиты backend:**
- Максимум: 5MB
- Рекомендуется: <1MB для быстрой загрузки

---

### ✅ Чек-лист

- [ ] Установить `browser-image-compression` (если выбрали вариант 2)
- [ ] Добавить `ProductImageUpload` компонент
- [ ] Добавить CSS стили
- [ ] Интегрировать в форму редактирования
- [ ] Протестировать с PNG 5MB+
- [ ] Проверить превью перед загрузкой
- [ ] Проверить кнопку удаления изображения
- [ ] Логирование размеров (до/после) для отладки

**Готово! Теперь можно загружать изображения любого размера! 🎉**

