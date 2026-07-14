import { useEffect, useState } from 'react';
import { AppIcon, type AppIconName } from '../../../components/AppIcon';
import { ActionButton } from '../../../components/admin/ActionButton';
import { AdminPanel } from '../../../components/admin/AdminPanel';
import { EmptyState } from '../../../components/admin/EmptyState';
import { StatusBadge } from '../../../components/admin/StatusBadge';
import type {
  ChurchArticle,
  ChurchArticlePayload,
  ChurchCalendarDay,
  ChurchCalendarDayPayload,
  ChurchContentStatus,
  ChurchGospel,
  ChurchGospelPayload,
  ChurchIcon,
  ChurchIconPayload,
  ChurchPrayer,
  ChurchPrayerPayload,
  ChurchSaint,
  ChurchSaintPayload
} from '../../../api/churchContent';
import type { CalendarDay as PublicCalendarDay } from '../../../api/iconsSite';
import { detailTabs, type CalendarDetailMode, type CmsLanguage, type DetailTab } from './churchTypes';
import { buildWorkflowSteps, formatDayDate, languageProgress } from './churchHelpers';
import {
  ArticleWorkflowStep,
  CalendarDayForm,
  CalendarSiteModelPreview,
  CalendarWorkflowIntro,
  GospelWorkflowStep,
  IconWorkflowStep,
  OverviewTab,
  PrayerWorkflowStep,
  SaintWorkflowStep,
  SeoPublishWorkflowStep
} from './ChurchWorkflowSteps';

type DeleteItemType = 'saint' | 'icon' | 'prayer' | 'article' | 'gospel';

