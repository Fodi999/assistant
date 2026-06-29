import { FormEvent, ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { AppIcon } from '../../components/AppIcon';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { AdminPanel } from '../../components/admin/AdminPanel';
import { EmptyState } from '../../components/admin/EmptyState';
import { StatusBadge } from '../../components/admin/StatusBadge';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { isValidSlug, isValidUrl } from '../../components/admin/forms/formUtils';
import { useActiveSite } from '../../lib/useActiveSite';
import {
  churchContentApi,
  type ChurchArticle,
  type ChurchArticlePayload,
  type ChurchCalendarType,
  type ChurchCalendarDay,
  type ChurchCalendarDayPayload,
  type ChurchContentStatus,
  type ChurchDayType,
  type ChurchIcon,
  type ChurchIconPayload,
  type ChurchImportPreview,
  type ChurchLanguage,
  type ChurchPrayer,
  type ChurchPrayerPayload,
  type ChurchPrayerType
} from '../../api/churchContent';

type DetailTab = 'overview' | 'icons' | 'prayers' | 'articles' | 'publish';
type DrawerState =
  | { type: 'day'; item?: ChurchCalendarDay }
  | { type: 'icon'; item?: ChurchIcon }
  | { type: 'prayer'; item?: ChurchPrayer }
  | { type: 'article'; item?: ChurchArticle }
  | null;

const detailTabs: Array<{ id: DetailTab; label: string; icon: 'dashboard' | 'qr' | 'sparkles' | 'cms' | 'seo' }> = [
  { id: 'overview', label: 'Overview', icon: 'dashboard' },
  { id: 'icons', label: 'Icons', icon: 'qr' },
  { id: 'prayers', label: 'Prayers', icon: 'sparkles' },
  { id: 'articles', label: 'Articles', icon: 'cms' },
  { id: 'publish', label: 'SEO / Publish', icon: 'seo' }
];

const dayTypes: ChurchDayType[] = ['feast', 'saint', 'fasting', 'memorial', 'gospel', 'quiet'];
const statuses: ChurchContentStatus[] = ['draft', 'published', 'archived'];
const languages: ChurchLanguage[] = ['uk', 'ru', 'en'];
const prayerTypes: ChurchPrayerType[] = ['prayer', 'akathist', 'troparion', 'kontakion'];
const churchPublicUrl = (import.meta.env.VITE_CHURCH_PUBLIC_URL || 'https://svet-ikony.fodi85999.workers.dev').replace(/\/+$/, '');
const churchPreviewToken = import.meta.env.VITE_CHURCH_PREVIEW_TOKEN || '';

export function ChurchContentPage() {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [days, setDays] = useState<ChurchCalendarDay[]>([]);
  const [icons, setIcons] = useState<ChurchIcon[]>([]);
  const [prayers, setPrayers] = useState<ChurchPrayer[]>([]);
  const [articles, setArticles] = useState<ChurchArticle[]>([]);
  const [selectedDayId, setSelectedDayId] = useState('');
  const [activeTab, setActiveTab] = useState<DetailTab>('overview');
  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [importPreview, setImportPreview] = useState<ChurchImportPreview | null>(null);
  const [importing, setImporting] = useState(false);

  const selectedDay = days.find((day) => day.id === selectedDayId) ?? days[0] ?? null;
  const selectedDayContent = useMemo(() => {
    if (!selectedDay) return { icons: [], prayers: [], articles: [] };
    return {
      icons: icons.filter((item) => item.calendarDayId === selectedDay.id),
      prayers: prayers.filter((item) => item.calendarDayId === selectedDay.id),
      articles: articles.filter((item) => item.calendarDayId === selectedDay.id)
    };
  }, [articles, icons, prayers, selectedDay]);

  const loadContent = useCallback(async () => {
    if (activeSiteId !== 'church') return;
    setLoading(true);
    setError('');
    try {
      const query = { site: 'church' };
      const [nextDays, nextIcons, nextPrayers, nextArticles] = await Promise.all([
        churchContentApi.listCalendarDays(query),
        churchContentApi.listIcons(query),
        churchContentApi.listPrayers(query),
        churchContentApi.listArticles(query)
      ]);
      setDays(nextDays);
      setIcons(nextIcons);
      setPrayers(nextPrayers);
      setArticles(nextArticles);
      setSelectedDayId((current) => current && nextDays.some((day) => day.id === current) ? current : nextDays[0]?.id ?? '');
    } catch (loadError) {
      const message = loadError instanceof Error ? loadError.message : 'Не удалось загрузить Church Content.';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, [activeSiteId, toast]);

  useEffect(() => {
    void loadContent();
  }, [loadContent]);

  async function saveDay(payload: ChurchCalendarDayPayload, id?: string) {
    setSaving(true);
    try {
      const saved = id
        ? await churchContentApi.updateCalendarDay(id, payload, { site: 'church' })
        : await churchContentApi.createCalendarDay(payload, { site: 'church' });
      toast.success(id ? 'День обновлен.' : 'День создан.');
      setDrawer(null);
      await loadContent();
      setSelectedDayId(saved.id);
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить день.');
    } finally {
      setSaving(false);
    }
  }

  async function saveIcon(payload: ChurchIconPayload, id?: string) {
    if (!selectedDay) return;
    setSaving(true);
    try {
      const body = { ...payload, calendarDayId: selectedDay.id };
      await (id ? churchContentApi.updateIcon(id, body, { site: 'church' }) : churchContentApi.createIcon(body, { site: 'church' }));
      toast.success(id ? 'Икона обновлена.' : 'Икона добавлена.');
      setDrawer(null);
      await loadContent();
      setActiveTab('icons');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить икону.');
    } finally {
      setSaving(false);
    }
  }

  async function savePrayer(payload: ChurchPrayerPayload, id?: string) {
    if (!selectedDay) return;
    setSaving(true);
    try {
      const body = { ...payload, calendarDayId: selectedDay.id };
      await (id ? churchContentApi.updatePrayer(id, body, { site: 'church' }) : churchContentApi.createPrayer(body, { site: 'church' }));
      toast.success(id ? 'Молитва обновлена.' : 'Молитва добавлена.');
      setDrawer(null);
      await loadContent();
      setActiveTab('prayers');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить молитву.');
    } finally {
      setSaving(false);
    }
  }

  async function saveArticle(payload: ChurchArticlePayload, id?: string) {
    if (!selectedDay) return;
    setSaving(true);
    try {
      const body = { ...payload, calendarDayId: selectedDay.id };
      await (id ? churchContentApi.updateArticle(id, body, { site: 'church' }) : churchContentApi.createArticle(body, { site: 'church' }));
      toast.success(id ? 'Статья обновлена.' : 'Статья добавлена.');
      setDrawer(null);
      await loadContent();
      setActiveTab('articles');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить статью.');
    } finally {
      setSaving(false);
    }
  }

  async function deleteItem(type: 'icon' | 'prayer' | 'article', id: string, title: string) {
    if (!window.confirm(`Удалить "${title}"?`)) return;
    setSaving(true);
    try {
      if (type === 'icon') await churchContentApi.deleteIcon(id, { site: 'church' });
      if (type === 'prayer') await churchContentApi.deletePrayer(id, { site: 'church' });
      if (type === 'article') await churchContentApi.deleteArticle(id, { site: 'church' });
      toast.success('Материал удален.');
      await loadContent();
    } catch (deleteError) {
      toast.error(deleteError instanceof Error ? deleteError.message : 'Не удалось удалить материал.');
    } finally {
      setSaving(false);
    }
  }

  async function publishDay(status: ChurchContentStatus) {
    if (!selectedDay) return;
    await saveDay({ status }, selectedDay.id);
  }

  async function previewOldJsonImport() {
    setImporting(true);
    try {
      const preview = await churchContentApi.previewImport({ site: 'church' });
      setImportPreview(preview);
      toast.info('Import preview готов.');
    } catch (previewError) {
      toast.error(previewError instanceof Error ? previewError.message : 'Не удалось подготовить import preview.');
    } finally {
      setImporting(false);
    }
  }

  async function applyOldJsonImport() {
    if (!window.confirm('Импортировать старый JSON в новую Church Content модель? Старый JSON останется на месте.')) return;
    setImporting(true);
    try {
      const preview = await churchContentApi.applyImport({ site: 'church' });
      setImportPreview(preview);
      toast.success('Импорт применен.');
      await loadContent();
    } catch (importError) {
      toast.error(importError instanceof Error ? importError.message : 'Не удалось применить импорт.');
    } finally {
      setImporting(false);
    }
  }

  function openPublicPreview() {
    if (!selectedDay) return;
    const date = selectedDay.dateNewStyle || selectedDay.dateOldStyle;
    if (!date) {
      toast.error('Для preview нужна дата дня.');
      return;
    }
    const token = churchPreviewToken ? `?preview_token=${encodeURIComponent(churchPreviewToken)}` : '';
    window.open(`${churchPublicUrl}/church/calendar/${date}${token}`, '_blank', 'noopener,noreferrer');
  }

  if (activeSiteId !== 'church') {
    return (
      <section className="admin-resource-page">
        <AdminPageHeader eyebrow="Church Content" title="Church Content" description="This editor is available only for the church site." icon="cms" />
        <EmptyState icon="cms" title="Выберите church" description="Модель календаря, икон, молитв и статей относится к церковному сайту." />
      </section>
    );
  }

  return (
    <section className="admin-resource-page church-content-page">
      <AdminPageHeader
        eyebrow="Church Content"
        title="Church Calendar Content"
        description="Calendar day is the main node. Icons, prayers and articles are created inside the selected day."
        icon="cms"
        meta={<StatusBadge status="online" label="normalized api" />}
        actions={
          <div className="admin-header-actions">
            <ActionButton icon="sparkles" onClick={previewOldJsonImport} disabled={importing}>Preview import</ActionButton>
            <ActionButton icon="save" onClick={applyOldJsonImport} disabled={importing}>Apply import</ActionButton>
            <ActionButton icon="sparkles" tone="primary" onClick={() => setDrawer({ type: 'day' })}>Create day</ActionButton>
          </div>
        }
      />

      {error ? <p className="admin-soft-alert">{error}</p> : null}
      {importPreview ? <ImportPreviewPanel preview={importPreview} /> : null}

      <div className="church-content-shell">
        <AdminPanel title="Calendar Days" icon="calendar" meta={`${days.length} days`}>
          {loading ? <p className="admin-table-empty">Loading church calendar...</p> : null}
          {!loading && !days.length ? (
            <EmptyState icon="calendar" title="No calendar days" description="Create the first church calendar day, then attach icons, prayers and articles." action={<ActionButton icon="sparkles" tone="primary" onClick={() => setDrawer({ type: 'day' })}>Create day</ActionButton>} />
          ) : null}
          <div className="church-day-list">
            {days.map((day) => {
              const counts = contentCounts(day.id, icons, prayers, articles);
              return (
                <button key={day.id} className={selectedDay?.id === day.id ? 'active' : ''} type="button" onClick={() => setSelectedDayId(day.id)}>
                  <span>{formatDayDate(day)}</span>
                  <strong>{day.title}</strong>
                  <small>{day.dayType} · {counts.icons} icons · {counts.prayers} prayers · {counts.articles} articles</small>
                  <StatusBadge status={day.status === 'published' ? 'published' : day.status === 'archived' ? 'offline' : 'draft'} label={day.status} />
                </button>
              );
            })}
          </div>
        </AdminPanel>

        <AdminPanel
          title={selectedDay?.title ?? 'Select calendar day'}
          icon="cms"
          meta={selectedDay ? formatDayDate(selectedDay) : 'No day'}
        >
          {selectedDay ? (
            <>
              <div className="church-detail-tabs">
                {detailTabs.map((tab) => (
                  <button key={tab.id} type="button" className={activeTab === tab.id ? 'active' : ''} onClick={() => setActiveTab(tab.id)}>
                    <AppIcon name={tab.icon} />
                    <span>{tab.label}</span>
                  </button>
                ))}
              </div>

              {activeTab === 'overview' ? (
                <OverviewTab
                  day={selectedDay}
                  icons={selectedDayContent.icons}
                  prayers={selectedDayContent.prayers}
                  articles={selectedDayContent.articles}
                  onEdit={() => setDrawer({ type: 'day', item: selectedDay })}
                />
              ) : null}

              {activeTab === 'icons' ? (
                <ContentList
                  title="Icons"
                  empty="Add an icon for this calendar day."
                  action="Add icon"
                  onAdd={() => setDrawer({ type: 'icon' })}
                  rows={selectedDayContent.icons}
                  render={(icon) => (
                    <ContentRow
                      key={icon.id}
                      title={icon.title}
                      meta={`${icon.language} · ${icon.status} · /icons/${icon.slug}`}
                      image={icon.imageUrl}
                      onEdit={() => setDrawer({ type: 'icon', item: icon })}
                      onDelete={() => void deleteItem('icon', icon.id, icon.title)}
                    />
                  )}
                />
              ) : null}

              {activeTab === 'prayers' ? (
                <ContentList
                  title="Prayers"
                  empty="Add prayers, troparion or kontakion for this day."
                  action="Add prayer"
                  onAdd={() => setDrawer({ type: 'prayer' })}
                  rows={selectedDayContent.prayers}
                  render={(prayer) => (
                    <ContentRow
                      key={prayer.id}
                      title={prayer.title}
                      meta={`${prayer.language} · ${prayer.prayerType} · ${prayer.status} · /prayers/${prayer.slug}`}
                      body={prayer.text}
                      onEdit={() => setDrawer({ type: 'prayer', item: prayer })}
                      onDelete={() => void deleteItem('prayer', prayer.id, prayer.title)}
                    />
                  )}
                />
              ) : null}

              {activeTab === 'articles' ? (
                <ContentList
                  title="Articles"
                  empty="Add a life, feast meaning or SEO article for this day."
                  action="Add article"
                  onAdd={() => setDrawer({ type: 'article' })}
                  rows={selectedDayContent.articles}
                  render={(article) => (
                    <ContentRow
                      key={article.id}
                      title={article.title}
                      meta={`${article.language} · ${article.status} · /p/${article.slug}`}
                      body={article.seoDescription || article.content}
                      onEdit={() => setDrawer({ type: 'article', item: article })}
                      onDelete={() => void deleteItem('article', article.id, article.title)}
                    />
                  )}
                />
              ) : null}

              {activeTab === 'publish' ? (
                <PublishTab
                  day={selectedDay}
                  icons={selectedDayContent.icons}
                  prayers={selectedDayContent.prayers}
                  articles={selectedDayContent.articles}
                  saving={saving}
                  onPublish={() => void publishDay('published')}
                  onDraft={() => void publishDay('draft')}
                  onPreview={openPublicPreview}
                />
              ) : null}
            </>
          ) : (
            <EmptyState icon="calendar" title="No day selected" description="Create or select a calendar day to attach content." />
          )}
        </AdminPanel>
      </div>

      <EditorDrawer
        drawer={drawer}
        day={selectedDay}
        icons={selectedDayContent.icons}
        saving={saving}
        onClose={() => setDrawer(null)}
        onSaveDay={saveDay}
        onSaveIcon={saveIcon}
        onSavePrayer={savePrayer}
        onSaveArticle={saveArticle}
      />
    </section>
  );
}

function OverviewTab({ day, icons, prayers, articles, onEdit }: { day: ChurchCalendarDay; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; onEdit: () => void }) {
  return (
    <div className="church-overview">
      <div>
        <span>{formatDayDate(day)}</span>
        <h3>{day.title}</h3>
        <p>{day.description || 'Add a clear description for editors and public page previews.'}</p>
        <ActionButton icon="sliders" onClick={onEdit}>Edit day</ActionButton>
      </div>
      <dl>
        <dt>Type</dt><dd>{day.dayType}</dd>
        <dt>Calendar</dt><dd>{day.calendarType}</dd>
        <dt>Rank</dt><dd>{day.rank}</dd>
        <dt>Status</dt><dd>{day.status}</dd>
        <dt>Icons</dt><dd>{icons.length}</dd>
        <dt>Prayers</dt><dd>{prayers.length}</dd>
        <dt>Articles</dt><dd>{articles.length}</dd>
      </dl>
    </div>
  );
}

function ContentList<T>({ title, empty, action, rows, render, onAdd }: { title: string; empty: string; action: string; rows: T[]; render: (row: T) => ReactNode; onAdd: () => void }) {
  return (
    <div className="church-content-list-panel">
      <div className="church-content-list-head">
        <strong>{title}</strong>
        <ActionButton icon="sparkles" tone="primary" onClick={onAdd}>{action}</ActionButton>
      </div>
      {rows.length ? <div className="church-content-list">{rows.map(render)}</div> : <EmptyState icon="cms" title="Nothing yet" description={empty} action={<ActionButton icon="sparkles" tone="primary" onClick={onAdd}>{action}</ActionButton>} />}
    </div>
  );
}

function ContentRow({ title, meta, body, image, onEdit, onDelete }: { title: string; meta: string; body?: string; image?: string; onEdit: () => void; onDelete: () => void }) {
  return (
    <article className="church-content-row">
      {image ? <img src={image} alt={title} loading="lazy" /> : <span><AppIcon name="cms" /></span>}
      <div>
        <strong>{title}</strong>
        <small>{meta}</small>
        {body ? <p>{body}</p> : null}
      </div>
      <div className="admin-table-actions">
        <button className="table-action" type="button" onClick={onEdit}>Edit</button>
        <button className="table-action danger" type="button" onClick={onDelete}>Delete</button>
      </div>
    </article>
  );
}

function ImportPreviewPanel({ preview }: { preview: ChurchImportPreview }) {
  return (
    <AdminPanel title="Import preview" icon="sparkles" meta="old JSON">
      <div className="church-import-preview">
        <span><strong>{preview.calendarDays}</strong> calendar days</span>
        <span><strong>{preview.icons}</strong> icons</span>
        <span><strong>{preview.prayers}</strong> prayers</span>
        <span><strong>{preview.articles}</strong> articles</span>
      </div>
      {preview.errors.length ? (
        <div className="admin-soft-alert danger">
          <strong>Ошибки импорта</strong>
          {preview.errors.slice(0, 8).map((error) => <small key={error}>{error}</small>)}
          {preview.errors.length > 8 ? <small>Еще {preview.errors.length - 8}...</small> : null}
        </div>
      ) : null}
      {preview.warnings.length ? (
        <div className="admin-soft-alert">
          <strong>Предупреждения</strong>
          {preview.warnings.slice(0, 6).map((warning) => <small key={warning}>{warning}</small>)}
        </div>
      ) : null}
    </AdminPanel>
  );
}

function PublishTab({ day, icons, prayers, articles, saving, onPublish, onDraft, onPreview }: { day: ChurchCalendarDay; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; saving: boolean; onPublish: () => void; onDraft: () => void; onPreview: () => void }) {
  const draftItems = [
    ...icons.filter((item) => item.status === 'draft').map((item) => item.title),
    ...prayers.filter((item) => item.status === 'draft').map((item) => item.title),
    ...articles.filter((item) => item.status === 'draft').map((item) => item.title)
  ];
  const checks: Array<{ label: string; ok: boolean; tone?: 'warn' }> = [
    { label: 'Calendar day title', ok: Boolean(day.title.trim()) },
    { label: 'New or old style date', ok: Boolean(day.dateNewStyle || day.dateOldStyle) },
    { label: 'At least one icon', ok: icons.length > 0 },
    { label: 'Prayer or troparion', ok: prayers.length > 0 },
    { label: 'Article / description', ok: articles.length > 0 || Boolean(day.description.trim()) },
    { label: 'No missing icon slug', ok: icons.every((item) => isValidSlug(item.slug)) },
    { label: 'No missing prayer slug', ok: prayers.every((item) => isValidSlug(item.slug)) },
    { label: 'No missing article slug', ok: articles.every((item) => isValidSlug(item.slug)) },
    { label: 'Icon image exists', ok: icons.length > 0 && icons.every((item) => isValidUrl(item.imageUrl)) },
    { label: 'SEO title exists', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoTitle.trim())) },
    { label: 'SEO description exists', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoDescription.trim())) },
    { label: draftItems.length ? `Draft without reason: ${draftItems.slice(0, 2).join(', ')}` : 'No unexplained drafts', ok: draftItems.length === 0, tone: 'warn' }
  ];
  return (
    <div className="church-publish-panel">
      <div className="church-publish-checks">
        {checks.map((check) => <span key={check.label} className={check.ok ? 'ok' : check.tone === 'warn' ? 'warn' : ''}><i />{check.label}</span>)}
      </div>
      <div className="church-publish-actions">
        <ActionButton icon="qr" onClick={onPreview} disabled={saving}>Preview</ActionButton>
        <ActionButton onClick={onDraft} disabled={saving}>Move to draft</ActionButton>
        <ActionButton tone="primary" icon="save" onClick={onPublish} disabled={saving}>Publish day</ActionButton>
      </div>
    </div>
  );
}

