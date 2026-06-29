import { useCallback, useEffect, useMemo, useState } from 'react';
import { revalidateSite } from '../../api/revalidate';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { ShopProductForm } from '../../components/admin/forms/ShopProductForm';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { listShopProducts, shopService, updateShopProduct, updateShopProductStatus } from '../../services/admin/shopService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type { CreateShopProductDto } from '../../types/adminApi';
import { AdminResourcePage } from './AdminResourcePage';

type DrawerMode = 'create' | 'edit';

const defaultShopCategories = [
  'sushi-rolls',
  'sushi-sets',
  'nigiri',
  'gunkan',
  'sashimi',
  'soups',
  'salads',
  'snacks',
  'sauces',
  'beverages',
  'desserts',
  'kitchen-tools',
  'tableware',
  'other'
];

type ShopBackend = {
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  name_en?: string;
  category?: string;
  sku?: string | null;
  image_urls?: string[];
  price_cents?: number | null;
  currency?: string;
  stock_quantity?: number;
};

function productImage(row: AdminResourceRow) {
  return ((row.backend || {}) as ShopBackend).image_urls?.[0] || '';
}

function productPrice(row: AdminResourceRow) {
  const product = (row.backend || {}) as ShopBackend;
  if (typeof product.price_cents !== 'number') return row.metric || 'No price';
  return `${(product.price_cents / 100).toFixed(2)} ${product.currency || 'PLN'}`;
}

