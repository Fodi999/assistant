import { useCallback, useEffect, useState } from 'react';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { listUsers, usersService } from '../../services/admin/usersService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import { AdminResourcePage } from './AdminResourcePage';

export function UsersPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const loadRows = useCallback(async () => listUsers(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  async function refreshRows() {
    setRows(await listUsers(activeSiteId as SiteId));
  }

  async function deleteUser(row: AdminResourceRow) {
    if (!window.confirm(`Удалить пользователя "${row.title}"? Это действие удаляет tenant и связанные данные.`)) return;

    try {
      await usersService.remove(row.id);
      toast.success('Пользователь удален.');
      await refreshRows();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Не удалось удалить пользователя.');
    }
  }

  return (
    <AdminResourcePage
      title="Users"
      eyebrow="Access"
      description="Editors, managers and role assignments."
      icon="users"
      actionLabel="Invite user"
      loadRows={loadRows}
      rowsOverride={rows}
      onRowsLoaded={setRows}
      capabilities={resourceCapabilities.users}
      onDelete={deleteUser}
    />
  );
}
