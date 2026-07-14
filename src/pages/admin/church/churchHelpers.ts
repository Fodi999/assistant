import type { AiGenerationResult } from '../../../types/admin';
import { isValidSlug } from '../../../components/admin/forms/formUtils';
import type {
  ChurchArticle,
  ChurchCalendarDay,
  ChurchContentStatus,
  ChurchDayType,
  ChurchGospel,
  ChurchIcon,
  ChurchLanguage,
  ChurchPrayer,
  ChurchPrayerType,
  ChurchSaint
} from '../../../api/churchContent';
import type { CalendarDay as PublicCalendarDay } from '../../../api/iconsSite';
import {
  cmsLanguages,
  dayTypes,
  iconPhotoSlots,
  prayerTypes,
  type CalendarDateBasis,
  type CalendarMonthSlot,
  type CalendarSlot,
  type CalendarViewMode,
  type CmsLanguage,
  type IconExtraPhotoRole,
  type LanguageFieldStatus,
  type LanguageStatus,
  type WorkflowStep,
  type WorkflowStepStatus
} from './churchTypes';

export const monthNames = ['Январь', 'Февраль', 'Март', 'Апрель', 'Май', 'Июнь', 'Июль', 'Август', 'Сентябрь', 'Октябрь', 'Ноябрь', 'Декабрь'];

export function dateKeyFromChurchDay(day: ChurchCalendarDay | undefined | null) {
  return day?.dateNewStyle || day?.dateOldStyle || '';
}

export function mapChurchDaysByDate(days: ChurchCalendarDay[]) {
  const map = new Map<string, ChurchCalendarDay>();
  days.forEach((day) => {
    const key = dateKeyFromChurchDay(day);
    if (key) map.set(key, day);
  });
  return map;
}

export function dateFromISO(date: string) {
  const [year, month, day] = date.split('-').map((part) => Number(part));
  return new Date(Date.UTC(year || new Date().getUTCFullYear(), (month || 1) - 1, day || 1));
}