export function ShopPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>('create');
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [manualCategories, setManualCategories] = useState<string[]>(defaultShopCategories);
  const [createCategory, setCreateCategory] = useState('');
  const [publishingId, setPublishingId] = useState<string | null>(null);

  const loadRows = useCallback(async () => listShopProducts(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  const categories = useMemo(() => {
    const fromRows = (rows || []).map((row) => row.type).filter(Boolean);
    return Array.from(new Set([...manualCategories, ...fromRows])).sort((a, b) => a.localeCompare(b));
  }, [manualCategories, rows]);

  const categoryStats = useMemo(() => categories.map((category) => ({
    category,
    count: (rows || []).filter((row) => row.type === category).length
  })), [categories, rows]);

  const activeProducts = useMemo(() => (rows || []).filter((row) => row.status === 'active' || row.status === 'published'), [rows]);

  function addCategory() {
    const category = window.prompt('Новая категория товара (slug, например: sushi-tools)');
    const normalized = category?.trim().toLowerCase();
    if (!normalized) return;
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(normalized)) {
      toast.error('Категория должна быть slug: lowercase, цифры и дефисы.');
      return;
    }
    setManualCategories((current) => Array.from(new Set([...current, normalized])));
    openCreate(normalized);
  }

  async function refreshRows() {
    setRows(await listShopProducts(activeSiteId as SiteId));
  }

  function openCreate(category = '') {
    setDrawerMode('create');
    setEditingRow(null);
    setCreateCategory(category);
    setFormError(null);
    setDrawerOpen(true);
  }

  function openEdit(row: AdminResourceRow) {
    setDrawerMode('edit');
    setEditingRow(row);
    setCreateCategory('');
    setFormError(null);
    setDrawerOpen(true);
  }

  function closeDrawer() {
    if (saving) return;
    setDrawerOpen(false);
    setEditingRow(null);
    setCreateCategory('');
    setFormError(null);
  }

  async function saveProduct(payload: CreateShopProductDto) {
    setSaving(true);
    setFormError(null);
    try {
      if (drawerMode === 'edit' && editingRow) {
        const saved = await updateShopProduct(editingRow.id, { ...payload, siteId: activeSiteId as SiteId });
        await revalidateSite({ type: 'shop', slug: saved.slug || payload.slug || editingRow.slug });
        toast.success('Товар обновлен.');
      } else {
        const saved = await shopService.create({ ...payload, siteId: activeSiteId as SiteId });
        await revalidateSite({ type: 'shop', slug: saved.slug || payload.slug });
        toast.success('Товар создан.');
      }
      await refreshRows();
      closeDrawer();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Не удалось сохранить товар.';
      setFormError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  async function deleteProduct(row: AdminResourceRow) {
    if (!window.confirm(`Удалить "${row.title}"?`)) return;
    try {
      await shopService.remove(row.id, row.siteId as SiteId);
      await revalidateSite({ type: 'shop', slug: row.slug });
      toast.success('Товар удален.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось удалить товар.');
    }
  }

  async function changeProductStatus(row: AdminResourceRow, status: 'active' | 'draft' | 'archived') {
    setPublishingId(row.id);
    try {
      const saved = await updateShopProductStatus(row.id, row.siteId as SiteId, status);
      await revalidateSite({ type: 'shop', slug: saved.slug || row.slug });
      toast.success(status === 'active' ? 'Товар опубликован на сайте.' : 'Товар скрыт с сайта.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось изменить статус товара.');
    } finally {
      setPublishingId(null);
    }
  }

  return (
    <>
      <AdminResourcePage
        title="Shop"
        eyebrow="Commerce"
        description="Products, affiliate offers and storefront collections."
        icon="shop"
        actionLabel="Add product"
        loadRows={loadRows}
        rowsOverride={rows}
        onRowsLoaded={setRows}
        capabilities={resourceCapabilities.shop}
        onCreate={() => openCreate()}
        onEdit={openEdit}
        onDelete={deleteProduct}
      />
      <section className="admin-panel-card shop-control-panel">
        <div className="admin-section-heading">
          <div>
            <span className="eyebrow">Storefront</span>
            <h3>Категории и карточки товаров</h3>
            <p>Категории сохраняются в поле товара. На сайте они появляются автоматически, когда товар опубликован.</p>
          </div>
          <ActionButton icon="sparkles" onClick={addCategory}>New category</ActionButton>
        </div>
        <div className="shop-category-strip">
          {categoryStats.map((item) => (
            <button key={item.category} type="button" onClick={() => {
              openCreate(item.category);
            }}>
              <strong>{item.category}</strong>
              <span>{item.count} products</span>
            </button>
          ))}
        </div>
        <div className="shop-admin-grid">
          {(rows || []).slice(0, 12).map((row) => (
            <article className="shop-admin-card" key={row.id}>
              {productImage(row) ? <img src={productImage(row)} alt={row.title} /> : <span className="shop-admin-card__empty">Photo</span>}
              <div>
                <small>{row.type}</small>
                <h4>{row.title}</h4>
                <p>{productPrice(row)}</p>
              </div>
              <div className="shop-admin-card__meta">
                <span>{row.status}</span>
                <span>{((row.backend || {}) as ShopBackend).stock_quantity ?? 0} stock</span>
              </div>
              <div className="shop-admin-card__actions">
                {row.status === 'active' || row.status === 'published' ? (
                  <ActionButton onClick={() => void changeProductStatus(row, 'draft')} disabled={publishingId === row.id}>Hide</ActionButton>
                ) : (
                  <ActionButton tone="primary" onClick={() => void changeProductStatus(row, 'active')} disabled={publishingId === row.id}>
                    {publishingId === row.id ? 'Publishing' : 'Publish'}
                  </ActionButton>
                )}
                <ActionButton onClick={() => openEdit(row)}>Edit</ActionButton>
                <ActionButton tone="danger" onClick={() => void deleteProduct(row)}>Delete</ActionButton>
              </div>
            </article>
          ))}
          {!rows?.length ? (
            <article className="shop-admin-empty">
              <strong>Добавьте первый товар</strong>
              <p>Фото, цена, остаток, категория и статус active сразу создадут карточку на сайте.</p>
              <ActionButton tone="primary" icon="shop" onClick={() => openCreate()}>Add product</ActionButton>
            </article>
          ) : null}
        </div>
        <div className="shop-admin-summary">
          <span>{rows?.length || 0} total</span>
          <span>{activeProducts.length} visible on site</span>
        </div>
      </section>
      <AdminDrawer
        open={drawerOpen}
        title={drawerMode === 'edit' ? 'Редактировать товар' : 'Добавить товар'}
        description={drawerMode === 'edit' ? 'Редактирование карточки товара и витрины сайта.' : 'Создание shop product через реальные CMS shop DTO.'}
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="shop-product-form" disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <ShopProductForm key={editingRow?.id || createCategory || 'new'} formId="shop-product-form" row={editingRow} disabled={saving} editMode={drawerMode === 'edit'} initialCategory={createCategory} categories={categories} onSubmit={saveProduct} />
      </AdminDrawer>
    </>
  );
}
