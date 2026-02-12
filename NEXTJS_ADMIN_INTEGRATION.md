# Next.js Admin Panel Integration Guide

## 🔗 API Base URL
```typescript
const API_BASE_URL = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";
```

## 🔐 Authentication

### Login
```typescript
// POST /api/admin/auth/login
const login = async (email: string, password: string) => {
  const response = await fetch(`${API_BASE_URL}/api/admin/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  });
  
  const data = await response.json();
  // data = { token: string, expires_in: number }
  
  // Save token to localStorage or cookie
  localStorage.setItem('admin_token', data.token);
  
  return data;
};

// Default credentials
// email: admin@fodi.app
// password: Admin123!
```

### Verify Token
```typescript
// POST /api/admin/auth/verify
const verifyToken = async (token: string) => {
  const response = await fetch(`${API_BASE_URL}/api/admin/auth/verify`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    }
  });
  
  return response.ok;
};
```

## 📦 Products API

### TypeScript Types
```typescript
type UnitType = 
  | "gram" 
  | "kilogram" 
  | "liter" 
  | "milliliter" 
  | "piece" 
  | "bunch" 
  | "can" 
  | "bottle" 
  | "package";

interface Product {
  id: string; // UUID
  name_en: string;
  name_pl: string; // empty string if not provided
  name_uk: string;
  name_ru: string;
  category_id: string; // UUID
  price: string; // e.g. "15.99"
  unit: UnitType;
  description: string | null;
  image_url: string | null; // Cloudflare R2 URL
}

interface CreateProductRequest {
  name_en: string;
  name_pl?: string; // optional, defaults to ""
  name_uk?: string;
  name_ru?: string;
  category_id: string;
  price: string; // decimal as string: "10.50"
  unit: UnitType;
  description?: string;
}

interface UpdateProductRequest {
  name_en?: string;
  name_pl?: string;
  name_uk?: string;
  name_ru?: string;
  category_id?: string;
  price?: string;
  unit?: UnitType;
  description?: string;
}
```

### List All Products
```typescript
// GET /api/admin/products
const getProducts = async (token: string): Promise<Product[]> => {
  const response = await fetch(`${API_BASE_URL}/api/admin/products`, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });
  
  return response.json();
};
```

### Get Product by ID
```typescript
// GET /api/admin/products/:id
const getProduct = async (id: string, token: string): Promise<Product> => {
  const response = await fetch(`${API_BASE_URL}/api/admin/products/${id}`, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });
  
  return response.json();
};
```

### Create Product
```typescript
// POST /api/admin/products
const createProduct = async (
  data: CreateProductRequest,
  token: string
): Promise<Product> => {
  const response = await fetch(`${API_BASE_URL}/api/admin/products`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    },
    body: JSON.stringify(data)
  });
  
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message);
  }
  
  return response.json();
};

// Example usage
const newProduct = await createProduct({
  name_en: "Fresh Tomatoes",
  category_id: "5a841ce0-2ea5-4230-a1f7-011fa445afdc", // Vegetables
  price: "5.99",
  unit: "kilogram",
  description: "Organic tomatoes from local farm"
}, token);
```

### Update Product
```typescript
// PUT /api/admin/products/:id
const updateProduct = async (
  id: string,
  data: UpdateProductRequest,
  token: string
): Promise<Product> => {
  const response = await fetch(`${API_BASE_URL}/api/admin/products/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    },
    body: JSON.stringify(data)
  });
  
  return response.json();
};

// Example: update only price
await updateProduct(productId, { price: "6.99" }, token);
```

### Delete Product
```typescript
// DELETE /api/admin/products/:id
const deleteProduct = async (id: string, token: string): Promise<void> => {
  const response = await fetch(`${API_BASE_URL}/api/admin/products/${id}`, {
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });
  
  if (!response.ok) {
    throw new Error('Failed to delete product');
  }
};
```

## 📸 Image Upload

### Upload Product Image
```typescript
// POST /api/admin/products/:id/image
const uploadProductImage = async (
  productId: string,
  file: File,
  token: string
): Promise<{ image_url: string }> => {
  const formData = new FormData();
  formData.append('file', file);
  
  const response = await fetch(
    `${API_BASE_URL}/api/admin/products/${productId}/image`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`
      },
      body: formData
    }
  );
  
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message);
  }
  
  return response.json();
};