export function ChurchDayEditor(props: {
  mode: CalendarDetailMode;
  selectedDate: string;
  day: ChurchCalendarDay | null;
  publicDay: PublicCalendarDay | null;
  icons: ChurchIcon[];
  saints: ChurchSaint[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  gospel: ChurchGospel[];
  allIcons: ChurchIcon[];
  allPrayers: ChurchPrayer[];
  allArticles: ChurchArticle[];
  allGospel: ChurchGospel[];
  saving: boolean;
  onCreate: () => void;
  onEdit: () => void;
  onCancel: () => void;
  onSaveDay: (payload: ChurchCalendarDayPayload, id?: string) => Promise<void>;
  onSaveIcon: (payload: ChurchIconPayload, id?: string) => Promise<void>;
  onSaveSaint: (payload: ChurchSaintPayload, id?: string) => Promise<void>;
  onSavePrayer: (payload: ChurchPrayerPayload, id?: string) => Promise<void>;
  onSaveArticle: (payload: ChurchArticlePayload, id?: string) => Promise<void>;
  onSaveGospel: (payload: ChurchGospelPayload, id?: string) => Promise<void>;
  onOpenPublic: () => void;
  onDeleteItem: (type: DeleteItemType, id: string, title: string) => Promise<void>;
  onPublish: (status: ChurchContentStatus) => void;
  onOpenPath: (path: string) => void;
  expanded: boolean;
  onToggleExpanded: () => void;
  focusStep?: DetailTab | null;
  focusToken?: string;
}) {
  const [activeStep, setActiveStep] = useState<DetailTab | null>(null);
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>('uk');
  const hasSelection = Boolean(props.selectedDate || props.day);
  const isCreate = props.mode === 'create' && !props.day;
  const isEditDay = props.mode === 'edit' && Boolean(props.day);

  useEffect(() => {
    setActiveStep(null);
  }, [props.day?.id]);

  useEffect(() => {
    if (!props.focusStep) return;
    setActiveStep(props.focusStep === 'overview' ? null : props.focusStep);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.focusToken]);

  if (!hasSelection || props.mode === 'idle') {
    return (
      <AdminPanel title="Редактор дня" icon="cms">
        <EmptyState icon="calendar" title="Выберите день в календаре" description="Слева выберите дату. Пустой день откроет форму создания, заполненный день откроет цепочку разделов." />
      </AdminPanel>
    );
  }

  if (isCreate) {
    return (
      <AdminPanel title="Создать день" icon="cms" meta={props.selectedDate}>
        <CalendarWorkflowIntro
          step={{ id: 'overview', index: '01', label: 'Новый день', status: 'empty', disabled: false, hint: '', icon: 'dashboard' }}
          description="Заполните базовые данные дня. После сохранения откроется цепочка: святой/праздник, икона, молитвы, статья и публикация."
        />
        <CalendarDayForm
          key={props.selectedDate}
          item={undefined}
          saving={props.saving}
          onSubmit={props.onSaveDay}
          variant="workspace"
        />
      </AdminPanel>
    );
  }

  if (isEditDay && props.day) {
    return (
      <AdminPanel title="Редактировать день" icon="cms" meta={formatDayDate(props.day)}>
        <CalendarDayForm
          key={props.day.id}
          item={props.day}
          saving={props.saving}
          onSubmit={props.onSaveDay}
          onCancel={props.onCancel}
        />
      </AdminPanel>
    );
  }

  if (!props.day) {
    return (
      <AdminPanel title="Редактор дня" icon="cms">
        <EmptyState icon="calendar" title="День не найден" description="Обновите страницу или выберите другой день." />
      </AdminPanel>
    );
  }

  const day = props.day;
  const workflowSteps = buildWorkflowSteps(day, props.saints, props.icons, props.prayers, props.articles, props.gospel);
  const completedCount = workflowSteps.filter((step) => step.status === 'completed').length;
  const progress = languageProgress(day, props.saints, props.icons, props.prayers, props.articles);

  if (activeStep) {
    return (
      <AdminPanel title={day.title} icon="cms" meta={formatDayDate(day)}>
        <button type="button" className="church-back-to-day" onClick={() => setActiveStep(null)}>
          <AppIcon name="chevron-left" /> Назад к дню
        </button>
        <ChurchStepDrillDown
          step={activeStep}
          day={day}
          publicDay={props.publicDay}
          saints={props.saints}
          icons={props.icons}
          prayers={props.prayers}
          articles={props.articles}
          gospel={props.gospel}
          allIcons={props.allIcons}
          allPrayers={props.allPrayers}
          allArticles={props.allArticles}
          allGospel={props.allGospel}
          saving={props.saving}
          onEditDay={props.onEdit}
          onSaveSaint={props.onSaveSaint}
          onSaveIcon={props.onSaveIcon}
          onSavePrayer={props.onSavePrayer}
          onSaveArticle={props.onSaveArticle}
          onSaveGospel={props.onSaveGospel}
          onDeleteItem={props.onDeleteItem}
          onOpenPublic={props.onOpenPublic}
          onOpenPath={props.onOpenPath}
          onPublish={props.onPublish}
        />
      </AdminPanel>
    );
  }

  return (
    <AdminPanel
      title={day.title}
      icon="cms"
      meta={
        <div className="church-day-editor-actions">
          <ActionButton icon={props.expanded ? 'chevron-left' : 'external'} onClick={props.onToggleExpanded}>{props.expanded ? 'Свернуть' : 'На весь экран'}</ActionButton>
          <ActionButton icon="globe" onClick={props.onOpenPublic}>Открыть на сайте</ActionButton>
        </div>
      }
    >
      <header className="church-day-editor-head">
        <div>
          <span>{formatDayDate(day)}</span>
          <StatusBadge status={day.status === 'published' ? 'published' : day.status === 'archived' ? 'offline' : 'draft'} label={day.status} />
        </div>
        <div className="church-day-editor-progress">
          <strong>{completedCount === workflowSteps.length ? 'Готово' : `${completedCount} из ${workflowSteps.length} разделов заполнено`}</strong>
          <div className="church-progress-meter"><span style={{ width: `${Math.round((completedCount / workflowSteps.length) * 100)}%` }} /></div>
        </div>
        <ChurchDayLanguageSwitcher active={activeLanguage} progress={progress} onChange={setActiveLanguage} />
      </header>

      <div className="church-day-chain">
        {workflowSteps.map((step, index) => (
          <ChurchChainRow
            key={step.id}
            step={step}
            index={index}
            isLast={index === workflowSteps.length - 1}
            language={activeLanguage}
            day={day}
            saints={props.saints}
            icons={props.icons}
            prayers={props.prayers}
            articles={props.articles}
            gospel={props.gospel}
            onEdit={() => (step.id === 'overview' ? props.onEdit() : setActiveStep(step.id))}
          />
        ))}
      </div>
    </AdminPanel>
  );
}

function ChurchDayLanguageSwitcher({ active, progress, onChange }: { active: CmsLanguage; progress: ReturnType<typeof languageProgress>; onChange: (language: CmsLanguage) => void }) {
  return (
    <div className="church-language-tabs church-day-language-switcher" role="tablist" aria-label="Язык предпросмотра дня">
      {progress.map((item) => (
        <button key={item.language.id} type="button" className={active === item.language.id ? 'active' : ''} onClick={() => onChange(item.language.id)}>
          <strong>{item.language.short}</strong>
          <span>{item.language.label}</span>
          <em>{item.ready}/{item.total}</em>
        </button>
      ))}
    </div>
  );
}

function chainRowIcon(id: DetailTab): AppIconName {
  return detailTabs.find((tab) => tab.id === id)?.icon ?? 'dashboard';
}

function chainRowSummary(step: DetailTab, language: CmsLanguage, day: ChurchCalendarDay, saints: ChurchSaint[], icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[], gospel: ChurchGospel[]): { title: string; lines: string[] } {
  const languageLabel = language.toUpperCase();
  if (step === 'overview') {
    return { title: day.title || 'Название не заполнено', lines: [day.description ? truncate(day.description, 90) : 'Описание не заполнено'] };
  }
  if (step === 'saints') {
    if (!saints.length) return { title: 'Святой не создан', lines: ['Добавьте житие и день памяти'] };
    const saint = saints.find((item) => item.language === language);
    if (!saint) return { title: `Нет жития на ${languageLabel}`, lines: [`Заполнено: ${saints.map((item) => item.language.toUpperCase()).join(', ')}`] };
    return { title: saint.name, lines: [saint.feastDay || 'день памяти не заполнен'] };
  }
  if (step === 'icons') {
    if (!icons.length) return { title: 'Икона не создана', lines: ['Добавьте икону дня'] };
    const icon = icons.find((item) => item.language === language);
    if (!icon) return { title: `Нет версии на ${languageLabel}`, lines: [`Заполнено: ${icons.map((item) => item.language.toUpperCase()).join(', ')}`] };
    return { title: icon.title, lines: [icon.imageUrl ? '1 изображение' : 'фото не загружено'] };
  }
  if (step === 'prayers') {
    const langPrayers = prayers.filter((item) => item.language === language);
    if (!prayers.length) return { title: 'Молитвы не созданы', lines: ['Добавьте молитву или тропарь'] };
    if (!langPrayers.length) return { title: `Нет молитв на ${languageLabel}`, lines: [`Заполнено: ${Array.from(new Set(prayers.map((item) => item.language.toUpperCase()))).join(', ')}`] };
    const names = langPrayers.map((item) => item.title).filter(Boolean).join(', ');
    return { title: `${langPrayers.length} ${pluralPrayers(langPrayers.length)}`, lines: [names || '—'] };
  }
  if (step === 'articles') {
    if (!articles.length) return { title: 'Статья не создана', lines: ['Добавьте статью дня'] };
    const article = articles.find((item) => item.language === language);
    if (!article) return { title: `Нет версии на ${languageLabel}`, lines: [`Заполнено: ${articles.map((item) => item.language.toUpperCase()).join(', ')}`] };
    return { title: article.title, lines: [`~${article.content.length} символов`] };
  }
  if (step === 'gospel') {
    if (!gospel.length) return { title: 'Чтение не добавлено', lines: ['Необязательный раздел'] };
    const item = gospel.find((entry) => entry.language === language);
    if (!item) return { title: `Нет версии на ${languageLabel}`, lines: [`Заполнено: ${gospel.map((entry) => entry.language.toUpperCase()).join(', ')}`] };
    return { title: item.title, lines: [item.reference || 'Ссылка не заполнена'] };
  }
  return {
    title: day.status === 'published' ? 'Опубликовано' : 'Готово к публикации',
    lines: [day.status === 'published' ? 'Материал виден на сайте' : 'Проверьте SEO перед публикацией']
  };
}

function pluralPrayers(count: number) {
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod10 === 1 && mod100 !== 11) return 'молитва';
  if ([2, 3, 4].includes(mod10) && ![12, 13, 14].includes(mod100)) return 'молитвы';
  return 'молитв';
}

