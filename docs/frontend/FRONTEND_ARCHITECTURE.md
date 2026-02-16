# 🏗️ Frontend Architecture для Auto-Translate

## Структура проекта (Recommended)

```
frontend/
├── src/
│   ├── components/
│   │   ├── ProductForm.tsx                    # Create & Edit form
│   │   ├── ProductForm.css
│   │   ├── EditProductForm.tsx                # Edit-specific form
│   │   ├── TranslationStats.tsx               # Stats dashboard
│   │   ├── TranslationStats.css
│   │   ├── ProductList.tsx                    # List view
│   │   ├── ProductImageUpload.tsx             # Image upload
│   │   └── ProductCard.tsx                    # Product card component
│   │
│   ├── services/
│   │   ├── translationService.ts              # 🔑 Core translation service
│   │   ├── inventoryService.ts               # NEW: Inventory & Batches
│   │   ├── recipeService.ts                  # NEW: Recipe V2 Management
│   │   ├── menuEngineeringService.ts           # NEW: Sales & Optimization
│   │   ├── productService.ts                  # Product CRUD
│   │   ├── categoryService.ts                 # Categories API
│   │   └── api.ts                             # Base HTTP client
│   │
│   ├── hooks/
│   │   ├── useDictionaryCache.ts              # Translation caching
│   │   ├── useCategories.ts                   # Load categories
│   │   ├── useProducts.ts                     # Load products
│   │   └── useTranslationStats.ts             # Track translation stats
│   │
│   ├── types/
│   │   ├── index.ts                           # Type definitions
│   │   ├── errors.ts                          # Error types
│   │   └── api.ts                             # API response types
│   │
│   ├── pages/
│   │   ├── AdminProductsPage.tsx              # Main admin page
│   │   ├── AdminProductsPage.css
│   │   ├── AdminDashboard.tsx                 # With stats
│   │   └── ...
│   │
│   ├── utils/
│   │   ├── formatters.ts                      # Format prices, dates, etc
│   │   ├── validators.ts                      # Validate forms
│   │   └── constants.ts                       # Constants (units, languages)
│   │
│   ├── App.tsx
│   ├── App.css
│   └── index.tsx
│
├── public/
├── .env.example
├── package.json
├── tsconfig.json
└── README.md
```

---

## 📝 Type Definitions

### types/index.ts

```typescript
// Product types
export interface Product {
  id: string;
  name_en: string;
  name_pl: string;
  name_ru: string;
  name_uk: string;
  category_id: string;
  unit: ProductUnit;
  description?: string;
  image_url?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProductRequest {
  name_en: string;
  name_pl?: string;
  name_ru?: string;
  name_uk?: string;
  category_id: string;
  unit: ProductUnit;
  description?: string;
  auto_translate?: boolean; // 🔑 NEW
}

export interface UpdateProductRequest {
  name_en?: string;
  name_pl?: string;
  name_ru?: string;
  name_uk?: string;
  category_id?: string;
  unit?: ProductUnit;
  description?: string;
  auto_translate?: boolean; // 🔑 NEW
}

/* --- NEW SECTIONS FOR RECIPE V2 & INVENTORY --- */

export interface InventoryProduct {
  id: string;
  name: string;
  quantity: number;
  unit: string;
  average_price: number;
  expiry_date?: string;
  tenant_id: string;
  batches: InventoryBatch[];
}

export interface InventoryBatch {
  id: string;
  quantity: number;
  received_at: string;
}

export interface RecipeV2 {
  id: string;
  name_en: string;
  name_ru: string;
  description_en?: string;
  description_ru?: string;
  instructions_en?: string;
  instructions_ru?: string;
  category_id: string;
  servings: number;
  status: 'Draft' | 'Published';
  cost_per_serving: number;
  ingredients: RecipeIngredient[];
}

export interface RecipeIngredient {
  ingredient_id: string; // References InventoryProduct
  quantity: number;
  unit: string;
  cost: number; // Calculated by backend
}

export interface SaleRecordRequest {
  dish_id: string;
  quantity: number;
  sold_at: string; // ISO String
}

// ...existing code...
```

