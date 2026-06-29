import { useMemo, useState } from 'react';
import type { AdminResourceRow, ResourceStatus } from '../../../types/admin';
import type { CreateShopProductDto, LocalizedAdminTextDto } from '../../../types/adminApi';
import { FieldError, LanguageTabs, csv, firstText, isValidSlug, isValidUrl, optionalInteger, useLangTab, type FormErrors } from './formUtils';

type BackendShop = {
  slug?: string; sku?: string | null; category?: string; name_uk?: string; name_ru?: string; name_en?: string;
  short_description_uk?: string; short_description_ru?: string; short_description_en?: string;
  description_uk?: string; description_ru?: string; description_en?: string;
  seo_title_uk?: string; seo_title_ru?: string; seo_title_en?: string;
  seo_description_uk?: string; seo_description_ru?: string; seo_description_en?: string;
  selling_points?: string[]; image_urls?: string[]; price_cents?: number | null; currency?: string; stock_quantity?: number; status?: string;
};

type ShopFormState = {
  name: LocalizedAdminTextDto; shortDescription: LocalizedAdminTextDto; description: LocalizedAdminTextDto; seoTitle: LocalizedAdminTextDto; seoDescription: LocalizedAdminTextDto;
  slug: string; sku: string; category: string; imageUrls: string; sellingPoints: string; price: string; currency: string; stockQuantity: string; status: ResourceStatus;
};

function initialState(row?: AdminResourceRow | null): ShopFormState {
  const backend = (row?.backend || {}) as BackendShop;
  return {
    name: { uk: backend.name_uk || '', ru: backend.name_ru || row?.title || '', en: backend.name_en || '' },
    shortDescription: { uk: backend.short_description_uk || '', ru: backend.short_description_ru || '', en: backend.short_description_en || '' },
    description: { uk: backend.description_uk || '', ru: backend.description_ru || '', en: backend.description_en || '' },
    seoTitle: { uk: backend.seo_title_uk || '', ru: backend.seo_title_ru || '', en: backend.seo_title_en || '' },
    seoDescription: { uk: backend.seo_description_uk || '', ru: backend.seo_description_ru || '', en: backend.seo_description_en || '' },
    slug: backend.slug || row?.slug || '',
    sku: backend.sku || '',
    category: backend.category || row?.type || '',
    imageUrls: backend.image_urls?.join(', ') || '',
    sellingPoints: backend.selling_points?.join(', ') || '',
    price: backend.price_cents ? String(backend.price_cents / 100) : '',
    currency: backend.currency || 'PLN',
    stockQuantity: backend.stock_quantity?.toString() || '0',
    status: (backend.status as ResourceStatus) || row?.status || 'draft'
  };
}

export function ShopProductForm({ formId, row, disabled, editMode, onSubmit }: { formId: string; row?: AdminResourceRow | null; disabled?: boolean; editMode?: boolean; onSubmit: (payload: CreateShopProductDto) => void }) {
  const [lang, setLang] = useLangTab();
  const [form, setForm] = useState<ShopFormState>(() => initialState(row));
  const [errors, setErrors] = useState<FormErrors>({});
  const title = useMemo(() => firstText(form.name), [form.name]);

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
      {editMode ? <p className="admin-soft-alert">Backend currently supports editing shop status only. Other fields are kept for future full update endpoints.</p> : null}
      <LanguageTabs active={lang} onChange={setLang} />
      <label><span>Name {lang.toUpperCase()}</span><input disabled={disabled || editMode} value={form.name[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, name: { ...current.name, [lang]: event.target.value } }))} /><FieldError message={errors.name} /></label>
      <label><span>Short description {lang.toUpperCase()}</span><input disabled={disabled || editMode} value={form.shortDescription[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, shortDescription: { ...current.shortDescription, [lang]: event.target.value } }))} /></label>
      <label><span>Description {lang.toUpperCase()}</span><textarea disabled={disabled || editMode} value={form.description[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, description: { ...current.description, [lang]: event.target.value } }))} /></label>
      <label><span>Slug</span><input disabled={disabled || editMode} value={form.slug} onChange={(event) => setForm((current) => ({ ...current, slug: event.target.value }))} /><FieldError message={errors.slug} /></label>
      <label><span>Category</span><input disabled={disabled || editMode} value={form.category} onChange={(event) => setForm((current) => ({ ...current, category: event.target.value }))} /></label>
      <label><span>Status</span><select disabled={disabled} value={form.status} onChange={(event) => setForm((current) => ({ ...current, status: event.target.value as ResourceStatus }))}><option value="draft">draft</option><option value="active">active</option><option value="published">published</option><option value="archived">archived</option></select></label>
      <label><span>SKU</span><input disabled={disabled || editMode} value={form.sku} onChange={(event) => setForm((current) => ({ ...current, sku: event.target.value }))} /></label>
      <label><span>Image URLs</span><input disabled={disabled || editMode} value={form.imageUrls} onChange={(event) => setForm((current) => ({ ...current, imageUrls: event.target.value }))} /><FieldError message={errors.imageUrls} /></label>
      <label><span>Selling points</span><input disabled={disabled || editMode} value={form.sellingPoints} onChange={(event) => setForm((current) => ({ ...current, sellingPoints: event.target.value }))} /></label>
      <div className="admin-form-columns">
        <label><span>Price</span><input disabled={disabled || editMode} value={form.price} onChange={(event) => setForm((current) => ({ ...current, price: event.target.value }))} /><FieldError message={errors.price} /></label>
        <label><span>Currency</span><input disabled={disabled || editMode} value={form.currency} onChange={(event) => setForm((current) => ({ ...current, currency: event.target.value }))} /></label>
        <label><span>Stock</span><input disabled={disabled || editMode} value={form.stockQuantity} onChange={(event) => setForm((current) => ({ ...current, stockQuantity: event.target.value }))} /><FieldError message={errors.stockQuantity} /></label>
      </div>
    </form>
  );
}