// Usage with React
const handleImageUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
  const file = e.target.files?.[0];
  if (!file) return;
  
  // Validate
  if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type)) {
    alert('Only JPG, PNG, WEBP allowed');
    return;
  }
  
  if (file.size > 5 * 1024 * 1024) { // 5MB
    alert('Max file size: 5MB');
    return;
  }
  
  try {
    const result = await uploadProductImage(productId, file, token);
    console.log('Uploaded:', result.image_url);
    // Update UI with new image URL
  } catch (error) {
    console.error('Upload failed:', error);
  }
};
```

### Delete Product Image
```typescript
// DELETE /api/admin/products/:id/image
const deleteProductImage = async (
  productId: string,
  token: string
): Promise<void> => {
  const response = await fetch(
    `${API_BASE_URL}/api/admin/products/${productId}/image`,
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
};
```

## 📂 Categories (Read-only)

### Get All Categories
```typescript
// GET /api/catalog/categories
interface Category {
  id: string;
  name_en: string;
  name_pl: string;
  name_uk: string;
  name_ru: string;
}

const getCategories = async (): Promise<Category[]> => {
  const response = await fetch(`${API_BASE_URL}/api/catalog/categories`);
  return response.json();
};

// Available categories:
// - Dairy & Eggs: b33520f3-e788-40a1-9f27-186cad5d96da
// - Meat & Poultry: eb707494-78f8-427f-9408-9c297a882ae0
// - Fish & Seafood: 503794cf-37e0-48c1-a6d8-b5c3f21e03a1
// - Vegetables: 5a841ce0-2ea5-4230-a1f7-011fa445afdc
// - Fruits: d4a64b25-a187-4ec0-9518-3e8954a138fa
```

## 🛡️ Error Handling

```typescript
interface APIError {
  code: string;
  message: string;
}

const handleAPIError = (error: APIError) => {
  switch (error.code) {
    case 'UNAUTHORIZED':
      // Token expired or invalid
      localStorage.removeItem('admin_token');
      window.location.href = '/admin/login';
      break;
    case 'VALIDATION_ERROR':
      // Show validation error to user
      alert(error.message);
      break;
    case 'NOT_FOUND':
      // Product not found
      alert('Product not found');
      break;
    case 'DATABASE_ERROR':
      // Server error
      console.error('Database error:', error.message);
      break;
    default:
      console.error('API error:', error);
  }
};
```

## 🎨 React Hook Example

```typescript
// hooks/useAdminProducts.ts
import { useState, useEffect } from 'react';

export const useAdminProducts = (token: string) => {
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchProducts = async () => {
      try {
        setLoading(true);
        const data = await getProducts(token);
        setProducts(data);
      } catch (err) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    fetchProducts();
  }, [token]);

  const createProduct = async (data: CreateProductRequest) => {
    const newProduct = await createProduct(data, token);
    setProducts(prev => [...prev, newProduct]);
    return newProduct;
  };

  const updateProduct = async (id: string, data: UpdateProductRequest) => {
    const updated = await updateProduct(id, data, token);
    setProducts(prev => prev.map(p => p.id === id ? updated : p));
    return updated;
  };

  const deleteProduct = async (id: string) => {
    await deleteProduct(id, token);
    setProducts(prev => prev.filter(p => p.id !== id));
  };

  return {
    products,
    loading,
    error,
    createProduct,
    updateProduct,
    deleteProduct
  };
};
```

## 📝 Notes

- **Base64 images are NOT supported** - use multipart/form-data for uploads
- **Image storage**: Cloudflare R2 (S3-compatible)
- **Image URL format**: `https://pub-85f883ab.r2.dev/products/{uuid}.{ext}`
- **Max image size**: 5MB
- **Allowed formats**: JPG, PNG, WEBP
- **JWT Token TTL**: 24 hours (86400 seconds)
- **CORS**: Enabled for all origins (`*`)

## 🔒 Security

- Always store JWT token securely (httpOnly cookie recommended for production)
- Never expose credentials in client-side code
- Use HTTPS only
- Validate file types and sizes before upload
- Handle token expiration gracefully

## 🚀 Production Checklist

- [ ] Store token in httpOnly cookie
- [ ] Implement automatic token refresh
- [ ] Add loading states for all API calls
- [ ] Add proper error boundaries
- [ ] Implement optimistic updates
- [ ] Add image preview before upload
- [ ] Add confirmation dialogs for delete operations
- [ ] Implement pagination for product list
- [ ] Add search and filter functionality
- [ ] Monitor API performance

## 📞 Support

- **Backend API**: https://ministerial-yetta-fodi999-c58d8823.koyeb.app
- **Admin Email**: admin@fodi.app
- **Admin Password**: Admin123! (change in production!)