function EditorDrawer(props: {
  drawer: DrawerState;
  day: ChurchCalendarDay | null;
  icons: ChurchIcon[];
  saving: boolean;
  onClose: () => void;
  onSaveDay: (payload: ChurchCalendarDayPayload, id?: string) => Promise<void>;
  onSaveIcon: (payload: ChurchIconPayload, id?: string) => Promise<void>;
  onSavePrayer: (payload: ChurchPrayerPayload, id?: string) => Promise<void>;
  onSaveArticle: (payload: ChurchArticlePayload, id?: string) => Promise<void>;
}) {
  if (!props.drawer) return null;
  const title = props.drawer.type === 'day' ? 'Calendar day'
    : props.drawer.type === 'icon' ? 'Icon'
      : props.drawer.type === 'prayer' ? 'Prayer'
        : 'Article';
  return (
    <AdminDrawer open title={title} description={props.day ? `Attached to: ${props.day.title}` : 'Create the calendar day first.'} onClose={props.onClose}>
      {props.drawer.type === 'day' ? <CalendarDayForm item={props.drawer.item} saving={props.saving} onSubmit={props.onSaveDay} /> : null}
      {props.drawer.type === 'icon' ? <ChurchIconForm item={props.drawer.item} saving={props.saving} onSubmit={props.onSaveIcon} /> : null}
      {props.drawer.type === 'prayer' ? <ChurchPrayerForm item={props.drawer.item} icons={props.icons} saving={props.saving} onSubmit={props.onSavePrayer} /> : null}
      {props.drawer.type === 'article' ? <ChurchArticleForm item={props.drawer.item} icons={props.icons} saving={props.saving} onSubmit={props.onSaveArticle} /> : null}
    </AdminDrawer>
  );
}

