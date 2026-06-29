import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { SupplierForm } from '../../components/admin/forms/SupplierForm';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { listSuppliers, suppliersService } from '../../services/admin/suppliersService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type { CreateSupplierDto } from '../../types/adminApi';
import { AdminResourcePage } from './AdminResourcePage';

type DrawerMode = 'create' | 'edit';

export function SuppliersPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>('create');
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const loadRows = useCallback(async () => listSuppliers(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  async function refreshRows() {
    setRows(await listSuppliers(activeSiteId as SiteId));
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

  async function saveSupplier(payload: CreateSupplierDto) {
    setSaving(true);
    setFormError(null);
    try {
      if (drawerMode === 'edit' && editingRow) {
        await suppliersService.update(editingRow.id, payload);
        toast.success('Поставщик обновлен.');
      } else {
        await suppliersService.create({ ...payload, siteId: activeSiteId as SiteId });
        toast.success('Поставщик создан.');
      }
      await refreshRows();
      closeDrawer();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Не удалось сохранить поставщика.';
      setFormError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <AdminResourcePage
        title="Suppliers"
        eyebrow="Network"
        description="Vendors, marketplaces and partner sources."
        icon="suppliers"
        actionLabel="Add supplier"
        loadRows={loadRows}
        rowsOverride={rows}
        onRowsLoaded={setRows}
        capabilities={resourceCapabilities.suppliers}
        onCreate={openCreate}
        onEdit={openEdit}
      />
      <AdminDrawer
        open={drawerOpen}
        title={drawerMode === 'edit' ? 'Редактировать поставщика' : 'Добавить поставщика'}
        description="Удаление поставщиков backend сейчас не поддерживает."
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="supplier-form" disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <SupplierForm key={editingRow?.id || 'new'} formId="supplier-form" row={editingRow} disabled={saving} onSubmit={saveSupplier} />
      </AdminDrawer>
    </>
  );
}
