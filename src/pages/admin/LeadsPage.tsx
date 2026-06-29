import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { leadsService, listLeads } from '../../services/admin/leadsService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import type { AdminResourceRow, LeadStatus, SiteId } from '../../types/admin';
import { AdminResourcePage } from './AdminResourcePage';

const leadStatuses: LeadStatus[] = ['new', 'contacted', 'quoted', 'won', 'lost'];

export function LeadsPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [rows, setRows] = useState<AdminResourceRow[] | null>(null);
  const [editingRow, setEditingRow] = useState<AdminResourceRow | null>(null);
  const [status, setStatus] = useState<LeadStatus>('new');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const loadRows = useCallback(async () => listLeads(activeSiteId as SiteId), [activeSiteId]);

  useEffect(() => setRows(null), [activeSiteId]);

  async function refreshRows() {
    setRows(await listLeads(activeSiteId as SiteId));
  }

  function openStatus(row: AdminResourceRow) {
    setEditingRow(row);
    setStatus((row.status === 'active' ? 'contacted' : row.status === 'archived' ? 'lost' : row.status) as LeadStatus);
    setError(null);
  }

  function closeDrawer() {
    if (saving) return;
    setEditingRow(null);
    setError(null);
  }

  async function saveStatus() {
    if (!editingRow) return;
    setSaving(true);
    setError(null);
    try {
      await leadsService.update(editingRow.id, { status: status as never, siteId: editingRow.siteId });
      toast.success('Статус лида обновлен.');
      await refreshRows();
      closeDrawer();
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : 'Не удалось обновить статус.';
      setError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <AdminResourcePage
        title="Leads"
        eyebrow="CRM"
        description="Incoming requests and contact pipeline."
        icon="leads"
        actionLabel="Add lead"
        loadRows={loadRows}
        rowsOverride={rows}
        onRowsLoaded={setRows}
        capabilities={resourceCapabilities.leads}
        onChangeStatus={openStatus}
      />
      <AdminDrawer
        open={Boolean(editingRow)}
        title="Изменить статус лида"
        description={editingRow?.title}
        onClose={closeDrawer}
        footer={(
          <>
            {error ? <p className="form-error">{error}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Отмена</ActionButton>
            <ActionButton tone="primary" icon="save" onClick={saveStatus} disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
          </>
        )}
      >
        <div className="admin-form-grid">
          <label>
            <span>Status</span>
            <select value={status} disabled={saving} onChange={(event) => setStatus(event.target.value as LeadStatus)}>
              {leadStatuses.map((item) => <option key={item} value={item}>{item}</option>)}
            </select>
          </label>
        </div>
      </AdminDrawer>
    </>
  );
}