### types/errors.ts

```typescript
export class AppError extends Error {
  constructor(
    public code: string,
    public message: string,
    public statusCode?: number,
    public details?: any
  ) {
    super(message);
    this.name = 'AppError';
  }

  static validation(message: string) {
    return new AppError('VALIDATION_ERROR', message, 400, message);
  }

  static conflict(message: string) {
    return new AppError('CONFLICT', message, 409, message);
  }

  static notFound(message: string) {
    return new AppError('NOT_FOUND', message, 404, message);
  }

  static internal(message: string) {
    return new AppError('INTERNAL_ERROR', message, 500, message);
  }

  static network(message: string) {
    return new AppError('NETWORK_ERROR', message, undefined, message);
  }

  static translation(message: string) {
    return new AppError('TRANSLATION_ERROR', message, 500, message);
  }
}
```

### types/api.ts

```typescript
// API response wrappers
export interface ApiResponse<T> {
  data?: T;
  error?: string;
  code?: string;
  details?: any;
  status: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}
```

---

## 🔑 Core Services

### services/api.ts (Base HTTP Client)

```typescript
import { AppError } from '../types/errors';

interface FetchOptions extends RequestInit {
  headers?: Record<string, string>;
  params?: Record<string, string | number>;
}

export class ApiClient {
  constructor(
    private baseUrl: string,
    private getToken: () => string | null = () => localStorage.getItem('admin_token')
  ) {}

  private async request<T>(
    endpoint: string,
    options: FetchOptions = {}
  ): Promise<T> {
    const url = new URL(endpoint, this.baseUrl);

    // Add query params
    if (options.params) {
      Object.entries(options.params).forEach(([key, value]) => {
        url.searchParams.append(key, String(value));
      });
    }

    // Add auth header
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers
    };

    const token = this.getToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    try {
      const response = await fetch(url.toString(), {
        ...options,
        headers
      });

      const data = await response.json();

      if (!response.ok) {
        throw this.handleError(data, response.status);
      }

      return data as T;
    } catch (error) {
      if (error instanceof AppError) throw error;
      throw AppError.network(
        error instanceof Error ? error.message : 'Network request failed'
      );
    }
  }

  private handleError(data: any, status: number): AppError {
    if (data.code === 'CONFLICT') {
      return AppError.conflict(data.details || 'Resource already exists');
    }
    if (data.code === 'VALIDATION_ERROR') {
      return AppError.validation(data.details || 'Validation failed');
    }
    if (data.code === 'NOT_FOUND') {
      return AppError.notFound(data.details || 'Resource not found');
    }
    if (status >= 500) {
      return AppError.internal(data.details || 'Server error');
    }
    return AppError.internal(data.message || 'Unknown error');
  }

  get<T>(endpoint: string, options?: FetchOptions) {
    return this.request<T>(endpoint, { ...options, method: 'GET' });
  }

  post<T>(endpoint: string, body?: any, options?: FetchOptions) {
    return this.request<T>(endpoint, {
      ...options,
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined
    });
  }

  put<T>(endpoint: string, body?: any, options?: FetchOptions) {
    return this.request<T>(endpoint, {
      ...options,
      method: 'PUT',
      body: body ? JSON.stringify(body) : undefined
    });
  }

  delete<T>(endpoint: string, options?: FetchOptions) {
    return this.request<T>(endpoint, { ...options, method: 'DELETE' });
  }
}

// Create singleton instance
const apiClient = new ApiClient(
  process.env.REACT_APP_API_URL || 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app'
);

export default apiClient;
```

### services/translationService.ts (Enhanced)

