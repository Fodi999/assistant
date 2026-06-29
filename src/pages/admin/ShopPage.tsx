import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { ShopProductForm } from '../../components/admin/forms/ShopProductForm';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { listShopProducts, shopService } from '../../services/admin/shopService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type { CreateShopProductDto } from '../../types/adminApi';
import { AdminResourcePage } from './AdminResourcePage';

type DrawerMode = 'create' | 'edit';

export function ShopPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>('create');
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const loadRows = useCallback(async () => listShopProducts(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  async function refreshRows() {
    setRows(await listShopProducts(activeSiteId as SiteId));
  }

  function openCreate() {
    setDrawerMode('create');
    setEditingRow(null);
    setFormError(null);
    setDrawerOpen(true);
  }

  function openEdit(row: AdminResourceRow) {
    setDrawerMode('edit');
    setEditingRow(row);
    setFormError(null);
    setDrawerOpen(true);
  }

  function closeDrawer() {
    if (saving) return;
    setDrawerOpen(false);
    setEditingRow(null);
    setFormError(null);
  }

  async function saveProduct(payload: CreateShopProductDto) {
    setSaving(true);
    setFormError(null);
    try {
      if (drawerMode === 'edit' && editingRow) {
        await shopService.update(editingRow.id, payload);
        toast.success('Статус товара обновлен.');
      } else {
        await shopService.create({ ...payload, siteId: activeSiteId as SiteId });
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
      toast.success('Товар удален.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось удалить товар.');
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
        onCreate={openCreate}
        onEdit={openEdit}
        onDelete={deleteProduct}
      />
      <AdminDrawer
        open={drawerOpen}
        title={drawerMode === 'edit' ? 'Редактировать товар' : 'Добавить товар'}
        description={drawerMode === 'edit' ? 'Backend поддерживает изменение статуса для shop products.' : 'Создание shop product через реальные CMS shop DTO.'}
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="shop-product-form" disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <ShopProductForm key={editingRow?.id || 'new'} formId="shop-product-form" row={editingRow} disabled={saving} editMode={drawerMode === 'edit'} onSubmit={saveProduct} />
      </AdminDrawer>
    </>
  );
}
