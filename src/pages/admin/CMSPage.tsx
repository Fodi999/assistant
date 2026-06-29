import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { CMSArticleForm } from '../../components/admin/forms/CMSArticleForm';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { cmsService, listCMSPages } from '../../services/admin/cmsService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type { CreateCMSPageDto } from '../../types/adminApi';
import { AdminResourcePage } from './AdminResourcePage';

type DrawerMode = 'create' | 'edit';

export function CMSPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<DrawerMode>('create');
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const loadRows = useCallback(async () => listCMSPages(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  async function refreshRows() {
    setRows(await listCMSPages(activeSiteId as SiteId));
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

  async function saveArticle(payload: CreateCMSPageDto) {
    setSaving(true);
    setFormError(null);
    try {
      if (drawerMode === 'edit' && editingRow) {
        await cmsService.update(editingRow.id, payload);
        toast.success('Статья обновлена.');
      } else {
        await cmsService.create({ ...payload, siteId: activeSiteId as SiteId });
        toast.success('Статья создана.');
      }
      await refreshRows();
      closeDrawer();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Не удалось сохранить статью.';
      setFormError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  async function deleteArticle(row: AdminResourceRow) {
    if (!window.confirm(`Удалить "${row.title}"?`)) return;
    try {
      await cmsService.remove(row.id, row.siteId as SiteId);
      toast.success('Статья удалена.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось удалить статью.');
    }
  }

  return (
    <>
      <AdminResourcePage
        title="CMS"
        eyebrow="Content"
        description="Pages, articles and structured content for the active site."
        icon="cms"
        actionLabel="Add page"
        loadRows={loadRows}
        rowsOverride={rows}
        onRowsLoaded={setRows}
        capabilities={resourceCapabilities.cms}
        onCreate={openCreate}
        onEdit={openEdit}
        onDelete={deleteArticle}
      />
      <AdminDrawer
        open={drawerOpen}
        title={drawerMode === 'edit' ? 'Редактировать статью' : 'Добавить статью'}
        description="UK/RU/EN поля будут отправлены в реальные CMS article DTO."
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="cms-article-form" disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <CMSArticleForm key={editingRow?.id || 'new'} formId="cms-article-form" row={editingRow} disabled={saving} onSubmit={saveArticle} />
      </AdminDrawer>
    </>
  );
}
