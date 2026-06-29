import { useCallback, useEffect, useMemo, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { AdminPanel } from '../../components/admin/AdminPanel';
import { AdminToolbar } from '../../components/admin/AdminToolbar';
import { DataTable } from '../../components/admin/DataTable';
import { EmptyState } from '../../components/admin/EmptyState';
import { FieldError, isValidUrl } from '../../components/admin/forms/formUtils';
import { StatusBadge } from '../../components/admin/StatusBadge';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { isApiMode } from '../../config/adminConfig';
import { useActiveSite } from '../../lib/useActiveSite';
import {
  calendarKindOptions,
  createBlankCalendarDay,
  listCalendarDays,
  removeCalendarDay,
  saveCalendarDay
} from '../../services/admin/calendarService';
import type { CalendarDay } from '../../api/iconsSite';

type FormErrors = Partial<Record<'day' | 'label' | 'imageUrl', string>>;

const monthOptions = [
  { value: 1, label: 'January' },
  { value: 2, label: 'February' },
  { value: 3, label: 'March' },
  { value: 4, label: 'April' },
  { value: 5, label: 'May' },
  { value: 6, label: 'June' },
  { value: 7, label: 'July' },
  { value: 8, label: 'August' },
  { value: 9, label: 'September' },
  { value: 10, label: 'October' },
  { value: 11, label: 'November' },
  { value: 12, label: 'December' }
];

export function CalendarPage() {
  const { activeSiteId, activeSite } = useActiveSite();
  const toast = useAdminToast();
  const today = new Date();
  const [year, setYear] = useState(today.getFullYear());
  const [month, setMonth] = useState(today.getMonth() + 1);
  const [query, setQuery] = useState('');
  const [days, setDays] = useState<CalendarDay[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingDay, setEditingDay] = useState<CalendarDay | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const connectedToApi = isApiMode && activeSiteId === 'church';

  const loadDays = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setDays(await listCalendarDays(activeSiteId, year, month));
    } catch (loadError) {
      const message = loadError instanceof Error ? loadError.message : 'Не удалось загрузить календарь.';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, [activeSiteId, month, toast, year]);

  useEffect(() => {
    void loadDays();
  }, [loadDays]);

  const filteredDays = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) return days;
    return days.filter((day) => [
      day.label,
      day.note,
      day.description,
      day.kind,
      day.iconSlug,
      day.prayerSlug,
      day.gospelSlug
    ].some((value) => value.toLowerCase().includes(normalizedQuery)));
  }, [days, query]);

  function openCreate() {
    setEditingDay(createBlankCalendarDay(month, String(Math.min(days.length + 1, 31))));
    setFormError(null);
    setDrawerOpen(true);
  }

  function openEdit(day: CalendarDay) {
    setEditingDay(day);
    setFormError(null);
    setDrawerOpen(true);
  }

  function closeDrawer() {
    if (saving) return;
    setDrawerOpen(false);
    setEditingDay(null);
    setFormError(null);
  }

  async function submitDay(day: CalendarDay) {
    setSaving(true);
    setFormError(null);
    try {
      const nextDays = await saveCalendarDay(activeSiteId, year, month, day);
      setDays(nextDays);
      toast.success('Календарь сохранен.');
      closeDrawer();
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : 'Не удалось сохранить день календаря.';
      setFormError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  async function deleteDay(day: CalendarDay) {
    const confirmed = window.confirm(`Удалить "${day.label || `day ${day.day}`}" из календаря?`);
    if (!confirmed) return;

    setSaving(true);
    try {
      const nextDays = await removeCalendarDay(activeSiteId, year, month, day.id);
      setDays(nextDays);
      toast.success('День календаря удален.');
    } catch (deleteError) {
      const message = deleteError instanceof Error ? deleteError.message : 'Не удалось удалить день календаря.';
      toast.error(message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="admin-resource-page">
      <AdminPageHeader
        eyebrow="Calendar"
        title={activeSiteId === 'church' ? 'Church calendar' : `${activeSite.name} calendar`}
        description={connectedToApi ? 'Connected to the church icons-site calendar API.' : 'Site-specific calendar draft storage for this CRM workspace.'}
        icon="calendar"
        meta={<StatusBadge status={connectedToApi ? 'online' : 'limited'} label={connectedToApi ? 'api connected' : 'local draft'} />}
        actions={<ActionButton icon="sparkles" tone="primary" onClick={openCreate}>Add day</ActionButton>}
      />

      {!connectedToApi ? (
        <p className="admin-soft-alert">
          This calendar is isolated per site in local CRM storage until a dedicated backend endpoint is added.
        </p>
      ) : null}

      <AdminToolbar
        searchValue={query}
        searchPlaceholder="Search calendar"
        onSearchChange={setQuery}
        filters={(
          <>
            <label className="admin-select-filter">
              <span>Month</span>
              <select value={month} onChange={(event) => setMonth(Number(event.target.value))}>
                {monthOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </select>
            </label>
            <label className="admin-select-filter">
              <span>Year</span>
              <input value={year} inputMode="numeric" onChange={(event) => setYear(Number(event.target.value) || today.getFullYear())} />
            </label>
          </>
        )}
        actions={<ActionButton icon="refresh" onClick={loadDays} disabled={loading}>Refresh</ActionButton>}
      />

      <AdminPanel title="Calendar days" icon="calendar" meta={`${filteredDays.length} / ${days.length}`}>
        {loading ? <p className="admin-table-empty">Loading calendar...</p> : null}
        {!loading && error ? (
          <EmptyState icon="calendar" title="Calendar unavailable" description={error} action={<ActionButton icon="refresh" onClick={loadDays}>Try again</ActionButton>} />
        ) : null}
        {!loading && !error ? (
          <DataTable
            rows={filteredDays}
            getRowKey={(day) => day.id}
            empty={<EmptyState icon="calendar" title="No calendar days" description="Add the first day for the selected site and month." action={<ActionButton icon="sparkles" tone="primary" onClick={openCreate}>Add day</ActionButton>} />}
            columns={[
              { key: 'day', header: 'Day', render: (day) => <strong>{day.day}</strong> },
              { key: 'label', header: 'Label', render: (day) => <CalendarLabel day={day} /> },
              { key: 'kind', header: 'Kind', render: (day) => <StatusBadge status={day.feast ? 'published' : 'draft'} label={day.kind} /> },
              { key: 'julian', header: 'Julian', render: (day) => day.julianDate || day.julianDay || '—' },
              { key: 'links', header: 'Links', render: (day) => <CalendarLinks day={day} /> },
              {
                key: 'actions',
                header: '',
                align: 'right',
                render: (day) => (
                  <div className="admin-table-actions">
                    <button className="table-action" type="button" onClick={() => openEdit(day)}>Edit</button>
                    <button className="table-action danger" type="button" disabled={saving} onClick={() => deleteDay(day)}>Delete</button>
                  </div>
                )
              }
            ]}
          />
        ) : null}
      </AdminPanel>

      <AdminDrawer
        open={drawerOpen}
        title={editingDay?.id && days.some((day) => day.id === editingDay.id) ? 'Edit calendar day' : 'Add calendar day'}
        description={connectedToApi ? 'Saved to the church backend calendar.' : 'Saved locally for the active site.'}
        onClose={closeDrawer}
        footer={(
          <>
            {formError ? <p className="form-error">{formError}</p> : null}
            <ActionButton onClick={closeDrawer} disabled={saving}>Cancel</ActionButton>
            <ActionButton tone="primary" icon="save" type="submit" form="calendar-day-form" disabled={saving}>{saving ? 'Saving' : 'Save'}</ActionButton>
          </>
        )}
      >
        {editingDay ? <CalendarDayForm key={editingDay.id} day={editingDay} disabled={saving} onSubmit={submitDay} /> : null}
      </AdminDrawer>
    </section>
  );
}

function CalendarLabel({ day }: { day: CalendarDay }) {
  return (
    <div className="calendar-label-cell">
      <strong>{day.label || 'Untitled day'}</strong>
      <span>{day.note || day.description || 'No note'}</span>
    </div>
  );
}

function CalendarLinks({ day }: { day: CalendarDay }) {
  const links = [day.iconSlug, day.prayerSlug, day.gospelSlug].filter(Boolean);
  return links.length ? <span className="calendar-link-list">{links.join(' / ')}</span> : <span className="page-muted">—</span>;
}

function CalendarDayForm({ day, disabled, onSubmit }: { day: CalendarDay; disabled: boolean; onSubmit: (day: CalendarDay) => void }) {
  const [form, setForm] = useState<CalendarDay>(day);
  const [errors, setErrors] = useState<FormErrors>({});

  function update(next: Partial<CalendarDay>) {
    setForm((current) => ({ ...current, ...next }));
  }

  function validate() {
    const nextErrors: FormErrors = {};
    const dayNumber = Number(form.day);
    if (!Number.isInteger(dayNumber) || dayNumber < 1 || dayNumber > 31) nextErrors.day = 'Day must be between 1 and 31.';
    if (!form.label.trim()) nextErrors.label = 'Label is required.';
    if (!isValidUrl(form.imageUrl)) nextErrors.imageUrl = 'Image must be a valid http/https URL.';
    setErrors(nextErrors);
    return Object.keys(nextErrors).length === 0;
  }

  return (
    <form id="calendar-day-form" className="admin-form-grid" onSubmit={(event) => {
      event.preventDefault();
      if (disabled || !validate()) return;
      onSubmit(form);
    }}>
      <div className="admin-form-columns">
        <label><span>Day</span><input disabled={disabled} value={form.day} inputMode="numeric" onChange={(event) => update({ day: event.target.value })} /><FieldError message={errors.day} /></label>
        <label><span>Kind</span><select disabled={disabled} value={form.kind} onChange={(event) => update({ kind: event.target.value as CalendarDay['kind'] })}>{calendarKindOptions.map((kind) => <option key={kind} value={kind}>{kind}</option>)}</select></label>
      </div>
      <label><span>Title</span><input disabled={disabled} value={form.label} onChange={(event) => update({ label: event.target.value })} /><FieldError message={errors.label} /></label>
      <label><span>Short note</span><input disabled={disabled} value={form.note} onChange={(event) => update({ note: event.target.value })} /></label>
      <label><span>Description</span><textarea disabled={disabled} value={form.description} onChange={(event) => update({ description: event.target.value })} /></label>
      <div className="admin-form-columns">
        <label><span>Gregorian date</span><input disabled={disabled} type="date" value={form.gregorianDate || ''} onChange={(event) => update({ gregorianDate: event.target.value })} /></label>
        <label><span>Julian date</span><input disabled={disabled} value={form.julianDate || ''} onChange={(event) => update({ julianDate: event.target.value })} /></label>
      </div>
      <label><span>Image URL</span><input disabled={disabled} value={form.imageUrl} onChange={(event) => update({ imageUrl: event.target.value })} /><FieldError message={errors.imageUrl} /></label>
      <div className="admin-form-columns">
        <label><span>Icon slug</span><input disabled={disabled} value={form.iconSlug} onChange={(event) => update({ iconSlug: event.target.value })} /></label>
        <label><span>Prayer slug</span><input disabled={disabled} value={form.prayerSlug} onChange={(event) => update({ prayerSlug: event.target.value })} /></label>
      </div>
      <div className="admin-form-columns">
        <label><span>Gospel slug</span><input disabled={disabled} value={form.gospelSlug} onChange={(event) => update({ gospelSlug: event.target.value })} /></label>
        <label><span>Detail link</span><input disabled={disabled} value={form.detailHref} onChange={(event) => update({ detailHref: event.target.value })} /></label>
      </div>
      <div className="calendar-checks">
        <label><input disabled={disabled} type="checkbox" checked={form.feast} onChange={(event) => update({ feast: event.target.checked })} /><span>Feast day</span></label>
        <label><input disabled={disabled} type="checkbox" checked={form.current} onChange={(event) => update({ current: event.target.checked })} /><span>Current day</span></label>
        <label><input disabled={disabled} type="checkbox" checked={form.textOnly} onChange={(event) => update({ textOnly: event.target.checked })} /><span>Text only</span></label>
      </div>
    </form>
  );
}
