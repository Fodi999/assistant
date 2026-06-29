import { useMemo, useState } from 'react';
import { aiCreateShopProductDraft, generateShopProductImage, uploadShopReference } from '../../../api/shop';
import type { AdminResourceRow, ResourceStatus } from '../../../types/admin';
import type { CreateShopProductDto, LocalizedAdminTextDto } from '../../../types/adminApi';
import { FieldError, LanguageTabs, csv, firstText, isValidSlug, isValidUrl, optionalInteger, useLangTab, type FormErrors } from './formUtils';

type BackendShop = {
  slug?: string; sku?: string | null; category?: string; name_uk?: string; name_ru?: string; name_pl?: string; name_en?: string;
  short_description_uk?: string; short_description_ru?: string; short_description_pl?: string; short_description_en?: string;
  description_uk?: string; description_ru?: string; description_pl?: string; description_en?: string;
  seo_title_uk?: string; seo_title_ru?: string; seo_title_pl?: string; seo_title_en?: string;
  seo_description_uk?: string; seo_description_ru?: string; seo_description_pl?: string; seo_description_en?: string;
  selling_points?: string[]; image_urls?: string[]; price_cents?: number | null; currency?: string; stock_quantity?: number; status?: string;
};

type ShopFormState = {
  name: LocalizedAdminTextDto; shortDescription: LocalizedAdminTextDto; description: LocalizedAdminTextDto; seoTitle: LocalizedAdminTextDto; seoDescription: LocalizedAdminTextDto;
  slug: string; sku: string; category: string; imageUrls: string; sellingPoints: string; price: string; currency: string; stockQuantity: string; status: ResourceStatus;
};

const GEMINI_REFERENCE_MAX_BYTES = 10 * 1024 * 1024;
const GEMINI_REFERENCE_TARGET_BYTES = 9 * 1024 * 1024;

function initialState(row?: AdminResourceRow | null, initialCategory = ''): ShopFormState {
  const backend = (row?.backend || {}) as BackendShop;
  return {
    name: { uk: backend.name_uk || '', ru: backend.name_ru || row?.title || '', pl: backend.name_pl || '', en: backend.name_en || '' },
    shortDescription: { uk: backend.short_description_uk || '', ru: backend.short_description_ru || '', pl: backend.short_description_pl || '', en: backend.short_description_en || '' },
    description: { uk: backend.description_uk || '', ru: backend.description_ru || '', pl: backend.description_pl || '', en: backend.description_en || '' },
    seoTitle: { uk: backend.seo_title_uk || '', ru: backend.seo_title_ru || '', pl: backend.seo_title_pl || '', en: backend.seo_title_en || '' },
    seoDescription: { uk: backend.seo_description_uk || '', ru: backend.seo_description_ru || '', pl: backend.seo_description_pl || '', en: backend.seo_description_en || '' },
    slug: backend.slug || row?.slug || '',
    sku: backend.sku || '',
    category: backend.category || row?.type || initialCategory,
    imageUrls: backend.image_urls?.join(', ') || '',
    sellingPoints: backend.selling_points?.join(', ') || '',
    price: backend.price_cents ? String(backend.price_cents / 100) : '',
    currency: backend.currency || 'PLN',
    stockQuantity: backend.stock_quantity?.toString() || '0',
    status: (backend.status as ResourceStatus) || row?.status || 'draft'
  };
}

function canvasToBlob(canvas: HTMLCanvasElement, quality: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error('Не удалось подготовить фото для Gemini.'));
    }, 'image/jpeg', quality);
  });
}

async function prepareReferenceImage(file: File): Promise<File> {
  if (file.size < GEMINI_REFERENCE_TARGET_BYTES) return file;

  let bitmap: ImageBitmap | null = null;
  try {
    bitmap = await createImageBitmap(file);
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Canvas недоступен.');

    let scale = Math.min(1, 2400 / Math.max(bitmap.width, bitmap.height));
    let quality = .84;
    let lastBlob: Blob | null = null;

    for (let attempt = 0; attempt < 12; attempt += 1) {
      canvas.width = Math.max(1, Math.round(bitmap.width * scale));
      canvas.height = Math.max(1, Math.round(bitmap.height * scale));
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const blob = await canvasToBlob(canvas, quality);
      lastBlob = blob;
      if (blob.size < GEMINI_REFERENCE_TARGET_BYTES) {
        return new File([blob], file.name.replace(/\.[^.]+$/, '.jpg'), { type: 'image/jpeg' });
      }
      if (quality > .62) quality -= .08;
      else {
        scale *= .82;
        quality = .82;
      }
    }

    if (lastBlob && lastBlob.size < GEMINI_REFERENCE_MAX_BYTES) {
      return new File([lastBlob], file.name.replace(/\.[^.]+$/, '.jpg'), { type: 'image/jpeg' });
    }
  } catch {
    throw new Error('Фото больше 10 MB. Не удалось автоматически сжать файл, попробуйте JPG/PNG меньшего размера.');
  } finally {
    bitmap?.close();
  }

  throw new Error('Reference image must be smaller than 10 MB. Фото слишком большое даже после сжатия.');
}

