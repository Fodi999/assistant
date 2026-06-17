import { useMemo, useState } from 'react';
import { aiCreateProductDraft, aiGenerateProductImage, createAdminProduct, deleteAdminProduct, deleteProductStates, generateProductStates, getProductDataQuality, listProductStates, publishAdminProduct, saveExtendedProductProfile, unpublishAdminProduct, updateAdminProduct, updateProductState, type AiCreateDraftResponse, type AiExtendedProductProfile, type CreateAdminProductRequest, type IngredientState, type ProductDataQuality } from '../../api/catalog';
import { revalidateSite } from '../../api/revalidate';
import { AiPhotoGallery } from '../../components/AiPhotoGallery';
import type { AdminCategory, AdminProduct } from '../../types/admin';

type CatalogLang = 'ru' | 'en' | 'pl' | 'uk';
type DraftForm = Omit<CreateAdminProductRequest, 'name_input' | 'auto_translate'>;
type DraftTab = 'basic' | 'nutrition' | 'micros' | 'culinary' | 'health' | 'content';

interface MenuPageProps {
  products: AdminProduct[];
  categories: AdminCategory[];
  loading: boolean;
  error: string | null;
  onReload: () => Promise<void>;
}

const UNIT_OPTIONS = ['gram', 'kilogram', 'liter', 'milliliter', 'piece', 'bunch', 'can', 'bottle', 'package'] as const;
const PRODUCT_TYPES = [
  ['fruit', 'Фрукт'], ['vegetable', 'Овощ'], ['seafood', 'Рыба и морепродукты'], ['meat', 'Мясо и птица'],
  ['dairy', 'Молочное и яйца'], ['grain', 'Крупы и паста'], ['legume', 'Бобовые'], ['nut', 'Орехи и семена'],
  ['spice', 'Специи и травы'], ['oil', 'Масла и жиры'], ['beverage', 'Напитки'], ['condiment', 'Соусы'],
  ['bakery', 'Выпечка'], ['supplement', 'Добавки']
] as const;
const SEASONS = [['Spring', 'Весна'], ['Summer', 'Лето'], ['Autumn', 'Осень'], ['Winter', 'Зима'], ['AllYear', 'Весь год']] as const;
const STATE_LABELS: Record<string, string> = {
  raw: 'сырой',
  boiled: 'варёный',
  steamed: 'на пару',
  baked: 'запечённый',
  grilled: 'на гриле',
  fried: 'жареный',
  smoked: 'копчёный',
  frozen: 'замороженный',
  dried: 'сушёный',
  pickled: 'маринованный'
};
const NUTRITION_FIELDS: Array<[keyof DraftForm, string, string]> = [
  ['calories_per_100g', 'Энергия', 'kcal'], ['protein_per_100g', 'Белки', 'г'], ['fat_per_100g', 'Жиры', 'г'],
  ['carbs_per_100g', 'Углеводы', 'г'], ['fiber_per_100g', 'Клетчатка', 'г'], ['sugar_per_100g', 'Сахар', 'г'],
  ['density_g_per_ml', 'Плотность', 'г/мл'], ['typical_portion_g', 'Порция', 'г'], ['shelf_life_days', 'Срок хранения', 'дней']
];
const TABS: Array<[DraftTab, string, string]> = [
  ['basic', 'Основное', 'Названия и фото'], ['nutrition', 'Nutrition', 'Макросы и сахара'],
  ['micros', 'Микроэлементы', 'Витамины и минералы'], ['culinary', 'Кулинария', 'Вкус и обработка'],
  ['health', 'Здоровье', 'Диеты и аллергены'], ['content', 'Контент', '4 языка и SEO']
];
const VITAMINS = [['vitamin_a', 'Витамин A', 'µg'], ['vitamin_c', 'Витамин C', 'mg'], ['vitamin_d', 'Витамин D', 'µg'], ['vitamin_e', 'Витамин E', 'mg'], ['vitamin_k', 'Витамин K', 'µg'], ['vitamin_b1', 'B1', 'mg'], ['vitamin_b2', 'B2', 'mg'], ['vitamin_b3', 'B3', 'mg'], ['vitamin_b5', 'B5', 'mg'], ['vitamin_b6', 'B6', 'mg'], ['vitamin_b7', 'B7', 'µg'], ['vitamin_b9', 'B9', 'µg'], ['vitamin_b12', 'B12', 'µg']] as const;
const MINERALS = [['calcium', 'Кальций', 'mg'], ['iron', 'Железо', 'mg'], ['magnesium', 'Магний', 'mg'], ['phosphorus', 'Фосфор', 'mg'], ['potassium', 'Калий', 'mg'], ['sodium', 'Натрий', 'mg'], ['zinc', 'Цинк', 'mg'], ['copper', 'Медь', 'mg'], ['manganese', 'Марганец', 'mg'], ['selenium', 'Селен', 'µg']] as const;
const DIET_FLAGS = [['vegan', 'Веганский'], ['vegetarian', 'Вегетарианский'], ['keto', 'Кето'], ['paleo', 'Палео'], ['gluten_free', 'Без глютена'], ['mediterranean', 'Средиземноморский'], ['low_carb', 'Низкоуглеводный']] as const;
const ALLERGENS = [['milk', 'Молоко'], ['fish', 'Рыба'], ['shellfish', 'Ракообразные'], ['nuts', 'Орехи'], ['soy', 'Соя'], ['gluten', 'Глютен'], ['eggs', 'Яйца'], ['peanuts', 'Арахис'], ['sesame', 'Кунжут'], ['celery', 'Сельдерей'], ['mustard', 'Горчица'], ['sulfites', 'Сульфиты'], ['lupin', 'Люпин'], ['molluscs', 'Моллюски']] as const;

function productName(product: AdminProduct, lang: CatalogLang): string {
  return String(product[`name_${lang}`] || product.name_ru || product.name_en || product.name_pl || '').trim();
}

function formatMacro(value?: number | null): string {
  if (value === null || value === undefined) return '-';
  return Number(value).toFixed(1).replace(/\.0$/, '');
}

function arrayText(value: unknown): string {
  return Array.isArray(value) ? value.join(', ') : '';
}