```typescript
import apiClient from './api';
import { TranslationResult, TranslationSource } from '../types';
import { AppError } from '../types/errors';

class TranslationService {
  /**
   * 🌍 Получить переводы для английского названия
   * 
   * Использует hybrid approach:
   * 1. Создаёт продукт с auto_translate=true
   * 2. Возвращает переводы с информацией об источнике
   */
  async getTranslations(name_en: string): Promise<TranslationResult> {
    if (!name_en.trim()) {
      throw AppError.validation('English name is required');
    }

    try {
      // Отправляем создание продукта с auto_translate
      // Это срабатывает гибридную логику на бэкенде:
      // Dictionary → Groq AI → English fallback
      const response = await apiClient.post<any>(
        '/api/admin/products',
        {
          name_en,
          name_pl: '',
          name_ru: '',
          name_uk: '',
          category_id: 'temp', // Временное значение
          unit: 'kilogram',
          auto_translate: true
        }
      );

      return {
        pl: response.name_pl,
        ru: response.name_ru,
        uk: response.name_uk,
        source: this.detectSource(name_en, response),
        cost: this.detectSource(name_en, response) === 'dictionary' ? 0 : 0.01
      };
    } catch (error) {
      throw AppError.translation(
        error instanceof Error ? error.message : 'Translation failed'
      );
    }
  }

  /**
   * ❌ TODO: Implement dedicated translation endpoint on backend
   * 
   * POST /api/admin/translate
   * Body: { "name_en": "Apple" }
   * Response: { "pl": "Jabłko", "ru": "Яблоко", "uk": "Яблуко", "source": "groq", "cost": 0.01 }
   * 
   * Benefits:
   * - No need to create temporary products
   * - Cleaner API contract
   * - Better performance tracking
   * - Easier to add auth/permissions
   */
  async getTranslationsV2(name_en: string): Promise<TranslationResult> {
    if (!name_en.trim()) {
      throw AppError.validation('English name is required');
    }

    try {
      return await apiClient.post<TranslationResult>(
        '/api/admin/translate',
        { name_en }
      );
    } catch (error) {
      throw AppError.translation(
        error instanceof Error ? error.message : 'Translation failed'
      );
    }
  }

  private detectSource(
    englishName: string,
    product: any
  ): TranslationSource {
    const allSame =
      product.name_pl === englishName &&
      product.name_ru === englishName &&
      product.name_uk === englishName;

    return allSame ? 'fallback' : 'groq';
  }
}

export default new TranslationService();
```

### services/productService.ts

```typescript
import apiClient from './api';
import { 
  Product, 
  CreateProductRequest, 
  UpdateProductRequest 
} from '../types';

class ProductService {
  async getProducts(): Promise<Product[]> {
    return apiClient.get<Product[]>('/api/admin/products');
  }

  async getProduct(id: string): Promise<Product> {
    return apiClient.get<Product>(`/api/admin/products/${id}`);
  }

  async createProduct(
    data: CreateProductRequest
  ): Promise<Product> {
    return apiClient.post<Product>('/api/admin/products', data);
  }

  async updateProduct(
    id: string,
    data: UpdateProductRequest
  ): Promise<Product> {
    return apiClient.put<Product>(`/api/admin/products/${id}`, data);
  }

  async deleteProduct(id: string): Promise<void> {
    await apiClient.delete(`/api/admin/products/${id}`);
  }

  async uploadImage(
    id: string,
    file: File
  ): Promise<{ image_url: string }> {
    const formData = new FormData();
    formData.append('image', file);

    const token = localStorage.getItem('admin_token');

    const response = await fetch(
      `${process.env.REACT_APP_API_URL || 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app'}/api/admin/products/${id}/image`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        },
        body: formData
      }
    );

    if (!response.ok) {
      throw new Error('Image upload failed');
    }

    return response.json();
  }

  async deleteImage(id: string): Promise<void> {
    await apiClient.delete(`/api/admin/products/${id}/image`);
  }
}

export default new ProductService();
```

### services/categoryService.ts

```typescript
import apiClient from './api';
import { Category } from '../types';

class CategoryService {
  async getCategories(): Promise<Category[]> {
    const response = await apiClient.get<{ categories: Category[] }>(
      '/api/admin/categories'
    );
    return response.categories;
  }

  async getCategory(id: string): Promise<Category> {
    return apiClient.get<Category>(`/api/admin/categories/${id}`);
  }
}

export default new CategoryService();
```

### services/menuEngineeringService.ts

```typescript
import api from './api';
import { SaleRecordRequest, RecipeV2 } from '../types';

