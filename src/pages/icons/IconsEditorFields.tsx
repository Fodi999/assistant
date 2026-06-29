import { useEffect, useState, type ReactNode } from 'react';
import QRCode from 'qrcode';
import { AppIcon } from '../../components/AppIcon';
import type { CalendarDay, IconPage, IconsLang, IconTranslation, QrPage } from '../../api/iconsSite';
import type { IconsSection } from '../../types/admin';
import type { EditorFieldsView, IconsItem, IconTextPatch, IconTextScopeKey, IconTextTabKey } from './iconsEditorTypes';
import { absolutePublicUrl, qrBackendPath, qrPreviewPath } from './iconsUrlUtils';

const MONTHS = [
  { value: '01', label: 'Январь' },
  { value: '02', label: 'Февраль' },
  { value: '03', label: 'Март' },
  { value: '04', label: 'Апрель' },
  { value: '05', label: 'Май' },
  { value: '06', label: 'Июнь' },
  { value: '07', label: 'Июль' },
  { value: '08', label: 'Август' },
  { value: '09', label: 'Сентябрь' },
  { value: '10', label: 'Октябрь' },
  { value: '11', label: 'Ноябрь' },
  { value: '12', label: 'Декабрь' }
];
const WEEKDAYS = ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'];
type CalendarCell = { key: string; dayNumber?: string; item?: CalendarDay };
type IconTextFieldKey = keyof Pick<IconPage, 'title' | 'shortDescription' | 'fullDescription' | 'category' | 'saintName' | 'prayerText' | 'gospelText' | 'lifeText' | 'historyText'>;