function CalendarDayForm({ item, saving, onSubmit }: { item?: ChurchCalendarDay; saving: boolean; onSubmit: (payload: ChurchCalendarDayPayload, id?: string) => Promise<void> }) {
  const [form, setForm] = useState({
    dateOldStyle: item?.dateOldStyle ?? '',
    dateNewStyle: item?.dateNewStyle ?? '',
    calendarType: item?.calendarType ?? 'both',
    title: item?.title ?? '',
    dayType: item?.dayType ?? 'saint',
    description: item?.description ?? '',
    rank: String(item?.rank ?? 0),
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название дня обязательно.');
    if (!form.dateOldStyle && !form.dateNewStyle) return setError('Укажите дату по старому или новому стилю.');
    setError('');
    void onSubmit({ ...form, rank: Number(form.rank) || 0 } as ChurchCalendarDayPayload, item?.id);
  }
  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label><span>Date old style</span><input type="date" value={form.dateOldStyle} onChange={(event) => setForm({ ...form, dateOldStyle: event.target.value })} /></label>
      <label><span>Date new style</span><input type="date" value={form.dateNewStyle} onChange={(event) => setForm({ ...form, dateNewStyle: event.target.value })} /></label>
      <label><span>Title</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Святитель Николай" /></label>
      <label><span>Type</span><select value={form.dayType} onChange={(event) => setForm({ ...form, dayType: event.target.value as ChurchDayType })}>{dayTypes.map((type) => <option key={type} value={type}>{type}</option>)}</select></label>
      <label><span>Calendar type</span><select value={form.calendarType} onChange={(event) => setForm({ ...form, calendarType: event.target.value as ChurchCalendarType })}><option value="both">both</option><option value="old_style">old style</option><option value="new_style">new style</option></select></label>
      <label><span>Rank</span><input type="number" value={form.rank} onChange={(event) => setForm({ ...form, rank: event.target.value })} /></label>
      <label><span>Status</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{status}</option>)}</select></label>
      <label><span>Description</span><textarea value={form.description} onChange={(event) => setForm({ ...form, description: event.target.value })} placeholder="Краткое описание дня, праздника или памяти святого" /></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="save" type="submit" disabled={saving}>{saving ? 'Saving' : 'Save'}</ActionButton>
    </form>
  );
}