export const menuEngineeringService = {
  // 💰 Record a sale (Backend will automatically handle FIFO deduction)
  recordSale: async (sale: SaleRecordRequest) => {
    const response = await api.post('/api/menu/sales', sale);
    return response.data;
  },

  // 📈 Get BCG Matrix and Optimization Insights
  getInsights: async () => {
    const response = await api.get('/api/menu/insights');
    return response.data; // Returns Stars, Cash Cows, Dogs, etc.
  }
};
```

### services/inventoryService.ts

```typescript
import api from './api';
import { InventoryProduct } from '../types';

export const inventoryService = {
  getAll: async () => {
    const response = await api.get<InventoryProduct[]>('/api/inventory');
    return response.data;
  },

  // Backend automatically handles batch splitting and multi-tenancy
  addInventory: async (data: Partial<InventoryProduct>) => {
    const response = await api.post('/api/inventory', data);
    return response.data;
  }
};
```

---

## 🪝 Custom Hooks

### hooks/useCategories.ts

```typescript
import { useState, useEffect } from 'react';
import { Category } from '../types';
import categoryService from '../services/categoryService';

export const useCategories = () => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const data = await categoryService.getCategories();
        setCategories(data);
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

### hooks/useProducts.ts

```typescript
import { useState, useEffect, useCallback } from 'react';
import { Product } from '../types';
import productService from '../services/productService';

export const useProducts = () => {
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadProducts = useCallback(async () => {
    setLoading(true);
    try {
      const data = await productService.getProducts();
      setProducts(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load products');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadProducts();
  }, []);

  return { products, loading, error, refetch: loadProducts };
};
```

### hooks/useTranslationStats.ts

```typescript
import { useState, useCallback } from 'react';
import { TranslationStats } from '../types';

const INITIAL_STATS: TranslationStats = {
  totalRequests: 0,
  cacheHits: 0,
  aiCalls: 0,
  totalCostUSD: 0
};

export const useTranslationStats = (storageKey = 'translation_stats') => {
  const [stats, setStats] = useState<TranslationStats>(() => {
    const stored = localStorage.getItem(storageKey);
    return stored ? JSON.parse(stored) : INITIAL_STATS;
  });

  const updateStats = useCallback(
    (source: 'dictionary' | 'groq' | 'fallback', cost: number) => {
      setStats(prev => {
        const updated: TranslationStats = {
          totalRequests: prev.totalRequests + 1,
          cacheHits: source === 'dictionary' ? prev.cacheHits + 1 : prev.cacheHits,
          aiCalls: source === 'groq' ? prev.aiCalls + 1 : prev.aiCalls,
          totalCostUSD: prev.totalCostUSD + cost
        };
        
        // Persist to localStorage
        localStorage.setItem(storageKey, JSON.stringify(updated));
        
        return updated;
      });
    },
    []
  );

  const reset = useCallback(() => {
    setStats(INITIAL_STATS);
    localStorage.removeItem(storageKey);
  }, [storageKey]);

  return { stats, updateStats, reset };
};
```

---

## 🌐 Utilities

### utils/constants.ts

```typescript
export const PRODUCT_UNITS = [
  { value: 'kilogram', label: 'Kilogram (kg)', symbol: 'kg' },
  { value: 'gram', label: 'Gram (g)', symbol: 'g' },
  { value: 'liter', label: 'Liter (L)', symbol: 'L' },
  { value: 'milliliter', label: 'Milliliter (ml)', symbol: 'ml' },
  { value: 'piece', label: 'Piece', symbol: 'pcs' },
  { value: 'bunch', label: 'Bunch', symbol: 'bch' },
  { value: 'can', label: 'Can', symbol: 'can' },
  { value: 'package', label: 'Package', symbol: 'pkg' }
];

export const LANGUAGES = [
  { code: 'en', name: 'English', flag: '🇬🇧' },
  { code: 'pl', name: 'Polish', flag: '🇵🇱' },
  { code: 'ru', name: 'Russian', flag: '🇷🇺' },
  { code: 'uk', name: 'Ukrainian', flag: '🇺🇦' }
];

export const TRANSLATION_SOURCES = {
  dictionary: {
    label: 'Dictionary Cache',
    icon: '💾',
    cost: 0,
    speed: 'instant'
  },
  groq: {
    label: 'AI (Groq)',
    icon: '🤖',
    cost: 0.01,
    speed: '1-2s'
  },
  fallback: {
    label: 'Fallback',
    icon: '⚪',
    cost: 0,
    speed: 'instant'
  }
};
```