export function formatISODate(date: Date) {
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, '0')}-${String(date.getUTCDate()).padStart(2, '0')}`;
}

export function getDaysInUtcMonth(year: number, monthIndex: number) {
  return new Date(Date.UTC(year, monthIndex + 1, 0)).getUTCDate();
}

export function weekdayShort(date: Date) {
  return ['ВС', 'ПН', 'ВТ', 'СР', 'ЧТ', 'ПТ', 'СБ'][date.getUTCDay()];
}

export function daysInWeek(date: string) {
  const parsed = dateFromISO(date);
  const day = parsed.getUTCDay() || 7;
  parsed.setUTCDate(parsed.getUTCDate() - day + 1);
  return Array.from({ length: 7 }, (_, index) => {
    const next = new Date(parsed);
    next.setUTCDate(parsed.getUTCDate() + index);
    return formatISODate(next);
  });
}

export function daysInMonth(date: string) {
  const parsed = dateFromISO(date);
  const year = parsed.getUTCFullYear();
  const month = parsed.getUTCMonth();
  return Array.from({ length: getDaysInUtcMonth(year, month) }, (_, index) => formatISODate(new Date(Date.UTC(year, month, index + 1))));
}

export function buildCalendarSlots(mode: CalendarViewMode, anchorDate: string, selectedDate: string, daysByDate: Map<string, ChurchCalendarDay>): CalendarSlot[] {
  const dates = mode === 'week' ? daysInWeek(anchorDate) : daysInMonth(anchorDate);
  const today = new Date().toISOString().slice(0, 10);
  return dates.map((date) => {
    const parsed = dateFromISO(date);
    const day = daysByDate.get(date) ?? null;
    return {
      key: date,
      date,
      label: String(parsed.getUTCDate()).padStart(2, '0'),
      weekday: weekdayShort(parsed),
      day,
      isToday: date === today,
      isSelected: date === selectedDate
    };
  });
}

export function buildCalendarMonthSlots(anchorDate: string, daysByDate: Map<string, ChurchCalendarDay>): CalendarMonthSlot[] {
  const year = dateFromISO(anchorDate).getUTCFullYear();
  return Array.from({ length: 12 }, (_, index) => {
    const date = formatISODate(new Date(Date.UTC(year, index, 1)));
    const total = getDaysInUtcMonth(year, index);
    const filled = Array.from(daysByDate.keys()).filter((key) => {
      const parsed = dateFromISO(key);
      return parsed.getUTCFullYear() === year && parsed.getUTCMonth() === index;
    }).length;
    return { key: date, date, label: monthNames[index], filled, total };
  });
}

export function shiftDateByMode(date: string, mode: CalendarViewMode, direction: -1 | 1) {
  const parsed = dateFromISO(date);
  if (mode === 'week') parsed.setUTCDate(parsed.getUTCDate() + direction * 7);
  else parsed.setUTCMonth(parsed.getUTCMonth() + direction);
  return formatISODate(parsed);
}

export function calendarPeriodTitle(mode: CalendarViewMode, date: string) {
  const parsed = dateFromISO(date);
  const month = monthNames[parsed.getUTCMonth()];
  if (mode === 'week') {
    const week = daysInWeek(date);
    const first = dateFromISO(week[0]);
    const last = dateFromISO(week[6]);
    return `${first.getUTCDate()} ${monthNames[first.getUTCMonth()]} - ${last.getUTCDate()} ${monthNames[last.getUTCMonth()]} ${last.getUTCFullYear()}`;
  }
  return `${month} ${parsed.getUTCFullYear()}`;
}

export function contentCounts(dayId: string, icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[]) {
  return {
    icons: icons.filter((item) => item.calendarDayId === dayId).length,
    prayers: prayers.filter((item) => item.calendarDayId === dayId).length,
    articles: articles.filter((item) => item.calendarDayId === dayId).length
  };
}

export function formatDayDate(day: ChurchCalendarDay) {
  return [day.dateNewStyle, day.dateOldStyle ? `${day.dateOldStyle} old` : ''].filter(Boolean).join(' / ') || 'No date';
}

export function createDraftCalendarDay(date: string): ChurchCalendarDay {
  return {
    id: `new-calendar-day-${date}`,
    siteId: 'church',
    dateOldStyle: '',
    dateNewStyle: date,
    calendarType: 'both',
    title: '',
    dayType: 'saint',
    description: '',
    rank: 0,
    status: 'draft',
    isGlobal: false,
    createdAt: '',
    updatedAt: ''
  };
}

export function isCalendarDayComplete(day: ChurchCalendarDay) {
  return Boolean(
    (day.dateNewStyle || day.dateOldStyle)
    && day.title.trim()
    && day.dayType
    && day.calendarType
    && day.status
    && day.description.trim()
  );
}

export function areSeoFieldsReady(articles: ChurchArticle[]) {
  return articles.some((article) => Boolean(article.slug.trim() && article.seoTitle.trim() && article.seoDescription.trim()));
}

export function buildWorkflowSteps(day: ChurchCalendarDay | null, saints: ChurchSaint[], icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[], gospel: ChurchGospel[] = []): WorkflowStep[] {
  const dayComplete = Boolean(day && isCalendarDayComplete(day));
  const seoReady = areSeoFieldsReady(articles);
  const publishReady = Boolean(day?.status === 'published' && dayComplete && saints.length && icons.length && prayers.length && articles.length && seoReady);
  const lockedHint = 'Сначала заполните данные дня';
  return [
    {
      id: 'overview',
      index: '01',
      label: 'Святой / праздник',
      status: dayComplete ? 'completed' : day ? 'warning' : 'empty',
      disabled: false,
      hint: dayComplete ? 'Базовые данные заполнены' : 'Заполните название, тип, стиль календаря, статус и описание',
      icon: 'dashboard'
    },
    {
      id: 'saints',
      index: '02',
      label: 'Святой',
      status: saints.length ? 'completed' : dayComplete ? 'empty' : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Создайте житие святого' : lockedHint,
      icon: 'sparkles'
    },
    {
      id: 'icons',
      index: '03',
      label: 'Икона',
      status: icons.length ? 'completed' : dayComplete ? 'empty' : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Свяжите икону' : lockedHint,
      icon: 'qr'
    },
    {
      id: 'prayers',
      index: '04',
      label: 'Молитвы',
      status: prayers.length ? 'completed' : dayComplete ? 'empty' : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Добавьте молитву' : lockedHint,
      icon: 'sparkles'
    },
    {
      id: 'articles',
      index: '05',
      label: 'Статья',
      status: articles.length ? 'completed' : dayComplete ? 'empty' : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Добавьте статью' : lockedHint,
      icon: 'cms'
    },
    {
      id: 'gospel',
      index: '06',
      label: 'Евангелие',
      status: gospel.length ? 'completed' : dayComplete ? 'empty' : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Добавьте чтение (необязательно)' : lockedHint,
      icon: 'globe'
    },
    {
      id: 'publish',
      index: '07',
      label: 'SEO и публикация',
      status: publishReady ? 'completed' : dayComplete ? (seoReady ? 'draft' : 'warning') : 'draft',
      disabled: !dayComplete,
      hint: dayComplete ? 'Проверьте SEO и публикацию' : lockedHint,
      icon: 'seo'
    }
  ];
}

export function workflowStatusLabel(status: WorkflowStepStatus) {
  if (status === 'completed') return 'completed';
  if (status === 'warning') return 'warning';
  if (status === 'draft') return 'draft';
  return 'empty';
}

export function languageStatusLabel(status: LanguageStatus) {
  if (status === 'needs-translation') return 'needs translation';
  return status;
}

export function languageStatusFromRecord<T>(record: Record<CmsLanguage, T>, pickFields: (value: T) => string[]): LanguageFieldStatus {
  const ukFilled = pickFields(record.uk).filter((value) => value.trim()).length;
  return cmsLanguages.reduce((acc, language) => {
    const fields = pickFields(record[language.id]);
    const filled = fields.filter((value) => value.trim()).length;
    acc[language.id] = filled === 0
      ? language.id === 'uk' || ukFilled === 0 ? 'empty' : 'needs-translation'
      : filled >= Math.max(2, fields.length - 1) ? 'completed' : 'draft';
    return acc;
  }, {} as LanguageFieldStatus);
}

export function contentLanguageStatus<T extends { language: ChurchLanguage; status: ChurchContentStatus }>(items: T[], isReady: (item: T) => boolean): LanguageFieldStatus {
  return cmsLanguages.reduce((acc, language) => {
    const item = items.find((entry) => entry.language === language.id);
    acc[language.id] = !item ? 'empty' : isReady(item) ? 'completed' : item.status === 'draft' ? 'draft' : 'needs-translation';
    return acc;
  }, {} as LanguageFieldStatus);
}

export function progressStateLabel(value: string) {
  const labels: Record<string, string> = {
    empty: 'пусто',
    draft: 'черновик',
    completed: 'готово',
    ready: 'готово',
    missing: 'не заполнено',
    published: 'опубликовано',
    'not ready': 'не готово',
    linked: 'связано'
  };
  return labels[value] ?? value;
}

export function languageProgress(day: ChurchCalendarDay | null, saints: ChurchSaint[], icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[]) {
  return cmsLanguages.map((language) => {
    const saint = saints.find((item) => item.language === language.id);
    const icon = icons.find((item) => item.language === language.id);
    const prayer = prayers.find((item) => item.language === language.id);
    const article = articles.find((item) => item.language === language.id);
    const checks = [
      Boolean(day && isCalendarDayComplete(day)),
      Boolean(saint?.name && saint.biography),
      Boolean(icon?.imageUrl),
      Boolean(prayer?.title && prayer.text),
      Boolean(article?.title && article.content),
      Boolean(article?.slug && article.seoTitle && article.seoDescription)
    ];
    const ready = checks.filter(Boolean).length;
    return {
      language,
      ready,
      total: checks.length,
      saint: saint ? (saint.status === 'published' ? 'completed' : 'draft') : 'empty',
      prayer: prayer ? (prayer.status === 'published' ? 'completed' : 'draft') : 'empty',
      article: article ? (article.status === 'published' ? 'completed' : 'draft') : 'empty',
      seo: article?.slug && article.seoTitle && article.seoDescription ? 'ready' : 'missing',
      publish: article?.status === 'published' || (day?.status === 'published' && language.id === 'uk') ? 'published' : ready >= 4 ? 'draft' : 'not ready'
    };
  });
}

export function imageForContent(iconId: string | null | undefined, dayIcons: ChurchIcon[], allIcons: ChurchIcon[]) {
  return [...dayIcons, ...allIcons].find((icon) => icon.id === iconId)?.imageUrl || dayIcons[0]?.imageUrl || '';
}

export function stripIconExtraPhotos(description: string) {
  return description.split(/\n\nДополнительные фото:/)[0]?.trim() ?? '';
}

export function parseIconExtraPhotos(description: string): Record<IconExtraPhotoRole, string> {
  const result: Record<IconExtraPhotoRole, string> = { icon: '', angleOne: '', angleTwo: '' };
  const block = description.split(/\n\nДополнительные фото:/)[1] || '';
  iconPhotoSlots.forEach((slot) => {
    if (slot.role === 'cover') return;
    const role = slot.role as IconExtraPhotoRole;
    const escapedTitle = slot.title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const match = block.match(new RegExp(`${escapedTitle}:\\s*(https?:\\/\\/\\S+)`, 'i'));
    if (match?.[1]) result[role] = match[1];
  });
  return result;
}

export function publicCalendarKindFromChurch(dayType: ChurchDayType): PublicCalendarDay['kind'] {
  if (dayType === 'fasting') return 'fast';
  if (dayType === 'gospel') return 'gospel';
  if (dayType === 'memorial' || dayType === 'saint') return 'prayer';
  if (dayType === 'feast') return 'feast';
  return 'quiet';
}

export function prayerQrPath(slug: string) {
  return slug ? `/prayers/${slug}` : '';
}

export function publicCalendarDayFromChurch(day: ChurchCalendarDay, icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[]): PublicCalendarDay {
  const date = day.dateNewStyle || day.dateOldStyle || '';
  const dayNumber = (date.split('-')[2] || day.dateNewStyle?.slice(-2) || day.dateOldStyle?.slice(-2) || '01').padStart(2, '0');
  const icon = icons[0];
  const prayer = prayers[0];
  const article = articles[0];
  const imageUrl = icon?.imageUrl || '';
  return {
    id: day.id,
    month: (date.split('-')[1] || '').replace(/^0/, '') || undefined,
    day: dayNumber,
    gregorianDate: day.dateNewStyle || undefined,
    julianDate: day.dateOldStyle || undefined,
    label: day.title,
    note: icon?.saintName || icon?.feastName || day.dayType,
    kind: publicCalendarKindFromChurch(day.dayType),
    imageUrl,
    iconSlug: icon?.slug || '',
    prayerSlug: prayer?.slug || '',
    gospelSlug: '',
    detailHref: date ? `/church/calendar/${date}` : `/church/articles/${article?.slug || ''}`,
    current: date === new Date().toISOString().slice(0, 10),
    feast: day.dayType === 'feast',
    textOnly: !imageUrl,
    description: day.description || article?.seoDescription || article?.content || ''
  };
}

export function publicCalendarSlotFromDate(date: string): PublicCalendarDay {
  const parsed = dateFromISO(date);
  return {
    id: `empty-${date}`,
    month: String(parsed.getUTCMonth() + 1),
    day: String(parsed.getUTCDate()).padStart(2, '0'),
    gregorianDate: date,
    julianDate: undefined,
    label: 'День не заполнен',
    note: 'Свободная дата календаря',
    kind: 'quiet',
    imageUrl: '',
    iconSlug: '',
    prayerSlug: '',
    gospelSlug: '',
    detailHref: `/church/calendar/${date}`,
    current: date === new Date().toISOString().slice(0, 10),
    feast: false,
    textOnly: true,
    description: 'Нажмите "Создать день", чтобы добавить святого, праздник, икону, молитву и статью.'
  };
}

function parseIsoDateParts(value: string) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return null;
  return { year: Number(match[1]), month: Number(match[2]), day: Number(match[3]) };
}

function formatDateParts(parts: { year: number; month: number; day: number }) {
  return `${String(parts.year).padStart(4, '0')}-${String(parts.month).padStart(2, '0')}-${String(parts.day).padStart(2, '0')}`;
}

function gregorianToJdn(year: number, month: number, day: number) {
  const a = Math.floor((14 - month) / 12);
  const y = year + 4800 - a;
  const m = month + 12 * a - 3;
  return day + Math.floor((153 * m + 2) / 5) + 365 * y + Math.floor(y / 4) - Math.floor(y / 100) + Math.floor(y / 400) - 32045;
}

function julianToJdn(year: number, month: number, day: number) {
  const a = Math.floor((14 - month) / 12);
  const y = year + 4800 - a;
  const m = month + 12 * a - 3;
  return day + Math.floor((153 * m + 2) / 5) + 365 * y + Math.floor(y / 4) - 32083;
}

function jdnToGregorian(jdn: number) {
  const a = jdn + 32044;
  const b = Math.floor((4 * a + 3) / 146097);
  const c = a - Math.floor((146097 * b) / 4);
  const d = Math.floor((4 * c + 3) / 1461);
  const e = c - Math.floor((1461 * d) / 4);
  const m = Math.floor((5 * e + 2) / 153);
  return {
    day: e - Math.floor((153 * m + 2) / 5) + 1,
    month: m + 3 - 12 * Math.floor(m / 10),
    year: 100 * b + d - 4800 + Math.floor(m / 10)
  };
}

function jdnToJulian(jdn: number) {
  const c = jdn + 32082;
  const d = Math.floor((4 * c + 3) / 1461);
  const e = c - Math.floor((1461 * d) / 4);
  const m = Math.floor((5 * e + 2) / 153);
  return {
    day: e - Math.floor((153 * m + 2) / 5) + 1,
    month: m + 3 - 12 * Math.floor(m / 10),
    year: d - 4800 + Math.floor(m / 10)
  };
}

export function convertGregorianToJulianDate(value: string) {
  const parsed = parseIsoDateParts(value);
  if (!parsed) return '';
  return formatDateParts(jdnToJulian(gregorianToJdn(parsed.year, parsed.month, parsed.day)));
}

export function convertJulianToGregorianDate(value: string) {
  const parsed = parseIsoDateParts(value);
  if (!parsed) return '';
  return formatDateParts(jdnToGregorian(julianToJdn(parsed.year, parsed.month, parsed.day)));
}

export function nextCalendarStyleDates(basis: CalendarDateBasis, value: string) {
  if (!value) return { newStyle: '', oldStyle: '' };
  return basis === 'new'
    ? { newStyle: value, oldStyle: convertGregorianToJulianDate(value) }
    : { newStyle: convertJulianToGregorianDate(value), oldStyle: value };
}

export function normalizeCalendarStyleDates(newStyle: string, oldStyle: string, preferredBasis: CalendarDateBasis) {
  if (preferredBasis === 'old' && oldStyle) return nextCalendarStyleDates('old', oldStyle);
  if (newStyle) return nextCalendarStyleDates('new', newStyle);
  if (oldStyle) return nextCalendarStyleDates('old', oldStyle);
  return { newStyle: '', oldStyle: '' };
}

export function compactText(value: string, max: number) {
  const text = value.replace(/\n{3,}/g, '\n\n').trim();
  return text.length > max ? `${text.slice(0, max - 1).trim()}…` : text;
}

export function slugify(value: string) {
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

function sectionFromText(text: string, names: string[]) {
  for (const name of names) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const pattern = new RegExp(`${escaped}\\s*[:\\-]\\s*([\\s\\S]*?)(?=\\n[А-ЯA-Z][^\\n:]{1,80}\\s*[:\\-]|$)`, 'i');
    const match = text.match(pattern);
    if (match?.[1]) return match[1].trim();
  }
  return '';
}

export type ChurchAiDraftPayload = {
  topic: string;
  dateOldStyle: string;
  dateNewStyle: string;
  calendarType: ChurchCalendarDay['calendarType'];
  dayType: ChurchDayType;
  language: ChurchLanguage;
  rank: string;
  imageUrl: string;
  generateImage: boolean;
};

export type ParsedChurchAiDraft = {
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

export function buildChurchAiPrompt(payload: ChurchAiDraftPayload) {
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

export function parseChurchAiDraft(result: AiGenerationResult, payload: ChurchAiDraftPayload): ParsedChurchAiDraft {
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
    dayType: dayTypes.includes(dayTypeRaw as ChurchDayType) ? (dayTypeRaw as ChurchDayType) : payload.dayType,
    dayDescription,
    iconTitle,
    iconSlug: result.slug && isValidSlug(result.slug) ? result.slug : slugify(iconTitle),
    iconDescription,
    saintName,
    feastName,
    prayerTitle,
    prayerSlug: slugify(prayerTitle),
    prayerText,
    prayerType: prayerTypes.includes(prayerTypeRaw as ChurchPrayerType) ? (prayerTypeRaw as ChurchPrayerType) : 'prayer',
    articleTitle,
    articleSlug: slugify(articleTitle),
    articleContent,
    seoTitle,
    seoDescription,
    photoPrompt
  };
}
