import { useMemo, useState } from 'react';
import { aiCreateShopProductDraft, createShopProduct, deleteShopProduct, generateShopProductImage, updateShopProductStatus, uploadShopReference } from '../../api/shop';
import { revalidateSite } from '../../api/revalidate';
import { AiPhotoGallery } from '../../components/AiPhotoGallery';
import { AiReferenceUpload, type AiReferenceImage } from '../../components/AiReferenceUpload';
import type { ShopProduct, ShopProductDraft } from '../../types/admin';

type ShopLang = 'ru' | 'en' | 'pl' | 'uk';

interface ShopPageProps {
  products: ShopProduct[];
  loading: boolean;
  error: string | null;
  onReload: () => Promise<void>;
}

const IMAGE_COUNTS = [1, 3, 4, 6];
const CATEGORIES = [
  ['delivery-food', 'Готовая еда'], ['kitchen-tools', 'Кухонные инструменты'],
  ['tableware', 'Посуда'], ['ingredients', 'Продукты'], ['beverages', 'Напитки'], ['other', 'Другое']
] as const;

export function ShopPage({ products, loading, error, onReload }: ShopPageProps) {
  const [query, setQuery] = useState('');
  const [input, setInput] = useState('');
  const [imageCount, setImageCount] = useState(4);
  const [draft, setDraft] = useState<ShopProductDraft | null>(null);
  const [language, setLanguage] = useState<ShopLang>('ru');
  const [images, setImages] = useState<string[]>([]);
  const [selectedImage, setSelectedImage] = useState(0);
  const [references, setReferences] = useState<AiReferenceImage[]>([]);
  const [commercial, setCommercial] = useState({ sku: '', price: '', currency: 'PLN', stock: '0', status: 'draft' as ShopProduct['status'] });
  const [aiBusy, setAiBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [singleImageBusy, setSingleImageBusy] = useState(false);
  const [saveBusy, setSaveBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  async function changeStatus(product: ShopProduct, status: ShopProduct['status']) {
    try {
      const updated = await updateShopProductStatus(product.id, status);
      await revalidateSite({ type: 'shop', slug: updated.slug || product.slug });
      setMessage(status === 'active' ? 'Товар опубликован на сайте' : 'Статус товара обновлён');
      await onReload();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось изменить статус');
    }
  }

  async function removeProduct(product: ShopProduct) {
    if (!window.confirm(`Удалить товар «${product.name_ru || product.name_en}»?`)) return;
    try {
      await deleteShopProduct(product.id);
      await revalidateSite({ type: 'shop', slug: product.slug });
      setMessage('Товар удалён');
      await onReload();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось удалить товар');
    }
  }

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    if (!term) return products;
    return products.filter((product) => [product.sku, product.slug, product.name_ru, product.name_en].some((value) => String(value || '').toLowerCase().includes(term)));
  }, [products, query]);

  function updateDraft(field: keyof ShopProductDraft, value: string | string[]) {
    setDraft((current) => current ? { ...current, [field]: value } : current);
  }

  async function generateImages(nextDraft: ShopProductDraft) {
    setImageBusy(true);
    setImages([]);
    try {
      const next: string[] = [];
      for (let index = 0; index < imageCount; index += 1) {
        const result = await generateShopProductImage(nextDraft.name_en, nextDraft.image_prompts[index], index, references.map((item) => item.url), false, { photoScenarios: ['white-background'], scaleReference: 'none' });
        next.push(result.image_url);
        setImages([...next]);
      }
      setSelectedImage(0);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать изображения товара');
    } finally {
      setImageBusy(false);
    }
  }

  async function generateAdditionalImage() {
    if (!draft || images.length >= 12) return;
    const index = images.length;
    setSingleImageBusy(true);
    setLocalError(null);
    try {
      const result = await generateShopProductImage(
        draft.name_en,
        draft.image_prompts[index],
        index,
        references.map((item) => item.url),
        false,
        { photoScenarios: ['white-background'], scaleReference: 'none' }
      );
      setImages((current) => [...current, result.image_url]);
      setImageCount((current) => Math.max(current, index + 1));
      setSelectedImage(index);
      setMessage(`Добавлено фото ${index + 1}`);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось добавить фото');
    } finally {
      setSingleImageBusy(false);
    }
  }

  function removeShopImage(index: number) {
    setImages((current) => {
      const next = current.filter((_, itemIndex) => itemIndex !== index);
      setImageCount(Math.max(1, next.length));
      setSelectedImage((selected) => {
        if (next.length === 0) return 0;
        if (selected > index) return selected - 1;
        if (selected === index) return Math.min(index, next.length - 1);
        return selected;
      });
      return next;
    });
  }

  async function generateDraft() {
    if (!input.trim()) return setLocalError('Опишите конкретный товар для магазина.');
    setAiBusy(true); setLocalError(null); setMessage(null);
    try {
      const next = await aiCreateShopProductDraft(input.trim(), imageCount);
      setDraft(next);
      setLanguage('ru');
      void generateImages(next);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать карточку товара');
    } finally {
      setAiBusy(false);
    }
  }

  async function addReferences(files: FileList | null) {
    if (!files) return;
    try {
      const next: AiReferenceImage[] = [];
      for (const file of Array.from(files).slice(0, 2 - references.length)) {
        const url = await uploadShopReference(file);
        next.push({ url, preview: URL.createObjectURL(file), name: file.name });
      }
      setReferences((current) => [...current, ...next].slice(0, 2));
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось загрузить фото товара');
    }
  }

  function removeReference(index: number) {
    setReferences((current) => current.filter((item, itemIndex) => {
      if (itemIndex === index) URL.revokeObjectURL(item.preview);
      return itemIndex !== index;
    }));
  }

  async function save() {
    if (!draft) return;
    setSaveBusy(true); setLocalError(null);
    try {
      const product = await createShopProduct({
        ...draft,
        sku: commercial.sku.trim() || null,
        image_urls: images,
        price_cents: commercial.price ? Math.round(Number(commercial.price) * 100) : null,
        currency: commercial.currency,
        stock_quantity: Math.max(0, Number(commercial.stock) || 0),
        status: commercial.status
      });
      await revalidateSite({ type: 'shop', slug: product.slug });
      setMessage('Товар сохранён в магазине');
      setDraft(null); setImages([]); setInput('');
      await onReload();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось сохранить товар');
    } finally {
      setSaveBusy(false);
    }
  }

  const nameField = `name_${language}` as keyof ShopProductDraft;
  const shortField = `short_description_${language}` as keyof ShopProductDraft;
  const descriptionField = `description_${language}` as keyof ShopProductDraft;
  const seoTitleField = `seo_title_${language}` as keyof ShopProductDraft;
  const seoDescriptionField = `seo_description_${language}` as keyof ShopProductDraft;

  return <section className="shop-page">
    <header className="shop-hero page-card"><div><span className="eyebrow">AI commerce studio</span><h2>Магазин</h2><p>Продающие карточки товаров с точными фото. Цена и остатки контролируются администратором.</p></div><button className="btn btn-quiet" onClick={() => void onReload()}>Обновить</button></header>

    <section className="shop-studio">
      <div className="shop-studio-copy"><span className="ai-orb">AI</span><div><span className="eyebrow">Gemini · ecommerce contract</span><h3>Новый товар магазина</h3><p>Опишите конкретный продаваемый товар. Gemini подготовит тексты, SEO, переводы и серию изображений без людей и посторонних объектов.</p></div></div>
      <div className="shop-command"><textarea value={input} onChange={(event) => setInput(event.target.value)} placeholder="Например: суши-сет Лайт, 1030 г, точный состав указан на фото, доставка по Варшаве" /><div className="shop-command-actions"><div className="generation-option"><div className="generation-option-head"><span>Серия изображений</span><strong>{imageCount} <small>фото</small></strong></div><div className="generation-segments">{IMAGE_COUNTS.map((count) => <button type="button" key={count} className={imageCount === count ? 'active' : ''} onClick={() => setImageCount(count)}>{count}</button>)}</div></div><button className="btn btn-ai" onClick={() => void generateDraft()} disabled={aiBusy}>{aiBusy ? 'Gemini создаёт...' : 'Создать карточку'}</button></div></div>
      <div className="shop-reference-row"><AiReferenceUpload title="Фото конкретного товара" hint="Gemini сохранит состав и форму, меняя только фон и свет." images={references} onAdd={(files) => void addReferences(files)} onRemove={removeReference} /></div>
      {localError && <p className="form-error notice">{localError}</p>}{message && <p className="form-success notice">{message}</p>}
    </section>

    {draft && <section className="shop-draft page-card">
      <div className="shop-draft-head"><div><span className="eyebrow">Review before store database</span><h3>{draft.name_ru || draft.name_en}</h3><p>AI заполнил контент. Коммерческие данные вводит администратор.</p></div><div className="language-tabs">{(['ru', 'en', 'pl', 'uk'] as const).map((lang) => <button type="button" key={lang} className={language === lang ? 'active' : ''} onClick={() => setLanguage(lang)}>{lang.toUpperCase()}</button>)}</div></div>
      <div className="shop-draft-grid">
        <section className="shop-visual"><AiPhotoGallery heading="Фото товара" subtitle={`${images.length || imageCount} фото · точный ecommerce-режим`} actions={<button className="btn btn-quiet" onClick={() => void generateImages(draft)} disabled={imageBusy || singleImageBusy}>{imageBusy ? 'Генерируем...' : 'Перегенерировать серию'}</button>} images={images} selectedIndex={selectedImage} title={draft.name_ru || draft.name_en} emptyText="Фото не создано" busyText={imageBusy ? 'Создаём фото товара...' : 'Добавляем фото...'} busy={imageBusy} addBusy={singleImageBusy} onSelect={setSelectedImage} onAdd={() => void generateAdditionalImage()} onRemove={removeShopImage} /></section>
        <section className="shop-editor">
          <div className="shop-commercial"><label><span>Цена</span><div><input type="number" step=".01" value={commercial.price} onChange={(event) => setCommercial({ ...commercial, price: event.target.value })} placeholder="0.00" /><select value={commercial.currency} onChange={(event) => setCommercial({ ...commercial, currency: event.target.value })}><option>PLN</option><option>EUR</option><option>USD</option></select></div></label><label><span>SKU</span><input value={commercial.sku} onChange={(event) => setCommercial({ ...commercial, sku: event.target.value })} placeholder="AUTO / вручную" /></label><label><span>Остаток</span><input type="number" min="0" value={commercial.stock} onChange={(event) => setCommercial({ ...commercial, stock: event.target.value })} /></label><label><span>Статус</span><select value={commercial.status} onChange={(event) => setCommercial({ ...commercial, status: event.target.value as ShopProduct['status'] })}><option value="draft">Черновик</option><option value="active">Активен</option><option value="archived">Архив</option></select></label></div>
          <div className="draft-grid two"><label className="field"><span className="field-label">Название · {language.toUpperCase()}</span><input value={String(draft[nameField])} onChange={(event) => updateDraft(nameField, event.target.value)} /></label><label className="field"><span className="field-label">Категория</span><select value={draft.category} onChange={(event) => updateDraft('category', event.target.value)}>{CATEGORIES.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label><label className="field span-2"><span className="field-label">Короткое описание · {language.toUpperCase()}</span><textarea value={String(draft[shortField])} onChange={(event) => updateDraft(shortField, event.target.value)} /></label><label className="field span-2 shop-description"><span className="field-label">Полное описание · {language.toUpperCase()}</span><textarea value={String(draft[descriptionField])} onChange={(event) => updateDraft(descriptionField, event.target.value)} /></label><label className="field"><span className="field-label">SEO title · {language.toUpperCase()}</span><input value={String(draft[seoTitleField])} onChange={(event) => updateDraft(seoTitleField, event.target.value)} /></label><label className="field"><span className="field-label">SEO description · {language.toUpperCase()}</span><textarea value={String(draft[seoDescriptionField])} onChange={(event) => updateDraft(seoDescriptionField, event.target.value)} /></label></div>
        </section>
      </div>
      <div className="shop-draft-actions"><span>Цена и остаток не генерируются AI</span><div><button className="btn btn-quiet" onClick={() => setDraft(null)}>Отменить</button><button className="btn btn-primary" onClick={() => void save()} disabled={saveBusy || imageBusy || singleImageBusy}>{saveBusy ? 'Сохраняем...' : 'Сохранить товар'}</button></div></div>
    </section>}

    <section className="page-card shop-list"><div className="section-head"><div><h3>Товары магазина</h3><p className="page-muted">{filtered.length} карточек</p></div><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Поиск по названию, SKU или slug" /></div>{loading && <p className="page-muted">Загрузка...</p>}{error && <p className="form-error">{error}</p>}<div className="orders-table-wrap"><table className="orders-table"><thead><tr><th>Товар</th><th>SKU</th><th>Категория</th><th>Цена</th><th>Остаток</th><th>Статус</th><th>Управление</th></tr></thead><tbody>{filtered.map((product) => <tr key={product.id}><td><strong>{product.name_ru || product.name_en}</strong><br /><span className="page-muted">{product.slug}</span></td><td>{product.sku || '—'}</td><td>{product.category}</td><td>{product.price_cents === null ? 'Не задана' : `${(product.price_cents / 100).toFixed(2)} ${product.currency}`}</td><td>{product.stock_quantity}</td><td><span className={`status-badge status-badge-${product.status === 'active' ? 'ok' : 'neutral'}`}>{product.status}</span></td><td><div className="table-actions">{product.status !== 'active' && <button className="btn btn-quiet" onClick={() => void changeStatus(product, 'active')}>Опубликовать</button>}{product.status === 'active' && <button className="btn btn-quiet" onClick={() => void changeStatus(product, 'archived')}>Снять</button>}<button className="btn btn-danger" onClick={() => void removeProduct(product)}>Удалить</button></div></td></tr>)}</tbody></table></div></section>
  </section>;
}