export function ShopProductForm({ formId, row, disabled, editMode, initialCategory, categories = [], onSubmit }: { formId: string; row?: AdminResourceRow | null; disabled?: boolean; editMode?: boolean; initialCategory?: string; categories?: string[]; onSubmit: (payload: CreateShopProductDto) => void }) {
  const [lang, setLang] = useLangTab();
  const [form, setForm] = useState<ShopFormState>(() => initialState(row, initialCategory));
  const [errors, setErrors] = useState<FormErrors>({});
  const [uploading, setUploading] = useState(false);
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiBusy, setAiBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [regeneratingIndex, setRegeneratingIndex] = useState<number | null>(null);
  const [imagePrompts, setImagePrompts] = useState<string[]>([]);
  const [originalReferenceUrl, setOriginalReferenceUrl] = useState('');
  const [generatedPhotoUrls, setGeneratedPhotoUrls] = useState<string[]>([]);
  const [lightboxUrl, setLightboxUrl] = useState('');
  const [lightboxZoom, setLightboxZoom] = useState(1);
  const [aiMessage, setAiMessage] = useState<string | null>(null);
  const title = useMemo(() => firstText(form.name), [form.name]);
  const images = useMemo(() => csv(form.imageUrls), [form.imageUrls]);
  const firstImage = images[0];
  const generatedPreviewImages = generatedPhotoUrls.length ? generatedPhotoUrls : (images.length > 1 || !originalReferenceUrl ? images.slice(0, 4) : []);

  function mergeDraftText(draft: Awaited<ReturnType<typeof aiCreateShopProductDraft>>) {
    setForm((current) => ({
      ...current,
      name: { uk: draft.name_uk, ru: draft.name_ru, pl: draft.name_pl, en: draft.name_en },
      shortDescription: { uk: draft.short_description_uk, ru: draft.short_description_ru, pl: draft.short_description_pl, en: draft.short_description_en },
      description: { uk: draft.description_uk, ru: draft.description_ru, pl: draft.description_pl, en: draft.description_en },
      seoTitle: { uk: draft.seo_title_uk, ru: draft.seo_title_ru, pl: draft.seo_title_pl, en: draft.seo_title_en },
      seoDescription: { uk: draft.seo_description_uk, ru: draft.seo_description_ru, pl: draft.seo_description_pl, en: draft.seo_description_en },
      slug: current.slug || draft.slug,
      category: current.category || draft.category,
      sellingPoints: draft.selling_points?.join(', ') || current.sellingPoints
    }));
    setImagePrompts(draft.image_prompts || []);
  }

  function productBrief() {
    return [
      aiPrompt.trim(),
      title ? `Название: ${title}` : '',
      form.category ? `Категория: ${form.category}` : '',
      form.sku ? `SKU: ${form.sku}` : '',
      form.shortDescription.ru || form.shortDescription.en ? `Описание: ${form.shortDescription.ru || form.shortDescription.en}` : ''
    ].filter(Boolean).join('\n');
  }

  async function generateTextWithGemini() {
    const brief = productBrief();
    if (!brief) {
      setErrors((current) => ({ ...current, ai: 'Опишите товар или загрузите название перед Gemini.' }));
      return;
    }
    setAiBusy(true);
    setAiMessage(null);
    setErrors((current) => {
      const { ai: _ai, ...rest } = current;
      return rest;
    });
    try {
      const draft = await aiCreateShopProductDraft(brief, 4);
      mergeDraftText(draft);
      setLang('ru');
      setAiMessage('Gemini заполнил тексты на 4 языка и подготовил промпты для 4 фото.');
    } catch (error) {
      setErrors((current) => ({ ...current, ai: error instanceof Error ? error.message : 'Gemini не смог заполнить товар.' }));
    } finally {
      setAiBusy(false);
    }
  }

  async function ensureImagePrompts(titleForImage: string) {
    if (imagePrompts.length) return imagePrompts;
    const draft = await aiCreateShopProductDraft(productBrief() || titleForImage, 4);
    const prompts = draft.image_prompts || [];
    setImagePrompts(prompts);
    mergeDraftText(draft);
    return prompts;
  }

  async function generateOnePhoto(index: number, referenceUrl: string, prompts: string[]) {
    const titleForImage = form.name.en || form.name.ru || form.name.pl || form.name.uk || title || 'Shop product';
    const result = await generateShopProductImage(
      titleForImage,
      prompts[index],
      index,
      [referenceUrl],
      false,
      { photoScenarios: ['white-background', 'catalog-card', 'detail-shot'], scaleReference: 'none' }
    );
    return result.image_url;
  }

  async function generatePhotosWithGemini() {
    const referenceUrl = originalReferenceUrl || firstImage;
    if (!referenceUrl) {
      setErrors((current) => ({ ...current, imageUrls: 'Сначала загрузите настоящее фото товара с ПК.' }));
      return;
    }
    const titleForImage = form.name.en || form.name.ru || form.name.pl || form.name.uk || title || 'Shop product';
    setImageBusy(true);
    setAiMessage(null);
    try {
      const prompts = await ensureImagePrompts(titleForImage);
      const generated: string[] = [];
      for (let index = 0; index < 4; index += 1) {
        const url = await generateOnePhoto(index, referenceUrl, prompts);
        generated.push(url);
        setGeneratedPhotoUrls([...generated]);
        setForm((current) => ({ ...current, imageUrls: generated.join(', ') }));
      }
      setAiMessage('Gemini создал 4 фото товара от загруженного оригинала.');
    } catch (error) {
      setErrors((current) => ({ ...current, imageUrls: error instanceof Error ? error.message : 'Не удалось сгенерировать фото товара.' }));
    } finally {
      setImageBusy(false);
    }
  }

  async function regeneratePhoto(index: number) {
    const referenceUrl = originalReferenceUrl || firstImage;
    if (!referenceUrl) {
      setErrors((current) => ({ ...current, imageUrls: 'Для перегенерации нужен оригинал. Загрузите настоящее фото товара.' }));
      return;
    }
    const titleForImage = form.name.en || form.name.ru || form.name.pl || form.name.uk || title || 'Shop product';
    setRegeneratingIndex(index);
    setAiMessage(null);
    try {
      const prompts = await ensureImagePrompts(titleForImage);
      const url = await generateOnePhoto(index, referenceUrl, prompts);
      setGeneratedPhotoUrls((current) => {
        const base = current.length ? current : images.slice(0, 4);
        const next = Array.from({ length: 4 }, (_, itemIndex) => base[itemIndex] || '');
        next[index] = url;
        setForm((formCurrent) => ({ ...formCurrent, imageUrls: next.filter(Boolean).join(', ') }));
        return next.filter(Boolean);
      });
      setAiMessage(`Фото #${index + 1} перегенерировано.`);
    } catch (error) {
      setErrors((current) => ({ ...current, imageUrls: error instanceof Error ? error.message : 'Не удалось перегенерировать фото.' }));
    } finally {
      setRegeneratingIndex(null);
    }
  }

  async function uploadImage(file?: File) {
    if (!file) return;
    setUploading(true);
    setErrors((current) => {
      const { imageUrls: _imageUrls, ...rest } = current;
      return rest;
    });
    try {
      const preparedFile = await prepareReferenceImage(file);
      const url = await uploadShopReference(preparedFile);
      setForm((current) => {
        const nextImages = [url, ...csv(current.imageUrls).filter((image) => image !== url)];
        return { ...current, imageUrls: nextImages.join(', ') };
      });
      setOriginalReferenceUrl(url);
      setGeneratedPhotoUrls([]);
      setAiMessage('Оригинальное фото загружено. Теперь можно сгенерировать 4 каталожных фото через Gemini.');
    } catch (error) {
      setErrors((current) => ({ ...current, imageUrls: error instanceof Error ? error.message : 'Не удалось загрузить фото.' }));
    } finally {
      setUploading(false);
    }
  }

  function openLightbox(url: string) {
    setLightboxUrl(url);
    setLightboxZoom(1);
  }

  function closeLightbox() {
    setLightboxUrl('');
    setLightboxZoom(1);
  }

  function validate() {
    const next: FormErrors = {};
    if (!editMode && !form.name.en?.trim()) next.name = 'Backend требует английское название. Заполните EN.';
    if (!title.trim()) next.name = 'Введите название хотя бы на одном языке.';
    if (!isValidSlug(form.slug)) next.slug = 'Slug: только lowercase letters, цифры и дефисы.';
    const images = csv(form.imageUrls);
    if (images.some((url) => !isValidUrl(url))) next.imageUrls = 'Все изображения должны быть корректными URL.';
    if (form.price && Number.isNaN(Number(form.price))) next.price = 'Введите цену числом.';
    if (form.stockQuantity && Number.isNaN(Number(form.stockQuantity))) next.stockQuantity = 'Введите количество числом.';
    setErrors(next);
    return !Object.keys(next).length;
  }

  return (
    <>
    <form id={formId} className="admin-form-grid" onSubmit={(event) => {
      event.preventDefault();
      if (!validate()) return;
      onSubmit({
        siteId: row?.siteId || 'construction',
        title,
        name: form.name,
        shortDescription: form.shortDescription,
        description: form.description,
        seoTitle: form.seoTitle,
        seoDescription: form.seoDescription,
        slug: form.slug.trim() || undefined,
        sku: form.sku.trim() || undefined,
        type: form.category.trim() || undefined,
        imageUrls: csv(form.imageUrls),
        sellingPoints: csv(form.sellingPoints),
        priceCents: form.price ? Math.round(Number(form.price) * 100) : undefined,
        currency: form.currency.trim() || undefined,
        stockQuantity: optionalInteger(form.stockQuantity),
        status: form.status
      });
    }}>
      {editMode ? <p className="admin-soft-alert">Редактирование отправляет полный товар в backend. Если backend вернет ограничение, панель покажет точную ошибку.</p> : null}
      <section className="shop-ai-panel">
        <div>
          <span className="eyebrow">Gemini catalog</span>
          <h3>AI карточка товара</h3>
          <p>Загрузите настоящее фото товара, опишите продукт коротко, затем заполните тексты и создайте 4 фото для витрины.</p>
        </div>
        <textarea disabled={disabled || aiBusy || imageBusy} value={aiPrompt} onChange={(event) => setAiPrompt(event.target.value)} placeholder="Например: японский нож Santoku 18 см, сталь VG10, деревянная ручка, для домашней и профессиональной кухни" />
        <div className="shop-ai-actions">
          <label className={`admin-btn secondary ${disabled || uploading || aiBusy || imageBusy ? 'disabled' : ''}`}>
            <input type="file" accept="image/*" disabled={disabled || uploading || aiBusy || imageBusy} onChange={(event) => {
              void uploadImage(event.target.files?.[0]);
              event.currentTarget.value = '';
            }} hidden />
            <span>{uploading ? 'Загружаем...' : 'Загрузить оригинал'}</span>
          </label>
          <button type="button" className="admin-btn secondary" disabled={disabled || aiBusy || imageBusy} onClick={() => void generateTextWithGemini()}><span>{aiBusy ? 'Gemini пишет...' : 'Заполнить текст Gemini'}</span></button>
          <button type="button" className="admin-btn primary" disabled={disabled || aiBusy || imageBusy || !firstImage} onClick={() => void generatePhotosWithGemini()}><span>{imageBusy ? 'Генерируем 4 фото...' : '4 фото от оригинала'}</span></button>
        </div>
        {errors.ai ? <FieldError message={errors.ai} /> : null}
        {aiMessage ? <p className="admin-soft-alert">{aiMessage}</p> : null}
        {generatedPreviewImages.length ? (
          <div className="shop-generated-preview">
            {Array.from({ length: 4 }, (_, index) => {
              const url = generatedPreviewImages[index];
              return (
                <article key={index}>
                  {url ? <button type="button" className="shop-generated-thumb" onClick={() => openLightbox(url)}><img src={url} alt={`Generated product ${index + 1}`} /><span>View</span></button> : <span>#{index + 1}</span>}
                  <div className="shop-generated-actions">
                    <button type="button" className="table-action" disabled={disabled || imageBusy || regeneratingIndex !== null} onClick={() => void regeneratePhoto(index)}>
                      {regeneratingIndex === index ? '...' : 'Regen'}
                    </button>
                    {url ? <button type="button" className="table-action" onClick={() => openLightbox(url)}>Zoom</button> : null}
                  </div>
                </article>
              );
            })}
          </div>
        ) : null}
      </section>
      <LanguageTabs active={lang} onChange={setLang} />
      <label><span>Name {lang.toUpperCase()}</span><input disabled={disabled} value={form.name[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, name: { ...current.name, [lang]: event.target.value } }))} /><FieldError message={errors.name} /></label>
      <label><span>Short description {lang.toUpperCase()}</span><input disabled={disabled} value={form.shortDescription[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, shortDescription: { ...current.shortDescription, [lang]: event.target.value } }))} /></label>
      <label><span>Description {lang.toUpperCase()}</span><textarea disabled={disabled} value={form.description[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, description: { ...current.description, [lang]: event.target.value } }))} /></label>
      <label><span>SEO title {lang.toUpperCase()}</span><input disabled={disabled} value={form.seoTitle[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, seoTitle: { ...current.seoTitle, [lang]: event.target.value } }))} /></label>
      <label><span>SEO description {lang.toUpperCase()}</span><textarea disabled={disabled} value={form.seoDescription[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, seoDescription: { ...current.seoDescription, [lang]: event.target.value } }))} /></label>
      <label><span>Slug</span><input disabled={disabled} value={form.slug} onChange={(event) => setForm((current) => ({ ...current, slug: event.target.value.toLowerCase() }))} /><FieldError message={errors.slug} /></label>
      <label><span>Category</span><input list="shop-category-options" disabled={disabled} value={form.category} onChange={(event) => setForm((current) => ({ ...current, category: event.target.value }))} /><datalist id="shop-category-options">{categories.map((category) => <option value={category} key={category} />)}</datalist></label>
      <label><span>Status</span><select disabled={disabled} value={form.status} onChange={(event) => setForm((current) => ({ ...current, status: event.target.value as ResourceStatus }))}><option value="draft">draft</option><option value="active">active</option><option value="published">published</option><option value="archived">archived</option></select></label>
      <label><span>SKU</span><input disabled={disabled} value={form.sku} onChange={(event) => setForm((current) => ({ ...current, sku: event.target.value }))} /></label>
      <div className="shop-product-photo-field">
        {firstImage ? <img src={firstImage} alt={title || 'Product'} /> : <span>Photo</span>}
        <div>
          <label><span>Image URLs</span><input disabled={disabled} value={form.imageUrls} onChange={(event) => setForm((current) => ({ ...current, imageUrls: event.target.value }))} /><FieldError message={errors.imageUrls} /></label>
          <label className={`table-action ${disabled || uploading ? 'disabled' : ''}`}>
            <input type="file" accept="image/*" disabled={disabled || uploading} onChange={(event) => {
              void uploadImage(event.target.files?.[0]);
              event.currentTarget.value = '';
            }} hidden />
            {uploading ? 'Uploading...' : 'Upload photo from PC'}
          </label>
        </div>
      </div>
      <label><span>Selling points</span><input disabled={disabled} value={form.sellingPoints} onChange={(event) => setForm((current) => ({ ...current, sellingPoints: event.target.value }))} /></label>
      <div className="admin-form-columns">
        <label><span>Price</span><input disabled={disabled} value={form.price} onChange={(event) => setForm((current) => ({ ...current, price: event.target.value }))} /><FieldError message={errors.price} /></label>
        <label><span>Currency</span><input disabled={disabled} value={form.currency} onChange={(event) => setForm((current) => ({ ...current, currency: event.target.value.toUpperCase() }))} /></label>
        <label><span>Stock</span><input disabled={disabled} value={form.stockQuantity} onChange={(event) => setForm((current) => ({ ...current, stockQuantity: event.target.value }))} /><FieldError message={errors.stockQuantity} /></label>
      </div>
    </form>
    {lightboxUrl ? (
      <div className="shop-photo-lightbox" role="dialog" aria-modal="true" aria-label="Product photo preview" onClick={closeLightbox}>
        <div className="shop-photo-lightbox__toolbar" onClick={(event) => event.stopPropagation()}>
          <strong>Photo preview</strong>
          <span>{Math.round(lightboxZoom * 100)}%</span>
          <button type="button" onClick={() => setLightboxZoom((current) => Math.max(.5, current - .25))}>-</button>
          <button type="button" onClick={() => setLightboxZoom(1)}>100%</button>
          <button type="button" onClick={() => setLightboxZoom((current) => Math.min(3, current + .25))}>+</button>
          <button type="button" onClick={closeLightbox}>Close</button>
        </div>
        <div className="shop-photo-lightbox__stage" onClick={(event) => event.stopPropagation()}>
          <img src={lightboxUrl} alt="Generated product fullscreen" style={{ transform: `scale(${lightboxZoom})` }} />
        </div>
      </div>
    ) : null}
    </>
  );
}
