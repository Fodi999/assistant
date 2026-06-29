import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { AdminPanel } from '../../components/admin/AdminPanel';
import { AdminState } from '../../components/admin/AdminState';
import { AdminToolbar } from '../../components/admin/AdminToolbar';
import { DataTable, type DataTableColumn } from '../../components/admin/DataTable';
import { StatusBadge } from '../../components/admin/StatusBadge';
import { useActiveSite } from '../../lib/useActiveSite';
import type { AppIconName } from '../../components/AppIcon';
import type { AdminResourceRow, ResourceStatus, SiteId } from '../../types/admin';
import { resourceNotice, type AdminResourceCapabilities } from '../../services/admin/resourceCapabilities';

type AdminResourcePageProps = {
  title: string;
  eyebrow: string;
  description: string;
  icon: AppIconName;
  actionLabel?: string;
  loadRows: (siteId: SiteId) => Promise<AdminResourceRow[]>;
  rowsOverride?: AdminResourceRow[] | null;
  actions?: ReactNode;
  onRowsLoaded?: (rows: AdminResourceRow[]) => void;
  capabilities?: AdminResourceCapabilities;
  onCreate?: () => void;
  onEdit?: (row: AdminResourceRow) => void;
  onDelete?: (row: AdminResourceRow) => void;
  onChangeStatus?: (row: AdminResourceRow) => void;
};

const statusOptions = ['all', 'active', 'published', 'draft', 'new', 'warning', 'archived'] as const;

export function AdminResourcePage({ title, eyebrow, description, icon, actionLabel = 'Add item', loadRows, rowsOverride, actions, onRowsLoaded, capabilities, onCreate, onEdit, onDelete, onChangeStatus }: AdminResourcePageProps) {
  const { activeSiteId, activeSite } = useActiveSite();
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState<(typeof statusOptions)[number]>('all');
  const [rows, setRows] = useState<AdminResourceRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    void loadRows(activeSiteId as SiteId)
      .then((nextRows) => {
        setRows(nextRows);
        onRowsLoaded?.(nextRows);
      })
      .catch((loadError) => setError(loadError instanceof Error ? loadError.message : 'Unknown error'))
      .finally(() => setLoading(false));
  }, [activeSiteId, loadRows, onRowsLoaded]);

  useEffect(() => {
    load();
  }, [load]);

  const sourceRows = rowsOverride ?? rows;

  const filteredRows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return sourceRows.filter((row) => {
      const matchesQuery = !needle || `${row.title} ${row.type} ${row.owner} ${row.metric}`.toLowerCase().includes(needle);
      const matchesStatus = status === 'all' || row.status === status;
      return matchesQuery && matchesStatus;
    });
  }, [query, sourceRows, status]);

  const columns = useMemo(() => getColumns({ capabilities, onEdit, onDelete, onChangeStatus }), [capabilities, onChangeStatus, onDelete, onEdit]);
  const notice = resourceNotice(capabilities);
  const canCreate = capabilities?.mockOnly ? false : capabilities?.canCreate !== false;
  const headerActions = actions ?? (canCreate ? <ActionButton icon="sparkles" tone="primary" onClick={onCreate}>{actionLabel}</ActionButton> : null);

  return (
    <section className="admin-resource-page">
      <AdminPageHeader
        eyebrow={eyebrow}
        title={title}
        description={description}
        icon={icon}
        meta={<StatusBadge status={activeSite.status} label={activeSite.id} />}
        actions={headerActions}
      />

      {notice ? <p className="admin-soft-alert">{notice}</p> : null}

      <AdminToolbar
        searchValue={query}
        searchPlaceholder={`Search ${title.toLowerCase()}`}
        onSearchChange={setQuery}
        filters={(
          <label className="admin-select-filter">
            <span>Status</span>
            <select value={status} onChange={(event) => setStatus(event.target.value as typeof status)}>
              {statusOptions.map((option) => <option key={option} value={option}>{option}</option>)}
            </select>
          </label>
        )}
      />

      <AdminPanel title={title} icon={icon} meta={`${filteredRows.length} / ${sourceRows.length}`}>
        <AdminState
          loading={loading && !rowsOverride}
          error={error}
          empty={!filteredRows.length}
          emptyTitle="No items"
          emptyDescription="Change filters or add the first record when backend endpoints are ready."
          icon={icon}
          onRetry={load}
        >
          <DataTable
            rows={filteredRows}
            getRowKey={(row) => row.id}
            columns={columns}
          />
        </AdminState>
      </AdminPanel>
    </section>
  );
}

function getColumns({ capabilities, onEdit, onDelete, onChangeStatus }: Pick<AdminResourcePageProps, 'capabilities' | 'onEdit' | 'onDelete' | 'onChangeStatus'>) {
  const columns: Array<DataTableColumn<AdminResourceRow>> = [
    { key: 'image', header: 'Photo', render: (row: AdminResourceRow) => <ResourceThumb row={row} /> },
    { key: 'title', header: 'Name', render: (row: AdminResourceRow) => <strong>{row.title}</strong> },
    { key: 'type', header: 'Type', render: (row: AdminResourceRow) => row.type },
    { key: 'status', header: 'Status', render: (row: AdminResourceRow) => <StatusBadge status={row.status as ResourceStatus} /> },
    { key: 'owner', header: 'Owner', render: (row: AdminResourceRow) => row.owner },
    { key: 'updated', header: 'Updated', render: (row: AdminResourceRow) => row.updated },
    { key: 'metric', header: 'Metric', render: (row: AdminResourceRow) => row.metric, align: 'right' as const }
  ];

  const canEdit = Boolean(onEdit && capabilities?.canEdit !== false && !capabilities?.mockOnly);
  const canDelete = Boolean(onDelete && capabilities?.canDelete);
  const canChangeStatus = Boolean(onChangeStatus && capabilities?.canChangeStatus);

  if (canEdit || canDelete || canChangeStatus) {
    columns.push({
      key: 'actions',
      header: '',
      align: 'right' as const,
      render: (row: AdminResourceRow) => (
        <div className="admin-table-actions">
          {canChangeStatus ? <button className="table-action" type="button" onClick={() => onChangeStatus?.(row)}>Status</button> : null}
          {canEdit ? <button className="table-action" type="button" onClick={() => onEdit?.(row)}>Edit</button> : null}
          {canDelete ? <button className="table-action danger" type="button" onClick={() => onDelete?.(row)}>Delete</button> : null}
        </div>
      )
    });
  }

  return columns;
}

function ResourceThumb({ row }: { row: AdminResourceRow }) {
  const backend = row.backend as { image_url?: string | null; imageUrl?: string | null } | undefined;
  const imageUrl = backend?.image_url || backend?.imageUrl || '';

  if (!imageUrl) {
    return <span className="admin-row-thumb empty">-</span>;
  }

  return <img className="admin-row-thumb" src={imageUrl} alt="" loading="lazy" />;
}