function textArray(value: string): string[] {
  return value.split(',').map((item) => item.trim()).filter(Boolean);
}

function titleCaseName(value: string): string {
  return value
    .trim()
    .split(/\s+/)
    .map((part) => part ? `${part[0].toUpperCase()}${part.slice(1).toLowerCase()}` : part)
    .join(' ');
}

function isSuspiciousProductName(value?: string | null): boolean {
  const text = String(value || '').trim();
  if (!text) return false;
  const words = text.split(/\s+/).filter(Boolean);
  return text.length > 70 || words.length > 8 || /[.!?]\s+\S/.test(text) || text.includes('\n');
}

function repairEnglishName(value?: string | null, fallback?: string | null): string {
  const text = String(value || '').trim();
  if (!isSuspiciousProductName(text)) return text;

  const compact = text.replace(/[_-]+/g, ' ');
  const latinWords = compact.match(/\b[a-zA-Z]{3,}(?:\s+[a-zA-Z]{2,}){0,2}\b/g) || [];
  const ignored = new Set([
    'rich', 'native', 'central', 'america', 'prized', 'healthy', 'fats',
    'potassium', 'bananas', 'folate', 'vitamins', 'creamy', 'fruit'
  ]);
  const candidate = latinWords.find((item) => {
    const first = item.toLowerCase().split(/\s+/)[0];
    return !ignored.has(first);
  });

  if (candidate) return titleCaseName(candidate);
  return String(fallback || '').trim();
}

function validateProductNamesForSave(draft: DraftForm): string | null {
  const checks: Array<[keyof DraftForm, string]> = [
    ['name_en', 'Название EN'],
    ['name_ru', 'Название RU'],
    ['name_pl', 'Название PL'],
    ['name_uk', 'Название UK'],
  ];
  const bad = checks.find(([field]) => isSuspiciousProductName(String(draft[field] || '')));
  return bad ? `${bad[1]} похоже на описание или SEO-текст. Исправьте короткое название перед сохранением.` : null;
}

function ProfileNumber({ label, value, suffix, onChange }: { label: string; value: unknown; suffix: string; onChange: (value: number | null) => void }) {
  return <label className="nutrition-field"><span>{label}</span><div>
    <input type="number" step="any" value={typeof value === 'number' ? value : ''} onChange={(event) => onChange(event.target.value === '' ? null : Number(event.target.value))} />
    <em>{suffix}</em>
  </div></label>;
}

function ProfileToggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className={`profile-toggle${checked ? ' active' : ''}`}><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span>{checked ? '✓' : ''}</span><strong>{label}</strong></label>;
}

function LanguageTabs({ value, onChange }: { value: CatalogLang; onChange: (language: CatalogLang) => void }) {
  return <div className="language-tabs" role="tablist" aria-label="Язык текста">
    {(['ru', 'en', 'pl', 'uk'] as const).map((language) => <button type="button" role="tab" aria-selected={value === language} className={value === language ? 'active' : ''} key={language} onClick={() => onChange(language)}>{language.toUpperCase()}</button>)}
  </div>;
}

function draftToForm(response: AiCreateDraftResponse): DraftForm {
  const draft = response.draft;
  return {
    name_en: draft.names.en.value || '',
    name_ru: draft.names.ru.value || '',
    name_pl: draft.names.pl.value || '',
    name_uk: draft.names.uk.value || '',
    product_type: draft.product_type.value || '',
    unit: UNIT_OPTIONS.includes(draft.unit.value as typeof UNIT_OPTIONS[number])
      ? draft.unit.value as typeof UNIT_OPTIONS[number]
      : 'kilogram',
    description: draft.description_en.value || '',
    description_en: draft.description_en.value || '',
    description_ru: draft.description_ru.value || '',
    description_pl: draft.description_pl.value || '',
    description_uk: draft.description_uk.value || '',
    calories_per_100g: draft.nutrition.calories_per_100g.value ?? undefined,
    protein_per_100g: draft.nutrition.protein_per_100g.value ?? undefined,
    fat_per_100g: draft.nutrition.fat_per_100g.value ?? undefined,
    carbs_per_100g: draft.nutrition.carbs_per_100g.value ?? undefined,
    fiber_per_100g: draft.nutrition.fiber_per_100g.value ?? undefined,
    sugar_per_100g: draft.nutrition.sugar_per_100g.value ?? undefined,
    density_g_per_ml: draft.nutrition.density_g_per_ml.value ?? undefined,
    typical_portion_g: draft.nutrition.typical_portion_g.value ?? undefined,
    shelf_life_days: draft.nutrition.shelf_life_days.value ?? undefined,
    seasons: draft.seasons.value || [],
    seo_title: draft.seo.seo_title.value || '',
    seo_description: draft.seo.seo_description.value || '',
    seo_h1: draft.seo.seo_h1.value || ''
  };
}

function productToForm(product: AdminProduct): DraftForm {
  const repairedNameEn = repairEnglishName(product.name_en, product.slug?.replace(/-/g, ' '));
  return {
    name_en: repairedNameEn || product.name_en || '',
    name_ru: product.name_ru || '',
    name_pl: product.name_pl || '',
    name_uk: product.name_uk || '',
    product_type: product.product_type || '',
    unit: UNIT_OPTIONS.includes(product.unit as typeof UNIT_OPTIONS[number])
      ? product.unit as typeof UNIT_OPTIONS[number]
      : 'kilogram',
    description: product.description || product.description_en || '',
    description_en: product.description_en || product.description || '',
    description_ru: product.description_ru || '',
    description_pl: product.description_pl || '',
    description_uk: product.description_uk || '',
    image_url: product.image_url || undefined,
    calories_per_100g: product.calories_per_100g ?? undefined,
    protein_per_100g: product.protein_per_100g ?? undefined,
    fat_per_100g: product.fat_per_100g ?? undefined,
    carbs_per_100g: product.carbs_per_100g ?? undefined,
    fiber_per_100g: product.fiber_per_100g ?? undefined,
    sugar_per_100g: product.sugar_per_100g ?? undefined,
    density_g_per_ml: product.density_g_per_ml ?? undefined,
    typical_portion_g: product.typical_portion_g ?? undefined,
    shelf_life_days: product.shelf_life_days ?? undefined,
    seasons: product.seasons || [],
    seo_title: product.seo_title || product.name_en || '',
    seo_description: product.seo_description || '',
    seo_h1: product.seo_h1 || product.name_ru || product.name_en || ''
  };
}

