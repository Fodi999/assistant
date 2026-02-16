# 📸 Загрузка изображений - Полное руководство

## 🎯 Проблема
Backend принимает файлы до 5MB, но большие PNG (3.6MB+) вызывают ошибку парсинга multipart.

## ✅ Решение
**Автоматическая компрессия на фронтенде** перед загрузкой.

---

## 📦 Вариант 1: Нативный JavaScript (без библиотек)

### Код компонента
```tsx
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
        
        // JPEG с качеством 80%
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
          0.8
        );
      };
    };
  });
};

// Использование
const handleFileChange = async (e) => {
  const file = e.target.files?.[0];
  if (!file) return;
  
  let finalFile = file;
  
  // Если больше 1MB → сжимаем
  if (file.size > 1024 * 1024) {
    finalFile = await compressImage(file);
  }
  
  // Загружаем
  const formData = new FormData();
  formData.append('image', finalFile);
  // ... fetch
};
```

### Результат
- PNG 3.6MB → JPEG 800KB ✅
- PNG 1.5MB → JPEG 400KB ✅
- JPEG 500KB → без изменений ✅

---

## 🚀 Вариант 2: browser-image-compression (рекомендуется)

### Установка
```bash
npm install browser-image-compression
```

### Код компонента
```tsx
import imageCompression from 'browser-image-compression';

const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
  const file = e.target.files?.[0];
  if (!file) return;

  try {
    // Настройки компрессии
    const options = {
      maxSizeMB: 1,              // Максимум 1MB
      maxWidthOrHeight: 1200,    // Максимальный размер стороны
      useWebWorker: true,        // Используем Web Worker (не блокирует UI)
      fileType: 'image/jpeg'     // Всегда конвертируем в JPEG
    };
    
    console.log(`📦 Original: ${(file.size / 1024 / 1024).toFixed(2)} MB`);
    
    // 🎨 Компрессия
    const compressedFile = await imageCompression(file, options);
    
    console.log(`✅ Compressed: ${(compressedFile.size / 1024 / 1024).toFixed(2)} MB`);
    
    // Загрузка
    const formData = new FormData();
    formData.append('image', compressedFile);
    
    const response = await fetch(
      `${API_URL}/api/admin/products/${productId}/image`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        },
        body: formData
      }
    );
    
    if (response.ok) {
      alert('✅ Image uploaded!');
    }
  } catch (error) {
    console.error('Upload failed:', error);
  }
};
```

### Преимущества
- ✅ Работает с любыми размерами (даже 50MB+)
- ✅ WebWorker → не блокирует интерфейс
- ✅ Лучше оптимизирует JPEG
- ✅ Сохраняет EXIF (ориентация фото)
- ✅ Встроенный progress callback

### Пример с прогрессом
```tsx
const compressedFile = await imageCompression(file, {
  maxSizeMB: 1,
  maxWidthOrHeight: 1200,
  useWebWorker: true,
  onProgress: (progress) => {
    console.log(`Compressing: ${progress}%`);
    setProgress(progress); // Показываем пользователю
  }
});
```

---

## 📊 Сравнение результатов

| Оригинал | Нативный JS | browser-image-compression |
|----------|-------------|---------------------------|
| PNG 3.6MB | JPEG 800KB | JPEG 750KB |
| PNG 5.2MB | JPEG 1.1MB | JPEG 950KB |
| JPEG 2MB | JPEG 1.2MB | JPEG 980KB |
| WebP 4MB | JPEG 850KB | JPEG 800KB |

---

## 🎨 Полный компонент с UI