function lines(value: string) {
  return value.split('\n').map((item) => item.trim()).filter(Boolean);
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function sectionValue(text: string, title: string, fallback = '') {
  const pattern = new RegExp(`(?:^|\\n)\\*\\*${escapeRegExp(title)}:\\*\\*\\s*\\n?([\\s\\S]*?)(?=\\n\\*\\*[^\\n]+:\\*\\*|$)`, 'i');
  return text.match(pattern)?.[1]?.trim() || fallback;
}

function setSectionValue(text: string, title: string, value: string) {
  const cleanValue = value.trim();
  const block = cleanValue ? `**${title}:**\n${cleanValue}` : '';
  const pattern = new RegExp(`(?:^|\\n)\\*\\*${escapeRegExp(title)}:\\*\\*\\s*\\n?[\\s\\S]*?(?=\\n\\*\\*[^\\n]+:\\*\\*|$)`, 'i');
  const next = pattern.test(text)
    ? text.replace(pattern, block ? `${text.match(pattern)?.[0]?.startsWith('\n') ? '\n' : ''}${block}` : '')
    : [text.trim(), block].filter(Boolean).join('\n\n');
  return next.replace(/\n{3,}/g, '\n\n').trim();
}

function StructuredFields(props: {
  value: string;
  fallbackFirst?: string;
  fields: string[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="editor-structured-grid">
      {props.fields.map((field, index) => (
        <TextField
          key={`${field}-${index}`}
          label={field}
          value={sectionValue(props.value, field, index === 0 ? props.fallbackFirst || '' : '')}
          onChange={(value) => props.onChange(setSectionValue(props.value, field, value))}
        />
      ))}
    </div>
  );
}

function localizedIconValue(item: IconPage, language: IconsLang, key: IconTextFieldKey) {
  if (language === 'ru') return String(item[key] || '');
  return String(item.translations?.[language]?.[key as keyof IconTranslation] || '');
}

function localizedIconPatch(item: IconPage, language: IconsLang, patch: Partial<Record<IconTextFieldKey, string>>): IconTextPatch {
  if (language === 'ru') return patch;
  return {
    translations: {
      ...(item.translations ?? {}),
      [language]: {
        ...(item.translations?.[language] ?? {}),
        ...patch
      }
    }
  };
}

function calendarMonthForDay(day: CalendarDay) {
  return String(day.month || '01').padStart(2, '0');
}

function monthCellCount(year: number, month: string) {
  return new Date(year, Number(month), 0).getDate();
}

function monthStartOffset(year: number, month: string) {
  const nativeDay = new Date(year, Number(month) - 1, 1).getDay();
  return nativeDay === 0 ? 6 : nativeDay - 1;
}

export function CalendarMonthCards(props: {
  year: string;
  days: CalendarDay[];
  selectedMonth: string;
  onYearChange: (year: string) => void;
  onMonthChange: (month: string) => void;
}) {
  const year = Number(props.year) || new Date().getFullYear();
  const selectedMonth = String(props.selectedMonth || '01').padStart(2, '0');
  const selectedMonthLabel = MONTHS.find((month) => month.value === selectedMonth)?.label || 'Месяц';
  const daysByNumber = new Map(
    props.days
      .filter((day) => !day.month || calendarMonthForDay(day) === selectedMonth)
      .map((day) => [String(day.day).padStart(2, '0'), day])
  );
  const cells: CalendarCell[] = [
    ...Array.from({ length: monthStartOffset(year, selectedMonth) }, (_, index) => ({ key: `blank-${index}` })),
    ...Array.from({ length: monthCellCount(year, selectedMonth) }, (_, index) => {
      const dayNumber = String(index + 1).padStart(2, '0');
      return { key: dayNumber, dayNumber, item: daysByNumber.get(dayNumber) };
    })
  ];

  return (
    <section className="ops-panel calendar-month-panel">
      <div className="panel-title">
        <span><AppIcon name="cms" />Календарь по месяцам</span>
        <div className="calendar-year-control">
          <button type="button" aria-label="Предыдущий год" onClick={() => props.onYearChange(String(year - 1))}>‹</button>
          <input
            type="number"
            min="1900"
            max="2099"
            value={year}
            onChange={(event) => props.onYearChange(event.target.value)}
          />
          <button type="button" aria-label="Следующий год" onClick={() => props.onYearChange(String(year + 1))}>›</button>
          <small>{selectedMonthLabel} {year}</small>
        </div>
      </div>

      <div className="calendar-month-tabs" role="tablist" aria-label="Месяцы календаря">
        {MONTHS.map((month) => (
          <button key={month.value} className={month.value === selectedMonth ? 'active' : ''} type="button" onClick={() => props.onMonthChange(month.value)}>
            {month.label}
          </button>
        ))}
      </div>

      <div className="calendar-board">
        {WEEKDAYS.map((weekday) => <div className="calendar-weekday" key={weekday}>{weekday}</div>)}
        {cells.map((cell) => {
          if (!cell.dayNumber) return <div className="calendar-day-card empty" key={cell.key} />;
          const dayNumber = cell.dayNumber;
          const item = cell.item;
          return (
            <article
              key={cell.key}
              className={'calendar-day-card ' + (item ? item.kind : 'empty-day') + (item?.feast ? ' feast' : '') + (item?.current ? ' current' : '')}
            >
              <span className="calendar-day-number">{dayNumber}</span>
              {item ? (
                <>
                  {item.imageUrl ? <img className="calendar-day-thumb" src={item.imageUrl} alt={item.label || item.julianDate || dayNumber} loading="lazy" /> : null}
                  {item.julianDate ? <span className="calendar-julian-date">{item.julianDate}</span> : null}
                  <strong>{item.label || 'Без названия'}</strong>
                  <small>{item.note || item.kind}</small>
                  {item.description ? <em>{item.description}</em> : null}
                </>
              ) : (
                <>
                  <strong>Не заполнено</strong>
                  <small>{selectedMonthLabel.toLowerCase()}</small>
                </>
              )}
            </article>
          );
        })}
      </div>
    </section>
  );
}

export function QrCodePreview({ item, onGenerate }: { item: IconPage | QrPage; onGenerate?: () => void }) {
  const [dataUrl, setDataUrl] = useState('');
  const [qrError, setQrError] = useState('');
  const path = qrPreviewPath(item);
  const targetUrl = absolutePublicUrl(path);
  const apiPath = qrBackendPath(item);

  useEffect(() => {
    let active = true;
    setQrError('');
    QRCode.toDataURL(targetUrl, {
      errorCorrectionLevel: 'M',
      margin: 2,
      width: 256,
      color: {
        dark: '#111111',
        light: '#ffffff'
      }
    })
      .then((url) => {
        if (active) setDataUrl(url);
      })
      .catch((error: unknown) => {
        if (!active) return;
        setDataUrl('');
        setQrError(error instanceof Error ? error.message : 'QR не создан');
      });
    return () => {
      active = false;
    };
  }, [targetUrl]);

  return (
    <div className="icon-qr-preview-card">
      <div className="icon-qr-preview-image">
        {dataUrl ? <img src={dataUrl} alt={`QR для ${targetUrl}`} /> : <span>{qrError || 'Генерация QR...'}</span>}
      </div>
      <div className="icon-qr-preview-meta">
        <strong>QR preview</strong>
        <span title={targetUrl}>{targetUrl}</span>
        <small title={apiPath}>Backend: {apiPath}</small>
        {onGenerate ? <button className="btn btn-quiet" type="button" onClick={onGenerate}>Создать / обновить QR</button> : null}
      </div>
    </div>
  );
}

export function IconTextTabs(props: {
  item: IconPage;
  activeScope: IconTextScopeKey;
  onScopeChange: (scope: IconTextScopeKey) => void;
  activeLanguage: IconsLang;
  onLanguageChange: (language: IconsLang) => void;
  activeTab: IconTextTabKey;
  onTabChange: (tab: IconTextTabKey) => void;
  patch: (patch: IconTextPatch) => void;
  onGenerate?: () => void;
  onGenerateAll?: () => void;
  generating?: boolean;
}) {
  const languages: Array<{ key: IconsLang; label: string }> = [
    { key: 'uk', label: 'UK' },
    { key: 'ru', label: 'RU' },
    { key: 'en', label: 'EN' }
  ];
  const scopes: Array<{ key: IconTextScopeKey; label: string }> = [
    { key: 'icon', label: 'Икона' },
    { key: 'saints', label: 'Святые' },
    { key: 'church', label: 'Храм' }
  ];
  const tabs: Array<{ key: IconTextTabKey; label: string }> = [
    { key: 'description', label: 'Описание' },
    { key: 'prayer', label: 'Молитва' },
    { key: 'gospel', label: 'Евангелие' },
    { key: 'life', label: 'Житие' },
    { key: 'history', label: 'История' }
  ];
  const isSaints = props.activeScope === 'saints';
  const isChurch = props.activeScope === 'church';
  const isIcon = props.activeScope === 'icon';
  const descriptionFields = isSaints
    ? ['Главные святые дня', 'Кратко кто это', 'Годы / век', 'Чем известен', 'Память по календарю', 'Источники']
    : isChurch
      ? ['Название храма', 'Кому посвящён', 'Страна / город', 'Адрес', 'Google Maps ссылка', 'Расписание богослужений', 'Телефон / сайт', 'Краткое описание', 'Святыни / иконы / мощи', 'Фото храма']
      : ['Краткое описание изображения', 'Символы на иконе', 'Alt для фото', 'Prompt для генерации', 'Источник изображения'];
  const mainDescriptionFields = ['Полное описание', 'Смысл праздника', 'Что важно знать', 'Для кого эта молитва/страница', 'Не писать'];
  const prayerFields = ['Тропарь', 'Кондак', 'Величание', 'Краткая молитва', 'Молитва своими словами', 'Язык', 'Источник текста'];
  const gospelFields = ['Апостольское чтение', 'Евангельское чтение', 'Цитата дня', 'Объяснение простыми словами', 'Связь с событием', 'Источник'];
  const lifeFields = ['Краткое житие', 'Подробное житие', 'Главные события жизни', 'Духовный смысл', 'Где почитается', 'Источники'];
  const historyFields = ['История праздника', 'Дата по старому стилю', 'Дата по новому стилю', 'Разные календарные традиции', 'Почему бывает путаница', 'Проверенные источники', 'Дата проверена', 'Календарный стиль', 'Найденное событие', 'Уверенность', 'Предупреждение'];
  const activeScopeLabel = scopes.find((scope) => scope.key === props.activeScope)?.label || 'Раздел';
  const activeTabLabel = tabs.find((tab) => tab.key === props.activeTab)?.label || 'Текст';
  const textValue = (key: IconTextFieldKey) => localizedIconValue(props.item, props.activeLanguage, key);
  const patchText = (patch: Partial<Record<IconTextFieldKey, string>>) => props.patch(localizedIconPatch(props.item, props.activeLanguage, patch));

  return (
    <EditorSection title="Тексты материала" note="Переключайте блоки и редактируйте только нужный текст.">
      <div className="editor-language-tabs" role="tablist" aria-label="Язык материала">
        {languages.map((language) => (
          <button key={language.key} className={props.activeLanguage === language.key ? 'active' : ''} type="button" onClick={() => props.onLanguageChange(language.key)}>
            {language.label}
          </button>
        ))}
      </div>

      <div className="editor-subtabs editor-combined-tabs" role="tablist" aria-label="Раздел сайта и текстовые блоки">
        {scopes.map((scope) => (
          <button key={scope.key} className={'scope-tab ' + (props.activeScope === scope.key ? 'active' : '')} type="button" onClick={() => props.onScopeChange(scope.key)}>
            {scope.label}
          </button>
        ))}
        <i aria-hidden="true" />
        {tabs.map((tab) => (
          <button key={tab.key} className={'text-tab ' + (props.activeTab === tab.key ? 'active' : '')} type="button" onClick={() => props.onTabChange(tab.key)}>
            {tab.label}
          </button>
        ))}
      </div>

      <div className="editor-text-ai-bar">
        <div>
          <strong>{activeScopeLabel} / {activeTabLabel}</strong>
          <small>Gemini перепроверит источники и заполнит поля сразу на UK/RU/EN. RU сохраняется в основные поля, UK/EN — в translations.</small>
        </div>
        {props.onGenerate ? (
          <div className="editor-text-ai-actions">
            <button className="btn btn-ai" type="button" onClick={props.onGenerate} disabled={props.generating}>
              <AppIcon name="bot" />
              {props.generating ? 'Gemini проверяет...' : 'Gemini: вкладку ×3'}
            </button>
            {props.onGenerateAll ? (
              <button className="btn btn-secondary" type="button" onClick={props.onGenerateAll} disabled={props.generating}>
                <AppIcon name="sparkles" />
                Все вкладки ×3
              </button>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="editor-subtab-panel">
        {props.activeTab === 'description' ? (
          <>
            <div className="settings-matrix">
              <Field label={isChurch ? 'Название материала для храмов' : 'Название иконы / события'} value={textValue('title')} onChange={(value) => patchText({ title: value })} />
              <Field label={isSaints ? 'Главный святой / праздник' : isChurch ? 'Категория / направление' : 'Святой / праздник'} value={textValue('saintName')} onChange={(value) => patchText({ saintName: value })} />
              <Field label="Категория" value={textValue('category')} onChange={(value) => patchText({ category: value })} />
            </div>
            <TextField
              label={isIcon ? 'Краткое описание' : isSaints ? 'Краткое описание святых' : 'Краткое описание для храмов'}
              value={textValue('shortDescription')}
              onChange={(value) => patchText({ shortDescription: value })}
            />
            <StructuredFields
              value={textValue('fullDescription')}
              fallbackFirst={textValue('fullDescription')}
              fields={[...descriptionFields, ...mainDescriptionFields]}
              onChange={(value) => patchText({ fullDescription: value })}
            />
          </>
        ) : null}
        {props.activeTab === 'prayer' ? <StructuredFields value={textValue('prayerText')} fallbackFirst={textValue('prayerText')} fields={prayerFields} onChange={(value) => patchText({ prayerText: value })} /> : null}
        {props.activeTab === 'gospel' ? <StructuredFields value={textValue('gospelText')} fallbackFirst={textValue('gospelText')} fields={gospelFields} onChange={(value) => patchText({ gospelText: value })} /> : null}
        {props.activeTab === 'life' ? <StructuredFields value={textValue('lifeText')} fallbackFirst={textValue('lifeText')} fields={lifeFields} onChange={(value) => patchText({ lifeText: value })} /> : null}
        {props.activeTab === 'history' ? <StructuredFields value={textValue('historyText')} fallbackFirst={textValue('historyText')} fields={historyFields} onChange={(value) => patchText({ historyText: value })} /> : null}
      </div>
    </EditorSection>
  );
}

export function EditorFields(props: {
  section: IconsSection;
  item: IconsItem;
  patch: (patch: Record<string, string | number | boolean | string[] | undefined>) => void;
  view?: EditorFieldsView;
}) {
  const { item, patch, view = 'all' } = props;

  if ('day' in item) {
    return (
      <>
        <div className="settings-matrix">
          <Field label="День" value={item.day} onChange={(value) => patch({ day: value })} />
          <Field label="Гражданская дата" value={item.gregorianDate || ''} onChange={() => undefined} readOnly />
          <Field label="Юлианская дата" value={item.julianDate || ''} onChange={() => undefined} readOnly />
          <Field label="Название в ячейке" value={item.label} onChange={(value) => patch({ label: value })} />
          <Field label="Тип подписи" value={item.note} onChange={(value) => patch({ note: value })} />
          <SelectField label="Тип дня" value={item.kind} options={['feast', 'fast', 'gospel', 'prayer', 'quiet']} onChange={(value) => patch({ kind: value })} />
          <Field label="Фото дня URL" value={item.imageUrl || ''} onChange={(value) => patch({ imageUrl: value })} />
          <Field label="Slug иконы" value={item.iconSlug} onChange={(value) => patch({ iconSlug: value })} />
          <Field label="Slug молитвы" value={item.prayerSlug} onChange={(value) => patch({ prayerSlug: value })} />
          <Field label="Slug Евангелия" value={item.gospelSlug} onChange={(value) => patch({ gospelSlug: value })} />
          <Field label="Подробнее URL" value={item.detailHref} onChange={(value) => patch({ detailHref: value })} />
          <SelectField label="Золотая метка" value={String(item.current)} options={['true', 'false']} onChange={(value) => patch({ current: value === 'true' })} />
          <SelectField label="Красная метка" value={String(item.feast)} options={['true', 'false']} onChange={(value) => patch({ feast: value === 'true' })} />
          <SelectField label="Пустая ячейка" value={String(item.textOnly)} options={['true', 'false']} onChange={(value) => patch({ textOnly: value === 'true' })} />
        </div>
        <TextField label="Краткое описание / list-view" value={item.description} onChange={(value) => patch({ description: value })} />
      </>
    );
  }

  if ('qrCodeUrl' in item) {
    return (
      <>
        {view === 'all' || view === 'main' ? <div className="editor-section-grid">
          <EditorSection title="Основное" note="Название, адрес страницы и тип материала. Это видно в заголовке и карточках сайта.">
            <div className="settings-matrix">
              <Field label="Название" value={item.title} onChange={(value) => patch({ title: value })} />
              <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} />
              <Field label="Категория" value={item.category} onChange={(value) => patch({ category: value })} />
              <Field label="Святой / праздник" value={item.saintName} onChange={(value) => patch({ saintName: value })} />
              <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} />
            </div>
          </EditorSection>

          <EditorSection title="Связи сайта" note="QR и календарь используют slug страницы. После сохранения backend привязывает календарную ячейку к этой странице.">
            <div className="settings-matrix">
              <Field label="QR code URL" value={item.qrCodeUrl} onChange={(value) => patch({ qrCodeUrl: value })} />
              <Field label="Страница на сайте" value={`/icons/${item.slug || item.id}`} onChange={() => undefined} readOnly />
            </div>
          </EditorSection>
        </div> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Описание на странице" note="Короткое описание видно в первом экране. Полное описание идет ниже заголовка.">
          <TextField label="Краткое описание" value={item.shortDescription} onChange={(value) => patch({ shortDescription: value })} />
          <TextField label="Полное описание" value={item.fullDescription} onChange={(value) => patch({ fullDescription: value })} />
        </EditorSection> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Тексты разделов" note="Эти блоки попадают в карточки «Молитва», «Евангелие», «Житие» и «История образа» на странице.">
          <TextField label="Молитва" value={item.prayerText} onChange={(value) => patch({ prayerText: value })} />
          <TextField label="Евангелие / чтение" value={item.gospelText} onChange={(value) => patch({ gospelText: value })} />
          <TextField label="Житие" value={item.lifeText} onChange={(value) => patch({ lifeText: value })} />
          <TextField label="История образа" value={item.historyText} onChange={(value) => patch({ historyText: value })} />
        </EditorSection> : null}

        {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Эти поля нужны для Google и превью страницы. На самой странице посетитель обычно их не видит.">
          <div className="settings-matrix">
            <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} />
            <Field label="SEO keywords" value={item.seoKeywords || ''} onChange={(value) => patch({ seoKeywords: value })} />
          </div>
          <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} />
        </EditorSection> : null}
      </>
    );
  }

  if ('reference' in item) {
    return (
      <>
        {view === 'all' || view === 'main' ? <EditorSection title="Евангелие дня" note="Дата, заголовок и ссылка используются на странице «Евангелие».">
          <div className="settings-matrix">
            <Field label="Дата" value={item.date} onChange={(value) => patch({ date: value })} />
            <Field label="Заголовок" value={item.title} onChange={(value) => patch({ title: value })} />
            <Field label="Ссылка / место чтения" value={item.reference} onChange={(value) => patch({ reference: value })} />
            <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} />
          </div>
        </EditorSection> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Тексты чтения" note="Эти поля заполняют страницу «Євангеліє».">
          <TextField label="Текст чтения" value={item.text} onChange={(value) => patch({ text: value })} />
          <TextField label="Пояснение" value={item.explanation} onChange={(value) => patch({ explanation: value })} />
        </EditorSection> : null}

        {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Заголовок и описание для поисковиков и превью.">
          <div className="settings-matrix">
            <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} />
          </div>
          <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} />
        </EditorSection> : null}
      </>
    );
  }

  if ('qrId' in item) {
    return (
      <>
        <QrCodePreview item={item} />
        <div className="settings-matrix">
          <Field label="QR ID" value={item.qrId} onChange={(value) => patch({ qrId: value })} />
          <Field label="Icon ID" value={item.iconId} onChange={(value) => patch({ iconId: value })} />
          <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} />
          <Field label="Название QR-страницы" value={item.title} onChange={(value) => patch({ title: value })} />
          <Field label="Владелец / подпись" value={item.ownerName || ''} onChange={(value) => patch({ ownerName: value })} />
          <Field label="Локация" value={item.location || ''} onChange={(value) => patch({ location: value })} />
          <Field label="Сканы" value={String(item.scanCount)} onChange={(value) => patch({ scanCount: Number(value) || 0 })} />
          <SelectField label="Активна" value={String(item.active)} options={['true', 'false']} onChange={(value) => patch({ active: value === 'true' })} />
          <TextField label="Особая молитва" value={item.customPrayer || ''} onChange={(value) => patch({ customPrayer: value })} />
        </div>
      </>
    );
  }

  if ('biography' in item) {
    return (
      <>
        {view === 'all' || view === 'main' ? <EditorSection title="Святой" note="Эти данные попадают в список и на страницу святого.">
          <div className="settings-matrix">
            <Field label="Имя" value={item.name} onChange={(value) => patch({ name: value })} />
            <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} />
            <Field label="Дата памяти" value={item.feastDay} onChange={(value) => patch({ feastDay: value })} />
            <Field label="Икона URL" value={item.imageUrl} onChange={(value) => patch({ imageUrl: value })} />
            <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} />
          </div>
          <TextField label="Краткое описание" value={item.shortDescription} onChange={(value) => patch({ shortDescription: value })} />
        </EditorSection> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Житие и связи" note="Биография открывает страницу святого; связанные slugs помогают вести человека к иконам и молитвам.">
          <TextField label="Биография" value={item.biography} onChange={(value) => patch({ biography: value })} />
          <div className="settings-matrix">
            <Field label="Связанные иконы" value={item.relatedIcons.join('\n')} onChange={(value) => patch({ relatedIcons: lines(value) })} />
            <Field label="Молитвы" value={item.prayers.join('\n')} onChange={(value) => patch({ prayers: lines(value) })} />
          </div>
        </EditorSection> : null}

        {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Заголовок и описание для поисковиков и превью.">
          <div className="settings-matrix">
            <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} />
          </div>
          <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} />
        </EditorSection> : null}
      </>
    );
  }

  if ('address' in item) {
    return (
      <>
        {view === 'all' || view === 'main' ? <EditorSection title="Храм" note="Название, город и адрес наполняют страницу «Храмам».">
          <div className="settings-matrix">
            <Field label="Название" value={item.title} onChange={(value) => patch({ title: value })} />
            <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} />
            <Field label="Город" value={item.city} onChange={(value) => patch({ city: value })} />
            <Field label="Адрес" value={item.address} onChange={(value) => patch({ address: value })} />
            <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} />
          </div>
        </EditorSection> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Описание и связи" note="Эти тексты выводятся в карточках храмов и партнерских блоках.">
          <TextField label="Описание" value={item.description} onChange={(value) => patch({ description: value })} />
          <div className="settings-matrix">
            <Field label="Расписание" value={item.schedule} onChange={(value) => patch({ schedule: value })} />
            <Field label="Donation URL" value={item.donationUrl || ''} onChange={(value) => patch({ donationUrl: value })} />
            <Field label="Связанные иконы" value={item.relatedIcons.join('\n')} onChange={(value) => patch({ relatedIcons: lines(value) })} />
          </div>
        </EditorSection> : null}

        {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Заголовок и описание для поисковиков и превью.">
          <div className="settings-matrix">
            <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} />
          </div>
          <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} />
        </EditorSection> : null}
      </>
    );
  }

  if (props.section === 'prayers' && 'text' in item) {
    return (
      <>
        {view === 'all' || view === 'main' ? <EditorSection title="Молитва" note="Название, категория и связанная икона наполняют список молитв.">
          <div className="settings-matrix">
            <Field label="Название" value={item.title} onChange={(value) => patch({ title: value })} />
            <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} />
            <Field label="Категория" value={item.category} onChange={(value) => patch({ category: value })} />
            <Field label="Связанная икона slug" value={item.relatedIcon || ''} onChange={(value) => patch({ relatedIcon: value })} />
            <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} />
          </div>
        </EditorSection> : null}

        {view === 'all' || view === 'texts' ? <EditorSection title="Текст молитвы" note="Полный текст выводится на странице молитвы.">
          <TextField label="Текст" value={item.text} onChange={(value) => patch({ text: value })} />
        </EditorSection> : null}

        {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Заголовок и описание для поисковиков и превью.">
          <div className="settings-matrix">
            <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} />
          </div>
          <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} />
        </EditorSection> : null}
      </>
    );
  }

  return (
    <>
      {view === 'all' || view === 'main' ? <EditorSection title="Страница" note="Основные поля SEO-страницы или служебного материала.">
        <div className="settings-matrix">
          <Field label="Название" value={item.title} onChange={(value) => patch({ title: value })} />
          {'slug' in item ? <Field label="URL slug" value={item.slug} onChange={(value) => patch({ slug: value })} /> : null}
          {'category' in item ? <Field label="Категория" value={item.category} onChange={(value) => patch({ category: value })} /> : null}
          {'h1' in item ? <Field label="H1" value={item.h1} onChange={(value) => patch({ h1: value })} /> : null}
          {'pageType' in item ? <Field label="Тип страницы" value={item.pageType} onChange={(value) => patch({ pageType: value })} /> : null}
          {'targetKeyword' in item ? <Field label="Ключевой запрос" value={item.targetKeyword} onChange={(value) => patch({ targetKeyword: value })} /> : null}
          {'language' in item ? <Field label="Язык" value={item.language} onChange={(value) => patch({ language: value })} /> : null}
          {'imageUrl' in item ? <Field label="Фото URL" value={item.imageUrl || ''} onChange={(value) => patch({ imageUrl: value })} /> : null}
          {'city' in item ? <Field label="Город" value={item.city || ''} onChange={(value) => patch({ city: value })} /> : null}
          {'status' in item ? <SelectField label="Статус" value={item.status} options={['draft', 'published']} onChange={(value) => patch({ status: value })} /> : null}
        </div>
      </EditorSection> : null}

      {view === 'all' || view === 'texts' ? <EditorSection title="Тексты страницы" note="Основной контент, который видит посетитель.">
        {'text' in item ? <TextField label="Текст" value={item.text} onChange={(value) => patch({ text: value })} /> : null}
        {'content' in item ? <TextField label="Тело SEO-страницы" value={item.content} onChange={(value) => patch({ content: value })} /> : null}
        {'blocks' in item ? <TextField label="Блоки страницы" value={item.blocks.join('\n')} onChange={(value) => patch({ blocks: lines(value) })} /> : null}
      </EditorSection> : null}

      {view === 'all' || view === 'seo' ? <EditorSection title="SEO" note="Заголовок и описание для поисковиков и превью.">
        <div className="settings-matrix">
          {'seoTitle' in item ? <Field label="SEO title" value={item.seoTitle || ''} onChange={(value) => patch({ seoTitle: value })} /> : null}
        </div>
        {'seoDescription' in item ? <TextField label="SEO description" value={item.seoDescription || ''} onChange={(value) => patch({ seoDescription: value })} /> : null}
      </EditorSection> : null}
    </>
  );
}

export function EditorSection(props: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className="editor-info-section">
      <div className="editor-info-head">
        <strong>{props.title}</strong>
        {props.note ? <span>{props.note}</span> : null}
      </div>
      <div className="editor-info-body">{props.children}</div>
    </section>
  );
}

function Field(props: { label: string; value: string; onChange: (value: string) => void; readOnly?: boolean }) {
  return <label className="editor-field"><span>{props.label}</span><input value={props.value} readOnly={props.readOnly} onChange={(event) => props.onChange(event.target.value)} /></label>;
}

function TextField(props: { label: string; value: string; onChange: (value: string) => void }) {
  return <label className="editor-field"><span>{props.label}</span><textarea value={props.value} onChange={(event) => props.onChange(event.target.value)} /></label>;
}

export function SelectField(props: { label: string; value: string; options: string[]; onChange: (value: string) => void }) {
  return <label className="editor-field"><span>{props.label}</span><select value={props.value} onChange={(event) => props.onChange(event.target.value)}>{props.options.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>;
}
