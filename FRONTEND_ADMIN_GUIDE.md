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

// Get Categories (для dropdown в форме)
GET /api/catalog/categories
Headers: Authorization: Bearer <admin_token>
Response: {
  "categories": [
    {
      "id": "uuid",
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
// hooks/useCategories.ts
import { useState, useEffect } from 'react';

interface Category {
  id: string;
  name: string;
  sort_order: number;
}

export const useCategories = () => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const response = await fetch('https://your-api.com/api/catalog/categories', {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('token')}`,
            'Accept-Language': 'ru'
          }
        });

        if (!response.ok) {
          throw new Error('Failed to fetch categories');
        }

        const data = await response.json();
        // Сортируем по sort_order
        setCategories(data.categories.sort((a: Category, b: Category) => 
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

function ProductList() {
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchProducts();
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

  if (loading) return <div>Loading...</div>;

  return (
    <table>
      <thead>
        <tr>
          <th>Image</th>
          <th>Name (EN)</th>
          <th>Unit</th>
          <th>Category</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {products.map(product => (
          <tr key={product.id}>
            <td>
              {product.image_url ? (
                <img src={product.image_url} width="50" />
              ) : (
                <div>No image</div>
              )}
            </td>
            <td>{product.name_en}</td>
            <td>{product.unit}</td>
            <td>{product.category_id}</td>
            <td>
              <button onClick={() => handleEdit(product)}>Edit</button>
              <button onClick={() => handleDelete(product.id)}>Delete</button>
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
          <option value="">Select category...</option>
          {categories.map(cat => (
            <option key={cat.id} value={cat.id}>
              {cat.name}
            </option>
          ))}
        </select>
        {categoriesLoading && <small>Loading categories...</small>}
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
        />
      </div>

      <div>
        <label>Category *</label>
        <select
          value={formData.category_id}
          onChange={e => setFormData({...formData, category_id: e.target.value})}
          required
        >
          <option value="">Select category</option>
          {/* Fetch categories from API */}
        </select>
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

#### 2.3 Image Upload Component
```tsx
function ProductImageUpload({ productId }: { productId: string }) {
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState('');

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Validate file
    const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/webp'];
    if (!validTypes.includes(file.type)) {
      setError('Invalid file type. Use JPG, PNG or WebP');
      return;
    }

    const maxSize = 5 * 1024 * 1024; // 5MB
    if (file.size > maxSize) {
      setError('File too large. Max 5MB');
      return;
    }

    // Upload
    setUploading(true);
    setError('');

    const formData = new FormData();
    formData.append('image', file);

    const token = localStorage.getItem('admin_token');

    try {
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
      setError('Network error');
    } finally {
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
        disabled={uploading}
      />
      {uploading && <div>Uploading...</div>}
      {error && <div className="error">{error}</div>}
      <small>Max 5MB, JPG/PNG/WebP</small>
    </div>
  );
}
```

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
