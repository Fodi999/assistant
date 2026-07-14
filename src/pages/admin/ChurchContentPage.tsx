import { useCallback, useEffect, useMemo, useState } from 'react';
import { AdminDrawer } from '../../components/admin/AdminDrawer';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { EmptyState } from '../../components/admin/EmptyState';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import { generateAiImage, generateSeo } from '../../api/ai';
import { getIconsSiteContent, type IconsSiteContent } from '../../api/iconsSite';
import {
  churchContentApi,
  type ChurchArticle,
  type ChurchArticlePayload,
  type ChurchCalendarDay,
  type ChurchCalendarDayPayload,
  type ChurchContentStatus,
  type ChurchGospel,
  type ChurchGospelPayload,
  type ChurchIcon,
  type ChurchIconPayload,
  type ChurchImportPreview,
  type ChurchPrayer,
  type ChurchPrayerPayload,
  type ChurchSaint,
  type ChurchSaintPayload
} from '../../api/churchContent';
import { ChurchAiDraftForm } from './church/ChurchAiDraftForm';
import { ChurchCalendarPanel } from './church/ChurchCalendarPanel';
import { ChurchDayEditor } from './church/ChurchDayEditor';
import { ChurchImportPanel } from './church/ChurchImportPanel';
import { ChurchTopBar } from './church/ChurchTopBar';
import {
  churchNavTabByKey,
  churchPreviewToken,
  churchPublicUrl,
  type CalendarDetailMode,
  type CalendarViewMode,
  type DetailTab,
  type DrawerState
} from './church/churchTypes';
import {
  buildCalendarSlots,
  buildChurchAiPrompt,
  calendarPeriodTitle,
  dateKeyFromChurchDay,
  mapChurchDaysByDate,
  parseChurchAiDraft,
  publicCalendarDayFromChurch,
  shiftDateByMode,
  type ChurchAiDraftPayload
} from './church/churchHelpers';

const CHURCH_YEAR_LENGTH = 365;