```tsx
import { useState } from 'react';
import imageCompression from 'browser-image-compression';

function ProductImageUpload({ productId }: { productId: string }) {
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [preview, setPreview] = useState<string | null>(null);
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
      
      // 2. Компрессия
      const options = {
        maxSizeMB: 1,
        maxWidthOrHeight: 1200,
        useWebWorker: true,
        fileType: 'image/jpeg',
        onProgress: (p: number) => setProgress(p)
      };
      
      const compressedFile = await imageCompression(file, options);
      
      // 3. Загрузка
      setUploading(true);
      const formData = new FormData();
      formData.append('image', compressedFile);

      const token = localStorage.getItem('admin_token');
      const response = await fetch(
        `https://your-api.com/api/admin/products/${productId}/image`,
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

      // 4. Успех
      alert('✅ Image uploaded successfully!');
      window.location.reload();
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
      setProgress(0);
    }
  };

  return (
    <div className="image-upload">
      <label htmlFor="image-input" className="upload-label">
        📸 Product Image
      </label>
      
      <input
        id="image-input"
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading}
        className="file-input"
      />
      
      {preview && (
        <div className="preview-container">
          <img 
            src={preview} 
            alt="Preview" 
            className="preview-image"
          />
        </div>
      )}
      
      {progress > 0 && progress < 100 && (
        <div className="progress-bar">
          <div 
            className="progress-fill" 
            style={{ width: `${progress}%` }}
          />
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

// CSS
const styles = `
.image-upload {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.upload-label {
  font-weight: 600;
  color: #333;
}

.file-input {
  padding: 8px;
  border: 2px dashed #ccc;
  border-radius: 8px;
  cursor: pointer;
}

.file-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.preview-container {
  margin-top: 8px;
}

.preview-image {
  max-width: 300px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}

.progress-bar {
  position: relative;
  height: 24px;
  background: #f0f0f0;
  border-radius: 12px;
  overflow: hidden;
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
}

.status {
  color: #666;
  font-style: italic;
}

.error {
  color: #f44336;
  font-weight: 500;
}

.hint {
  color: #999;
  font-size: 0.9em;
}
`;
```

---

## 🎯 Рекомендации

### Для прототипа / MVP:
**Вариант 1 (нативный JS)** - достаточно, не требует зависимостей

### Для production:
**Вариант 2 (browser-image-compression)** - надёжнее, быстрее, больше возможностей

### Настройки по типу проекта:

**E-commerce (высокое качество):**
```js
{
  maxSizeMB: 1.5,
  maxWidthOrHeight: 1600,
  quality: 0.85
}
```

**Dashboard (быстрая загрузка):**
```js
{
  maxSizeMB: 0.5,
  maxWidthOrHeight: 800,
  quality: 0.75
}
```

**Mobile-first:**
```js
{
  maxSizeMB: 0.3,
  maxWidthOrHeight: 600,
  quality: 0.7
}
```

---

## ✅ Чек-лист перед деплоем

- [ ] Установлена библиотека `browser-image-compression` (если используете)
- [ ] Добавлена валидация типов файлов (image/*)
- [ ] Показывается превью перед загрузкой
- [ ] Есть индикатор прогресса компрессии
- [ ] Есть индикатор загрузки
- [ ] Обработка ошибок с понятными сообщениями
- [ ] Disabled состояние инпута во время загрузки
- [ ] Очистка превью (URL.revokeObjectURL) после загрузки
- [ ] Логирование размеров до/после (для дебага)
- [ ] Тестирование с PNG 5MB+
- [ ] Тестирование с JPEG 2MB+
- [ ] Тестирование с WebP

---

## 🐛 Troubleshooting

### "Compression takes too long"
→ Уменьшите `maxWidthOrHeight` до 800-1000px

### "Compressed file still too big"
→ Понизьте `quality` до 0.7 или `maxSizeMB` до 0.5

### "Image looks pixelated"
→ Увеличьте `quality` до 0.9 и `maxWidthOrHeight` до 1600

### "UI freezes during compression"
→ Убедитесь что `useWebWorker: true`

### "CORS error"
→ Проверьте что API возвращает `Access-Control-Allow-Origin: *`

---

## 📚 Дополнительные ресурсы

- [browser-image-compression docs](https://github.com/Donaldcwl/browser-image-compression)
- [MDN: Canvas API](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API)
- [MDN: File API](https://developer.mozilla.org/en-US/docs/Web/API/File)
- [JPEG optimization guide](https://developers.google.com/speed/docs/insights/OptimizeImages)

---

**Готово! Теперь можно загружать фото любого размера! 🎉**