function ChurchIconForm({ item, saving, onSubmit }: { item?: ChurchIcon; saving: boolean; onSubmit: (payload: ChurchIconPayload, id?: string) => Promise<void> }) {
  const [form, setForm] = useState({
    title: item?.title ?? '',
    slug: item?.slug ?? '',
    imageUrl: item?.imageUrl ?? '',
    saintName: item?.saintName ?? '',
    feastName: item?.feastName ?? '',
    description: item?.description ?? '',
    language: item?.language ?? 'uk',
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название иконы обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!isValidUrl(form.imageUrl)) return setError('Фото должно быть http/https URL.');
    setError('');
    void onSubmit(form as ChurchIconPayload, item?.id);
  }
  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label><span>Title</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Ікона святителя Миколая" /></label>
      <label><span>Slug</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="nikolay-chudotvorets" /></label>
      <label><span>Image URL</span><input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://..." /></label>
      <label><span>Saint name</span><input value={form.saintName} onChange={(event) => setForm({ ...form, saintName: event.target.value })} /></label>
      <label><span>Feast name</span><input value={form.feastName} onChange={(event) => setForm({ ...form, feastName: event.target.value })} /></label>
      <label><span>Language</span><select value={form.language} onChange={(event) => setForm({ ...form, language: event.target.value as ChurchLanguage })}>{languages.map((lang) => <option key={lang} value={lang}>{lang.toUpperCase()}</option>)}</select></label>
      <label><span>Status</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{status}</option>)}</select></label>
      <label><span>Description</span><textarea value={form.description} onChange={(event) => setForm({ ...form, description: event.target.value })} /></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="save" type="submit" disabled={saving}>{saving ? 'Saving' : 'Save icon'}</ActionButton>
    </form>
  );
}