function truncate(value: string, max: number) {
  const trimmed = value.trim();
  return trimmed.length > max ? `${trimmed.slice(0, max - 1)}…` : trimmed;
}

function ChurchChainRow({ step, index, isLast, language, day, saints, icons, prayers, articles, gospel, onEdit }: {
  step: ReturnType<typeof buildWorkflowSteps>[number];
  index: number;
  isLast: boolean;
  language: CmsLanguage;
  day: ChurchCalendarDay;
  saints: ChurchSaint[];
  icons: ChurchIcon[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  gospel: ChurchGospel[];
  onEdit: () => void;
}) {
  const summary = chainRowSummary(step.id, language, day, saints, icons, prayers, articles, gospel);
  return (
    <article className={`church-chain-row ${step.status}`}>
      <div className="church-chain-row__rail">
        <span className="church-chain-row__dot">{step.status === 'completed' ? <AppIcon name="check" /> : index + 1}</span>
        {!isLast ? <i className="church-chain-row__line" aria-hidden="true" /> : null}
      </div>
      <div className="church-chain-row__icon"><AppIcon name={chainRowIcon(step.id)} /></div>
      <div className="church-chain-row__body">
        <strong>{step.label}</strong>
        <span>{summary.title}</span>
        {summary.lines.map((line) => <small key={line}>{line}</small>)}
      </div>
      <div className="church-chain-row__actions">
        <ActionButton icon="sliders" onClick={onEdit} disabled={step.disabled}>Редактировать</ActionButton>
      </div>
    </article>
  );
}

function ChurchStepDrillDown(props: {
  step: DetailTab;
  day: ChurchCalendarDay;
  publicDay: PublicCalendarDay | null;
  saints: ChurchSaint[];
  icons: ChurchIcon[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  gospel: ChurchGospel[];
  allIcons: ChurchIcon[];
  allPrayers: ChurchPrayer[];
  allArticles: ChurchArticle[];
  allGospel: ChurchGospel[];
  saving: boolean;
  onEditDay: () => void;
  onSaveSaint: (payload: ChurchSaintPayload, id?: string) => Promise<void>;
  onSaveIcon: (payload: ChurchIconPayload, id?: string) => Promise<void>;
  onSavePrayer: (payload: ChurchPrayerPayload, id?: string) => Promise<void>;
  onSaveArticle: (payload: ChurchArticlePayload, id?: string) => Promise<void>;
  onSaveGospel: (payload: ChurchGospelPayload, id?: string) => Promise<void>;
  onDeleteItem: (type: DeleteItemType, id: string, title: string) => Promise<void>;
  onOpenPublic: () => void;
  onOpenPath: (path: string) => void;
  onPublish: (status: ChurchContentStatus) => void;
}) {
  if (props.step === 'overview' && props.publicDay) {
    return (
      <>
        <CalendarWorkflowIntro step={{ id: 'overview', index: '01', label: 'Святой / праздник', status: 'completed', disabled: false, hint: '', icon: 'dashboard' }} description="Проверьте базовые данные дня. Если нужно изменить дату, тип или описание, нажмите Редактировать." />
        <CalendarSiteModelPreview item={props.publicDay} status={props.day.status} onEdit={props.onEditDay} onOpen={props.onOpenPublic} />
        <OverviewTab day={props.day} icons={props.icons} prayers={props.prayers} articles={props.articles} onEdit={props.onEditDay} />
      </>
    );
  }
  if (props.step === 'icons') {
    return <IconWorkflowStep icons={props.icons} allIcons={props.allIcons} saving={props.saving} onSaveIcon={props.onSaveIcon} onDeleteItem={props.onDeleteItem} />;
  }
  if (props.step === 'saints') {
    return <SaintWorkflowStep saints={props.saints} icons={props.icons} allIcons={props.allIcons} saving={props.saving} onSaveSaint={props.onSaveSaint} onDeleteItem={props.onDeleteItem} />;
  }
  if (props.step === 'prayers') {
    return <PrayerWorkflowStep prayers={props.prayers} icons={props.icons} allIcons={props.allIcons} saving={props.saving} onSavePrayer={props.onSavePrayer} onDeleteItem={props.onDeleteItem} />;
  }
  if (props.step === 'articles') {
    return <ArticleWorkflowStep articles={props.articles} icons={props.icons} allIcons={props.allIcons} saving={props.saving} onSaveArticle={props.onSaveArticle} onDeleteItem={props.onDeleteItem} />;
  }
  if (props.step === 'gospel') {
    return <GospelWorkflowStep gospel={props.gospel} icons={props.icons} allIcons={props.allIcons} saving={props.saving} onSaveGospel={props.onSaveGospel} onDeleteItem={props.onDeleteItem} />;
  }
  return (
    <SeoPublishWorkflowStep
      day={props.day}
      saints={props.saints}
      icons={props.icons}
      prayers={props.prayers}
      articles={props.articles}
      saving={props.saving}
      onPublish={() => props.onPublish('published')}
      onDraft={() => props.onPublish('draft')}
      onPreview={props.onOpenPublic}
      onOpenPath={props.onOpenPath}
    />
  );
}
