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
import { generateAiImage, generateSeo } from '../../api/ai';
import { getIconsSiteContent, type IconsSiteContent } from '../../api/iconsSite';
import { uploadCmsReference } from '../../api/cms';
import type { AiGenerationResult } from '../../types/admin';
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
  | { type: 'ai' }
  | null;

const detailTabs: Array<{ id: DetailTab; label: string; icon: 'dashboard' | 'qr' | 'sparkles' | 'cms' | 'seo' }> = [
  { id: 'overview', label: 'Saint / Feast', icon: 'dashboard' },
  { id: 'icons', label: 'Icon', icon: 'qr' },
  { id: 'prayers', label: 'Prayer', icon: 'sparkles' },
  { id: 'articles', label: 'Article', icon: 'cms' },
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
  const [legacyContent, setLegacyContent] = useState<IconsSiteContent | null>(null);
  const [importing, setImporting] = useState(false);
  const [aiCreating, setAiCreating] = useState(false);

  const selectedDay = days.find((day) => day.id === selectedDayId) ?? days[0] ?? null;
  const unlinkedArticles = useMemo(() => articles.filter((item) => !item.calendarDayId), [articles]);
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
      const legacy = await getIconsSiteContent().catch(() => null);
      setDays(nextDays);
      setIcons(nextIcons);
      setPrayers(nextPrayers);
      setArticles(nextArticles);
      setLegacyContent(legacy);
      setSelectedDayId((current) => current && nextDays.some((day) => day.id === current) ? current : nextDays[0]?.id ?? '');
      if (!nextDays.length && !nextIcons.length && !nextPrayers.length && !nextArticles.length) {
        const preview = await churchContentApi.previewImport(query).catch(() => null);
        setImportPreview(preview);
      }
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
      setActiveTab(id ? 'icons' : 'prayers');
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
      const fallbackIconId = selectedDayContent.icons[0]?.id;
      const body = { ...payload, calendarDayId: selectedDay.id, iconId: payload.iconId || fallbackIconId };
      await (id ? churchContentApi.updatePrayer(id, body, { site: 'church' }) : churchContentApi.createPrayer(body, { site: 'church' }));
      toast.success(id ? 'Молитва обновлена.' : 'Молитва добавлена.');
      setDrawer(null);
      await loadContent();
      setActiveTab(id ? 'prayers' : 'articles');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить молитву.');
    } finally {
      setSaving(false);
    }
  }

  async function saveArticle(payload: ChurchArticlePayload, id?: string) {
    if (!selectedDay && !id) return;
    setSaving(true);
    try {
      const fallbackIconId = selectedDayContent.icons[0]?.id;
      const calendarDayId = payload.calendarDayId !== undefined ? payload.calendarDayId : id ? undefined : selectedDay?.id;
      const iconId = payload.iconId !== undefined ? payload.iconId : id ? undefined : fallbackIconId;
      const body = { ...payload, calendarDayId, iconId };
      await (id ? churchContentApi.updateArticle(id, body, { site: 'church' }) : churchContentApi.createArticle(body, { site: 'church' }));
      toast.success(id ? 'Статья обновлена.' : 'Статья добавлена.');
      setDrawer(null);
      await loadContent();
      setActiveTab(id ? 'articles' : 'publish');
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
      setLegacyContent(null);
      await loadContent();
    } catch (importError) {
      toast.error(importError instanceof Error ? importError.message : 'Не удалось применить импорт.');
    } finally {
      setImporting(false);
    }
  }

  async function createGeminiChain(payload: ChurchAiDraftPayload) {
    setAiCreating(true);
    setSaving(true);
    setError('');
    try {
      const result = await generateSeo({
        site: 'icons',
        language: 'ru',
        type: 'seo',
        tone: 'seo',
        keywords: ['church-content', payload.topic, payload.dayType, 'икона', 'молитва', 'статья'],
        sourceText: buildChurchAiPrompt(payload)
      });
      const draft = parseChurchAiDraft(result, payload);
      let imageUrl = payload.imageUrl.trim();
      if (payload.generateImage && !imageUrl) {
        try {
          const image = await generateAiImage({
            site: 'icons',
            title: draft.iconTitle,
            description: draft.iconDescription,
            scene: draft.photoPrompt,
            imageType: 'calendar',
            enhanced: true,
            variant: Date.now() % 9
          });
          imageUrl = image.imageUrl;
        } catch (imageError) {
          toast.error(imageError instanceof Error ? imageError.message : 'Gemini не смог сгенерировать фото, текстовая цепочка будет создана без фото.');
        }
      }

      const day = await churchContentApi.createCalendarDay({
        dateOldStyle: payload.dateOldStyle || undefined,
        dateNewStyle: payload.dateNewStyle || undefined,
        calendarType: payload.calendarType,
        title: draft.dayTitle,
        dayType: draft.dayType,
        description: draft.dayDescription,
        rank: Number(payload.rank) || 0,
        status: 'draft'
      }, { site: 'church' });
      const icon = await churchContentApi.createIcon({
        calendarDayId: day.id,
        title: draft.iconTitle,
        slug: draft.iconSlug,
        imageUrl,
        saintName: draft.saintName,
        feastName: draft.feastName,
        description: draft.iconDescription,
        language: payload.language,
        status: 'draft'
      }, { site: 'church' });
      await churchContentApi.createPrayer({
        calendarDayId: day.id,
        iconId: icon.id,
        title: draft.prayerTitle,
        slug: draft.prayerSlug,
        text: draft.prayerText,
        language: payload.language,
        prayerType: draft.prayerType,
        status: 'draft'
      }, { site: 'church' });
      await churchContentApi.createArticle({
        calendarDayId: day.id,
        iconId: icon.id,
        title: draft.articleTitle,
        slug: draft.articleSlug,
        content: draft.articleContent,
        language: payload.language,
        seoTitle: draft.seoTitle,
        seoDescription: draft.seoDescription,
        status: 'draft'
      }, { site: 'church' });
      setImportPreview(null);
      setDrawer(null);
      await loadContent();
      setSelectedDayId(day.id);
      setActiveTab('icons');
      toast.success('Gemini создал черновик: день, икона, молитва и статья.');
    } catch (aiError) {
      const message = aiError instanceof Error ? aiError.message : 'Gemini не смог создать цепочку.';
      setError(message);
      toast.error(message);
    } finally {
      setAiCreating(false);
      setSaving(false);
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
        description="Create content as a connected chain: day, saint or feast, icon, prayer, article, publish."
        icon="cms"
        meta={<StatusBadge status="online" label="normalized api" />}
        actions={
          <div className="admin-header-actions">
            <ActionButton icon="sparkles" onClick={previewOldJsonImport} disabled={importing}>Preview import</ActionButton>
            <ActionButton icon="save" onClick={applyOldJsonImport} disabled={importing}>Apply import</ActionButton>
            <ActionButton icon="bot" tone="primary" onClick={() => setDrawer({ type: 'ai' })} disabled={aiCreating}>{aiCreating ? 'Gemini...' : 'Gemini fill'}</ActionButton>
            <ActionButton icon="sparkles" tone="primary" onClick={() => setDrawer({ type: 'day' })}>Create day</ActionButton>
          </div>
        }
      />

      {error ? <p className="admin-soft-alert">{error}</p> : null}
      {importPreview && !days.length ? <ImportPreviewPanel preview={importPreview} /> : null}
      {legacyContent ? <LegacyContentPanel content={legacyContent} importing={importing} onImport={applyOldJsonImport} /> : null}
      {unlinkedArticles.length ? (
        <UnlinkedArticlesPanel
          articles={unlinkedArticles}
          icons={icons}
          onEdit={(article) => setDrawer({ type: 'article', item: article })}
          onDelete={(article) => void deleteItem('article', article.id, article.title)}
        />
      ) : null}

      <div className="church-content-shell">
        <AdminPanel title="Calendar Days" icon="calendar" meta={`${days.length} days`}>
          {loading ? <p className="admin-table-empty">Loading church calendar...</p> : null}
          {!loading && !days.length ? (
            <EmptyState icon="calendar" title="No calendar days" description="Create the first church calendar day, then attach icons, prayers and articles." action={<div className="church-empty-actions"><ActionButton icon="bot" tone="primary" onClick={() => setDrawer({ type: 'ai' })} disabled={aiCreating}>Gemini fill</ActionButton><ActionButton icon="sparkles" onClick={() => setDrawer({ type: 'day' })}>Create manually</ActionButton></div>} />
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

              <ChurchPublishingFlow
                day={selectedDay}
                icons={selectedDayContent.icons}
                prayers={selectedDayContent.prayers}
                articles={selectedDayContent.articles}
                onEditDay={() => {
                  setActiveTab('overview');
                  setDrawer({ type: 'day', item: selectedDay });
                }}
                onAddIcon={() => {
                  setActiveTab('icons');
                  setDrawer({ type: 'icon' });
                }}
                onAddPrayer={() => {
                  setActiveTab('prayers');
                  setDrawer({ type: 'prayer' });
                }}
                onAddArticle={() => {
                  setActiveTab('articles');
                  setDrawer({ type: 'article' });
                }}
                onPublish={() => setActiveTab('publish')}
              />

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
                  empty={selectedDayContent.icons.length ? 'Add prayers, troparion or kontakion for this icon and day.' : 'Create an icon first, then attach prayers to it.'}
                  action="Add prayer"
                  onAdd={() => setDrawer({ type: 'prayer' })}
                  actionDisabled={!selectedDayContent.icons.length}
                  disabledReason="Create icon first"
                  rows={selectedDayContent.prayers}
                  render={(prayer) => (
                    <ContentRow
                      key={prayer.id}
                      title={prayer.title}
                      meta={`${prayer.language} · ${prayer.prayerType} · ${prayer.status} · /prayers/${prayer.slug}`}
                      body={prayer.text}
                      image={imageForContent(prayer.iconId, selectedDayContent.icons, icons)}
                      onEdit={() => setDrawer({ type: 'prayer', item: prayer })}
                      onDelete={() => void deleteItem('prayer', prayer.id, prayer.title)}
                    />
                  )}
                />
              ) : null}

              {activeTab === 'articles' ? (
                <ContentList
                  title="Articles"
                  empty={selectedDayContent.prayers.length ? 'Add a life, feast meaning or SEO article linked to the icon and day.' : 'Create an icon and prayer first, then attach article content to the chain.'}
                  action="Add article"
                  onAdd={() => setDrawer({ type: 'article' })}
                  actionDisabled={!selectedDayContent.prayers.length}
                  disabledReason="Create prayer first"
                  rows={selectedDayContent.articles}
                  render={(article) => (
                    <ContentRow
                      key={article.id}
                      title={article.title}
                      meta={`${article.language} · ${article.status} · /p/${article.slug}`}
                      body={article.seoDescription || article.content}
                      image={imageForContent(article.iconId, selectedDayContent.icons, icons)}
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
                  onOpenPath={(path) => window.open(`${churchPublicUrl}${path}`, '_blank', 'noopener,noreferrer')}
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
        days={days}
        icons={selectedDayContent.icons}
        allIcons={icons}
        saving={saving}
        aiCreating={aiCreating}
        onClose={() => setDrawer(null)}
        onSaveDay={saveDay}
        onSaveIcon={saveIcon}
        onSavePrayer={savePrayer}
        onSaveArticle={saveArticle}
        onCreateGeminiChain={createGeminiChain}
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
        <p>{day.description || 'Опишите, кто или какой праздник связан с этим днем. Это главный узел для икон, молитв и статьи.'}</p>
        <ActionButton icon="sliders" onClick={onEdit}>Edit saint / feast</ActionButton>
      </div>
      <dl>
        <dt>Node</dt><dd>{day.dayType}</dd>
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

function ChurchPublishingFlow({ day, icons, prayers, articles, onEditDay, onAddIcon, onAddPrayer, onAddArticle, onPublish }: {
  day: ChurchCalendarDay;
  icons: ChurchIcon[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  onEditDay: () => void;
  onAddIcon: () => void;
  onAddPrayer: () => void;
  onAddArticle: () => void;
  onPublish: () => void;
}) {
  const hasDay = Boolean(day.title && (day.dateNewStyle || day.dateOldStyle));
  const hasIcon = icons.length > 0;
  const hasPrayer = prayers.length > 0;
  const hasArticle = articles.length > 0;
  const steps = [
    { index: '01', label: 'День', value: formatDayDate(day), ready: hasDay, action: 'Редактировать', onClick: onEditDay },
    { index: '02', label: 'Святой / праздник', value: day.title || 'Заполните название', ready: Boolean(day.title), action: 'Открыть', onClick: onEditDay },
    { index: '03', label: 'Икона', value: hasIcon ? `${icons.length} linked` : 'Создайте икону', ready: hasIcon, action: hasIcon ? 'Добавить ещё' : 'Создать', onClick: onAddIcon, disabled: !hasDay },
    { index: '04', label: 'Молитва', value: hasPrayer ? `${prayers.length} linked` : 'После иконы', ready: hasPrayer, action: hasPrayer ? 'Добавить ещё' : 'Создать', onClick: onAddPrayer, disabled: !hasIcon },
    { index: '05', label: 'Статья', value: hasArticle ? `${articles.length} linked` : 'После молитвы', ready: hasArticle, action: hasArticle ? 'Добавить ещё' : 'Создать', onClick: onAddArticle, disabled: !hasPrayer },
    { index: '06', label: 'Публикация', value: day.status, ready: day.status === 'published', action: 'Проверить', onClick: onPublish, disabled: !hasIcon || !hasPrayer || !hasArticle }
  ];

  return (
    <div className="church-flow">
      {steps.map((step) => (
        <button key={step.index} type="button" className={step.ready ? 'ready' : ''} onClick={step.onClick} disabled={step.disabled}>
          <span>{step.index}</span>
          <strong>{step.label}</strong>
          <small>{step.value}</small>
          <em>{step.disabled ? 'Сначала предыдущий шаг' : step.action}</em>
        </button>
      ))}
    </div>
  );
}

function ContentList<T>({ title, empty, action, rows, render, onAdd, actionDisabled, disabledReason }: { title: string; empty: string; action: string; rows: T[]; render: (row: T) => ReactNode; onAdd: () => void; actionDisabled?: boolean; disabledReason?: string }) {
  return (
    <div className="church-content-list-panel">
      <div className="church-content-list-head">
        <strong>{title}</strong>
        <ActionButton icon="sparkles" tone="primary" onClick={onAdd} disabled={actionDisabled}>{actionDisabled ? disabledReason || action : action}</ActionButton>
      </div>
      {rows.length ? <div className="church-content-list">{rows.map(render)}</div> : <EmptyState icon="cms" title="Nothing yet" description={empty} action={<ActionButton icon="sparkles" tone="primary" onClick={onAdd} disabled={actionDisabled}>{actionDisabled ? disabledReason || action : action}</ActionButton>} />}
    </div>
  );
}

function ContentRow({ title, meta, body, image, onEdit, onDelete }: { title: string; meta: string; body?: string; image?: string; onEdit: () => void; onDelete: () => void }) {
  return (
    <article className="church-content-row">
      {image ? <img src={image} alt={title} loading="lazy" /> : <span><AppIcon name="image" /></span>}
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

function LegacyContentPanel({ content, importing, onImport }: { content: IconsSiteContent; importing: boolean; onImport: () => void }) {
  const legacyItems = [
    ...content.icons.map((item) => ({ key: `icon-${item.id}`, title: item.title, meta: `${item.status} · ${item.calendarDate || 'no date'} · icon` })),
    ...content.prayers.map((item) => ({ key: `prayer-${item.id}`, title: item.title, meta: `${item.status} · prayer` })),
    ...content.pages.map((item) => ({ key: `page-${item.id}`, title: item.title || item.h1, meta: `${item.status} · SEO page` }))
  ].filter((item) => item.title).slice(0, 8);

  return (
    <AdminPanel title="Old site publications" icon="cms" meta={`${content.icons.length} icons · ${content.prayers.length} prayers · ${content.pages.length} pages`}>
      <div className="legacy-content-bridge">
        <div>
          <strong>Старые публикации сайта не удалены</strong>
          <span>Январские карточки на сайте сейчас берутся из старого хранилища icons-site. Нажмите Import to editor, чтобы перенести их в новый редактор Church Content.</span>
        </div>
        <ActionButton icon="save" tone="primary" onClick={onImport} disabled={importing}>{importing ? 'Importing' : 'Import to editor'}</ActionButton>
      </div>
      {legacyItems.length ? (
        <div className="legacy-content-list">
          {legacyItems.map((item) => (
            <span key={item.key}>
              <strong>{item.title}</strong>
              <small>{item.meta}</small>
            </span>
          ))}
        </div>
      ) : null}
    </AdminPanel>
  );
}

function UnlinkedArticlesPanel({ articles, icons, onEdit, onDelete }: { articles: ChurchArticle[]; icons: ChurchIcon[]; onEdit: (article: ChurchArticle) => void; onDelete: (article: ChurchArticle) => void }) {
  return (
    <AdminPanel title="Needs linking" icon="cms" meta={`${articles.length} articles`}>
      <p className="church-panel-note">Это уже созданные статьи из базы. Откройте Edit, выберите календарный день и икону, затем сохраните.</p>
      <div className="church-content-list">
        {articles.map((article) => (
          <ContentRow
            key={article.id}
            title={article.title}
            meta={`${article.language} · ${article.status} · /church/articles/${article.slug}`}
            body={article.seoDescription || article.content}
            image={imageForContent(article.iconId, [], icons)}
            onEdit={() => onEdit(article)}
            onDelete={() => onDelete(article)}
          />
        ))}
      </div>
    </AdminPanel>
  );
}

function PublishTab({ day, icons, prayers, articles, saving, onPublish, onDraft, onPreview, onOpenPath }: { day: ChurchCalendarDay; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; saving: boolean; onPublish: () => void; onDraft: () => void; onPreview: () => void; onOpenPath: (path: string) => void }) {
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
    { label: 'Icon image exists', ok: icons.length > 0 && icons.every((item) => Boolean(item.imageUrl.trim()) && isValidUrl(item.imageUrl)) },
    { label: 'SEO title exists', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoTitle.trim())) },
    { label: 'SEO description exists', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoDescription.trim())) },
    { label: draftItems.length ? `Draft without reason: ${draftItems.slice(0, 2).join(', ')}` : 'No unexplained drafts', ok: draftItems.length === 0, tone: 'warn' }
  ];
  const publicDate = day.dateNewStyle || day.dateOldStyle || '';
  const publicLinks = [
    publicDate ? { label: 'Calendar day', path: `/church/calendar/${publicDate}` } : null,
    icons[0]?.slug ? { label: 'Icon page', path: `/church/icons/${icons[0].slug}` } : null,
    prayers[0]?.slug ? { label: 'Prayer page', path: `/church/prayers/${prayers[0].slug}` } : null,
    articles[0]?.slug ? { label: 'Article page', path: `/church/articles/${articles[0].slug}` } : null
  ].filter(Boolean) as Array<{ label: string; path: string }>;
  return (
    <div className="church-publish-panel">
      <div className="church-public-links">
        <div>
          <strong>Public URLs</strong>
          <span>Publish делает доступными эти прямые страницы. Главная сетка месяца обновляется отдельным public calendar feed.</span>
        </div>
        <div>
          {publicLinks.map((link) => (
            <button key={link.path} type="button" onClick={() => onOpenPath(link.path)} disabled={saving || day.status !== 'published'}>
              <AppIcon name="external" />
              <span>{link.label}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="church-generated-preview">
        <strong>Created content preview</strong>
        <div>
          {icons[0] ? (
            <article>
              {icons[0].imageUrl ? <img src={icons[0].imageUrl} alt={icons[0].title} loading="lazy" /> : <span><AppIcon name="image" /></span>}
              <div>
                <b>{icons[0].title}</b>
                <small>{icons[0].imageUrl ? 'Фото иконы подключено' : 'Фото не добавлено. Откройте Icon и загрузите с ПК или вставьте URL.'}</small>
                <p>{icons[0].description || 'Описание иконы пока пустое.'}</p>
              </div>
            </article>
          ) : <p>Икона ещё не создана.</p>}
          {prayers[0] ? (
            <article>
              <span><AppIcon name="sparkles" /></span>
              <div>
                <b>{prayers[0].title}</b>
                <small>{prayers[0].prayerType} · {prayers[0].language.toUpperCase()}</small>
                <p>{prayers[0].text || 'Текст молитвы пока пустой.'}</p>
              </div>
            </article>
          ) : <p>Молитва ещё не создана.</p>}
          {articles[0] ? (
            <article>
              <span><AppIcon name="cms" /></span>
              <div>
                <b>{articles[0].title}</b>
                <small>{articles[0].seoTitle || 'SEO title не заполнен'}</small>
                <p>{articles[0].seoDescription || articles[0].content || 'Текст статьи пока пустой.'}</p>
              </div>
            </article>
          ) : <p>Статья ещё не создана.</p>}
        </div>
      </div>
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
  days: ChurchCalendarDay[];
  icons: ChurchIcon[];
  allIcons: ChurchIcon[];
  saving: boolean;
  aiCreating: boolean;
  onClose: () => void;
  onSaveDay: (payload: ChurchCalendarDayPayload, id?: string) => Promise<void>;
  onSaveIcon: (payload: ChurchIconPayload, id?: string) => Promise<void>;
  onSavePrayer: (payload: ChurchPrayerPayload, id?: string) => Promise<void>;
  onSaveArticle: (payload: ChurchArticlePayload, id?: string) => Promise<void>;
  onCreateGeminiChain: (payload: ChurchAiDraftPayload) => Promise<void>;
}) {
  if (!props.drawer) return null;
  const title = props.drawer.type === 'day' ? 'Calendar day'
    : props.drawer.type === 'icon' ? 'Icon'
      : props.drawer.type === 'prayer' ? 'Prayer'
        : props.drawer.type === 'article' ? 'Article'
          : 'Gemini content draft';
  return (
    <AdminDrawer open title={title} description={props.day ? `Attached to: ${props.day.title}` : 'Create the calendar day first.'} onClose={props.onClose}>
      {props.drawer.type === 'ai' ? <ChurchAiDraftForm saving={props.aiCreating || props.saving} onSubmit={props.onCreateGeminiChain} /> : null}
      {props.drawer.type === 'day' ? <CalendarDayForm item={props.drawer.item} saving={props.saving} onSubmit={props.onSaveDay} /> : null}
      {props.drawer.type === 'icon' ? <ChurchIconForm item={props.drawer.item} saving={props.saving} onSubmit={props.onSaveIcon} /> : null}
      {props.drawer.type === 'prayer' ? <ChurchPrayerForm item={props.drawer.item} icons={props.icons} saving={props.saving} onSubmit={props.onSavePrayer} /> : null}
      {props.drawer.type === 'article' ? <ChurchArticleForm item={props.drawer.item} day={props.day} days={props.days} icons={props.allIcons} saving={props.saving} onSubmit={props.onSaveArticle} /> : null}
    </AdminDrawer>
  );
}

type ChurchAiDraftPayload = {
  topic: string;
  dateOldStyle: string;
  dateNewStyle: string;
  calendarType: ChurchCalendarType;
  dayType: ChurchDayType;
  language: ChurchLanguage;
  rank: string;
  imageUrl: string;
  generateImage: boolean;
};

function ChurchAiDraftForm({ saving, onSubmit }: { saving: boolean; onSubmit: (payload: ChurchAiDraftPayload) => Promise<void> }) {
  const today = new Date().toISOString().slice(0, 10);
  const [form, setForm] = useState<ChurchAiDraftPayload>({
    topic: '',
    dateOldStyle: '',
    dateNewStyle: today,
    calendarType: 'both',
    dayType: 'saint',
    language: 'uk',
    rank: '0',
    imageUrl: '',
    generateImage: true
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.topic.trim()) return setError('Напишите святого, праздник или церковный день для Gemini.');
    if (!form.dateOldStyle && !form.dateNewStyle) return setError('Укажите дату по новому или старому стилю.');
    if (form.imageUrl && !isValidUrl(form.imageUrl)) return setError('Фото должно быть http/https URL.');
    setError('');
    void onSubmit(form);
  }
  return (
    <form className="admin-form-grid church-ai-draft-form" onSubmit={submit}>
      <div className="church-ai-help">
        <AppIcon name="bot" />
        <div>
          <strong>Gemini заполнит всю цепочку</strong>
          <span>Будет создан черновик: день → икона → молитва → статья. После этого можно открыть каждый блок и поправить текст вручную.</span>
        </div>
      </div>
      <label className="admin-form-wide"><span>Что создать</span><textarea value={form.topic} onChange={(event) => setForm({ ...form, topic: event.target.value })} placeholder="Например: Святитель Николай Чудотворец, Рождество Христово, Казанская икона Божией Матери" /></label>
      <label><span>Date new style</span><input type="date" value={form.dateNewStyle} onChange={(event) => setForm({ ...form, dateNewStyle: event.target.value })} /></label>
      <label><span>Date old style</span><input type="date" value={form.dateOldStyle} onChange={(event) => setForm({ ...form, dateOldStyle: event.target.value })} /></label>
      <label><span>Type</span><select value={form.dayType} onChange={(event) => setForm({ ...form, dayType: event.target.value as ChurchDayType })}>{dayTypes.map((type) => <option key={type} value={type}>{type}</option>)}</select></label>
      <label><span>Calendar</span><select value={form.calendarType} onChange={(event) => setForm({ ...form, calendarType: event.target.value as ChurchCalendarType })}><option value="both">both</option><option value="old_style">old style</option><option value="new_style">new style</option></select></label>
      <label><span>Language</span><select value={form.language} onChange={(event) => setForm({ ...form, language: event.target.value as ChurchLanguage })}>{languages.map((lang) => <option key={lang} value={lang}>{lang.toUpperCase()}</option>)}</select></label>
      <label><span>Rank</span><input type="number" value={form.rank} onChange={(event) => setForm({ ...form, rank: event.target.value })} /></label>
      <label className="admin-form-wide"><span>Existing image URL, optional</span><input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://..." /></label>
      <label className="admin-inline-check"><input type="checkbox" checked={form.generateImage} onChange={(event) => setForm({ ...form, generateImage: event.target.checked })} /><span>Сгенерировать фото для иконы, если URL не указан</span></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="bot" type="submit" disabled={saving}>{saving ? 'Gemini создает...' : 'Заполнить с помощью Gemini'}</ActionButton>
    </form>
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
  const [uploading, setUploading] = useState(false);
  async function uploadPhoto(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setError('Выберите файл изображения.');
      return;
    }
    if (file.size > 10 * 1024 * 1024) {
      setError('Фото должно быть меньше 10 MB.');
      return;
    }
    setUploading(true);
    setError('');
    try {
      const imageUrl = await uploadCmsReference(file);
      setForm((current) => ({ ...current, imageUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото.');
    } finally {
      setUploading(false);
    }
  }
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
      <label className="admin-form-wide"><span>Image URL</span><input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://..." /></label>
      <div className="church-image-editor">
        {form.imageUrl ? <img src={form.imageUrl} alt={form.title || 'Icon photo preview'} loading="lazy" /> : <span><AppIcon name="image" /><small>Фото ещё не добавлено</small></span>}
        <div>
          <strong>Фото иконы</strong>
          <p>Загрузите файл с компьютера или вставьте готовый URL. Это фото будет видно в карточке, молитве, статье и публичном preview.</p>
          <label className="church-upload-button">
            <input type="file" accept="image/*" onChange={(event) => void uploadPhoto(event.target.files?.[0] ?? null)} disabled={saving || uploading} />
            <span>{uploading ? 'Загрузка...' : 'Загрузить фото с ПК'}</span>
          </label>
        </div>
      </div>
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
    iconId: item?.iconId ?? icons[0]?.id ?? '',
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

function ChurchArticleForm({ item, day, days, icons, saving, onSubmit }: { item?: ChurchArticle; day: ChurchCalendarDay | null; days: ChurchCalendarDay[]; icons: ChurchIcon[]; saving: boolean; onSubmit: (payload: ChurchArticlePayload, id?: string) => Promise<void> }) {
  const initialDayId = item?.calendarDayId ?? day?.id ?? '';
  const initialIconId = item?.iconId ?? icons.find((icon) => icon.calendarDayId === initialDayId)?.id ?? '';
  const [form, setForm] = useState({
    calendarDayId: initialDayId,
    iconId: initialIconId,
    title: item?.title ?? '',
    slug: item?.slug ?? '',
    content: item?.content ?? '',
    language: item?.language ?? 'uk',
    seoTitle: item?.seoTitle ?? '',
    seoDescription: item?.seoDescription ?? '',
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  const availableIcons = form.calendarDayId ? icons.filter((icon) => icon.calendarDayId === form.calendarDayId) : icons;
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название статьи обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.content.trim()) return setError('Текст статьи обязателен.');
    setError('');
    void onSubmit({ ...form, calendarDayId: form.calendarDayId || undefined, iconId: form.iconId || undefined } as ChurchArticlePayload, item?.id);
  }
  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label><span>Calendar day</span><select value={form.calendarDayId} onChange={(event) => {
        const nextDayId = event.target.value;
        const nextIcon = icons.find((icon) => icon.calendarDayId === nextDayId);
        setForm({ ...form, calendarDayId: nextDayId, iconId: nextIcon?.id ?? '' });
      }}><option value="">Needs linking</option>{days.map((calendarDay) => <option key={calendarDay.id} value={calendarDay.id}>{formatDayDate(calendarDay)} · {calendarDay.title}</option>)}</select></label>
      <label><span>Related icon</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">No icon selected</option>{availableIcons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
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

type ParsedChurchAiDraft = {
  dayTitle: string;
  dayType: ChurchDayType;
  dayDescription: string;
  iconTitle: string;
  iconSlug: string;
  iconDescription: string;
  saintName: string;
  feastName: string;
  prayerTitle: string;
  prayerSlug: string;
  prayerText: string;
  prayerType: ChurchPrayerType;
  articleTitle: string;
  articleSlug: string;
  articleContent: string;
  seoTitle: string;
  seoDescription: string;
  photoPrompt: string;
};

function buildChurchAiPrompt(payload: ChurchAiDraftPayload) {
  return [
    `Создай черновик для церковного сайта Свет Иконы.`,
    `Тема: ${payload.topic}`,
    `Дата новый стиль: ${payload.dateNewStyle || 'не указана'}`,
    `Дата старый стиль: ${payload.dateOldStyle || 'не указана'}`,
    `Тип дня: ${payload.dayType}`,
    `Язык публикации: ${payload.language.toUpperCase()}`,
    '',
    'Верни структурированный ответ с точными заголовками:',
    'Название дня:',
    'Тип дня: saint | feast | fasting | memorial | gospel | quiet',
    'Краткое описание дня:',
    'Название иконы:',
    'Святой:',
    'Праздник:',
    'Описание иконы:',
    'Название молитвы:',
    'Тип молитвы: prayer | akathist | troparion | kontakion',
    'Текст молитвы:',
    'Название статьи:',
    'Статья:',
    'SEO title:',
    'SEO description:',
    'PhotoPrompt:',
    '',
    'Пиши аккуратно, без спорных утверждений. Если точных церковных данных не хватает, отметь это мягко в статье и не выдумывай дату.'
  ].join('\n');
}

function parseChurchAiDraft(result: AiGenerationResult, payload: ChurchAiDraftPayload): ParsedChurchAiDraft {
  const source = [result.description, ...(result.suggestions || [])].filter(Boolean).join('\n');
  const dayTitle = compactText(sectionFromText(source, ['Название дня']) || result.title || payload.topic, 120);
  const iconTitle = compactText(sectionFromText(source, ['Название иконы']) || `Икона: ${dayTitle}`, 120);
  const dayDescription = compactText(sectionFromText(source, ['Краткое описание дня', 'Описание дня']) || result.description || payload.topic, 900);
  const iconDescription = compactText(sectionFromText(source, ['Описание иконы']) || dayDescription, 900);
  const saintName = compactText(sectionFromText(source, ['Святой']) || (payload.dayType === 'saint' ? dayTitle : ''), 120);
  const feastName = compactText(sectionFromText(source, ['Праздник']) || (payload.dayType === 'feast' ? dayTitle : ''), 120);
  const prayerTitle = compactText(sectionFromText(source, ['Название молитвы']) || `Молитва: ${dayTitle}`, 140);
  const prayerText = compactText(sectionFromText(source, ['Текст молитвы', 'Молитва']) || result.description || `Молитвенный текст для материала "${dayTitle}". Проверьте и дополните перед публикацией.`, 4000);
  const articleTitle = compactText(sectionFromText(source, ['Название статьи']) || dayTitle, 140);
  const articleContent = compactText(sectionFromText(source, ['Статья']) || result.description || dayDescription, 8000);
  const seoTitle = compactText(sectionFromText(source, ['SEO title']) || result.title || articleTitle, 160);
  const seoDescription = compactText(sectionFromText(source, ['SEO description']) || result.description || dayDescription, 260);
  const photoPrompt = compactText(sectionFromText(source, ['PhotoPrompt', 'Промпт фото']) || result.photoPrompt || `Orthodox icon-inspired respectful image for ${dayTitle}, church calendar visual, warm light, no text`, 900);
  const dayTypeRaw = sectionFromText(source, ['Тип дня']).toLowerCase();
  const prayerTypeRaw = sectionFromText(source, ['Тип молитвы']).toLowerCase();
  return {
    dayTitle,
    dayType: dayTypes.includes(dayTypeRaw as ChurchDayType) ? dayTypeRaw as ChurchDayType : payload.dayType,
    dayDescription,
    iconTitle,
    iconSlug: result.slug && isValidSlug(result.slug) ? result.slug : slugify(iconTitle),
    iconDescription,
    saintName,
    feastName,
    prayerTitle,
    prayerSlug: slugify(prayerTitle),
    prayerText,
    prayerType: prayerTypes.includes(prayerTypeRaw as ChurchPrayerType) ? prayerTypeRaw as ChurchPrayerType : 'prayer',
    articleTitle,
    articleSlug: slugify(articleTitle),
    articleContent,
    seoTitle,
    seoDescription,
    photoPrompt
  };
}

function sectionFromText(text: string, names: string[]) {
  for (const name of names) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const pattern = new RegExp(`${escaped}\\s*[:\\-]\\s*([\\s\\S]*?)(?=\\n[А-ЯA-Z][^\\n:]{1,80}\\s*[:\\-]|$)`, 'i');
    const match = text.match(pattern);
    if (match?.[1]) return match[1].trim();
  }
  return '';
}

function compactText(value: string, max: number) {
  const text = value.replace(/\n{3,}/g, '\n\n').trim();
  return text.length > max ? `${text.slice(0, max - 1).trim()}…` : text;
}

function slugify(value: string) {
  const translit: Record<string, string> = {
    а: 'a', б: 'b', в: 'v', г: 'g', д: 'd', е: 'e', ё: 'e', ж: 'zh', з: 'z', и: 'i', й: 'y', к: 'k', л: 'l', м: 'm', н: 'n', о: 'o', п: 'p', р: 'r', с: 's', т: 't', у: 'u', ф: 'f', х: 'h', ц: 'ts', ч: 'ch', ш: 'sh', щ: 'sch', ы: 'y', э: 'e', ю: 'yu', я: 'ya',
    і: 'i', ї: 'yi', є: 'ye', ґ: 'g', ъ: '', ь: ''
  };
  const normalized = value
    .trim()
    .toLowerCase()
    .split('')
    .map((char) => translit[char] ?? char)
    .join('')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return normalized || `church-${Date.now()}`;
}

function imageForContent(iconId: string | null | undefined, dayIcons: ChurchIcon[], allIcons: ChurchIcon[]) {
  return [...dayIcons, ...allIcons].find((icon) => icon.id === iconId)?.imageUrl || dayIcons[0]?.imageUrl || '';
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