function ChurchPrayerForm({ item, icons, saving, onSubmit }: { item?: ChurchPrayer; icons: ChurchIcon[]; saving: boolean; onSubmit: (payload: ChurchPrayerPayload, id?: string) => Promise<void> }) {
  const [form, setForm] = useState({
    iconId: item?.iconId ?? '',
    slug: item?.slug ?? '',
    title: item?.title ?? '',
    text: item?.text ?? '',
    language: item?.language ?? 'uk',
    prayerType: item?.prayerType ?? 'prayer',
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название молитвы обязательно.');
    if (!isValidSlug(form.slug)) return setError('Public URL slug должен быть в формате latin-slug.');
    if (!form.text.trim()) return setError('Текст молитвы обязателен.');
    setError('');
    void onSubmit({ ...form, iconId: form.iconId || undefined } as ChurchPrayerPayload, item?.id);
  }
  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label><span>Related icon in this day</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">No icon selected</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
      <label><span>Type</span><select value={form.prayerType} onChange={(event) => setForm({ ...form, prayerType: event.target.value as ChurchPrayerType })}>{prayerTypes.map((type) => <option key={type} value={type}>{type}</option>)}</select></label>
      <label><span>Title</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Тропарь Рождества" /></label>
      <label><span>Public URL slug</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="tropar-rozhdestva" /></label>
      <label><span>Language</span><select value={form.language} onChange={(event) => setForm({ ...form, language: event.target.value as ChurchLanguage })}>{languages.map((lang) => <option key={lang} value={lang}>{lang.toUpperCase()}</option>)}</select></label>
      <label><span>Status</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{status}</option>)}</select></label>
      <label><span>Text</span><textarea value={form.text} onChange={(event) => setForm({ ...form, text: event.target.value })} /></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="save" type="submit" disabled={saving}>{saving ? 'Saving' : 'Save prayer'}</ActionButton>
    </form>
  );
}