function filled(value: unknown): boolean {
  if (Array.isArray(value)) return value.length > 0;
  if (value === null || value === undefined) return false;
  if (typeof value === 'string') return value.trim().length > 0;
  return true;
}

function completionScore(draft: DraftForm | null, states: IngredientState[]): { percent: number; filled: number; total: number } {
  if (!draft) return { percent: 0, filled: 0, total: 0 };
  const fields: unknown[] = [
    draft.name_en, draft.name_ru, draft.name_pl, draft.name_uk,
    draft.product_type, draft.unit, draft.image_url,
    draft.description_en, draft.description_ru, draft.description_pl, draft.description_uk,
    draft.calories_per_100g, draft.protein_per_100g, draft.fat_per_100g, draft.carbs_per_100g,
    draft.fiber_per_100g, draft.sugar_per_100g, draft.density_g_per_ml, draft.typical_portion_g, draft.shelf_life_days,
    draft.seasons, draft.seo_title, draft.seo_description, draft.seo_h1
  ];
  const stateFields = states.flatMap((state) => [
    state.name_suffix_ru || state.name_suffix_en,
    state.notes_ru || state.notes_en,
    state.calories_per_100g,
    state.storage_temp_c,
    state.shelf_life_hours,
    state.image_url
  ]);
  const allFields = [...fields, ...stateFields];
  const filledCount = allFields.filter(filled).length;
  return {
    percent: allFields.length ? Math.round((filledCount / allFields.length) * 100) : 0,
    filled: filledCount,
    total: allFields.length
  };
}

function localizedDescriptionCount(draft: DraftForm | null): number {
  if (!draft) return 0;
  return (['en', 'ru', 'pl', 'uk'] as const)
    .filter((language) => filled(draft[`description_${language}` as keyof DraftForm])).length;
}

function seoPageStats(draft: DraftForm | null, states: IngredientState[]) {
  const languageCount = 4;
  const maxStateCount = 10;
  const stateCount = states.length;
  const completeStates = states.filter((state) => (
    filled(state.notes_en || state.notes_ru || state.notes_pl || state.notes_uk)
    && filled(state.calories_per_100g)
    && filled(state.storage_temp_c)
    && filled(state.shelf_life_hours)
  )).length;
  const filledDescriptions = localizedDescriptionCount(draft);
  const productPages = languageCount;
  const statePages = stateCount * languageCount;
  const possiblePages = productPages + statePages;
  const maxPages = productPages + (maxStateCount * languageCount);

  return {
    languageCount,
    stateCount,
    completeStates,
    filledDescriptions,
    productPages,
    statePages,
    possiblePages,
    maxPages,
  };
}

function productImageStats(draft: DraftForm | null, states: IngredientState[]) {
  const productImages = filled(draft?.image_url) ? 1 : 0;
  const stateImages = states.filter((state) => filled(state.image_url)).length;
  const stateSlots = Math.max(states.length, 10);

  return {
    productImages,
    stateImages,
    totalImages: productImages + stateImages,
    totalSlots: 1 + stateSlots,
    stateSlots,
  };
}

