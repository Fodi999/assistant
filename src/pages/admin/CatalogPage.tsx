import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { CatalogProductForm } from '../../components/admin/forms/CatalogProductForm';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { catalogService, listCatalogItems } from '../../services/admin/catalogService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type { CreateCatalogItemDto } from '../../types/adminApi';
import { AdminResourcePage } from './AdminResourcePage';

type DrawerMode = 'create' | 'edit';

export function CatalogPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>('create');
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const loadRows = useCallback(async () => listCatalogItems(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => {
    setRows(null);
  }, [activeSiteId]);

  const handleRowsLoaded = useCallback((nextRows: AdminResourceRow[]) => {
    setRows(nextRows);
  }, []);

  async function refreshRows() {
    const nextRows = await listCatalogItems(activeSiteId as SiteId);
    setRows(nextRows);
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

  async function saveItem(payload: CreateCatalogItemDto) {
    setSaving(true);
    setFormError(null);

    try {
      if (drawerMode === 'edit' && editingRow) {
        await catalogService.update(editingRow.id, payload);
        toast.success('Запись каталога обновлена.');
      } else {
        await catalogService.create({ ...payload, siteId: activeSiteId as SiteId });
        toast.success('Запись каталога создана.');
      }

      await refreshRows();
      closeDrawer();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Не удалось сохранить запись каталога.';
      setFormError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  async function deleteItem(row: AdminResourceRow) {
    if (!window.confirm(`Удалить "${row.title}"?`)) return;

    setDeletingId(row.id);
    try {
      await catalogService.remove(row.id, row.siteId as SiteId);
      toast.success('Запись каталога удалена.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось удалить запись каталога.');
    } finally {
      setDeletingId(null);
    }
  }

  return (
    <>
      <AdminResourcePage
        title="Catalog"
        eyebrow="Inventory"
        description="Categories, products and site-specific collections."
        icon="catalog"
        actionLabel="Add item"
        loadRows={loadRows}
        rowsOverride={rows}
        onRowsLoaded={handleRowsLoaded}
        capabilities={resourceCapabilities.catalog}
        onCreate={openCreate}
        onEdit={openEdit}
        onDelete={deleteItem}
      />

      <AdminDrawer
        open={drawerOpen}
        title={drawerMode === 'edit' ? 'Редактировать продукт' : 'Добавить продукт'}
        description="Поля будут преобразованы adapters в реальные DTO backend."
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="catalog-product-form" disabled={saving || Boolean(deletingId)}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <CatalogProductForm
          key={editingRow?.id || 'new'}
          formId="catalog-product-form"
          row={editingRow}
          disabled={saving}
          onSubmit={saveItem}
        />
      </AdminDrawer>
    </>
  );
}
