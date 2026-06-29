import { useMemo, useState } from 'react';
import type { AdminResourceRow } from '../../../types/admin';
import type { CreateCatalogItemDto, LocalizedAdminTextDto } from '../../../types/adminApi';
import { FieldError, LanguageTabs, csv, firstText, isValidUrl, optionalInteger, optionalNumber, useLangTab, type FormErrors } from './formUtils';

type CatalogBackend = {
  name_uk?: string | null; name_ru?: string | null; name_en?: string | null;
  description_uk?: string | null; description_ru?: string | null; description_en?: string | null;
  image_url?: string | null; product_type?: string | null; unit?: string | null;
  calories_per_100g?: number | null; protein_per_100g?: number | null; fat_per_100g?: number | null; carbs_per_100g?: number | null;
  seasons?: string[]; seo_title?: string | null; seo_description?: string | null; seo_h1?: string | null;
};

type CatalogFormState = {
  name: LocalizedAdminTextDto;
  description: LocalizedAdminTextDto;
  productType: string;
  unit: string;
  imageUrl: string;
  calories: string;
  protein: string;
  fat: string;
  carbs: string;
  seasons: string;
  seoTitle: string;
  seoDescription: string;
  seoH1: string;
};

const units = ['gram', 'kilogram', 'liter', 'milliliter', 'piece', 'bunch', 'can', 'bottle', 'package'];
const productTypes = ['seafood', 'meat', 'dairy', 'vegetable', 'fruit', 'grain', 'legume', 'nut', 'spice', 'oil', 'beverage', 'condiment', 'bakery', 'supplement'];

function initialState(row?: AdminResourceRow | null): CatalogFormState {
  const backend = (row?.backend || {}) as CatalogBackend;
  return {
    name: { uk: backend.name_uk || '', ru: backend.name_ru || row?.title || '', en: backend.name_en || '' },
    description: { uk: backend.description_uk || '', ru: backend.description_ru || '', en: backend.description_en || '' },
    productType: backend.product_type || row?.type || 'vegetable',
    unit: backend.unit || 'gram',
    imageUrl: backend.image_url || '',
    calories: backend.calories_per_100g?.toString() || '',
    protein: backend.protein_per_100g?.toString() || '',
    fat: backend.fat_per_100g?.toString() || '',
    carbs: backend.carbs_per_100g?.toString() || '',
    seasons: backend.seasons?.join(', ') || '',
    seoTitle: backend.seo_title || '',
    seoDescription: backend.seo_description || '',
    seoH1: backend.seo_h1 || ''
  };
}

export function CatalogProductForm({ formId, row, disabled, onSubmit }: { formId: string; row?: AdminResourceRow | null; disabled?: boolean; onSubmit: (payload: CreateCatalogItemDto) => void }) {
  const [lang, setLang] = useLangTab();
  const [form, setForm] = useState<CatalogFormState>(() => initialState(row));
  const [errors, setErrors] = useState<FormErrors>({});
  const title = useMemo(() => firstText(form.name), [form.name]);

  function validate() {
    const next: FormErrors = {};
    if (!title.trim()) next.name = 'Введите название хотя бы на одном языке.';
    if (!form.productType) next.productType = 'Выберите тип продукта.';
    if (!form.unit) next.unit = 'Выберите единицу.';
    if (!isValidUrl(form.imageUrl)) next.imageUrl = 'Введите корректный URL изображения.';
    for (const [key, value] of [['calories', form.calories], ['protein', form.protein], ['fat', form.fat], ['carbs', form.carbs]]) {
      if (value && Number.isNaN(Number(value))) next[key] = 'Введите число.';
    }
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
        description: form.description,
        productType: form.productType,
        unit: form.unit,
        imageUrl: form.imageUrl.trim() || undefined,
        caloriesPer100g: optionalNumber(form.calories),
        proteinPer100g: optionalNumber(form.protein),
        fatPer100g: optionalNumber(form.fat),
        carbsPer100g: optionalNumber(form.carbs),
        seasons: csv(form.seasons),
        seoTitle: form.seoTitle.trim() || undefined,
        seoDescription: form.seoDescription.trim() || undefined,
        seoH1: form.seoH1.trim() || undefined
      });
    }}>
      <LanguageTabs active={lang} onChange={setLang} />
      <label><span>Name {lang.toUpperCase()}</span><input disabled={disabled} value={form.name[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, name: { ...current.name, [lang]: event.target.value } }))} /><FieldError message={errors.name} /></label>
      <label><span>Description {lang.toUpperCase()}</span><textarea disabled={disabled} value={form.description[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, description: { ...current.description, [lang]: event.target.value } }))} /></label>
      <label><span>Product type</span><select disabled={disabled} value={form.productType} onChange={(event) => setForm((current) => ({ ...current, productType: event.target.value }))}>{productTypes.map((item) => <option key={item} value={item}>{item}</option>)}</select><FieldError message={errors.productType} /></label>
      <label><span>Unit</span><select disabled={disabled} value={form.unit} onChange={(event) => setForm((current) => ({ ...current, unit: event.target.value }))}>{units.map((item) => <option key={item} value={item}>{item}</option>)}</select><FieldError message={errors.unit} /></label>
      <label><span>Image URL</span><input disabled={disabled} value={form.imageUrl} onChange={(event) => setForm((current) => ({ ...current, imageUrl: event.target.value }))} /><FieldError message={errors.imageUrl} /></label>
      <div className="admin-form-columns">
        <label><span>Calories</span><input disabled={disabled} value={form.calories} onChange={(event) => setForm((current) => ({ ...current, calories: event.target.value }))} /><FieldError message={errors.calories} /></label>
        <label><span>Protein</span><input disabled={disabled} value={form.protein} onChange={(event) => setForm((current) => ({ ...current, protein: event.target.value }))} /><FieldError message={errors.protein} /></label>
        <label><span>Fat</span><input disabled={disabled} value={form.fat} onChange={(event) => setForm((current) => ({ ...current, fat: event.target.value }))} /><FieldError message={errors.fat} /></label>
        <label><span>Carbs</span><input disabled={disabled} value={form.carbs} onChange={(event) => setForm((current) => ({ ...current, carbs: event.target.value }))} /><FieldError message={errors.carbs} /></label>
      </div>
      <label><span>Seasons</span><input disabled={disabled} placeholder="spring, summer" value={form.seasons} onChange={(event) => setForm((current) => ({ ...current, seasons: event.target.value }))} /></label>
      <label><span>SEO title</span><input disabled={disabled} value={form.seoTitle} onChange={(event) => setForm((current) => ({ ...current, seoTitle: event.target.value }))} /></label>
      <label><span>SEO description</span><input disabled={disabled} value={form.seoDescription} onChange={(event) => setForm((current) => ({ ...current, seoDescription: event.target.value }))} /></label>
      <label><span>SEO H1</span><input disabled={disabled} value={form.seoH1} onChange={(event) => setForm((current) => ({ ...current, seoH1: event.target.value }))} /></label>
    </form>
  );
}