export function MenuPage({ products, categories, loading, error, onReload }: MenuPageProps) {
  const [query, setQuery] = useState('');
  const [lang, setLang] = useState<CatalogLang>('ru');
  const [aiInput, setAiInput] = useState('');
  const [aiBusy, setAiBusy] = useState(false);
  const [saveBusy, setSaveBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [imageError, setImageError] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiSuccess, setAiSuccess] = useState<string | null>(null);
  const [draftResponse, setDraftResponse] = useState<AiCreateDraftResponse | null>(null);
  const [draft, setDraft] = useState<DraftForm | null>(null);
  const [editingProduct, setEditingProduct] = useState<AdminProduct | null>(null);
  const [profile, setProfile] = useState<AiExtendedProductProfile>({});
  const [productStates, setProductStates] = useState<IngredientState[]>([]);
  const [quality, setQuality] = useState<ProductDataQuality | null>(null);
  const [statesBusy, setStatesBusy] = useState(false);
  const [stateImageBusy, setStateImageBusy] = useState<string | null>(null);
  const [draftTab, setDraftTab] = useState<DraftTab>('basic');
  const [draftLanguage, setDraftLanguage] = useState<CatalogLang>('ru');

  async function togglePublish(product: AdminProduct) {
    setAiError(null);
    try {
      const updated = product.is_published
        ? await unpublishAdminProduct(product.id)
        : await publishAdminProduct(product.id);
      await revalidateSite({ type: 'ingredient', slug: updated.slug || product.slug });
      setAiSuccess(product.is_published ? 'Ингредиент снят с сайта' : 'Ингредиент опубликован на сайте');
      await onReload();
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось изменить публикацию');
    }
  }

  async function removeProduct(product: AdminProduct) {
    if (!window.confirm(`Удалить ингредиент «${product.name_ru || product.name_en}»?`)) return;
    try {
      await deleteAdminProduct(product.id);
      await revalidateSite({ type: 'ingredient', slug: product.slug });
      setAiSuccess('Ингредиент удалён');
      await onReload();
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось удалить ингредиент');
    }
  }
  const categoryMap = useMemo(() => new Map(categories.map((category) => [category.id, category])), [categories]);

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    if (!term) return products;
    return products.filter((product) => [product.slug, product.name_ru, product.name_en, product.name_pl, product.name_uk]
      .some((value) => String(value || '').toLowerCase().includes(term)));
  }, [products, query]);

  function setText(field: keyof DraftForm, value: string) {
    setDraft((current) => current ? { ...current, [field]: value } : current);
  }

  function setNumber(field: keyof DraftForm, value: string) {
    setDraft((current) => current ? { ...current, [field]: value === '' ? undefined : Number(value) } : current);
  }

  function toggleSeason(season: string) {
    if (!draft) return;
    const current = draft.seasons || [];
    const seasons = season === 'AllYear'
      ? (current.includes('AllYear') ? [] : ['AllYear'])
      : current.includes(season)
        ? current.filter((item) => item !== season)
        : [...current.filter((item) => item !== 'AllYear'), season];
    setDraft({ ...draft, seasons });
  }

  function setProfileValue(section: keyof AiExtendedProductProfile, field: string, value: string | number | boolean | null | string[]) {
    setProfile((current) => ({ ...current, [section]: { ...(current[section] || {}), [field]: value } }));
  }

  function profileValue(section: keyof AiExtendedProductProfile, field: string): string | number | boolean | null | string[] | unknown[] | undefined {
    return profile[section]?.[field];
  }

  const completion = useMemo(() => completionScore(draft, productStates), [draft, productStates]);
  const seoStats = useMemo(() => seoPageStats(draft, productStates), [draft, productStates]);
  const imageStats = useMemo(() => productImageStats(draft, productStates), [draft, productStates]);

  async function loadProductExtras(productId: string) {
    setStatesBusy(true);
    try {
      const [nextStates, nextQuality] = await Promise.all([
        listProductStates(productId).catch(() => []),
        getProductDataQuality(productId).catch(() => null)
      ]);
      setProductStates(nextStates);
      setQuality(nextQuality);
    } finally {
      setStatesBusy(false);
    }
  }

  function startEditProduct(product: AdminProduct) {
    setDraft(productToForm(product));
    setEditingProduct(product);
    setDraftResponse(null);
    setProfile({});
    setProductStates([]);
    setQuality(null);
    setDraftTab('basic');
    setDraftLanguage('ru');
    setAiError(null);
    setAiSuccess(null);
    window.scrollTo({ top: 0, behavior: 'smooth' });
    void loadProductExtras(product.id);
  }

  function closeDraft() {
    setDraft(null);
    setDraftResponse(null);
    setEditingProduct(null);
    setProfile({});
    setProductStates([]);
    setQuality(null);
  }

  async function regenerateDraftData() {
    const input = [draft?.name_ru, draft?.name_en, draft?.description_ru, draft?.description_en].filter(Boolean).join('. ');
    if (!input.trim()) return;
    setAiBusy(true);
    setAiError(null);
    try {
      const response = await aiCreateProductDraft(input);
      const generated = draftToForm(response);
      const nextDraft = editingProduct && draft
        ? {
          ...generated,
          name_en: draft.name_en,
          name_ru: draft.name_ru,
          name_pl: draft.name_pl,
          name_uk: draft.name_uk,
          product_type: draft.product_type,
          unit: draft.unit,
          image_url: draft.image_url
        }
        : { ...generated, image_url: draft?.image_url };
      setDraftResponse(response);
      setDraft(nextDraft);
      setProfile(response.draft.extended || {});
      setAiSuccess('AI-данные перегенерированы. Проверьте и сохраните изменения.');
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось перегенерировать AI-данные');
    } finally {
      setAiBusy(false);
    }
  }

  async function regenerateStates() {
    if (!editingProduct) return;
    if (!window.confirm('Перегенерировать состояния обработки? Старые состояния будут удалены и созданы заново.')) return;
    setStatesBusy(true);
    setAiError(null);
    try {
      await deleteProductStates(editingProduct.id);
      await generateProductStates(editingProduct.id);
      await loadProductExtras(editingProduct.id);
      await revalidateSite({ type: 'ingredient', slug: editingProduct.slug });
      setAiSuccess('Состояния обработки перегенерированы');
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось перегенерировать состояния');
    } finally {
      setStatesBusy(false);
    }
  }

  async function updateStateField(stateName: string, field: keyof IngredientState, value: string | number | null) {
    if (!editingProduct) return;
    setProductStates((current) => current.map((state) => state.state === stateName ? { ...state, [field]: value } : state));
    try {
      const updated = await updateProductState(editingProduct.id, stateName, { [field]: value });
      setProductStates((current) => current.map((state) => state.state === stateName ? updated : state));
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось обновить состояние');
    }
  }

  async function generateStateImage(state: IngredientState) {
    if (!editingProduct || !draft) return;
    const label = STATE_LABELS[state.state] || state.state;
    const name = `${draft.name_en || draft.name_ru || editingProduct.slug} ${state.state}`;
    const description = `${label}. ${state.notes_en || state.notes_ru || draft.description_en || ''}`;
    setStateImageBusy(state.state);
    try {
      const result = await aiGenerateProductImage(name, description, true);
      const updated = await updateProductState(editingProduct.id, state.state, { image_url: result.image_url });
      setProductStates((current) => current.map((item) => item.state === state.state ? updated : item));
      await revalidateSite({ type: 'ingredient', slug: editingProduct.slug });
      setAiSuccess(`Фото состояния «${label}» обновлено`);
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось сгенерировать фото состояния');
    } finally {
      setStateImageBusy(null);
    }
  }

  async function generateDraft() {
    const input = aiInput.trim();
    if (!input) {
      setAiError('Введите название продукта или короткое описание.');
      return;
    }
    setAiBusy(true);
    setAiError(null);
    setAiSuccess(null);
    try {
      const response = await aiCreateProductDraft(input);
      setDraftResponse(response);
      setEditingProduct(null);
      const nextDraft = draftToForm(response);
      setDraft(nextDraft);
      setProfile(response.draft.extended || {});
      setDraftTab('basic');
      setDraftLanguage('ru');
      void generateImage(response, nextDraft);
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось получить ответ Gemini');
    } finally {
      setAiBusy(false);
    }
  }

  async function generateImage(response = draftResponse, currentDraft = draft, force = false) {
    if (!currentDraft) return;
    const name = currentDraft.name_en || currentDraft.name_ru || response?.raw_input || editingProduct?.slug || 'product';
    setImageBusy(true);
    setImageError(null);
    try {
      const result = await aiGenerateProductImage(String(name), currentDraft.description_en, force);
      setDraft((value) => value ? { ...value, image_url: result.image_url } : value);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Не удалось сгенерировать изображение';
      setImageError(message === 'HTTP 404'
        ? 'Генерация фото еще не развернута на Koyeb. Задеплойте обновленный backend и повторите.'
        : message);
    } finally {
      setImageBusy(false);
    }
  }

  async function saveDraftProduct() {
    if (!draft) return;
    const nameError = validateProductNamesForSave(draft);
    if (nameError) {
      setAiError(nameError);
      return;
    }
    setSaveBusy(true);
    setAiError(null);
    setAiSuccess(null);
    try {
      const product = editingProduct
        ? await updateAdminProduct(editingProduct.id, { ...draft, auto_translate: false })
        : await createAdminProduct({ ...draft, name_input: draftResponse?.raw_input || draft.name_en || draft.name_ru || 'product', auto_translate: false });
      if (!editingProduct) {
        await saveExtendedProductProfile(product.id, {
          ...profile,
          macros: {
            ...(profile.macros || {}),
            calories_kcal: draft.calories_per_100g ?? null,
            protein_g: draft.protein_per_100g ?? null,
            fat_g: draft.fat_per_100g ?? null,
            carbs_g: draft.carbs_per_100g ?? null,
            fiber_g: draft.fiber_per_100g ?? null,
            sugar_g: draft.sugar_per_100g ?? null
          }
        });
      }
      await revalidateSite({ type: 'ingredient', slug: product.slug || editingProduct?.slug });
      setAiSuccess(editingProduct ? `«${product.name_ru || product.name_en}» обновлён` : `«${product.name_ru || product.name_en}» сохранен в базе`);
      closeDraft();
      setAiInput('');
      await onReload();
    } catch (err) {
      setAiError(err instanceof Error ? err.message : 'Не удалось сохранить продукт в базе');
    } finally {
      setSaveBusy(false);
    }
  }

  return (
    <section className="menu-page">
      <header className="catalog-hero">
        <div>
          <span className="eyebrow">Global food intelligence</span>
          <h2>Каталог продуктов</h2>
          <p>{products.length} продуктов в единой базе</p>
        </div>
        <button className="btn btn-quiet" onClick={() => void onReload()} disabled={loading}>Обновить данные</button>
      </header>

      <section className="ai-studio">
        <div className="ai-studio-copy">
          <span className="ai-orb">AI</span>
          <div>
            <span className="eyebrow">Gemini 3.1 Pro</span>
            <h3>Создать новый продукт</h3>
            <p>Опишите ингредиент свободным текстом. Gemini заполнит переводы, nutrition, SEO и параметры хранения.</p>
          </div>
        </div>
        <div className="ai-command">
          <textarea value={aiInput} onChange={(event) => setAiInput(event.target.value)}
            placeholder="Например: свежий атлантический лосось, филе без кожи для ресторанного меню"
            onKeyDown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') void generateDraft(); }} />
          <button className="btn btn-ai" onClick={() => void generateDraft()} disabled={aiBusy || saveBusy}>
            {aiBusy ? 'Gemini анализирует...' : 'Создать AI-черновик'}
          </button>
        </div>
        {aiError && <p className="form-error notice">{aiError}</p>}
        {aiSuccess && <p className="form-success notice">{aiSuccess}</p>}
      </section>

      {draft && (
        <section className="draft-review">
          <div className="draft-review-head">
            <div className="draft-title">
              <span className="draft-title-icon">{editingProduct ? '✎' : '✓'}</span>
              <div>
              <span className="eyebrow">{editingProduct ? 'Edit product' : 'Review before database'}</span>
              <h3>{editingProduct ? 'Редактирование продукта' : 'Проверьте черновик'}</h3>
                <p>{editingProduct ? 'Измените данные продукта и сохраните обновление.' : 'Gemini подготовил данные. Изменения сохранятся только после подтверждения.'}</p>
              </div>
            </div>
            <div className="draft-meta">
              {draftResponse && (draftResponse.draft.quality_warnings.length > 0 || draftResponse.corrections.length > 0) && (
                <span className="draft-warning-tip" tabIndex={0} aria-label="Проверить данные перед сохранением">
                  <b>!</b>
                  <span className="draft-warning-popover">
                    <strong>Проверьте данные перед сохранением</strong>
                    {[...new Set(draftResponse.draft.quality_warnings.map((warning) => warning.message))]
                      .map((message) => <small key={message}>{message}</small>)}
                    {draftResponse.corrections.length > 0 && <small>{draftResponse.corrections.length} автоматических исправлений</small>}
                  </span>
                </span>
              )}
              {draftResponse ? <span className="confidence">{Math.round(draftResponse.draft.confidence * 100)}% уверенность</span> : <span className="confidence">Ручное редактирование</span>}
              <span className={`completion-pill ${completion.percent >= 85 ? 'good' : completion.percent >= 60 ? 'warn' : 'bad'}`}>{completion.percent}% заполнено</span>
              <span className={`image-count-pill ${imageStats.totalImages === imageStats.totalSlots ? 'good' : imageStats.totalImages > 0 ? 'warn' : 'bad'}`} title={`Продукт: ${imageStats.productImages}/1 · состояния: ${imageStats.stateImages}/${imageStats.stateSlots}`}>
                Фото {imageStats.totalImages}/{imageStats.totalSlots}
              </span>
              {quality && <span>{Math.round(quality.weighted_score || quality.score || 0)}% backend</span>}
              <span>{draftResponse ? draftResponse.cached ? 'Из кеша' : draftResponse.model : editingProduct?.slug || 'existing product'}</span>
            </div>
          </div>

          <div className="draft-quick-actions">
            <button className="btn btn-quiet" type="button" onClick={() => void regenerateDraftData()} disabled={aiBusy || saveBusy}>{aiBusy ? 'Gemini пишет...' : 'Перегенерировать AI-данные'}</button>
            {editingProduct && <button className="btn btn-quiet" type="button" onClick={() => void regenerateStates()} disabled={statesBusy}>{statesBusy ? 'Генерируем состояния...' : 'Перегенерировать состояния'}</button>}
          </div>

          <nav className="draft-tabs" aria-label="Разделы AI-черновика">
            {TABS.map(([value, label, hint]) => <button type="button" key={value} className={draftTab === value ? 'active' : ''} onClick={() => setDraftTab(value)}>
              <strong>{label}</strong><span>{hint}</span>
            </button>)}
          </nav>

          <div className="draft-body">
          {draftTab === 'basic' && <section className="draft-section">
            <div className="draft-section-head"><span>01</span><div><h4>Основные данные</h4><p>Названия, классификация, фото и сезонность</p></div></div>
            <div className="draft-grid four">
              {(['name_en', 'name_ru', 'name_pl', 'name_uk'] as const).map((field) => (
                <label className="field" key={field}><span className="field-label">Название · {field.replace('name_', '').toUpperCase()}</span>
                  <input value={String(draft[field] || '')} onChange={(event) => setText(field, event.target.value)} /></label>
              ))}
              <label className="field span-2"><span className="field-label">Тип продукта</span>
                <select className={draft.product_type === 'other' || !draft.product_type ? 'field-invalid' : ''} value={draft.product_type || ''} onChange={(event) => setText('product_type', event.target.value)}>
                  <option value="">Выберите тип продукта</option>
                  {PRODUCT_TYPES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                </select>
                {(draft.product_type === 'other' || !draft.product_type) && <span className="field-hint danger">Нужно выбрать перед сохранением</span>}
              </label>
              <label className="field"><span className="field-label">Единица</span>
                <select value={draft.unit} onChange={(event) => setText('unit', event.target.value)}>
                  {UNIT_OPTIONS.map((unit) => <option key={unit} value={unit}>{unit}</option>)}
                </select></label>
            </div>
            <div className={`product-image-studio unified${imageBusy ? ' loading' : ''}`}>
              <div className="product-image-gallery">
                <AiPhotoGallery heading="Визуал продукта" subtitle="1 фото · белый каталожный фон" actions={<button className="btn btn-quiet" type="button" onClick={() => void generateImage(draftResponse, draft, true)} disabled={imageBusy}>{imageBusy ? 'Генерируем...' : 'Перегенерировать фото'}</button>} images={draft.image_url ? [draft.image_url] : []} selectedIndex={0} title={String(draft.name_ru || draft.name_en || 'Фото продукта')} emptyText="Фото продукта не создано" busyText="Gemini создаёт фото продукта" busy={imageBusy} maxImages={1} onSelect={() => undefined} onAdd={() => void generateImage(draftResponse, draft, true)} onRemove={() => setDraft((current) => current ? { ...current, image_url: undefined } : current)} />
              </div>
              <div className="product-image-info">
                <span className="eyebrow">Gemini 3.1 Flash Image</span>
                <h4>Визуал продукта</h4>
                <p>Фото генерируется на бэкенде, сохраняется в R2 и будет привязано к продукту после подтверждения. Продукт можно сохранить и без фото.</p>
                {imageError && <span className="field-hint danger">{imageError}</span>}
              </div>
            </div>
            <div className="season-field">
              <span className="field-label">Сезонность</span>
              <div className="season-options">
                {SEASONS.map(([value, label]) => <button type="button" key={value} className={(draft.seasons || []).includes(value) ? 'active' : ''} onClick={() => toggleSeason(value)}>{label}</button>)}
              </div>
            </div>
          </section>}

          {draftTab === 'nutrition' && <section className="draft-section">
            <div className="draft-section-head"><span>02</span><div><h4>Nutrition и хранение</h4><p>Значения указаны на 100 грамм продукта</p></div></div>
            <div className="nutrition-grid">
              {NUTRITION_FIELDS.map(([field, label, suffix]) => (
                <label className="nutrition-field" key={field}><span>{label}</span>
                  <div><input type="number" step="any" value={draft[field] as number ?? ''} onChange={(event) => setNumber(field, event.target.value)} /><em>{suffix}</em></div></label>
              ))}
            </div>
            <div className="subsection-title"><h5>Расширенные макросы и жирные кислоты</h5><span>на 100 г</span></div>
            <div className="nutrition-grid">
              {[['macros', 'starch_g', 'Крахмал'], ['macros', 'water_g', 'Вода'], ['macros', 'alcohol_g', 'Алкоголь'], ['fatty_acids', 'saturated_fat', 'Насыщенные жиры'], ['fatty_acids', 'monounsaturated_fat', 'Мононенасыщенные'], ['fatty_acids', 'polyunsaturated_fat', 'Полиненасыщенные'], ['fatty_acids', 'omega3', 'Omega-3'], ['fatty_acids', 'omega6', 'Omega-6'], ['fatty_acids', 'epa', 'EPA'], ['fatty_acids', 'dha', 'DHA']].map(([section, field, label]) => (
                <ProfileNumber key={field} label={label} value={profileValue(section as keyof AiExtendedProductProfile, field)} onChange={(value) => setProfileValue(section as keyof AiExtendedProductProfile, field, value)} suffix="г" />
              ))}
            </div>
            <div className="subsection-title"><h5>Профиль сахаров</h5><span>на 100 г</span></div>
            <div className="nutrition-grid">
              {[['glucose', 'Глюкоза'], ['fructose', 'Фруктоза'], ['sucrose', 'Сахароза'], ['lactose', 'Лактоза'], ['maltose', 'Мальтоза'], ['total_sugars', 'Всего сахаров'], ['added_sugars', 'Добавленные'], ['sugar_alcohols', 'Сахарные спирты'], ['sweetness_perception', 'Ощущение сладости']].map(([field, label]) => (
                <ProfileNumber key={field} label={label} value={profileValue('sugar_profile', field)} onChange={(value) => setProfileValue('sugar_profile', field, value)} suffix={field === 'sweetness_perception' ? '/10' : 'г'} />
              ))}
            </div>
          </section>}

          {draftTab === 'micros' && <section className="draft-section">
            <div className="draft-section-head"><span>03</span><div><h4>Витамины и минералы</h4><p>Gemini заполняет значения в mg на 100 грамм</p></div></div>
            <div className="subsection-title"><h5>Витамины</h5><span>{VITAMINS.length} показателей</span></div>
            <div className="nutrition-grid">{VITAMINS.map(([field, label, unit]) => <ProfileNumber key={field} label={label} value={profileValue('vitamins', field)} onChange={(value) => setProfileValue('vitamins', field, value)} suffix={unit} />)}</div>
            <div className="subsection-title"><h5>Минералы</h5><span>{MINERALS.length} показателей</span></div>
            <div className="nutrition-grid">{MINERALS.map(([field, label, unit]) => <ProfileNumber key={field} label={label} value={profileValue('minerals', field)} onChange={(value) => setProfileValue('minerals', field, value)} suffix={unit} />)}</div>
          </section>}

          {draftTab === 'culinary' && <section className="draft-section">
            <div className="draft-section-head"><span>04</span><div><h4>Кулинарный профиль</h4><p>Вкус, физические свойства и поведение при обработке</p></div></div>
            <div className="nutrition-grid">
              {[['sweetness', 'Сладость'], ['acidity', 'Кислотность'], ['bitterness', 'Горечь'], ['umami', 'Умами'], ['aroma', 'Аромат']].map(([field, label]) => <ProfileNumber key={field} label={label} value={profileValue('culinary', field)} onChange={(value) => setProfileValue('culinary', field, value)} suffix="/10" />)}
              {[['glycemic_index', 'Гликемический индекс'], ['glycemic_load', 'Гликемическая нагрузка'], ['ph', 'pH'], ['smoke_point', 'Температура дымления'], ['water_activity', 'Активность воды']].map(([field, label]) => <ProfileNumber key={field} label={label} value={profileValue('food_properties', field)} onChange={(value) => setProfileValue('food_properties', field, value)} suffix={field === 'smoke_point' ? '°C' : ''} />)}
            </div>
            <label className="field"><span className="field-label">Текстура</span><input value={String(profileValue('culinary', 'texture') || '')} onChange={(event) => setProfileValue('culinary', 'texture', event.target.value)} /></label>
            <div className="draft-grid two">
              {(['en', 'ru', 'pl', 'uk'] as const).map((language) => <label className="field editorial-field compact" key={language}><span className="field-label">Эффекты обработки · {language.toUpperCase()}</span><textarea value={String(profileValue('processing_effects', `processing_notes_${language}`) || '')} onChange={(event) => setProfileValue('processing_effects', `processing_notes_${language}`, event.target.value)} /></label>)}
            </div>
            <p className="data-note">Кулинарное поведение: {Array.isArray(profileValue('culinary_behavior', 'behaviors')) ? (profileValue('culinary_behavior', 'behaviors') as unknown[]).length : 0} сценариев. Они сохранятся вместе с продуктом.</p>
            <div className="processing-states-editor">
              <div className="subsection-title"><h5>Состояния обработки</h5><span>{statesBusy ? 'загрузка...' : `${productStates.length}/10 состояний`}</span></div>
              {editingProduct && productStates.length === 0 && <div className="state-empty"><p>Состояния ещё не созданы для этого продукта.</p><button className="btn btn-primary" type="button" onClick={() => void regenerateStates()} disabled={statesBusy}>{statesBusy ? 'Генерируем...' : 'Создать состояния'}</button></div>}
              <div className="state-editor-grid">
                {productStates.map((state) => {
                  const label = STATE_LABELS[state.state] || state.state;
                  return <article className="state-editor-card" key={state.id}>
                    <div className="state-editor-image">
                      {state.image_url ? <img src={state.image_url} alt={label} /> : <span>{label}</span>}
                      <button type="button" className="btn btn-quiet" onClick={() => void generateStateImage(state)} disabled={stateImageBusy === state.state}>{stateImageBusy === state.state ? 'Фото...' : state.image_url ? 'Заменить фото' : 'Добавить фото'}</button>
                    </div>
                    <div className="state-editor-content">
                      <div className="state-editor-title"><strong>{label}</strong><small>{state.state}</small></div>
                      <div className="state-editor-numbers">
                        <label><span>kcal</span><input type="number" step="any" value={state.calories_per_100g ?? ''} onChange={(event) => void updateStateField(state.state, 'calories_per_100g', event.target.value === '' ? null : Number(event.target.value))} /></label>
                        <label><span>°C</span><input type="number" value={state.storage_temp_c ?? ''} onChange={(event) => void updateStateField(state.state, 'storage_temp_c', event.target.value === '' ? null : Number(event.target.value))} /></label>
                        <label><span>ч</span><input type="number" value={state.shelf_life_hours ?? ''} onChange={(event) => void updateStateField(state.state, 'shelf_life_hours', event.target.value === '' ? null : Number(event.target.value))} /></label>
                      </div>
                      <label className="field compact"><span className="field-label">Название RU</span><input value={state.name_suffix_ru || ''} onChange={(event) => void updateStateField(state.state, 'name_suffix_ru', event.target.value)} /></label>
                      <label className="field editorial-field compact"><span className="field-label">Описание RU</span><textarea value={state.notes_ru || ''} onChange={(event) => void updateStateField(state.state, 'notes_ru', event.target.value)} /></label>
                    </div>
                  </article>;
                })}
              </div>
            </div>
          </section>}

          {draftTab === 'health' && <section className="draft-section">
            <div className="draft-section-head"><span>05</span><div><h4>Здоровье и ограничения</h4><p>Диетические признаки, аллергены и биоактивные соединения</p></div></div>
            <div className="subsection-title"><h5>Подходит для диет</h5><span>проверьте вывод Gemini</span></div>
            <div className="toggle-grid">{DIET_FLAGS.map(([field, label]) => <ProfileToggle key={field} label={label} checked={Boolean(profileValue('diet_flags', field))} onChange={(value) => setProfileValue('diet_flags', field, value)} />)}</div>
            <div className="subsection-title"><h5>Аллергены EU-14</h5><span>включите только присутствующие</span></div>
            <div className="toggle-grid danger">{ALLERGENS.map(([field, label]) => <ProfileToggle key={field} label={label} checked={Boolean(profileValue('allergens', field))} onChange={(value) => setProfileValue('allergens', field, value)} />)}</div>
            <LanguageTabs value={draftLanguage} onChange={setDraftLanguage} />
            <div className="draft-grid two">
              {(['bioactive_compounds', 'health_effects', 'contraindications'] as const).map((group) => {
                const field = `${group}_${draftLanguage}`;
                return <label className="field" key={field}><span className="field-label">{group.replace(/_/g, ' ')} · {draftLanguage.toUpperCase()}</span><textarea value={arrayText(profileValue('health_profile', field))} onChange={(event) => setProfileValue('health_profile', field, textArray(event.target.value))} /></label>;
              })}
              <label className="field"><span className="field-label">Заметки об усвоении · {draftLanguage.toUpperCase()}</span><textarea value={String(profileValue('health_profile', `absorption_notes_${draftLanguage}`) || '')} onChange={(event) => setProfileValue('health_profile', `absorption_notes_${draftLanguage}`, event.target.value)} /></label>
            </div>
          </section>}

          {draftTab === 'content' && <section className="draft-section">
            <div className="draft-section-head"><span>06</span><div><h4>Контент и SEO</h4><p>Полные тексты на четырех языках и поисковая выдача</p></div></div>
            <div className="seo-page-summary">
              <div className="seo-page-summary-main">
                <span>SEO-страниц из одного продукта</span>
                <strong>{seoStats.possiblePages}</strong>
                <small>максимум {seoStats.maxPages}, если созданы все 10 состояний</small>
              </div>
              <div className="seo-page-formula">
                <span><b>{seoStats.productPages}</b> страницы продукта</span>
                <span><b>{seoStats.statePages}</b> страницы состояний</span>
                <span><b>{seoStats.filledDescriptions}/{seoStats.languageCount}</b> языков заполнено</span>
                <span><b>{seoStats.completeStates}/{seoStats.stateCount || 10}</b> состояний готово</span>
              </div>
            </div>
            <div className="description-editor">
              <div className="description-editor-head">
                <div><strong>Описание продукта</strong><span>Переключайтесь между языками, изменения сохраняются в черновике</span></div>
                <LanguageTabs value={draftLanguage} onChange={setDraftLanguage} />
              </div>
              {(() => {
                const field = `description_${draftLanguage}` as keyof DraftForm;
                return <label className="field editorial-field description-main"><span className="field-label">Полное описание · {draftLanguage.toUpperCase()}</span><textarea value={String(draft[field] || '')} onChange={(event) => setText(field, event.target.value)} placeholder={`Описание продукта на ${draftLanguage.toUpperCase()}`} /><span className="character-count">{String(draft[field] || '').length} символов</span></label>;
              })()}
            </div>
            <div className="seo-global-note">
              <strong>SEO meta сейчас глобальная для продукта.</strong>
              <span>Языковые табы переключают описания страниц. Для отдельных SEO title/H1/description на RU, EN, PL и UK нужно добавить языковые SEO-поля в backend-схему продукта.</span>
            </div>
            <div className="draft-grid two">
              <label className="field"><span className="field-label">SEO title · GLOBAL</span><input value={draft.seo_title || ''} onChange={(event) => setText('seo_title', event.target.value)} /></label>
              <label className="field"><span className="field-label">SEO H1 · GLOBAL</span><input value={draft.seo_h1 || ''} onChange={(event) => setText('seo_h1', event.target.value)} /></label>
              <label className="field span-2 editorial-field compact"><span className="field-label">SEO description · GLOBAL</span><textarea value={draft.seo_description || ''} onChange={(event) => setText('seo_description', event.target.value)} /><span className="character-count">{String(draft.seo_description || '').length} / 160</span></label>
            </div>
          </section>}
          </div>

          <div className="draft-actions">
            <div><strong>{draft.name_ru || draft.name_en}</strong><span>{editingProduct ? 'Изменения обновят существующий продукт' : 'После сохранения продукт появится в глобальном каталоге'}</span></div>
            <div className="draft-action-buttons"><button className="btn btn-quiet" onClick={closeDraft}>Отменить</button>
            <button className="btn btn-primary" onClick={() => void saveDraftProduct()} disabled={saveBusy || imageBusy || draft.product_type === 'other' || !draft.product_type}>
              {saveBusy ? 'Сохраняем...' : imageBusy ? 'Ожидаем фото...' : editingProduct ? 'Сохранить изменения' : 'Подтвердить и сохранить'}
            </button></div>
          </div>
        </section>
      )}

      <section className="catalog-list page-card">
        <div className="catalog-toolbar">
          <label className="search-box"><span>⌕</span><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Поиск по названию или slug" /></label>
          <select value={lang} onChange={(event) => setLang(event.target.value as CatalogLang)}>
            <option value="ru">Русский</option><option value="en">English</option><option value="pl">Polski</option><option value="uk">Українська</option>
          </select>
          <span className="result-count">{filtered.length} найдено</span>
        </div>
        {loading && <p className="page-muted">Загружаем каталог...</p>}
        {error && <p className="form-error">{error}</p>}
        {!loading && !error && (
          <div className="orders-table-wrap"><table className="orders-table ingredient-table">
            <thead><tr><th>Ингредиент</th><th>Категория</th><th>Ед.</th><th>kcal</th><th>Б</th><th>Ж</th><th>У</th><th>Статус</th><th>Управление</th></tr></thead>
            <tbody>{filtered.map((product) => {
              const category = categoryMap.get(product.category_id);
              return <tr key={product.id}><td><div className="ingredient-cell-main"><strong>{productName(product, lang)}</strong><span>{product.slug || product.id}</span></div></td>
                <td>{category ? category[`name_${lang}`] || category.name_en : '-'}</td><td>{product.unit}</td>
                <td>{formatMacro(product.calories_per_100g)}</td><td>{formatMacro(product.protein_per_100g)}</td><td>{formatMacro(product.fat_per_100g)}</td><td>{formatMacro(product.carbs_per_100g)}</td>
                <td><span className={`status-badge status-badge-${product.is_published ? 'ok' : 'neutral'}`}>{product.is_published ? 'Опубликован' : 'Черновик'}</span></td><td><div className="table-actions"><button className="btn btn-quiet" onClick={() => startEditProduct(product)}>Редактировать</button><button className="btn btn-quiet" onClick={() => void togglePublish(product)}>{product.is_published ? 'Снять' : 'Опубликовать'}</button><button className="btn btn-danger" onClick={() => void removeProduct(product)}>Удалить</button></div></td></tr>;
            })}</tbody>
          </table></div>
        )}
      </section>
    </section>
  );
}