export function ChurchContentPage({ activeNavKey }: { activeNavKey?: string }) {
  const { activeSiteId } = useActiveSite();
  const toast = useAdminToast();
  const [days, setDays] = useState<ChurchCalendarDay[]>([]);
  const [icons, setIcons] = useState<ChurchIcon[]>([]);
  const [saints, setSaints] = useState<ChurchSaint[]>([]);
  const [prayers, setPrayers] = useState<ChurchPrayer[]>([]);
  const [articles, setArticles] = useState<ChurchArticle[]>([]);
  const [gospel, setGospel] = useState<ChurchGospel[]>([]);
  const [selectedDayId, setSelectedDayId] = useState('');
  const [drawer, setDrawer] = useState<DrawerState>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [importPreview, setImportPreview] = useState<ChurchImportPreview | null>(null);
  const [legacyContent, setLegacyContent] = useState<IconsSiteContent | null>(null);
  const [importing, setImporting] = useState(false);
  const [importPanelOpen, setImportPanelOpen] = useState(false);
  const [aiCreating, setAiCreating] = useState(false);
  const [calendarViewMode, setCalendarViewMode] = useState<CalendarViewMode>('month');
  const [calendarAnchorDate, setCalendarAnchorDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [selectedCalendarDate, setSelectedCalendarDate] = useState('');
  const [calendarDetailMode, setCalendarDetailMode] = useState<CalendarDetailMode>('idle');
  const [detailFullscreen, setDetailFullscreen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [focusStep, setFocusStep] = useState<DetailTab | null>(null);

  const daysByDate = useMemo(() => mapChurchDaysByDate(days), [days]);
  const selectedDay = selectedCalendarDate ? daysByDate.get(selectedCalendarDate) ?? null : selectedDayId ? days.find((day) => day.id === selectedDayId) ?? null : null;
  const selectedDate = selectedCalendarDate || (selectedDay ? dateKeyFromChurchDay(selectedDay) : '') || calendarAnchorDate;
  const calendarSlots = useMemo(() => buildCalendarSlots(calendarViewMode, calendarAnchorDate, selectedDate, daysByDate), [calendarAnchorDate, calendarViewMode, daysByDate, selectedDate]);
  const calendarPeriodLabel = useMemo(() => calendarPeriodTitle(calendarViewMode, calendarAnchorDate), [calendarAnchorDate, calendarViewMode]);
  const visibleCalendarSlots = useMemo(() => {
    const needle = searchQuery.trim().toLowerCase();
    if (!needle) return calendarSlots;
    return calendarSlots.filter((slot) => slot.day && (slot.day.title.toLowerCase().includes(needle) || slot.date.includes(needle)));
  }, [calendarSlots, searchQuery]);
  const unlinkedArticles = useMemo(() => articles.filter((item) => !item.calendarDayId), [articles]);
  const selectedDayContent = useMemo(() => {
    if (!selectedDay) return { saints: [], icons: [], prayers: [], articles: [], gospel: [] };
    return {
      saints: saints.filter((item) => item.calendarDayId === selectedDay.id),
      icons: icons.filter((item) => item.calendarDayId === selectedDay.id),
      prayers: prayers.filter((item) => item.calendarDayId === selectedDay.id),
      articles: articles.filter((item) => item.calendarDayId === selectedDay.id),
      gospel: gospel.filter((item) => item.calendarDayId === selectedDay.id)
    };
  }, [articles, gospel, icons, prayers, saints, selectedDay]);
  const selectedPublicCalendarDay = useMemo(() => selectedDay
    ? publicCalendarDayFromChurch(selectedDay, selectedDayContent.icons, selectedDayContent.prayers, selectedDayContent.articles)
    : null, [selectedDay, selectedDayContent]);

  const loadContent = useCallback(async () => {
    if (activeSiteId !== 'church') return;
    setLoading(true);
    setError('');
    try {
      const query = { site: 'church' };
      const [nextDays, nextIcons, nextSaints, nextPrayers, nextArticles, nextGospel] = await Promise.all([
        churchContentApi.listCalendarDays(query),
        churchContentApi.listIcons(query),
        churchContentApi.listSaints(query),
        churchContentApi.listPrayers(query),
        churchContentApi.listArticles(query),
        churchContentApi.listGospel(query)
      ]);
      const legacy = await getIconsSiteContent().catch(() => null);
      setDays(nextDays);
      setIcons(nextIcons);
      setSaints(nextSaints);
      setPrayers(nextPrayers);
      setArticles(nextArticles);
      setGospel(nextGospel);
      setLegacyContent(legacy);
      setSelectedDayId((current) => current && nextDays.some((day) => day.id === current) ? current : '');
      setCalendarAnchorDate((current) => {
        const firstDate = dateKeyFromChurchDay(nextDays[0]);
        return firstDate && current === new Date().toISOString().slice(0, 10) ? firstDate : current;
      });
      if (!nextDays.length && !nextIcons.length && !nextSaints.length && !nextPrayers.length && !nextArticles.length) {
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

  useEffect(() => {
    if (activeSiteId !== 'church' || !activeNavKey) return;
    const nextTab = churchNavTabByKey[activeNavKey];
    if (nextTab) setFocusStep(nextTab);
    window.setTimeout(() => {
      document.getElementById('church-content-detail')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }, 0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeNavKey, activeSiteId]);

  async function saveDay(payload: ChurchCalendarDayPayload, id?: string) {
    setSaving(true);
    try {
      const saved = id
        ? await churchContentApi.updateCalendarDay(id, payload, { site: 'church' })
        : await churchContentApi.createCalendarDay(payload, { site: 'church' });
      toast.success(id ? 'День обновлен.' : 'День создан.');
      await loadContent();
      setSelectedDayId(saved.id);
      setSelectedCalendarDate(dateKeyFromChurchDay(saved) || selectedCalendarDate);
      setCalendarDetailMode('view');
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
      await loadContent();
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить икону.');
    } finally {
      setSaving(false);
    }
  }

  async function saveSaint(payload: ChurchSaintPayload, id?: string) {
    if (!selectedDay) return;
    setSaving(true);
    try {
      const fallbackIconId = selectedDayContent.icons[0]?.id;
      const body = { ...payload, calendarDayId: selectedDay.id, iconId: payload.iconId || fallbackIconId };
      await (id ? churchContentApi.updateSaint(id, body, { site: 'church' }) : churchContentApi.createSaint(body, { site: 'church' }));
      toast.success(id ? 'Святой обновлен.' : 'Святой добавлен.');
      await loadContent();
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить святого.');
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
      await loadContent();
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
      await loadContent();
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить статью.');
    } finally {
      setSaving(false);
    }
  }

  async function saveGospel(payload: ChurchGospelPayload, id?: string) {
    if (!selectedDay && !id) return;
    setSaving(true);
    try {
      const fallbackIconId = selectedDayContent.icons[0]?.id;
      const calendarDayId = payload.calendarDayId !== undefined ? payload.calendarDayId : id ? undefined : selectedDay?.id;
      const iconId = payload.iconId !== undefined ? payload.iconId : id ? undefined : fallbackIconId;
      const body = { ...payload, calendarDayId, iconId };
      await (id ? churchContentApi.updateGospel(id, body, { site: 'church' }) : churchContentApi.createGospel(body, { site: 'church' }));
      toast.success(id ? 'Евангелие обновлено.' : 'Евангелие добавлено.');
      await loadContent();
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить Евангелие.');
    } finally {
      setSaving(false);
    }
  }

  async function deleteItem(type: 'saint' | 'icon' | 'prayer' | 'article' | 'gospel', id: string, title: string) {
    if (!window.confirm(`Удалить "${title}"?`)) return;
    setSaving(true);
    try {
      if (type === 'saint') await churchContentApi.deleteSaint(id, { site: 'church' });
      if (type === 'icon') await churchContentApi.deleteIcon(id, { site: 'church' });
      if (type === 'prayer') await churchContentApi.deletePrayer(id, { site: 'church' });
      if (type === 'article') await churchContentApi.deleteArticle(id, { site: 'church' });
      if (type === 'gospel') await churchContentApi.deleteGospel(id, { site: 'church' });
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
      await churchContentApi.createSaint({
        calendarDayId: day.id,
        iconId: icon.id,
        name: draft.saintName || draft.iconTitle,
        slug: draft.iconSlug,
        shortDescription: draft.dayDescription,
        biography: draft.articleContent,
        feastDay: draft.feastName || draft.dayTitle,
        imageUrl,
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
      setCalendarDetailMode('view');
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

  function selectCalendarSlot(slot: { date: string; day: ChurchCalendarDay | null }) {
    setCalendarAnchorDate(slot.date);
    setSelectedCalendarDate(slot.date);
    setFocusStep(null);
    if (slot.day) {
      setSelectedDayId(slot.day.id);
      setCalendarDetailMode('view');
      return;
    }
    setSelectedDayId('');
    setCalendarDetailMode('create');
  }

  function startCreateDay(date = selectedCalendarDate || calendarAnchorDate) {
    setCalendarAnchorDate(date);
    setSelectedCalendarDate(date);
    setSelectedDayId('');
    setCalendarDetailMode('create');
  }

  function startEditDay() {
    if (!selectedDay) return;
    setSelectedCalendarDate(dateKeyFromChurchDay(selectedDay) || selectedCalendarDate);
    setSelectedDayId(selectedDay.id);
    setCalendarDetailMode('edit');
  }

  function cancelCalendarDetailEdit() {
    if (saving) return;
    setCalendarDetailMode(selectedDay ? 'view' : selectedCalendarDate ? 'create' : 'idle');
  }

  function shiftCalendarPeriod(direction: -1 | 1) {
    setCalendarAnchorDate((current) => shiftDateByMode(current, calendarViewMode, direction));
  }

  if (activeSiteId !== 'church') {
    return (
      <section className="admin-resource-page">
        <AdminPageHeader eyebrow="Church Content" title="Church Content" description="Этот редактор доступен только для сайта Свет Иконы." icon="cms" />
        <EmptyState icon="cms" title="Выберите church" description="Модель календаря, икон, молитв и статей относится к церковному сайту." />
      </section>
    );
  }

  return (
    <section className="admin-resource-page church-content-page">
      <ChurchTopBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        filledDays={days.length}
        totalDays={CHURCH_YEAR_LENGTH}
        aiCreating={aiCreating}
        onCreateDay={() => startCreateDay(calendarAnchorDate)}
        onGeminiFill={() => setDrawer({ type: 'ai' })}
        importAvailable={Boolean(importPreview || legacyContent)}
        onToggleImport={() => setImportPanelOpen((current) => !current)}
      />

      {error ? <p className="admin-soft-alert">{error}</p> : null}

      {importPanelOpen ? (
        <ChurchImportPanel
          importPreview={importPreview}
          legacyContent={legacyContent}
          unlinkedArticles={unlinkedArticles}
          icons={icons}
          importing={importing}
          onPreviewImport={() => void previewOldJsonImport()}
          onApplyImport={() => void applyOldJsonImport()}
          onDeleteArticle={(article) => void deleteItem('article', article.id, article.title)}
          onClose={() => setImportPanelOpen(false)}
        />
      ) : null}

      <div className={'church-content-shell' + (detailFullscreen ? ' detail-fullscreen' : '')}>
        <div id="church-calendar-days">
          {loading ? <p className="admin-table-empty">Загружаем церковный календарь...</p> : null}
          <ChurchCalendarPanel
            mode={calendarViewMode}
            periodLabel={calendarPeriodLabel}
            slots={visibleCalendarSlots}
            icons={icons}
            prayers={prayers}
            articles={articles}
            onModeChange={setCalendarViewMode}
            onShift={shiftCalendarPeriod}
            onToday={() => setCalendarAnchorDate(new Date().toISOString().slice(0, 10))}
            onSelectSlot={selectCalendarSlot}
          />
        </div>

        <div id="church-content-detail">
          <ChurchDayEditor
            mode={calendarDetailMode}
            selectedDate={selectedCalendarDate}
            day={selectedDay}
            publicDay={selectedPublicCalendarDay}
            saints={selectedDayContent.saints}
            icons={selectedDayContent.icons}
            prayers={selectedDayContent.prayers}
            articles={selectedDayContent.articles}
            gospel={selectedDayContent.gospel}
            allIcons={icons}
            allPrayers={prayers}
            allArticles={articles}
            allGospel={gospel}
            saving={saving}
            onCreate={() => startCreateDay(selectedCalendarDate || calendarAnchorDate)}
            onEdit={startEditDay}
            onCancel={cancelCalendarDetailEdit}
            onSaveDay={saveDay}
            onSaveSaint={saveSaint}
            onSaveIcon={saveIcon}
            onSavePrayer={savePrayer}
            onSaveArticle={saveArticle}
            onSaveGospel={saveGospel}
            onOpenPublic={openPublicPreview}
            onDeleteItem={deleteItem}
            onPublish={(status) => void publishDay(status)}
            onOpenPath={(path) => window.open(`${churchPublicUrl}${path}`, '_blank', 'noopener,noreferrer')}
            expanded={detailFullscreen}
            onToggleExpanded={() => setDetailFullscreen((current) => !current)}
            focusStep={focusStep}
            focusToken={activeNavKey ?? ''}
          />
        </div>
      </div>

      <AdminDrawer open={Boolean(drawer)} title="Gemini content draft" description="Создайте день, икону, молитву и статью одним запросом." onClose={() => setDrawer(null)}>
        {drawer?.type === 'ai' ? <ChurchAiDraftForm saving={aiCreating || saving} onSubmit={createGeminiChain} /> : null}
      </AdminDrawer>
    </section>
  );
}