function ChurchArticleForm({ item, icons, saving, onSubmit }: { item?: ChurchArticle; icons: ChurchIcon[]; saving: boolean; onSubmit: (payload: ChurchArticlePayload, id?: string) => Promise<void> }) {
  const [form, setForm] = useState({
    iconId: item?.iconId ?? '',
    title: item?.title ?? '',
    slug: item?.slug ?? '',
    content: item?.content ?? '',
    language: item?.language ?? 'uk',
    seoTitle: item?.seoTitle ?? '',
    seoDescription: item?.seoDescription ?? '',
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название статьи обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.content.trim()) return setError('Текст статьи обязателен.');
    setError('');
    void onSubmit({ ...form, iconId: form.iconId || undefined } as ChurchArticlePayload, item?.id);
  }
  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label><span>Related icon in this day</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">No icon selected</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
      <label><span>Title</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} /></label>
      <label><span>Slug</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} /></label>
      <label><span>Language</span><select value={form.language} onChange={(event) => setForm({ ...form, language: event.target.value as ChurchLanguage })}>{languages.map((lang) => <option key={lang} value={lang}>{lang.toUpperCase()}</option>)}</select></label>
      <label><span>SEO title</span><input value={form.seoTitle} onChange={(event) => setForm({ ...form, seoTitle: event.target.value })} /></label>
      <label><span>SEO description</span><textarea value={form.seoDescription} onChange={(event) => setForm({ ...form, seoDescription: event.target.value })} /></label>
      <label><span>Status</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{status}</option>)}</select></label>
      <label><span>Content</span><textarea value={form.content} onChange={(event) => setForm({ ...form, content: event.target.value })} /></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="save" type="submit" disabled={saving}>{saving ? 'Saving' : 'Save article'}</ActionButton>
    </form>
  );
}

function contentCounts(dayId: string, icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[]) {
  return {
    icons: icons.filter((item) => item.calendarDayId === dayId).length,
    prayers: prayers.filter((item) => item.calendarDayId === dayId).length,
    articles: articles.filter((item) => item.calendarDayId === dayId).length
  };
}

function formatDayDate(day: ChurchCalendarDay) {
  return [day.dateNewStyle, day.dateOldStyle ? `${day.dateOldStyle} old` : ''].filter(Boolean).join(' / ') || 'No date';
}