### utils/formatters.ts

```typescript
export const formatCost = (cost: number): string => {
  if (cost === 0) return 'FREE';
  return `$${cost.toFixed(2)}`;
};

export const formatDate = (date: string | Date): string => {
  return new Date(date).toLocaleDateString('ru-RU', {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  });
};

export const formatDateTime = (date: string | Date): string => {
  return new Date(date).toLocaleString('ru-RU');
};

export const truncate = (text: string, maxLength: number): string => {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
};
```

### utils/validators.ts

```typescript
import { ProductFormData, FormErrors } from '../types';

export const validateProductForm = (data: ProductFormData): FormErrors => {
  const errors: FormErrors = {};

  if (!data.name_en?.trim()) {
    errors.name_en = 'English name is required';
  }

  if (data.name_en && data.name_en.length > 100) {
    errors.name_en = 'English name must be less than 100 characters';
  }

  if (!data.category_id) {
    errors.category_id = 'Category is required';
  }

  if (!data.unit) {
    errors.unit = 'Unit is required';
  }

  if (data.description && data.description.length > 500) {
    errors.description = 'Description must be less than 500 characters';
  }

  return errors;
};

export const hasErrors = (errors: FormErrors): boolean => {
  return Object.values(errors).some(error => error !== undefined);
};
```

---

## 🚀 Environment Setup

### .env.example

```bash
REACT_APP_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
REACT_APP_ENABLE_ANALYTICS=false
REACT_APP_LOG_LEVEL=info
```

### .env.development

```bash
REACT_APP_API_URL=http://localhost:8080
REACT_APP_ENABLE_ANALYTICS=true
REACT_APP_LOG_LEVEL=debug
```

### .env.production

```bash
REACT_APP_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
REACT_APP_ENABLE_ANALYTICS=true
REACT_APP_LOG_LEVEL=error
```

---

## 📦 Dependencies

```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "typescript": "^5.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@types/node": "^20.0.0"
  }
}
```

---

## 🎯 Integration Summary

| Component | Purpose | Status |
|-----------|---------|--------|
| `ProductForm` | Create products with auto-translate | ✅ Ready |
| `EditProductForm` | Edit with optional re-translation | ✅ Ready |
| `TranslationService` | API integration for translations | ✅ Ready |
| `TranslationStats` | Track costs and cache hits | ✅ Ready |
| `ProductService` | CRUD operations | ✅ Ready |
| `CategoryService` | Category API | ✅ Ready |
| `MenuEngineeringService` | Sales & Optimization | ✅ Ready |
| `InventoryService` | Inventory & Batches | ✅ Ready |
| Custom Hooks | State management | ✅ Ready |
| Type Definitions | TypeScript support | ✅ Ready |

---

## 🔄 Data Flow

```
User Input (name_en)
    ↓
ProductForm.handleAutoTranslate()
    ↓
TranslationService.getTranslations()
    ↓
apiClient.post(/api/admin/products, { auto_translate: true })
    ↓
Backend Hybrid Logic:
  1. Dictionary.find_by_en() → Cache hit? Return
  2. Groq.translate() → AI call? Save + return
  3. Fallback → English for all languages
    ↓
TranslationResult { pl, ru, uk, source, cost }
    ↓
Display Preview
    ↓
User confirms
    ↓
Full product submission
    ↓
Success ✅
```

---

## 📚 Next Steps

1. ✅ Скопировать все файлы из гайдов
2. ✅ Обновить типы для вашего проекта
3. ✅ Протестировать на Koyeb
4. ⏳ (Optional) Добавить dedicated `/api/admin/translate` endpoint на бэкенде
5. ⏳ (Optional) Добавить batch translation для импорта CSV
6. ⏳ (Optional) Добавить translation history tracking
