import { useEffect, useMemo, useRef, useState } from 'react';
import QRCode from 'qrcode';
import { AppIcon } from '../../components/AppIcon';
import { AdminPageHeader } from '../../components/admin/AdminLayout';
import { analyzePhotoWithGemini, generateAiImage, generateSeo } from '../../api/ai';
import { uploadCmsReference } from '../../api/cms';
import { AiReferenceUpload, type AiReferenceImage } from '../../components/AiReferenceUpload';
import {
  getIconsSiteContent,
  saveIconsSiteContent,
  type CalendarDay,
  type ChurchPage,
  type GospelReading,
  type IconPage,
  type IconsLang,
  type IconsSiteContent,
  type IconTranslation,
  type PrayerPage,
  type QrPage,
  type SaintPage,
  type SeoPage
} from '../../api/iconsSite';
import type { AiGenerationResult, IconsSection, LanguageCode } from '../../types/admin';
import { CalendarMonthCards, EditorFields, EditorSection, IconTextTabs, QrCodePreview, SelectField } from './IconsEditorFields';
import type { EditorTabKey, IconPhotoAspect, IconsItem, IconTextPatch, IconTextScopeKey, IconTextTabKey } from './iconsEditorTypes';
import { absolutePublicUrl, isGeneratedQrImage, QR_GALLERY_MARKER, qrPageUrlForIcon } from './iconsUrlUtils';

const ICON_IMAGE_MAX = 8;
const initialCalendarDate = new Date();
const initialCalendarYear = String(initialCalendarDate.getFullYear());
const initialCalendarMonth = String(initialCalendarDate.getMonth() + 1).padStart(2, '0');
const defaultCalendarAiPrompt = 'Проверь именно юлианскую дату ячейки. Если есть память святого или праздник, опиши этот день и создай календарную иллюстрацию. Если нет, не выдумывай святого: сделай простой день церковного календаря.';

const iconPhotoAspectPrompts: Record<IconPhotoAspect, string> = {
  source: 'Keep the same aspect ratio and framing as Reference 1.',
  square: 'Create a square 1:1 product image, centered composition, no important object cropped.',
  landscape: 'Create a horizontal 4:3 product image, balanced catalog composition, no important object cropped.',
  portrait: 'Create a vertical 3:4 product image, full product visible, no important object cropped.',
  wide: 'Create a wide horizontal 16:9 product image, full product visible with clean catalog margins.'
};

const sections: Array<{ key: IconsSection; label: string; note: string; icon: 'cms' | 'seo' | 'globe' | 'leads' | 'sparkles' }> = [
  { key: 'calendar', label: 'Календарь', note: 'заполнение по датам', icon: 'seo' },
  { key: 'icons', label: 'Иконы', note: 'карточки, QR, молитвы', icon: 'cms' },
  { key: 'prayers', label: 'Молитвы', note: 'тексты и SEO', icon: 'sparkles' },
  { key: 'saints', label: 'Святые', note: 'биографии и дни памяти', icon: 'globe' },
  { key: 'gospel', label: 'Евангелие', note: 'чтения по дням', icon: 'seo' },
  { key: 'qr', label: 'QR-страницы', note: 'публичные ссылки', icon: 'leads' },
  { key: 'seo', label: 'SEO-страницы', note: 'лендинги под запросы', icon: 'seo' },
  { key: 'churches', label: 'Храмы', note: 'партнерские страницы', icon: 'globe' }
];

const defaultCalendar: IconsSiteContent['calendar'] = {
  hero: {
    year: '2026',
    title: 'Свет Иконы',
    monthTitle: 'Январь 2026',
    prevLabel: '← Декабрь',
    prevHref: '#',
    nextLabel: 'Февраль →',
    nextHref: '#',
    featureTitle: 'Святитель Василий Великий',
    featureNote: 'Память святого',
    featureDate: '14 января (ст. ст.)',
    featureHref: '/saints/nikolay-chudotvorets',
    iconDayTitle: 'Икона святителя Николая Чудотворца',
    iconDayIconSlug: 'nikolay-chudotvorets',
    iconDayDate: '14 января 2026',
    iconDayPrayerSlug: 'molitva-kazanskoy-ikone',
    infoPrimary: 'Сегодняшний праздник',
    infoSecondary: 'Важный день',
    todayDate: '14 января 2026',
    todayGospel: 'Мф. 5:14-16',
    todayPrayerTitle: 'Молитва перед Казанской иконой Божией Матери',
    todayHref: '/gospel'
  },
  days: [
    { id: 'calendar-jan-01', day: '01', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-02', day: '02', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-03', day: '03', label: 'Икона Божией Матери «Казанская»', note: 'Праздничная икона', kind: 'feast', imageUrl: '', iconSlug: 'kazan-icon', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/kazan-icon', current: false, feast: true, textOnly: false, description: 'Молитва о семье, мире и укреплении в вере.' },
    { id: 'calendar-jan-04', day: '04', label: 'Святитель Николай Чудотворец', note: 'Память святого', kind: 'feast', imageUrl: '', iconSlug: 'nikolay-chudotvorets', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/nikolay-chudotvorets', current: false, feast: false, textOnly: false, description: 'Почитание святого, помощника в пути и нужде.' },
    { id: 'calendar-jan-05', day: '05', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-06', day: '06', label: 'Крещение Господне', note: 'Праздник', kind: 'feast', imageUrl: '', iconSlug: 'kazan-icon', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/kazan-icon', current: true, feast: false, textOnly: false, description: 'Воспоминание Богоявления и освящения вод.' },
    { id: 'calendar-jan-07', day: '07', label: 'Рождество Христово', note: 'Празднество', kind: 'fast', imageUrl: '', iconSlug: 'nikolay-chudotvorets', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/nikolay-chudotvorets', current: false, feast: true, textOnly: false, description: 'Праздничное чтение и домашняя молитва.' },
    { id: 'calendar-jan-08', day: '08', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-09', day: '09', label: 'Блаженная Матрона Московская', note: 'Память святой', kind: 'prayer', imageUrl: '', iconSlug: 'kazan-icon', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/kazan-icon', current: false, feast: false, textOnly: false, description: 'Молитва о помощи в житейских обстоятельствах.' },
    { id: 'calendar-jan-10', day: '10', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-11', day: '11', label: 'Великомученик Пантелеимон', note: 'Память святого', kind: 'prayer', imageUrl: '', iconSlug: 'nikolay-chudotvorets', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/nikolay-chudotvorets', current: false, feast: false, textOnly: false, description: 'Молитвенное обращение о болящих.' },
    { id: 'calendar-jan-12', day: '12', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-13', day: '13', label: 'Собор Предтечи и Крестителя Господня Иоанна', note: 'Память святого', kind: 'feast', imageUrl: '', iconSlug: 'nikolay-chudotvorets', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/nikolay-chudotvorets', current: false, feast: false, textOnly: false, description: 'День молитвенного почитания Предтечи.' },
    { id: 'calendar-jan-14', day: '14', label: 'Обрезание Господне', note: 'Господский праздник', kind: 'feast', imageUrl: '', iconSlug: 'obrezanie-gospodne', prayerSlug: 'obrezanie-gospodne', gospelSlug: 'today', detailHref: '/icons/obrezanie-gospodne', current: true, feast: true, textOnly: false, description: 'Праздник Обрезания Господня: 1 января по церковному юлианскому календарю, 14 января по гражданскому календарю.' },
    { id: 'calendar-jan-15', day: '15', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-16', day: '16', label: 'Икона Божией Матери «Умиление»', note: 'Праздничная икона', kind: 'feast', imageUrl: '', iconSlug: 'kazan-icon', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/kazan-icon', current: false, feast: false, textOnly: false, description: 'Молитва о мире сердца и покаянии.' },
    { id: 'calendar-jan-17', day: '17', label: '', note: '', kind: 'quiet', imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: true, description: '' },
    { id: 'calendar-jan-18', day: '18', label: 'Неделя 32-я по Пятидесятнице', note: 'Евангельское чтение', kind: 'gospel', imageUrl: '', iconSlug: 'kazan-icon', prayerSlug: 'molitva-kazanskoy-ikone', gospelSlug: 'today', detailHref: '/icons/kazan-icon', current: false, feast: false, textOnly: false, description: 'Чтение напоминает о тихом свидетельстве веры через добрые дела.' }
  ],
  services: [
    { id: 'service-prayers', index: '01', title: 'Молитвы на каждый день', description: 'Краткое правило и молитвы перед иконой.', href: '/prayers' },
    { id: 'service-gospel', index: '02', title: 'Евангелие дня', description: 'Чтение, ссылка и спокойное объяснение.', href: '/gospel' },
    { id: 'service-feasts', index: '03', title: 'Праздники и посты', description: 'Церковные даты, важные дни и отметки.', href: '/p/pravoslavnaya-ikona-s-qr-kodom' },
    { id: 'service-icons', index: '04', title: 'Иконы святых', description: 'История образов, жития и QR-страницы.', href: '/icons' }
  ]
};

const emptyContent: IconsSiteContent = {
  icons: [],
  prayers: [],
  gospel: [],
  saints: [],
  pages: [],
  qrPages: [],
  churches: [],
  calendar: defaultCalendar,
  dashboard: { publishedPages: 0, icons: 0, prayers: 0, qrPages: 0, qrScans: 0, churches: 0, latestPages: [], seo: [] }
};

function slugify(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/ё/g, 'e')
    .replace(/[^a-zа-я0-9]+/gi, '-')
    .replace(/^-+|-+$/g, '') || `page-${Date.now()}`;
}

function statusClass(status: IconPage['status']) {
  return status === 'published' ? 'good' : 'warning';
}

function uniqueImages(images: Array<string | undefined | null>) {
  return Array.from(new Set(images.map((url) => (url || '').trim()).filter(Boolean)));
}

function iconImages(item: IconsItem | undefined) {
  if (!item || !('qrCodeUrl' in item)) return [];
  return uniqueImages([item.imageUrl, ...(item.imageUrls ?? [])]);
}

function supportsDirectPhotoForItem(item: IconsItem | undefined) {
  return Boolean(item && ('qrCodeUrl' in item || 'biography' in item || 'address' in item || 'imageUrl' in item));
}

function getItemPreviewImage(item: IconsItem) {
  if ('imageUrl' in item) return uniqueImages([item.imageUrl, ...('imageUrls' in item ? (item.imageUrls ?? []) : [])])[0] || '';
  return '';
}

async function createQrGalleryImageUrl(item: IconPage) {
  const svg = await QRCode.toString(absolutePublicUrl(qrPageUrlForIcon(item)), {
    type: 'svg',
    errorCorrectionLevel: 'M',
    margin: 2,
    width: 512,
    color: {
      dark: '#111111',
      light: '#ffffff'
    }
  });
  const markedSvg = svg.replace('<svg ', `<svg ${QR_GALLERY_MARKER} `);
  return `data:image/svg+xml;utf8,${encodeURIComponent(markedSvg)}`;
}

async function prepareIconForSave(item: IconPage) {
  const normalizedItem = normalizeIconPageText(item);
  const baseImages = uniqueImages([item.imageUrl, ...(item.imageUrls ?? [])]).filter((url) => !isGeneratedQrImage(url));
  const qrCodeUrl = item.qrCodeUrl || qrPageUrlForIcon(item);
  const qrImageUrl = await createQrGalleryImageUrl({ ...normalizedItem, qrCodeUrl });
  const gallery = uniqueImages([...baseImages.slice(0, 2), qrImageUrl, ...baseImages.slice(2)]).slice(0, ICON_IMAGE_MAX);

  return {
    ...normalizedItem,
    qrCodeUrl,
    imageUrl: gallery[0] || item.imageUrl,
    imageUrls: gallery.slice(1),
    updatedAt: new Date().toISOString()
  };
}

function mergeBySlug<T extends { slug: string }>(manualItems: T[], generatedItems: T[]) {
  const existing = new Set(manualItems.map((item) => item.slug));
  return [...manualItems, ...generatedItems.filter((item) => !existing.has(item.slug))];
}

function firstTextLine(value: string, fallback: string) {
  return value.split(/\n{2,}|\n/).map((line) => line.trim()).find(Boolean) || fallback;
}

function gospelReferenceFromText(value: string) {
  return value.match(/\(([^()]*?(?:Мф|Мк|Лк|Ин|Деян|Рим|Кор|Гал|Еф|Флп|Кол|Фес|Тим|Тит|Евр|Пет|Иак|Иуд|Отк)[^()]*)\)/i)?.[1]?.trim() || 'Чтение дня';
}

function derivedPrayersFromIcons(icons: IconPage[]): PrayerPage[] {
  return icons
    .filter((icon) => icon.status === 'published' && icon.prayerText.trim())
    .map((icon) => ({
      id: `prayer-${icon.slug}`,
      slug: icon.slug,
      title: icon.title.includes('молит') || icon.title.includes('Молит') ? icon.title : `Молитва: ${icon.title}`,
      category: icon.category || 'Молитвы перед иконой',
      text: icon.prayerText,
      relatedIcon: icon.slug,
      status: 'published' as const,
      seoTitle: `Молитва перед ${icon.title}`,
      seoDescription: compactText(icon.prayerText, 220)
    }));
}

function derivedSaintsFromIcons(icons: IconPage[]): SaintPage[] {
  return icons
    .filter((icon) => icon.status === 'published' && (icon.saintName.trim() || icon.lifeText.trim()))
    .map((icon) => ({
      id: `saint-${icon.slug}`,
      slug: icon.slug,
      name: icon.saintName || icon.title,
      feastDay: icon.calendarDate || '',
      imageUrl: icon.imageUrl,
      shortDescription: icon.shortDescription || firstTextLine(icon.lifeText, icon.title),
      biography: icon.lifeText || icon.fullDescription || icon.historyText,
      relatedIcons: [icon.slug],
      prayers: icon.prayerText.trim() ? [icon.slug] : [],
      status: 'published' as const,
      seoTitle: `${icon.saintName || icon.title}: житие и день памяти`,
      seoDescription: compactText(icon.lifeText || icon.shortDescription || icon.fullDescription, 220)
    }));
}

function derivedGospelFromIcons(icons: IconPage[]): GospelReading[] {
  return icons
    .filter((icon) => icon.status === 'published' && icon.gospelText.trim())
    .map((icon) => ({
      id: `gospel-${icon.slug}`,
      date: icon.calendarDate || new Date().toISOString().slice(0, 10),
      title: `Евангелие: ${icon.title}`,
      reference: gospelReferenceFromText(icon.gospelText),
      text: icon.gospelText,
      explanation: icon.shortDescription || icon.fullDescription || 'Евангельское чтение связано с молитвой перед образом и внимательной христианской жизнью.',
      status: 'published' as const,
      seoTitle: `Евангелие дня: ${icon.title}`,
      seoDescription: compactText(icon.gospelText, 220)
    }));
}

function derivedChurchesFromIcons(icons: IconPage[]): ChurchPage[] {
  const publishedIcons = icons.filter((icon) => icon.status === 'published');
  if (!publishedIcons.length) return [];
  const source = publishedIcons[0];
  return [{
    id: 'church-svet-ikony-qr',
    slug: 'svet-ikony-dlya-hramov',
    title: 'Свет Иконы для храмов',
    city: 'Онлайн',
    address: 'QR-страницы православных икон',
    description: source.historyText || source.fullDescription || 'Храм может подключить QR-страницы икон, чтобы прихожане открывали молитву, житие святого, Евангелие дня и историю образа прямо рядом со святыней.',
    schedule: 'Подключение и наполнение страниц настраивается в админке.',
    relatedIcons: publishedIcons.map((icon) => icon.slug),
    status: 'published' as const,
    donationUrl: '',
    seoTitle: 'QR-иконы и молитвенные страницы для храмов',
    seoDescription: 'Материалы для храмов: QR-страницы икон, молитвы, жития, Евангелие дня и описание святынь.'
  }];
}

type FixedCalendarRule = {
  day: string;
  label: string;
  note: string;
  kind: CalendarDay['kind'];
  feast: boolean;
  priority: number;
  aliases: string[];
  description: string;
  source: string;
};

type PendingItemPatch = {
  section: IconsSection;
  patch: Record<string, string | number | boolean | string[] | undefined | IconPage['translations']>;
};

const fixedCalendarRules: FixedCalendarRule[] = [
  {
    day: '14',
    label: 'Обрезание Господне',
    note: 'Господский праздник',
    kind: 'feast',
    feast: true,
    priority: 100,
    aliases: ['обрезание господне', 'обрезанию господню', 'circumcision of our lord'],
    description: 'Праздник Обрезания Господня: 1 января по церковному юлианскому календарю, 14 января по гражданскому календарю. Источник: OCA Feasts & Saints; православный календарь 1/14 января.',
    source: 'OCA Feasts & Saints; православный календарь 1/14 января'
  },
  {
    day: '07',
    label: 'Рождество Христово',
    note: 'Двунадесятый праздник',
    kind: 'feast',
    feast: true,
    priority: 100,
    aliases: ['рождество христово', 'рождеству христову', 'nativity of christ'],
    description: 'Рождество Христово: 25 декабря по юлианскому календарю, 7 января по гражданскому календарю.',
    source: 'Православный календарь 25 декабря / 7 января'
  },
  {
    day: '19',
    label: 'Крещение Господне',
    note: 'Богоявление',
    kind: 'feast',
    feast: true,
    priority: 100,
    aliases: ['крещение господне', 'богоявление', 'theophany', 'baptism of the lord'],
    description: 'Крещение Господне, или Богоявление: 6 января по юлианскому календарю, 19 января по гражданскому календарю.',
    source: 'Православный календарь 6/19 января'
  },
  {
    day: '20',
    label: 'Собор Предтечи и Крестителя Господня Иоанна',
    note: 'Память святого',
    kind: 'feast',
    feast: false,
    priority: 60,
    aliases: ['собор предтечи', 'собор иоанна предтечи', 'ioann predtecha', 'john the baptist synaxis'],
    description: 'Собор Иоанна Предтечи: 7 января по юлианскому календарю, 20 января по гражданскому календарю.',
    source: 'Православный календарь 7/20 января'
  },
  {
    day: '14',
    label: 'Святитель Василий Великий',
    note: 'Память святого',
    kind: 'feast',
    feast: false,
    priority: 50,
    aliases: ['василий великий', 'святитель василий', 'basil the great'],
    description: 'Память святителя Василия Великого совершается 1 января по юлианскому календарю, 14 января по гражданскому календарю.',
    source: 'Православный календарь 1/14 января'
  }
];

function normalizeLookupText(value: string) {
  return value.toLowerCase().replace(/ё/g, 'е').replace(/[^a-zа-я0-9]+/gi, ' ').replace(/\s+/g, ' ').trim();
}

function calendarRuleForIcon(icon: IconPage) {
  const haystack = normalizeLookupText([
    icon.title,
    icon.slug,
    icon.category,
    icon.saintName,
    icon.shortDescription,
    icon.fullDescription,
    icon.seoTitle,
    icon.seoDescription,
    icon.seoKeywords
  ].filter(Boolean).join(' '));
  return fixedCalendarRules.find((rule) => rule.aliases.some((alias) => haystack.includes(normalizeLookupText(alias))));
}

function calendarRuleForLabel(label: string) {
  const normalized = normalizeLookupText(label);
  return fixedCalendarRules.find((rule) => rule.aliases.some((alias) => normalized.includes(normalizeLookupText(alias))));
}

function syncFixedFeastsWithCalendar(calendar: IconsSiteContent['calendar'], icons: IconPage[]) {
  const days = calendar.days.map((day) => ({ ...day }));

  icons.forEach((icon) => {
    const rule = calendarRuleForIcon(icon);
    if (!rule) return;
    const iconSlug = icon.slug || icon.id;
    const targetIndex = days.findIndex((day) => day.day === rule.day);
    const currentTargetRule = targetIndex >= 0 ? calendarRuleForLabel(days[targetIndex].label) : undefined;
    if (currentTargetRule && currentTargetRule.priority > rule.priority) return;
    const nextDay: CalendarDay = {
      ...(targetIndex >= 0 ? days[targetIndex] : {
        id: `calendar-jan-${rule.day}`,
        day: rule.day,
        label: '',
        note: '',
        kind: rule.kind,
        imageUrl: '',
        iconSlug: '',
        prayerSlug: '',
        gospelSlug: 'today',
        detailHref: '/icons',
        current: false,
        feast: false,
        textOnly: false,
        description: ''
      }),
      label: rule.label,
      note: rule.note,
      kind: rule.kind,
      imageUrl: icon.imageUrl,
      iconSlug,
      prayerSlug: iconSlug,
      detailHref: `/icons/${iconSlug}`,
      feast: rule.feast,
      textOnly: false,
      description: rule.description
    };

    days.forEach((day, index) => {
      const sameIcon = day.iconSlug === iconSlug || day.detailHref === `/icons/${iconSlug}`;
      const sameRuleWrongDay = rule.aliases.some((alias) => normalizeLookupText(day.label).includes(normalizeLookupText(alias))) && day.day !== rule.day;
      if (index !== targetIndex && (sameIcon || sameRuleWrongDay)) {
        days[index] = {
          ...day,
          label: '',
          note: '',
          kind: 'quiet',
          imageUrl: '',
          iconSlug: '',
          prayerSlug: '',
          detailHref: '/icons',
          feast: false,
          textOnly: true,
          description: ''
        };
      }
    });

    if (targetIndex >= 0) days[targetIndex] = nextDay;
    else days.push(nextDay);
  });

  return {
    ...calendar,
    days: days.sort((a, b) => Number(a.day) - Number(b.day))
  };
}

async function prepareContentForSave(content: IconsSiteContent) {
  const icons = await Promise.all(content.icons.map(prepareIconForSave));
  const derivedPrayers = derivedPrayersFromIcons(icons);
  const derivedSaints = derivedSaintsFromIcons(icons);
  const derivedGospel = derivedGospelFromIcons(icons);
  const derivedChurches = derivedChurchesFromIcons(icons);
  const qrPagesById = new Map(content.qrPages.map((page) => [page.qrId, page]));
  const now = new Date().toISOString();
  const ensuredQrPages = icons.map((icon) => {
    const qrId = icon.slug || icon.id;
    const current = qrPagesById.get(qrId);
    qrPagesById.delete(qrId);
    return {
      id: current?.id || `qr-${qrId}`,
      qrId,
      iconId: icon.id,
      slug: qrId,
      title: current?.title || icon.title,
      active: current?.active ?? true,
      scanCount: current?.scanCount ?? 0,
      ownerName: current?.ownerName || 'Страница иконы',
      location: current?.location || 'QR на иконе',
      customPrayer: current?.customPrayer || icon.prayerText,
      createdAt: current?.createdAt || now,
      updatedAt: now
    };
  });

  return {
    ...content,
    icons,
    prayers: mergeBySlug(content.prayers, derivedPrayers),
    saints: mergeBySlug(content.saints, derivedSaints),
    gospel: derivedGospel.length ? derivedGospel : content.gospel,
    churches: mergeBySlug(content.churches, derivedChurches),
    calendar: syncFixedFeastsWithCalendar(content.calendar, icons),
    qrPages: [...ensuredQrPages, ...Array.from(qrPagesById.values())]
  };
}

function getSectionItems(content: IconsSiteContent, section: IconsSection) {
  if (section === 'calendar') return (content.calendar ?? defaultCalendar).days;
  if (section === 'qr') return content.qrPages;
  if (section === 'seo') return content.pages;
  return content[section];
}

function normalizeContent(content: IconsSiteContent): IconsSiteContent {
  return {
    ...emptyContent,
    ...content,
    dashboard: { ...emptyContent.dashboard, ...(content.dashboard ?? {}) },
    calendar: {
      hero: { ...defaultCalendar.hero, ...(content.calendar?.hero ?? {}) },
      days: content.calendar?.days?.length
        ? content.calendar.days.map((day) => ({ ...day, imageUrl: day.imageUrl ?? '' }))
        : defaultCalendar.days,
      services: content.calendar?.services?.length ? content.calendar.services : defaultCalendar.services
    }
  };
}

function applyPendingItemPatches(content: IconsSiteContent, patches: Record<string, PendingItemPatch>) {
  let next = content;
  Object.entries(patches).forEach(([id, pending]) => {
    if (pending.section === 'calendar') {
      next = {
        ...next,
        calendar: {
          ...next.calendar,
          days: next.calendar.days.map((item) => getItemId(item) === id ? { ...item, ...pending.patch } : item)
        }
      };
      return;
    }
    if (pending.section === 'qr') {
      next = { ...next, qrPages: next.qrPages.map((item) => getItemId(item) === id ? { ...item, ...pending.patch } : item) };
      return;
    }
    if (pending.section === 'seo') {
      next = { ...next, pages: next.pages.map((item) => getItemId(item) === id ? { ...item, ...pending.patch } : item) };
      return;
    }
    next = {
      ...next,
      [pending.section]: (next[pending.section] as IconsItem[]).map((item) => getItemId(item) === id ? { ...item, ...pending.patch } : item)
    };
  });
  return next;
}

function getItemId(item: IconsItem) {
  return item.id;
}

function getItemTitle(item: IconsItem) {
  if ('day' in item) return item.label || `День ${item.day}`;
  if ('name' in item) return item.name;
  if ('qrId' in item) return item.qrId;
  return item.title;
}

function getItemSubline(section: IconsSection, item: IconsItem) {
  if ('day' in item) return `день ${item.day} / ${item.kind}`;
  if ('iconId' in item) return `икона: ${item.iconId}`;
  if ('slug' in item) return `/${section}/${item.slug}`;
  if ('date' in item) return item.reference;
  return '';
}

function iconReadiness(icon: IconPage) {
  const checks = [
    Boolean(icon.title.trim()),
    Boolean(icon.slug.trim()),
    Boolean(icon.imageUrl.trim()),
    Boolean(icon.shortDescription.trim()),
    Boolean(icon.fullDescription.trim()),
    Boolean(icon.prayerText.trim()),
    Boolean(icon.seoTitle?.trim()),
    Boolean(icon.seoDescription?.trim()),
    Boolean(icon.qrCodeUrl?.trim()),
    Boolean(icon.calendarDate?.trim())
  ];
  const complete = checks.filter(Boolean).length;
  return {
    complete,
    total: checks.length,
    percent: Math.round((complete / checks.length) * 100)
  };
}

function iconMissingFields(icon: IconPage) {
  const checks: Array<[string, unknown]> = [
    ['photo', icon.imageUrl],
    ['description', icon.shortDescription && icon.fullDescription],
    ['prayer', icon.prayerText],
    ['SEO', icon.seoTitle && icon.seoDescription],
    ['QR', icon.qrCodeUrl],
    ['calendar', icon.calendarDate]
  ];
  return checks.filter(([, value]) => !value).map(([label]) => label);
}

type BackendIconAudit = {
  score: number;
  total: number;
  ok: boolean;
  message: string;
  missing: string[];
  warnings: string[];
  checkedAt: string;
  source: 'backend-rulebot';
};

function IconEditorOverview(props: {
  item: IconPage;
  image: string;
  onPatch: (patch: Record<string, string | boolean | undefined>) => void;
  backendAudit?: BackendIconAudit;
  backendChecking?: boolean;
  onBackendCheck: () => void;
}) {
  const readiness = iconReadiness(props.item);
  const missing = iconMissingFields(props.item);
  const backendAudit = props.backendAudit;

  return (
    <section className="icon-editor-overview">
      <div className="icon-editor-cover">
        {props.image ? <img src={props.image} alt={props.item.title} /> : <span><AppIcon name="image" />Добавить фото</span>}
      </div>
      <div className="icon-editor-summary">
        <div className="icon-editor-title-row">
          <span className={'status-pill ' + statusClass(props.item.status)}><i />{props.item.status === 'published' ? 'опубликовано' : 'черновик'}</span>
          <strong>{props.item.title || 'Новая икона'}</strong>
        </div>
        <span className="icon-editor-slug">/icons/{props.item.slug || props.item.id}</span>
        <p>{props.item.shortDescription || props.item.seoDescription || 'Добавьте краткое описание, SEO, фото и дату календаря перед публикацией.'}</p>
        <div className="icon-readiness large">
          <span style={{ width: `${readiness.percent}%` }} />
        </div>
        <div className="icon-editor-missing">
          <strong>local {readiness.complete}/{readiness.total}</strong>
          <span>{missing.length ? `Нужно: ${missing.join(', ')}` : 'Локально заполнено'}</span>
          {backendAudit ? (
            <span className={backendAudit.ok ? 'backend-ok' : 'backend-warn'}>
              backend {backendAudit.score}/{backendAudit.total}: {backendAudit.message}
            </span>
          ) : (
            <span className="backend-warn">backend: нужна проверка RuleBot</span>
          )}
        </div>
        {backendAudit && (backendAudit.missing.length || backendAudit.warnings.length) ? (
          <div className="icon-backend-audit-list">
            {[...backendAudit.missing, ...backendAudit.warnings].slice(0, 4).map((item) => <span key={item}>{item}</span>)}
          </div>
        ) : null}
      </div>
      <div className="icon-editor-status-actions">
        <button className="table-action" type="button" onClick={props.onBackendCheck} disabled={props.backendChecking}>
          {props.backendChecking ? 'Проверяет...' : 'Проверить ботом'}
        </button>
        <button
          className={props.item.status === 'published' ? 'table-action danger' : 'table-action publish'}
          type="button"
          onClick={() => props.onPatch({ status: props.item.status === 'published' ? 'draft' : 'published' })}
        >
          {props.item.status === 'published' ? 'В черновик' : 'Опубликовать'}
        </button>
      </div>
    </section>
  );
}

function SiteSectionVisualPreview({ section, content, items }: { section: IconsSection; content: IconsSiteContent; items: IconsItem[] }) {
  if (section === 'prayers') {
    const prayers = items.filter(isPrayerPage);
    const heroImage = prayers
      .map((prayer) => prayer.relatedIcon ? content.icons.find((item) => item.slug === prayer.relatedIcon)?.imageUrl : '')
      .find(Boolean);
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="Молитвы" path="/uk/prayers" note="Так блок молитв будет выглядеть на сайте: карточки собираются из текстов икон." />
        <div className="site-preview-hero" style={heroImage ? { backgroundImage: `linear-gradient(180deg, rgba(20,24,31,.18), rgba(8,10,14,.94)), url("${heroImage}")` } : undefined}>
          <p>Молитви</p>
          <h3>Молитви перед іконами та на кожен день</h3>
          <span>Короткий вступ, список молитв и переход к связанному образу.</span>
        </div>
        <div className="site-prayer-grid">
          {prayers.slice(0, 6).map((prayer) => {
            const icon = prayer.relatedIcon ? content.icons.find((item) => item.slug === prayer.relatedIcon) : undefined;
            return (
              <article className="site-prayer-card" key={prayer.id}>
                {icon?.imageUrl ? <img src={icon.imageUrl} alt={prayer.title} loading="lazy" /> : <span><AppIcon name="sparkles" /></span>}
                <div>
                  <small>{prayer.category || 'Молитва'}</small>
                  <strong>{prayer.title}</strong>
                  <p>{compactText(prayer.text || prayer.seoDescription || 'Текст молитвы появится здесь.', 210)}</p>
                  <em>/prayers/{prayer.slug}</em>
                </div>
              </article>
            );
          })}
        </div>
      </section>
    );
  }

  if (section === 'saints') {
    const saints = items.filter(isSaintPage);
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="Святые" path="/uk/saints" note="Карточки житий, дни памяти и связанные иконы." />
        <div className="site-card-grid-preview">
          {saints.slice(0, 6).map((saint) => (
            <article className="site-saint-card" key={saint.id}>
              {saint.imageUrl ? <img src={saint.imageUrl} alt={saint.name} loading="lazy" /> : <span><AppIcon name="globe" /></span>}
              <small>{saint.feastDay || 'День памяти'}</small>
              <strong>{saint.name}</strong>
              <p>{compactText(saint.shortDescription || saint.biography, 170)}</p>
            </article>
          ))}
        </div>
      </section>
    );
  }

  if (section === 'gospel') {
    const reading = items.find(isGospelReading) ?? content.gospel[0];
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="Евангелие" path="/uk/gospel" note="Страница чтения дня: ссылка, текст и краткое объяснение." />
        <article className="site-gospel-preview">
          <small>{reading?.date || 'Дата'} · {reading?.reference || 'Место чтения'}</small>
          <h3>{reading?.title || 'Евангелие дня'}</h3>
          <blockquote>{reading?.text || 'Текст чтения появится здесь.'}</blockquote>
          <p>{reading?.explanation || 'Краткое объяснение будет показано под чтением.'}</p>
        </article>
      </section>
    );
  }

  if (section === 'seo') {
    const pages = items.filter(isSeoPage);
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="SEO-страницы" path="/uk/p/:slug" note="Лендинги под поисковые запросы: hero, H1, контентные блоки и SEO snippet." />
        <div className="site-seo-grid">
          {pages.slice(0, 4).map((page) => (
            <article className="site-seo-card" key={page.id}>
              {page.imageUrl ? <img src={page.imageUrl} alt={page.title} loading="lazy" /> : null}
              <small>{page.targetKeyword || page.pageType}</small>
              <h3>{page.h1 || page.title}</h3>
              <p>{compactText(page.content || page.seoDescription || 'Текст страницы будет здесь.', 220)}</p>
              <em>/p/{page.slug}</em>
            </article>
          ))}
        </div>
      </section>
    );
  }

  if (section === 'qr') {
    const qrPages = items.filter(isQrPage);
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="QR-страницы" path="/uk/qr/:id" note="Публичная страница после скана QR: молитва, ссылка на икону, владелец и место." />
        <div className="site-qr-grid">
          {qrPages.slice(0, 4).map((page) => (
            <article className="site-qr-card" key={page.id}>
              <QrCodePreview item={page} />
              <strong>{page.title}</strong>
              <p>{compactText(page.customPrayer || 'Особая молитва появится здесь.', 160)}</p>
              <small>{page.ownerName || 'Страница иконы'} · {page.location || 'QR'}</small>
            </article>
          ))}
        </div>
      </section>
    );
  }

  if (section === 'churches') {
    const churches = items.filter(isChurchPage);
    return (
      <section className="site-preview-panel">
        <SitePreviewHeader title="Храмы" path="/uk/churches" note="Партнерские страницы храмов: адрес, описание, расписание и связанные иконы." />
        <div className="site-church-grid">
          {churches.slice(0, 4).map((church) => (
            <article className="site-church-card" key={church.id}>
              <small>{church.city}</small>
              <h3>{church.title}</h3>
              <p>{compactText(church.description, 190)}</p>
              <span>{church.address}</span>
              <em>{church.schedule}</em>
            </article>
          ))}
        </div>
      </section>
    );
  }

  return null;
}

function SitePreviewHeader({ title, path, note }: { title: string; path: string; note: string }) {
  return (
    <div className="site-preview-head">
      <div>
        <small>Предпросмотр сайта</small>
        <strong>{title}</strong>
        <span>{note}</span>
      </div>
      <code>{path}</code>
    </div>
  );
}

function isPrayerPage(item: IconsItem): item is PrayerPage {
  return 'relatedIcon' in item && 'text' in item;
}

function isSaintPage(item: IconsItem): item is SaintPage {
  return 'biography' in item;
}

function isGospelReading(item: IconsItem): item is GospelReading {
  return 'reference' in item;
}

function isSeoPage(item: IconsItem): item is SeoPage {
  return 'content' in item && 'pageType' in item;
}

function isQrPage(item: IconsItem): item is QrPage {
  return 'qrId' in item;
}

function isChurchPage(item: IconsItem): item is ChurchPage {
  return 'address' in item;
}

function buildIconBackendAuditPrompt(icon: IconPage) {
  return `Ты backend RuleBot / главный оркестратор для CRM сайта православных икон.
Проверь материал и верни ТОЛЬКО JSON без markdown.

Схема ответа:
{
  "score": 0-10,
  "total": 10,
  "ok": true|false,
  "message": "короткий итог",
  "missing": ["что обязательно заполнить"],
  "warnings": ["что желательно исправить"]
}

10 обязательных проверок:
1. title заполнен и понятен посетителю
2. slug заполнен и выглядит как URL slug
3. imageUrl заполнен
4. shortDescription заполнен
5. fullDescription заполнен и не выглядит шаблонным
6. prayerText заполнен
7. seoTitle заполнен
8. seoDescription заполнен
9. qrCodeUrl заполнен
10. calendarDate заполнен, если материал должен попадать в календарь

Важно:
- Не ставь 10/10, если есть пустые обязательные поля.
- Если изображение не является канонической иконой, но материал честно описан как христианский пейзаж/иллюстрация, это warning, не missing.
- Проверяй украинскую, русскую и английскую локали в translations, если они есть.
- Не исправляй текст, только оцени готовность.

Материал:
${JSON.stringify(icon, null, 2)}`;
}

function parseBackendIconAudit(result: AiGenerationResult, icon: IconPage): BackendIconAudit {
  const raw = [result.description, result.suggestions.join('\n'), result.title].filter(Boolean).join('\n');
  const jsonText = raw.match(/\{[\s\S]*\}/)?.[0] || '';
  try {
    const parsed = JSON.parse(jsonText) as Partial<BackendIconAudit>;
    const total = clampAuditNumber(parsed.total, 10);
    const score = Math.min(total, clampAuditNumber(parsed.score, 0));
    return {
      score,
      total,
      ok: Boolean(parsed.ok ?? score >= total),
      message: String(parsed.message || (score >= total ? 'Материал готов' : 'Есть незаполненные поля')),
      missing: Array.isArray(parsed.missing) ? parsed.missing.map(String).filter(Boolean) : [],
      warnings: Array.isArray(parsed.warnings) ? parsed.warnings.map(String).filter(Boolean) : [],
      checkedAt: new Date().toISOString(),
      source: 'backend-rulebot'
    };
  } catch {
    const fallback = buildLocalBackendAuditFallback(icon, 'Backend вернул ответ без строгого JSON.');
    const qualityScore = typeof result.qualityScore === 'number' ? Math.round(result.qualityScore / 10) : fallback.score;
    return { ...fallback, score: Math.min(10, Math.max(0, qualityScore)) };
  }
}

function buildLocalBackendAuditFallback(icon: IconPage, message: string): BackendIconAudit {
  const readiness = iconReadiness(icon);
  const missing = iconMissingFields(icon);
  return {
    score: readiness.complete,
    total: readiness.total,
    ok: readiness.complete >= readiness.total && missing.length === 0,
    message,
    missing,
    warnings: missing.length ? [] : ['Backend RuleBot не вернул строгий JSON, показана локальная оценка.'],
    checkedAt: new Date().toISOString(),
    source: 'backend-rulebot'
  };
}

function clampAuditNumber(value: unknown, fallback: number) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? Math.max(0, Math.round(numeric)) : fallback;
}

function iconPageDraft(title: string, description: string) {
  const source = description.trim();
  const intro = source || `${title}: молитва, история образа и духовные материалы для спокойного чтения.`;
  const fullDescription = sectionFromText(source, ['Полное описание', 'История образа', 'История', 'Описание']) || intro;
  const prayerText = sectionFromText(source, ['Молитва перед иконой', 'Молитва', 'Текст молитвы']);
  const helpText = sectionFromText(source, ['В чем помогает', 'О чем молятся', 'Как молиться']);
  const historyText = sectionFromText(source, ['История образа', 'История', 'Предание']);
  const gospelText = sectionFromText(source, ['Евангельская связка', 'Евангелие', 'Евангельское чтение']);
  const lifeText = sectionFromText(source, ['Святой', 'Житие', 'Память святого']);
  return {
    shortDescription: compactText(sectionFromText(source, ['Краткое описание', 'SEO description']) || intro, 260),
    fullDescription: fullDescription.length > 700 ? fullDescription : richIconText(title, fullDescription, helpText),
    prayerText: prayerText || `О Пресвятая Владычице Богородице и все святые Божии, пред образом "${title}" с верою припадаем и просим: укрепите нас в покаянии, мире и терпении, наставьте на путь Христов и испросите у Господа милость, помощь в нуждах и благодарное сердце. Аминь.`,
    gospelText: gospelText || 'Евангельское чтение дня помогает соединить молитву перед иконой с внимательным словом Христа: свет веры должен быть виден в тихих делах, милосердии и верности заповедям.',
    lifeText: lifeText || `${title} напоминает о живой церковной памяти: святой образ не является украшением, а ведет человека к молитве, покаянию, благодарности и вниманию к ближним.`,
    historyText: historyText || `История почитания образа "${title}" раскрывается через церковное предание, богослужебную память и опыт людей, которые обращались к святыне за духовной поддержкой. В публикации важно сохранять точность: даты, места явления и списки образа следует сверять с православными источниками.`
  };
}

function compactText(value: string, limit: number) {
  const normalized = value.replace(/\s+/g, ' ').trim();
  return normalized.length > limit ? `${normalized.slice(0, limit - 1).trim()}…` : normalized;
}

function isThinText(value?: string | null) {
  if (!value) return true;
  const normalized = value.trim();
  return normalized.length < 180 || /AI draft|духовные материалы|История почитания образа/.test(normalized);
}

function sectionFromText(text: string, names: string[]) {
  if (!text.trim()) return '';
  const escaped = names.map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
  const pattern = new RegExp(`(?:^|\\n)\\s*(?:#{1,3}\\s*)?(?:${escaped})\\s*:?\\s*\\n([\\s\\S]*?)(?=\\n\\s*(?:#{1,3}\\s*)?[А-ЯA-Z][^\\n]{2,80}:?\\s*\\n|$)`, 'i');
  return text.match(pattern)?.[1]?.trim() || '';
}

function structuredSectionsFromText(text: string, titles: string[]) {
  return titles
    .map((title) => {
      const escaped = title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const value = text.match(new RegExp(`(?:^|\\n)\\*\\*${escaped}:\\*\\*\\s*\\n?([\\s\\S]*?)(?=\\n\\*\\*[^\\n]+:\\*\\*|$)`, 'i'))?.[1]?.trim();
      return value ? `**${title}:**\n${value}` : '';
    })
    .filter(Boolean)
    .join('\n\n');
}

function dedupeStructuredSections(text: string, titles: string[]) {
  const structured = structuredSectionsFromText(text, titles);
  if (structured) return structured;
  const seen = new Set<string>();
  return text
    .split(/\n{2,}/)
    .map((section) => section.trim())
    .filter(Boolean)
    .filter((section) => {
      const key = section
        .replace(/^\*\*([^*]+):\*\*[\s\S]*$/i, '$1')
        .replace(/\s+/g, ' ')
        .trim()
        .toLowerCase();
      if (!key || seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .join('\n\n');
}

const iconDescriptionFields = ['Краткое описание изображения', 'Символы на иконе', 'Alt для фото', 'Prompt для генерации', 'Источник изображения'];
const saintsDescriptionFields = ['Главные святые дня', 'Кратко кто это', 'Годы / век', 'Чем известен', 'Память по календарю', 'Источники'];
const churchDescriptionFields = ['Название храма', 'Кому посвящён', 'Страна / город', 'Адрес', 'Google Maps ссылка', 'Расписание богослужений', 'Телефон / сайт', 'Краткое описание', 'Святыни / иконы / мощи', 'Фото храма'];
const mainDescriptionFields = ['Полное описание', 'Смысл праздника', 'Что важно знать', 'Для кого эта молитва/страница', 'Не писать'];
const prayerFields = ['Основной текст молитвы', 'Короткая молитва', 'Источник / примечание'];
const gospelFields = ['Апостольское чтение', 'Евангельское чтение', 'Цитата дня', 'Объяснение простыми словами', 'Связь с событием', 'Источник'];
const lifeFields = ['Краткое житие', 'Подробное житие', 'Главные события жизни', 'Духовный смысл', 'Где почитается', 'Источники'];
const historyFields = ['История праздника', 'Дата по старому стилю', 'Дата по новому стилю', 'Разные календарные традиции', 'Почему бывает путаница', 'Проверенные источники', 'Дата проверена', 'Календарный стиль', 'Найденное событие', 'Уверенность', 'Предупреждение'];
const iconTextLanguages: IconsLang[] = ['uk', 'ru', 'en'];
const iconTextLanguageLabels: Record<IconsLang, string> = {
  uk: 'украинском',
  ru: 'русском',
  en: 'английском'
};

function structuredTitlesForTextTab(scope: IconTextScopeKey, tab: IconTextTabKey) {
  if (tab === 'description') {
    const scopeFields = scope === 'saints' ? saintsDescriptionFields : scope === 'church' ? churchDescriptionFields : iconDescriptionFields;
    return [...scopeFields, ...mainDescriptionFields];
  }
  if (tab === 'prayer') return prayerFields;
  if (tab === 'gospel') return gospelFields;
  if (tab === 'life') return lifeFields;
  return historyFields;
}

function patchKeyForTextTab(tab: IconTextTabKey) {
  if (tab === 'description') return 'fullDescription';
  if (tab === 'prayer') return 'prayerText';
  if (tab === 'gospel') return 'gospelText';
  if (tab === 'life') return 'lifeText';
  return 'historyText';
}

function localizedIconTextPatch(item: IconPage, language: IconsLang, patch: Partial<IconTranslation>) {
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

function ensureStructuredTabText(text: string, titles: string[]) {
  const structured = dedupeStructuredSections(text, titles);
  if (structured) return structured;
  const firstTitle = titles[0] || 'Материал';
  return text.trim() ? `**${firstTitle}:**\n${text.trim()}` : '';
}

function mergeLocalizedTextPatch(
  item: IconPage,
  basePatch: IconTextPatch,
  language: IconsLang,
  patch: Partial<IconTranslation>
): IconTextPatch {
  if (language === 'ru') return { ...basePatch, ...patch };
  const currentTranslations = (basePatch.translations as IconPage['translations'] | undefined) ?? item.translations ?? {};
  return {
    ...basePatch,
    translations: {
      ...currentTranslations,
      [language]: {
        ...(item.translations?.[language] ?? {}),
        ...(currentTranslations?.[language] ?? {}),
        ...patch
      }
    }
  };
}

function normalizeIconTranslationLayer<T extends Partial<IconTranslation>>(layer: T): T {
  const descriptionTitles = [...iconDescriptionFields, ...saintsDescriptionFields, ...churchDescriptionFields, ...mainDescriptionFields];
  return {
    ...layer,
    fullDescription: layer.fullDescription ? ensureStructuredTabText(layer.fullDescription, descriptionTitles) : layer.fullDescription,
    prayerText: layer.prayerText ? ensureStructuredTabText(layer.prayerText, prayerFields) : layer.prayerText,
    gospelText: layer.gospelText ? ensureStructuredTabText(layer.gospelText, gospelFields) : layer.gospelText,
    lifeText: layer.lifeText ? ensureStructuredTabText(layer.lifeText, lifeFields) : layer.lifeText,
    historyText: layer.historyText ? ensureStructuredTabText(layer.historyText, historyFields) : layer.historyText
  };
}

function normalizeIconPageText(item: IconPage): IconPage {
  const translations = item.translations ? { ...item.translations } : undefined;
  iconTextLanguages.forEach((language) => {
    if (language === 'ru') return;
    const layer = translations?.[language];
    if (layer) translations![language] = normalizeIconTranslationLayer(layer);
  });
  return {
    ...item,
    ...normalizeIconTranslationLayer({
      fullDescription: item.fullDescription,
      prayerText: item.prayerText,
      gospelText: item.gospelText,
      lifeText: item.lifeText,
      historyText: item.historyText
    }),
    translations
  };
}

function richIconText(title: string, intro: string, helpText = '') {
  return [
    intro,
    `Образ "${title}" на странице должен читаться как спокойный православный материал: сначала кратко объясняется, что это за икона или праздник, затем раскрывается история почитания, духовный смысл и место молитвы в церковной жизни.`,
    helpText || 'Перед иконой молятся не самой доске или краскам, а Господу, Пресвятой Богородице или святому, изображенному на образе. Поэтому текст важно вести бережно: без обещаний чудес как услуги, без суеверий и без выдуманных фактов.',
    'Для публикации добавьте проверенные даты, церковное именование образа, связь с праздником или памятью святого, а также молитву, которую человек сможет прочитать перед образом дома или в храме.'
  ].filter(Boolean).join('\n\n');
}

function buildIconEditorialPrompt(item: IconsItem | undefined, prompt: string) {
  const title = item ? getItemTitle(item) : prompt;
  return `Ты православный редактор и SEO-редактор сайта "Свет Иконы".

Задача: создать или улучшить полноценную страницу иконы/праздника, а не короткую карточку.
Тема администратора: ${prompt}
Текущий материал: ${item ? JSON.stringify(item) : '{}'}

Верни материал строго в структурных секциях, которые редактор сможет разложить по вкладкам:

Описание:
**Краткое описание изображения:**
**Символы на иконе:**
**Alt для фото:**
**Prompt для генерации:**
**Источник изображения:**
**Полное описание:**
**Смысл праздника:**
**Что важно знать:**
**Для кого эта молитва/страница:**
**Не писать:**

Молитва:
**Основной текст молитвы:**
**Короткая молитва:**
**Источник / примечание:**

Евангелие:
**Апостольское чтение:**
**Евангельское чтение:**
**Цитата дня:**
**Объяснение простыми словами:**
**Связь с событием:**
**Источник:**

Житие:
**Краткое житие:**
**Подробное житие:**
**Главные события жизни:**
**Духовный смысл:**
**Где почитается:**
**Источники:**

История:
**История праздника:**
**Дата по старому стилю:**
**Дата по новому стилю:**
**Разные календарные традиции:**
**Почему бывает путаница:**
**Проверенные источники:**
**Дата проверена:**
**Календарный стиль:**
**Найденное событие:**
**Уверенность:**
**Предупреждение:**

Требования:
- 3000-4500 знаков, живой православный тон, музейная строгость, без рекламной суеты.
- Если точные даты, место явления или канонические детали не указаны, пиши осторожно и не выдумывай.
- Для праздника или памяти святого укажи дату по православному календарю и понятный источник/тип источника: богослужебный календарь, Минеи, Synaxarion, OCA Feasts & Saints, Православная энциклопедия или официальный церковный календарь.
- Официальные молитвенные тексты не придумывай: если не уверен в тропаре, кондаке или величании, напиши "требует ручной проверки" и укажи, где проверить.
- Краткая молитва и молитва своими словами допустимы только как осторожный частный текст, не вместо церковного текста.
- Объясни, о чем молятся, как правильно относиться к иконе, почему образ важен.
- Slug только латиницей lower-case через дефис.
- PhotoPrompt на английском: строгая иконографическая реставрация/улучшение именно этого образа, без фотолюдей, без текста, без watermark.`;
}

function buildIconTextTabPrompt(item: IconPage, scope: IconTextScopeKey, tab: IconTextTabKey, language: IconsLang, prompt: string) {
  const scopeLabels: Record<IconTextScopeKey, string> = {
    icon: 'Икона',
    saints: 'Святые',
    church: 'Храм'
  };
  const tabLabels: Record<IconTextTabKey, string> = {
    description: 'Описание',
    prayer: 'Молитва',
    gospel: 'Евангелие',
    life: 'Житие',
    history: 'История'
  };
  const titles = structuredTitlesForTextTab(scope, tab);
  const churchRules = scope === 'church'
    ? '\n- Для раздела "Храм" заполняй паломнические данные: название храма, посвящение, страна/город, адрес, Google Maps ссылка, расписание богослужений, телефон/сайт, краткое описание, святыни/иконы/мощи, фото храма. Расписание и контакты указывай только если найден проверяемый источник; иначе пиши "требует ручной проверки".'
    : '';

  return `Ты православный редактор сайта "Свет Иконы" и фактчекер церковного календаря.

Задача: перепроверить актуальные православные источники и перезаполнить только одну вкладку материала.
Язык ответа: ${iconTextLanguageLabels[language]}. Все значения секций пиши только на этом языке.
Материал: ${item.title}
Slug: ${item.slug}
Категория: ${item.category}
Святой / праздник: ${item.saintName || 'не указан'}
Дата в календаре: ${item.calendarDate || 'не указана'}
Активный раздел: ${scopeLabels[scope]}
Активная вкладка: ${tabLabels[tab]}
Уточнение администратора: ${prompt || 'нет'}

Текущий материал целиком:
${JSON.stringify(item)}

Верни только эти секции, строго в таком формате Markdown, без лишних заголовков:
${titles.map((title) => `**${title}:**`).join('\n')}

Правила проверки:
- Проверь информацию по доступным актуальным источникам в интернете или церковным справочникам, если у модели есть доступ к web/grounding.
- Если интернет-проверка недоступна, не выдумывай: в "Источник/Источники/Проверенные источники" напиши "требует ручной проверки" и укажи, где проверять: официальный церковный календарь, Минеи, Synaxarion, OCA Feasts & Saints, Православная энциклопедия.
- При повторном запуске перезаполни секции заново: старый текст можно заменить более точным.
- Не придумывай официальные молитвы, тропари, кондаки, даты, житие и источники. Если нет уверенности, явно напиши "требует ручной проверки".
- Сохраняй православный спокойный тон, без суеверий, рекламных обещаний и спорных утверждений.
- Не смешивай языки: если выбран украинский, все поля на украинском; если английский, все поля на английском.
- Для дат всегда различай гражданскую дату и старый стиль.
- Для вкладки "Храм" пиши так, чтобы блок можно было показать на странице для храмов и QR-материалов.
- Для вкладки "Святые" пиши только про святых и память дня, а не общую рекламу сайта.${churchRules}`;
}

function buildIconVisionInstruction(item: IconsItem | undefined, prompt: string) {
  return `Проанализируй загруженное фото как православную икону или церковный образ.
Тема администратора: ${prompt}
Текущий материал: ${item ? JSON.stringify(item) : '{}'}

Нужно распознать и описать:
- кто изображен, если это можно определить;
- тип иконографии, композицию, жесты, цвета, фон, ризу, надписи, состояние изображения;
- правильное название, категорию, slug, SEO title/description;
- основу для полноценного текста страницы и молитвы.
- если это праздник или память святого, вероятную дату по православному календарю и источник, но только если уверен.

Не превращай икону в случайную фотографию храма. Не выдумывай имя святого, если не уверен: укажи осторожно в suggestions. Верни JSON, а в description дай богатый материал для редактора: история/смысл/в чем помогает/как молиться/молитва.`;
}

function buildDefaultIconPhotoPrompt(_item: IconsItem | undefined) {
  return [
    'Use Reference 1 as the product mockup template.',
    'Place Reference 2 inside the framed artwork area.',
    'Preserve Reference 2 visually.',
    'Keep all product elements, lighting, QR module, button and phone from Reference 1.'
  ].filter(Boolean).join(' ');
}

function buildIconReferenceUrls(images: string[], selectedIndex: number) {
  const usableImages = images.filter((url) => url && !isGeneratedQrImage(url));
  const selected = usableImages.includes(images[selectedIndex]) ? images[selectedIndex] : usableImages[0];
  return uniqueImages([
    selected,
    usableImages[0],
    ...usableImages
  ]).slice(0, 4);
}

function buildIconImagePrompt(title: string, prompt: string, referenceUrls: string[], extraVariant: boolean) {
  const referenceLine = referenceUrls.length
    ? `Use ALL uploaded reference images as visual references. Reference 1 is the product mockup template: wooden frame, warm light, QR module, button, phone, camera angle and background. Reference 2 is the framed artwork to place inside the frame. References: ${referenceUrls.join(' | ')}.`
    : 'Create a product mockup with a framed artwork area, QR module and clean catalog presentation.';
  return `${referenceLine}
Task: create a product mockup using the provided references.
Admin prompt: ${prompt || buildDefaultIconPhotoPrompt(undefined)}

Instructions:
- Use Reference 1 as the product template: wooden frame, warm light, QR module, button, phone, camera angle and background.
- Place Reference 2 inside the framed artwork area.
- Keep Reference 2 visually recognizable and preserve its composition, colors and details.
- Adjust only perspective, crop and lighting so it fits naturally inside the frame.
- Keep all product elements from Reference 1 unchanged.
- Avoid adding readable new text, logos, watermarks, UI captions or marketing text.

Output: realistic premium product photo.
${extraVariant ? 'Create one additional product mockup variation using the same Reference 2 artwork.' : 'Create the main product mockup.'}`;
}

function buildCalendarDayPrompt(day: CalendarDay, prompt: string) {
  return `Ты православный редактор церковного календаря для сайта "Свет Иконы".

Задача: проверить и заполнить материал одного дня календаря.
Гражданская дата: ${day.gregorianDate || day.day}
Юлианская дата: ${day.julianDate || day.julianDay || ''}
Текущее название: ${day.label || 'пусто'}
Текущий тип: ${day.kind}
Текущее описание: ${day.description || 'пусто'}
Дополнительное указание администратора: ${prompt || 'нет'}

Верни короткий русский материал с разделами:
# Название дня
# Тип дня
# Тип подписи
# Что произошло
# Святой или праздник
# Библейская связь
# Короткое описание
# PhotoPrompt

Требования:
- главная дата для проверки: ЮЛИАНСКАЯ дата; гражданскую дату используй только как отображение в календаре;
- не используй темы из других промптов редактора икон, если они не совпадают с юлианской датой;
- не подменяй 10 июня ст. ст. на 23 июня церковного календаря: это разные церковные даты;
- не выдумывай точные факты, если дата не уверенная; лучше напиши осторожно;
- сверь смысл дня по православному юлианскому календарю;
- если по этой юлианской дате в доступных православных календарных данных нет отдельной памяти святого, праздника или особого библейского чтения, обязательно верни:
  # Название дня: Обычный день церковного календаря
  # Тип дня: quiet
  # Тип подписи: Без отдельного праздника
  # Что произошло: В этот день в текущих данных нет отдельного праздника или памяти святого.
  # Святой или праздник: Не указан в текущих данных.
  # Библейская связь: Общее чтение дня и личная молитва.
- если событие найдено, # Тип дня должен быть одним из: feast, gospel, prayer, fast;
- если юлианская дата 10 июня, не называй день Казанской, Владимирской или другой Богородичной иконой, если это явно не подтверждено источником именно для 10 июня ст. ст.;
- описание 2-3 коротких предложения: что вспоминается, кто святой/праздник, какая библейская связь;
- для пустого буднего дня дай нейтральный материал церковного календаря, без ложного святого;
- PhotoPrompt на английском: простой православный образ дня, крупно и целиком в кадре, без книги, без свечи, без текста, без watermark, спокойный церковный стиль.`;
}

function buildCalendarDayImagePrompt(day: CalendarDay, title: string, description: string, photoPrompt: string) {
  const quietDay = /обычный день|нет отдельного|не указан|quiet/i.test(`${title} ${description} ${photoPrompt}`);
  return [
    photoPrompt || (quietDay
      ? 'Simple Orthodox calendar image: a single centered sacred Orthodox-style image, calm church light, no book, no candle, no table, no specific saint.'
      : `Orthodox calendar image for ${title}: one centered sacred image, full subject visible.`),
    `Civil date: ${day.gregorianDate || day.day}. Julian date: ${day.julianDate || day.julianDay || ''}.`,
    `Context: ${description}`,
    quietDay
      ? 'Create a simple calm image of an ordinary church calendar day, without inventing a saint, feast or biblical event.'
      : 'Create a respectful Orthodox icon-inspired image for the saint, feast or biblical event of this calendar day.',
    'Show the main sacred image fully inside the frame: no cropped face, no cut-off halo, no partial body.',
    'No readable text, no watermark, no modern random people, no decorative UI, no typography, no book, no candle, no table.',
    'Use warm church lighting, iconographic restraint, gold accents, calm devotional composition.'
  ].filter(Boolean).join(' ');
}

function isInvalidTheotokosForJulianDate(day: CalendarDay, value: string) {
  const julian = `${day.julianDate || ''} ${day.julianDay || ''}`.toLowerCase();
  if (!/10\s+июня|^10\b/.test(julian)) return false;
  return /казанск|владимирск|богородиц|mother of god|theotokos/i.test(value);
}

function makeNewItem(section: IconsSection) {
  const id = `${section}-${Date.now()}`;
  const now = new Date().toISOString();
  if (section === 'calendar') return { id, day: '19', label: 'Новый день календаря', note: 'Память святого', kind: 'feast' as const, imageUrl: '', iconSlug: '', prayerSlug: '', gospelSlug: 'today', detailHref: '/icons', current: false, feast: false, textOnly: false, description: '' };
  if (section === 'icons') {
    const title = 'Новая икона';
    return {
      id,
      slug: slugify(title),
      title,
      shortDescription: 'Краткое описание образа, праздника или святого для карточки на сайте.',
      fullDescription: 'Опишите историю образа, церковный смысл, день памяти и то, как правильно молиться перед этой святыней.',
      imageUrl: '',
      imageUrls: [],
      qrCodeUrl: '',
      category: 'Праздничная икона',
      saintName: '',
      prayerText: 'Господи Иисусе Христе, молитвами Пречистой Твоей Матери и всех святых укрепи нас в вере, мире и покаянии. Аминь.',
      gospelText: 'Добавьте евангельское чтение или краткую духовную связку с темой образа.',
      lifeText: 'Добавьте житие святого или описание церковного события, если материал связан с памятью святого или праздником.',
      historyText: 'Добавьте историю почитания образа, проверенные даты, источники и важные детали.',
      seoTitle: title,
      seoDescription: 'Православный материал: описание, молитва, история и церковный смысл образа.',
      seoKeywords: `${title}, православная икона, молитва, Свет Иконы`,
      createdAt: now,
      updatedAt: now,
      status: 'draft' as const
    };
  }
  if (section === 'prayers') return {
    id,
    slug: 'novaya-molitva',
    title: 'Новая молитва',
    category: 'Ежедневная',
    text: 'Господи, наставь меня на путь мира, терпения и покаяния. Укрепи сердце в вере, помоги творить добро и принимать каждый день с благодарностью. Аминь.',
    relatedIcon: '',
    status: 'draft' as const,
    seoTitle: 'Новая молитва',
    seoDescription: 'Текст православной молитвы для ежедневного чтения.'
  };
  if (section === 'saints') return {
    id,
    slug: 'novyj-svyatoj',
    name: 'Новый святой',
    shortDescription: 'Краткое описание святого, дня памяти и духовного значения его почитания.',
    biography: 'Добавьте житие святого: годы жизни, подвиг, церковную память, основные источники и то, о чем молятся святому.',
    feastDay: '',
    imageUrl: '',
    relatedIcons: [],
    prayers: [],
    status: 'draft' as const,
    seoTitle: 'Новый святой',
    seoDescription: 'Житие, день памяти и молитвенное почитание святого.'
  };
  if (section === 'gospel') return {
    id,
    date: new Date().toISOString().slice(0, 10),
    title: 'Евангелие дня',
    reference: 'Мф. 5:14-16',
    text: 'Вы — свет мира. Не может укрыться город, стоящий на верху горы.',
    explanation: 'Кратко объясните смысл чтения: как слово Евангелия связано с молитвой, покаянием и ежедневной христианской жизнью.',
    status: 'draft' as const,
    seoTitle: 'Евангелие дня',
    seoDescription: 'Евангельское чтение дня с кратким пояснением.'
  };
  if (section === 'qr') return { id, qrId: `qr-${Date.now()}`, iconId: '', slug: `qr-${Date.now()}`, title: 'Новая QR-страница', active: true, scanCount: 0, ownerName: '', location: '', customPrayer: '', createdAt: now, updatedAt: now };
  if (section === 'seo') return { id, slug: 'novaya-seo-stranica', title: 'Новая SEO-страница', h1: 'Новая SEO-страница', content: '', pageType: 'landing', targetKeyword: '', language: 'ru', blocks: [], imageUrl: '', city: '', status: 'draft' as const, seoTitle: '', seoDescription: '', createdAt: now, updatedAt: now };
  return {
    id,
    slug: 'novyj-hram',
    title: 'Новый храм',
    city: 'Город',
    address: 'Адрес храма',
    description: 'Опишите храм, приходскую жизнь, святыни, расписание и возможности для прихожан.',
    schedule: 'Расписание богослужений уточняется.',
    relatedIcons: [],
    status: 'draft' as const,
    donationUrl: '',
    seoTitle: 'Новый храм',
    seoDescription: 'Информация о храме, адрес, расписание и приходские материалы.'
  };
}

export function IconsPage({ activeSection: initialSection = 'icons' }: { activeSection?: IconsSection }) {
  const [activeSection, setActiveSection] = useState<IconsSection>(initialSection);
  const [content, setContent] = useState<IconsSiteContent>(emptyContent);
  const [editingId, setEditingId] = useState('');
  const [editorOpen, setEditorOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [aiPrompt, setAiPrompt] = useState('Казанская икона Божией Матери: молитва, история, QR-страница и церковный календарь');
  const [aiTextLoading, setAiTextLoading] = useState(false);
  const [aiCreateLoading, setAiCreateLoading] = useState(false);
  const [aiCalendarLoading, setAiCalendarLoading] = useState(false);
  const [calendarAiPrompt, setCalendarAiPrompt] = useState(defaultCalendarAiPrompt);
  const [aiImageLoading, setAiImageLoading] = useState(false);
  const [photoUploadLoading, setPhotoUploadLoading] = useState(false);
  const [aiImagePlusLoading, setAiImagePlusLoading] = useState(false);
  const [aiSelectedImageLoading, setAiSelectedImageLoading] = useState(false);
  const [iconPhotoGenerateCount, setIconPhotoGenerateCount] = useState(1);
  const [selectedPhotoAspect, setSelectedPhotoAspect] = useState<IconPhotoAspect>('source');
  const [selectedIconImage, setSelectedIconImage] = useState(0);
  const [activeEditorTab, setActiveEditorTab] = useState<EditorTabKey>('main');
  const [activeIconTextScope, setActiveIconTextScope] = useState<IconTextScopeKey>('icon');
  const [activeIconTextLanguage, setActiveIconTextLanguage] = useState<IconsLang>('uk');
  const [activeIconTextTab, setActiveIconTextTab] = useState<IconTextTabKey>('description');
  const [activeCalendarYear, setActiveCalendarYear] = useState(initialCalendarYear);
  const [activeCalendarMonth, setActiveCalendarMonth] = useState(initialCalendarMonth);
  const [iconReferenceImages, setIconReferenceImages] = useState<AiReferenceImage[]>([]);
  const [iconReferenceBusy, setIconReferenceBusy] = useState(false);
  const [fullscreenIconImage, setFullscreenIconImage] = useState<number | null>(null);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [resourceQuery, setResourceQuery] = useState('');
  const [resourceStatus, setResourceStatus] = useState<'all' | 'draft' | 'published'>('all');
  const [backendAudits, setBackendAudits] = useState<Record<string, BackendIconAudit>>({});
  const [backendAuditLoadingId, setBackendAuditLoadingId] = useState('');
  const pendingItemPatchesRef = useRef<Record<string, PendingItemPatch>>({});

  const items = useMemo(() => getSectionItems(content, activeSection), [activeSection, content]);
  const visibleItems = useMemo(() => {
    const query = resourceQuery.trim().toLowerCase();
    return items.filter((item) => {
      const matchesQuery = !query || [
        getItemTitle(item),
        getItemSubline(activeSection, item),
        'category' in item ? item.category : '',
        'saintName' in item ? item.saintName : '',
        'seoDescription' in item ? item.seoDescription || '' : ''
      ].some((value) => value.toLowerCase().includes(query));
      const matchesStatus = resourceStatus === 'all' || !('status' in item) || item.status === resourceStatus;
      return matchesQuery && matchesStatus;
    });
  }, [activeSection, items, resourceQuery, resourceStatus]);
  const iconStats = useMemo(() => {
    const icons = content.icons;
    return {
      total: icons.length,
      published: icons.filter((item) => item.status === 'published').length,
      draft: icons.filter((item) => item.status === 'draft').length,
      ready: icons.filter((item) => iconReadiness(item).percent >= 80).length
    };
  }, [content.icons]);
  const activeSectionMeta = sections.find((section) => section.key === activeSection) ?? sections[0];
  const sectionIsEditable = activeSection === 'icons' || activeSection === 'seo';
  const editing = items.find((item) => getItemId(item) === editingId) ?? items[0];
  const editingImages = iconImages(editing);
  const editorTabs = useMemo(() => {
    const tabs: Array<{ key: EditorTabKey; label: string; icon: 'home' | 'cms' | 'seo' | 'image' | 'qr' | 'deploy' }> = [
      { key: 'main', label: 'Основное', icon: 'home' },
      { key: 'texts', label: 'Тексты', icon: 'cms' },
      { key: 'seo', label: 'SEO', icon: 'seo' }
    ];
    if (editing && ('day' in editing || supportsDirectPhotoForItem(editing))) tabs.push({ key: 'photo-ai', label: 'Фото + AI', icon: 'image' });
    if (editing && 'qrCodeUrl' in editing) tabs.push({ key: 'calendar', label: 'Календарь', icon: 'seo' });
    if (editing && ('qrCodeUrl' in editing || 'qrId' in editing)) tabs.push({ key: 'qr', label: 'QR', icon: 'qr' });
    tabs.push({ key: 'publish', label: 'Публикация', icon: 'deploy' });
    return tabs;
  }, [editing]);

  useEffect(() => {
    setActiveSection(initialSection);
  }, [initialSection]);

  useEffect(() => {
    void loadContent({ year: activeCalendarYear, month: activeCalendarMonth });
  }, []);

  useEffect(() => {
    setSelectedIconImage(0);
    setActiveEditorTab('main');
    setActiveIconTextScope('icon');
    setActiveIconTextTab('description');
    setFullscreenIconImage(null);
    setIconReferenceImages((current) => {
      current.forEach((image) => URL.revokeObjectURL(image.preview));
      return [];
    });
  }, [editingId, activeSection]);

  useEffect(() => {
    const sectionItems = getSectionItems(content, activeSection);
    if (sectionItems.length && !sectionItems.some((item) => getItemId(item) === editingId)) {
      setEditingId(getItemId(sectionItems[0]));
    }
  }, [activeSection, content, editingId]);

  useEffect(() => {
    if (!editorTabs.some((tab) => tab.key === activeEditorTab)) {
      setActiveEditorTab(editorTabs[0]?.key ?? 'main');
    }
  }, [activeEditorTab, editorTabs]);

  useEffect(() => {
    if (editing && 'qrCodeUrl' in editing) {
      setAiPrompt(buildDefaultIconPhotoPrompt(editing));
    }
  }, [editingId, activeSection]);

  async function loadContent(params?: { year?: string; month?: string }) {
    const requestedYear = params?.year ?? activeCalendarYear;
    const requestedMonth = params?.month ?? activeCalendarMonth;
    setLoading(true);
    setError('');
    try {
      const next = normalizeContent(await getIconsSiteContent({ year: requestedYear, month: requestedMonth }));
      setContent(next);
      setActiveCalendarYear(next.calendar.hero.year || requestedYear);
      setActiveCalendarMonth(String(requestedMonth).padStart(2, '0'));
      const first = getSectionItems(next, activeSection)[0];
      setEditingId(first ? getItemId(first) : '');
      setMessage('Контент svet-ikony загружен из backend');
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : 'Не удалось загрузить контент');
    } finally {
      setLoading(false);
    }
  }

  async function saveContent() {
    setSaving(true);
    setError('');
    try {
      const mergedContent = applyPendingItemPatches(content, pendingItemPatchesRef.current);
      const contentToSave = await prepareContentForSave(mergedContent);
      await saveIconsSiteContent({
        ...contentToSave,
        dashboard: {
          ...contentToSave.dashboard,
          publishedPages: contentToSave.icons.filter((item) => item.status === 'published').length + contentToSave.prayers.filter((item) => item.status === 'published').length + contentToSave.saints.filter((item) => item.status === 'published').length + contentToSave.pages.filter((item) => item.status === 'published').length + contentToSave.churches.filter((item) => item.status === 'published').length,
          icons: contentToSave.icons.length,
          prayers: contentToSave.prayers.length,
          churches: contentToSave.churches.length,
          qrPages: contentToSave.qrPages.length,
          qrScans: contentToSave.qrPages.reduce((sum, page) => sum + page.scanCount, 0)
        }
      });
      const preservedEditingId = editing ? getItemId(editing) : editingId;
      const savedIcon = contentToSave.icons.find((item) => getItemId(item) === preservedEditingId);
      const savedCalendarDate = savedIcon?.calendarDate || '';
      const nextCalendarYear = savedCalendarDate.slice(0, 4) || activeCalendarYear;
      const nextCalendarMonth = savedCalendarDate.slice(5, 7) || activeCalendarMonth;
      const refreshed = normalizeContent(await getIconsSiteContent({ year: nextCalendarYear, month: nextCalendarMonth }));
      const refreshedItems = getSectionItems(refreshed, activeSection);
      setContent(refreshed);
      setActiveCalendarYear(nextCalendarYear);
      setActiveCalendarMonth(nextCalendarMonth);
      setEditingId(refreshedItems.some((item) => getItemId(item) === preservedEditingId) ? preservedEditingId : (refreshedItems[0] ? getItemId(refreshedItems[0]) : ''));
      pendingItemPatchesRef.current = {};
      setMessage(savedCalendarDate ? `Сохранено в backend. Дата календаря: ${savedCalendarDate}` : 'Сохранено в backend: галерея икон включает оригинал, Gemini-фото и QR-код');
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Не удалось сохранить');
    } finally {
      setSaving(false);
    }
  }

  function replaceSection(nextItems: typeof items) {
    setContent((current) => {
      if (activeSection === 'calendar') return { ...current, calendar: { ...current.calendar, days: nextItems as CalendarDay[] } };
      if (activeSection === 'qr') return { ...current, qrPages: nextItems as QrPage[] };
      if (activeSection === 'seo') return { ...current, pages: nextItems as SeoPage[] };
      return { ...current, [activeSection]: nextItems };
    });
  }

  function patchEditing(patch: Record<string, string | number | boolean | string[] | undefined | IconPage['translations']>) {
    if (!editing) return;
    const id = getItemId(editing);
    const current = pendingItemPatchesRef.current[id];
    pendingItemPatchesRef.current[id] = {
      section: activeSection,
      patch: { ...(current?.patch ?? {}), ...patch, updatedAt: new Date().toISOString() }
    };
    replaceSection(items.map((item) => getItemId(item) === getItemId(editing) ? { ...item, ...patch, updatedAt: new Date().toISOString() } : item) as typeof items);
  }

  function addItem(preset?: Partial<IconsItem>, options?: { openEditor?: boolean }) {
    const item = makeNewItem(activeSection);
    const next = { ...item, ...preset };
    replaceSection([next, ...items] as typeof items);
    setEditingId(getItemId(next));
    if (options?.openEditor !== false) setEditorOpen(true);
    return next;
  }

  function openMaterialEditor(item: IconsItem, tab: EditorTabKey = 'main') {
    setEditingId(getItemId(item));
    setActiveEditorTab(tab);
    setEditorOpen(true);
  }

  function createMaterial() {
    const item = addItem(undefined, { openEditor: false });
    setActiveEditorTab('main');
    setActiveIconTextLanguage('uk');
    setActiveIconTextScope('icon');
    setActiveIconTextTab('description');
    setEditingId(getItemId(item));
    setEditorOpen(true);
  }

  function addCalendarDay(month: string, day: string) {
    const item = addItem({ month, day, id: `calendar-${month}-${day}-${Date.now()}` } as Partial<IconsItem>);
    setActiveCalendarMonth(month);
    setEditingId(getItemId(item));
    setEditorOpen(true);
  }

  function switchSection(section: IconsSection) {
    setActiveSection(section);
    setEditorOpen(false);
    setEditingId('');
  }

  function changeCalendarMonth(month: string) {
    const nextMonth = String(month).padStart(2, '0');
    setActiveCalendarMonth(nextMonth);
    void loadContent({ year: activeCalendarYear, month: nextMonth });
  }

  function changeCalendarYear(year: string) {
    const nextYear = year.replace(/[^\d]/g, '').slice(0, 4);
    setActiveCalendarYear(nextYear);
    if (nextYear.length === 4) {
      void loadContent({ year: nextYear, month: activeCalendarMonth });
    }
  }

  function deleteEditing() {
    if (!editing) return;
    const next = items.filter((item) => getItemId(item) !== getItemId(editing)) as typeof items;
    replaceSection(next);
    setEditingId(next[0] ? getItemId(next[0]) : '');
    if (!next[0]) setEditorOpen(false);
  }

  function applyAiResult(item: typeof editing, result: AiGenerationResult) {
    if (!item) return;
    const title = result.title || getItemTitle(item);
    const description = result.description || '';
    const slug = result.slug || slugify(title);
    const photoPrompt = result.photoPrompt || `Orthodox icon editorial image, ${title}, museum lighting, gold and cream palette`;

    if (activeSection === 'calendar' && 'day' in item) {
      patchEditing({
        label: title,
        note: result.description?.split('.')[0]?.slice(0, 80) || item.note,
        description: description.slice(0, 220),
        detailHref: result.slug ? `/icons/${result.slug}` : item.detailHref,
        kind: item.kind || 'feast'
      });
      setAiPrompt(photoPrompt);
      return;
    }

    if (activeSection === 'icons' && 'qrCodeUrl' in item) {
      const draft = iconPageDraft(title, description);
      const structuredDescription = structuredSectionsFromText(description, ['Краткое описание изображения', 'Символы на иконе', 'Alt для фото', 'Prompt для генерации', 'Источник изображения', 'Полное описание', 'Смысл праздника', 'Что важно знать', 'Для кого эта молитва/страница', 'Не писать']);
      const structuredPrayer = structuredSectionsFromText(description, prayerFields);
      const structuredGospel = structuredSectionsFromText(description, ['Апостольское чтение', 'Евангельское чтение', 'Цитата дня', 'Объяснение простыми словами', 'Связь с событием', 'Источник']);
      const structuredLife = structuredSectionsFromText(description, ['Краткое житие', 'Подробное житие', 'Главные события жизни', 'Духовный смысл', 'Где почитается', 'Источники']);
      const structuredHistory = structuredSectionsFromText(description, ['История праздника', 'Дата по старому стилю', 'Дата по новому стилю', 'Разные календарные традиции', 'Почему бывает путаница', 'Проверенные источники', 'Дата проверена', 'Календарный стиль', 'Найденное событие', 'Уверенность', 'Предупреждение']);
      patchEditing({
        title,
        slug,
        seoTitle: sectionFromText(description, ['SEO title']) || title,
        seoDescription: compactText(sectionFromText(description, ['SEO description']) || description, 260),
        seoKeywords: `${title}, православная икона, молитва, Свет Иконы`,
        shortDescription: draft.shortDescription,
        fullDescription: structuredDescription || draft.fullDescription,
        prayerText: structuredPrayer || (isThinText(item.prayerText) ? draft.prayerText : item.prayerText),
        gospelText: structuredGospel || (isThinText(item.gospelText) ? draft.gospelText : item.gospelText),
        lifeText: structuredLife || (isThinText(item.lifeText) ? draft.lifeText : item.lifeText),
        historyText: structuredHistory || (isThinText(item.historyText) ? draft.historyText : item.historyText)
      });
      setAiPrompt(photoPrompt);
      return;
    }

    if (activeSection === 'prayers' && 'text' in item) {
      patchEditing({ title, slug, category: 'Молитвы', text: description, seoTitle: title, seoDescription: description.slice(0, 260), status: 'draft' });
      return;
    }

    if (activeSection === 'saints' && 'biography' in item) {
      patchEditing({ name: title, slug, shortDescription: description.slice(0, 220), biography: description, seoTitle: title, seoDescription: description.slice(0, 260), status: 'draft' });
      return;
    }

    if (activeSection === 'gospel' && 'reference' in item) {
      patchEditing({ title, reference: item.reference || 'Мф. 5:14-16', text: description, explanation: result.suggestions.join('\n'), seoTitle: title, seoDescription: description.slice(0, 260), status: 'draft' });
      return;
    }

    if (activeSection === 'seo' && 'content' in item) {
      patchEditing({ title, h1: title, slug, content: description, seoTitle: title, seoDescription: description.slice(0, 260), targetKeyword: aiPrompt, status: 'draft' });
      return;
    }

    if (activeSection === 'churches' && 'address' in item) {
      patchEditing({ title, slug, city: item.city || 'Город', address: item.address || 'Адрес храма', schedule: item.schedule || 'Расписание богослужений уточняется.', description, seoTitle: title, seoDescription: description.slice(0, 260), status: 'draft' });
      return;
    }

    if (activeSection === 'qr' && 'iconId' in item) {
      patchEditing({ title, slug, customPrayer: description, active: true });
    }
  }

  async function runAiDraft() {
    if (!editing) return;
    setAiTextLoading(true);
    setError('');
    try {
      const result = await generateSeo({
        site: 'icons',
        language: 'ru' as LanguageCode,
        type: 'seo',
        sourceText: activeSection === 'icons'
          ? buildIconEditorialPrompt(editing, aiPrompt)
          : `${aiPrompt}\n\nРаздел: ${activeSection}\nТекущий материал: ${JSON.stringify(editing)}`,
        tone: 'seo',
        keywords: [activeSection, getItemTitle(editing), 'Свет Иконы', 'православие', 'молитва']
      });
      applyAiResult(editing, result);
      setMessage('Gemini подготовил текст и SEO для выбранного материала');
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог создать текст');
    } finally {
      setAiTextLoading(false);
    }
  }

  async function generateIconTextTabDraft(item: IconPage, language: IconsLang, scope: IconTextScopeKey, tab: IconTextTabKey) {
    const titles = structuredTitlesForTextTab(scope, tab);
    const result = await generateSeo({
      site: 'icons',
      language: (language === 'en' ? 'en' : 'ru') as LanguageCode,
      type: 'seo',
      sourceText: buildIconTextTabPrompt(item, scope, tab, language, aiPrompt),
      tone: 'seo',
      keywords: [
        'Свет Иконы',
        getItemTitle(item),
        scope,
        tab,
        language,
        'православный календарь',
        'проверенные источники'
      ]
    });
    const description = result.description || result.suggestions.join('\n') || '';
    const structured = ensureStructuredTabText(description, titles);
    const key = patchKeyForTextTab(tab);
    const textPatch: Partial<IconTranslation> = { [key]: structured };
    if (tab === 'description') {
      textPatch.title = result.title || item.title;
      textPatch.category = sectionFromText(structured, ['Категория / направление', 'Категория', 'Направление']) || item.category;
      textPatch.saintName = sectionFromText(structured, ['Святой / праздник', 'Главные святые дня', 'Кому посвящён']) || item.saintName;
      textPatch.shortDescription = compactText(
        sectionFromText(structured, ['Краткое описание изображения', 'Кратко кто это', 'Что показать на храмовой странице', 'Полное описание'])
        || result.title
        || description
        || item.shortDescription,
        260
      );
    }
    return textPatch;
  }

  async function runAiTextTabDraft() {
    if (!editing || activeSection !== 'icons' || !('qrCodeUrl' in editing)) return;
    setAiTextLoading(true);
    setError('');
    try {
      let finalPatch: IconTextPatch = {};
      for (const language of iconTextLanguages) {
        const textPatch = await generateIconTextTabDraft(editing, language, activeIconTextScope, activeIconTextTab);
        finalPatch = mergeLocalizedTextPatch(editing, finalPatch, language, textPatch);
      }
      patchEditing(finalPatch);
      setMessage(`Gemini обновил вкладку сразу на UK/RU/EN: ${activeIconTextScope} / ${activeIconTextTab}`);
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог обновить вкладку');
    } finally {
      setAiTextLoading(false);
    }
  }

  async function runAiTextScopeDraft() {
    if (!editing || activeSection !== 'icons' || !('qrCodeUrl' in editing)) return;
    setAiTextLoading(true);
    setError('');
    try {
      const tabs: IconTextTabKey[] = ['description', 'prayer', 'gospel', 'life', 'history'];
      let finalPatch: IconTextPatch = {};
      for (const language of iconTextLanguages) {
        let languagePatch: Partial<IconTranslation> = {};
        for (const tab of tabs) {
          languagePatch = {
            ...languagePatch,
            ...(await generateIconTextTabDraft(editing, language, activeIconTextScope, tab))
          };
        }
        finalPatch = mergeLocalizedTextPatch(editing, finalPatch, language, languagePatch);
      }
      patchEditing(finalPatch);
      setMessage(`Gemini обновил все вкладки сразу на UK/RU/EN: ${activeIconTextScope}`);
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог обновить все вкладки');
    } finally {
      setAiTextLoading(false);
    }
  }

  async function createWithGemini() {
    setAiCreateLoading(true);
    setError('');
    try {
      const result = await generateSeo({
        site: 'icons',
        language: 'ru',
        type: 'seo',
        sourceText: activeSection === 'icons'
          ? buildIconEditorialPrompt(undefined, aiPrompt)
          : `${aiPrompt}\n\nСоздай новый материал для раздела ${activeSection}. Верни православный, аккуратный, SEO-подготовленный текст.`,
        tone: 'seo',
        keywords: [activeSection, 'Свет Иконы', 'иконы', 'молитвы']
      });
      const preset = activeSection === 'icons'
        ? { title: result.title || 'Новая икона', slug: result.slug || slugify(result.title || aiPrompt), ...iconPageDraft(result.title || aiPrompt, result.description || ''), seoTitle: result.title || '', seoDescription: result.description?.slice(0, 260) || '', seoKeywords: `${result.title || aiPrompt}, православная икона, молитва, Свет Иконы`, status: 'draft' as const }
        : undefined;
      const next = addItem(preset);
      applyAiResult(next as typeof editing, result);
      setEditorOpen(true);
      setMessage('Gemini создал новый материал. Проверь и нажми «Сохранить».');
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог создать материал');
    } finally {
      setAiCreateLoading(false);
    }
  }

  async function runBackendIconAudit(item: IconPage) {
    setBackendAuditLoadingId(item.id);
    setError('');
    try {
      const result = await generateSeo({
        site: 'icons',
        language: 'ru',
        type: 'seo',
        tone: 'expert',
        keywords: ['rulebot', 'icons-readiness', item.slug || item.id, '10/10'],
        sourceText: buildIconBackendAuditPrompt(item)
      });
      const audit = parseBackendIconAudit(result, item);
      setBackendAudits((current) => ({ ...current, [item.id]: audit }));
      setMessage(audit.ok
        ? 'Backend RuleBot подтвердил 10/10: материал готов к публикации.'
        : `Backend RuleBot: ${audit.score}/${audit.total}. Нужно исправить: ${audit.missing.slice(0, 3).join(', ') || audit.warnings.slice(0, 3).join(', ')}`);
    } catch (auditError) {
      const fallback = buildLocalBackendAuditFallback(item, auditError instanceof Error ? auditError.message : 'Backend audit failed');
      setBackendAudits((current) => ({ ...current, [item.id]: fallback }));
      setError(fallback.message);
    } finally {
      setBackendAuditLoadingId('');
    }
  }

  async function generateCalendarDayMaterial() {
    if (!editing || !('day' in editing)) return;
    setAiCalendarLoading(true);
    setError('');
    try {
      const textResult = await generateSeo({
        site: 'icons',
        language: 'ru',
        type: 'seo',
        sourceText: buildCalendarDayPrompt(editing, calendarAiPrompt),
        tone: 'seo',
        keywords: ['церковный календарь', editing.julianDate || editing.day, editing.label || 'день календаря']
      });

      const title = sectionFromText(textResult.description || '', ['Название дня']) || textResult.title || editing.label || `День ${editing.day}`;
      const generatedKindRaw = sectionFromText(textResult.description || '', ['Тип дня']).toLowerCase();
      const generatedKind = ['feast', 'gospel', 'prayer', 'fast', 'quiet'].includes(generatedKindRaw)
        ? generatedKindRaw as CalendarDay['kind']
        : editing.kind;
      const note = compactText(sectionFromText(textResult.description || '', ['Тип подписи']) || textResult.suggestions?.[0] || editing.note || 'День церковного календаря', 80);
      const whatHappened = sectionFromText(textResult.description || '', ['Что произошло']);
      const saintOrFeast = sectionFromText(textResult.description || '', ['Святой или праздник']);
      const bibleLink = sectionFromText(textResult.description || '', ['Библейская связь']);
      const noSpecificEvent = generatedKind === 'quiet'
        || /нет отдельного|не указан|ничего нет|обычный день/i.test(`${title} ${note} ${whatHappened} ${saintOrFeast}`);
      const invalidTheotokos = isInvalidTheotokosForJulianDate(editing, `${title} ${note} ${whatHappened} ${saintOrFeast}`);
      const shortDescription = compactText(
        sectionFromText(textResult.description || '', ['Короткое описание'])
        || [whatHappened, saintOrFeast, bibleLink].filter(Boolean).join(' ')
        || (noSpecificEvent || invalidTheotokos ? 'В этот день в текущих данных нет подтвержденной Богородичной памяти для выбранной юлианской даты. Это обычный день церковного календаря для спокойной молитвы и чтения.' : textResult.description)
        || editing.description,
        320
      );
      const photoPrompt = invalidTheotokos
        ? 'Simple Orthodox calendar image: one centered sacred Orthodox-style image, warm church light, no specific saint, no Theotokos icon, no book, no candle, no table.'
        : sectionFromText(textResult.description || '', ['PhotoPrompt']) || textResult.photoPrompt || calendarAiPrompt;
      const imageResult = await generateAiImage({
        site: 'icons',
        title,
        description: shortDescription,
        scene: buildCalendarDayImagePrompt(editing, title, shortDescription, photoPrompt),
        imageType: 'calendar',
        enhanced: false,
        variant: Date.now() % 7
      });

      patchEditing({
        label: noSpecificEvent || invalidTheotokos ? 'Обычный день церковного календаря' : title,
        note: noSpecificEvent || invalidTheotokos ? 'Без отдельного праздника' : note,
        kind: noSpecificEvent || invalidTheotokos ? 'quiet' : generatedKind,
        feast: noSpecificEvent || invalidTheotokos ? false : editing.feast,
        textOnly: false,
        description: shortDescription,
        imageUrl: imageResult.imageUrl || editing.imageUrl,
        detailHref: editing.detailHref || '/icons'
      });
      setCalendarAiPrompt(photoPrompt);
      setMessage('Gemini проверил день календаря, подготовил короткий текст и сгенерировал фото');
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог подготовить день календаря');
    } finally {
      setAiCalendarLoading(false);
    }
  }

  function supportsDirectPhoto() {
    return Boolean(editing && 'imageUrl' in editing && !('day' in editing));
  }

  function setIconImages(nextImages: string[]) {
    if (!supportsDirectPhoto()) return;
    const cleanImages = uniqueImages(nextImages).slice(0, ICON_IMAGE_MAX);
    patchEditing({
      imageUrl: cleanImages[0] || '',
      imageUrls: cleanImages.slice(1)
    });
    setSelectedIconImage((current) => Math.min(current, Math.max(0, cleanImages.length - 1)));
  }

  function setIconImageAt(index: number, url: string) {
    const nextImages = [...editingImages];
    nextImages[index] = url;
    setIconImages(nextImages);
    setSelectedIconImage(index);
  }

  function addIconImage(url = '') {
    if (editingImages.length >= ICON_IMAGE_MAX) return;
    const nextImages = [...editingImages, url];
    setIconImages(nextImages);
    setSelectedIconImage(nextImages.length - 1);
  }

  function promoteIconImage(index: number) {
    if (!editingImages[index]) return;
    const nextImages = [editingImages[index], ...editingImages.filter((_, imageIndex) => imageIndex !== index)];
    setIconImages(nextImages);
    setSelectedIconImage(0);
  }

  function moveIconImage(index: number, direction: -1 | 1) {
    const targetIndex = index + direction;
    if (!editingImages[index] || targetIndex < 0 || targetIndex >= editingImages.length) return;
    const nextImages = [...editingImages];
    [nextImages[index], nextImages[targetIndex]] = [nextImages[targetIndex], nextImages[index]];
    setIconImages(nextImages);
    setSelectedIconImage(targetIndex);
  }

  function removeIconImage(index: number) {
    const nextImages = editingImages.filter((_, imageIndex) => imageIndex !== index);
    setIconImages(nextImages);
  }

  function iconImageLabel(url: string | undefined, index: number) {
    if (!url) return `Фото ${index + 1}`;
    if (isGeneratedQrImage(url)) return 'QR';
    if (index === 0) return 'Главное';
    return `Фото ${index + 1}`;
  }

  function ensureQrPageForEditing() {
    if (!editing || !('qrCodeUrl' in editing)) return;
    const qrId = editing.slug || editing.id;
    setContent((current) => {
      const exists = current.qrPages.some((page) => page.qrId === qrId);
      const nextQrPages = exists
        ? current.qrPages.map((page) => page.qrId === qrId ? {
          ...page,
          iconId: editing.id,
          slug: qrId,
          title: editing.title,
          customPrayer: page.customPrayer || editing.prayerText,
          updatedAt: new Date().toISOString()
        } : page)
        : [{
          id: `qr-${Date.now()}`,
          qrId,
          iconId: editing.id,
          slug: qrId,
          title: editing.title,
          active: true,
          scanCount: 0,
          ownerName: 'Страница иконы',
          location: 'QR на иконе',
          customPrayer: editing.prayerText,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString()
        }, ...current.qrPages];

      return { ...current, qrPages: nextQrPages };
    });
    patchEditing({ qrCodeUrl: qrPageUrlForIcon(editing) });
    setMessage(`QR-страница готова: ${qrPageUrlForIcon(editing)}`);
  }

  async function uploadPhotosFromPc(files: FileList | null) {
    if (!supportsDirectPhoto() || !files) return;
    const selected = Array.from(files)
      .filter((file) => file.type.startsWith('image/'))
      .slice(0, Math.max(0, ICON_IMAGE_MAX - editingImages.length));
    if (selected.length === 0) return;
    setPhotoUploadLoading(true);
    setError('');
    try {
      for (const file of selected) {
        if (file.size > 10 * 1024 * 1024) throw new Error('Каждое фото должно быть меньше 10 MB');
      }
      const result = await analyzePhotoWithGemini(
        selected[0],
        'icons',
        'ru',
        buildIconVisionInstruction(editing, aiPrompt)
      );
      if (!result.imageUrl) throw new Error('Backend загрузил фото, но не вернул imageUrl');
      const uploadedUrls = [result.imageUrl];
      for (const file of selected.slice(1)) {
        uploadedUrls.push(await uploadCmsReference(file));
      }
      const draft = iconPageDraft(result.title || getItemTitle(editing), result.description || '');
      const nextImages = uniqueImages([...uploadedUrls, ...editingImages]).slice(0, ICON_IMAGE_MAX);
      patchEditing({
        imageUrl: nextImages[0] || result.imageUrl,
        imageUrls: nextImages.slice(1),
        title: result.title || getItemTitle(editing),
        slug: result.slug || ('slug' in editing ? editing.slug : slugify(result.title || getItemTitle(editing))),
        category: result.category || ('category' in editing ? editing.category : 'Православная икона'),
        seoTitle: result.seoTitle || result.title || ('seoTitle' in editing ? editing.seoTitle : ''),
        seoDescription: result.seoDescription || compactText(result.description || '', 260),
        shortDescription: draft.shortDescription,
        fullDescription: draft.fullDescription,
        prayerText: draft.prayerText,
        gospelText: draft.gospelText,
        lifeText: draft.lifeText,
        historyText: draft.historyText
      });
      setSelectedIconImage(0);
      setMessage(selected.length === 1
        ? 'Фото загружено: Gemini Vision распознал образ, обновил текст, SEO и молитву'
        : `Загружено фото: ${selected.length}. Первое стало главным, остальные добавлены в галерею.`);
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото');
    } finally {
      setPhotoUploadLoading(false);
    }
  }

  async function addIconReferenceImages(files: FileList | null) {
    if (!files) return;
    const selected = Array.from(files).filter((file) => file.type.startsWith('image/')).slice(0, 2 - iconReferenceImages.length);
    if (selected.length === 0) return;
    setIconReferenceBusy(true);
    setError('');
    try {
      const uploaded: AiReferenceImage[] = [];
      for (const file of selected) {
        if (file.size > 10 * 1024 * 1024) throw new Error('Каждый референс должен быть меньше 10 MB');
        const url = await uploadCmsReference(file);
        uploaded.push({ url, preview: URL.createObjectURL(file), name: file.name });
      }
      setIconReferenceImages((current) => [...current, ...uploaded].slice(0, 2));
      setMessage(`Добавлено референсов: ${uploaded.length}. Gemini возьмет их перед генерацией макета.`);
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить референс');
    } finally {
      setIconReferenceBusy(false);
    }
  }

  function removeIconReference(index: number) {
    setIconReferenceImages((current) => {
      const target = current[index];
      if (target) URL.revokeObjectURL(target.preview);
      return current.filter((_, itemIndex) => itemIndex !== index);
    });
  }

  async function generatePhotoWithGemini(extraVariant = false) {
    if (!editing) return;
    if (extraVariant) setAiImagePlusLoading(true);
    else setAiImageLoading(true);
    setError('');
    try {
      const title = getItemTitle(editing);
      const referenceUrls = iconReferenceImages.length
        ? iconReferenceImages.map((item) => item.url)
        : buildIconReferenceUrls(editingImages, selectedIconImage);
      const result = await generateAiImage({
        site: 'icons',
        title,
        description: `Православная икона ${title}. Использовать ${referenceUrls.length} референса для точной генерации.`,
        scene: buildIconImagePrompt(title, aiPrompt, referenceUrls, extraVariant),
        imageType: 'article',
        referenceUrls,
        enhanced: false,
        variant: extraVariant ? Date.now() % 7 : 0
      });
      if (!result.imageUrl) throw new Error('Backend не вернул imageUrl для Gemini-фото');
      if (supportsDirectPhoto()) {
        if (extraVariant) {
          addIconImage(result.imageUrl);
        } else {
          const sourceImage = referenceUrls[0] || editingImages[selectedIconImage] || editingImages[0];
          const existingImages = editingImages.filter((url) => url && url !== sourceImage && !isGeneratedQrImage(url));
          const cleanImages = uniqueImages([sourceImage, result.imageUrl, ...existingImages]).slice(0, ICON_IMAGE_MAX);
          setIconImages(cleanImages);
          setSelectedIconImage(1);
        }
      }
      const modelNote = result.imageModel ? ` · модель: ${result.imageModel}` : '';
      setMessage(extraVariant ? `Gemini сгенерировал новый вариант по ${referenceUrls.length} референсам и добавил его в галерею${modelNote}` : `Gemini улучшил выбранное фото по ${referenceUrls.length} референсам${modelNote}`);
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог создать фото');
    } finally {
      if (extraVariant) setAiImagePlusLoading(false);
      else setAiImageLoading(false);
    }
  }

  async function regenerateSelectedIconPhoto() {
    if (!editing || !editingImages[selectedIconImage] || isGeneratedQrImage(editingImages[selectedIconImage])) return;
    setAiSelectedImageLoading(true);
    setError('');
    try {
      const title = getItemTitle(editing);
      const selectedImage = editingImages[selectedIconImage];
      const result = await generateAiImage({
        site: 'icons',
        title,
        description: `Перегенерировать выбранное фото ${selectedIconImage + 1} для ${title}.`,
        scene: [
          'Use Reference 1 as the exact selected product photo.',
          'Regenerate it as a cleaner premium product photo.',
          iconPhotoAspectPrompts[selectedPhotoAspect],
          'Preserve the same object, frame, phone, QR module, composition, camera angle and lighting mood.',
          'Improve clarity, detail, color balance and product lighting.',
          'Do not add readable new text, logos, watermarks or extra objects.'
        ].join(' '),
        imageType: 'article',
        referenceUrls: [selectedImage],
        enhanced: false,
        variant: Date.now() % 7
      });
      if (!result.imageUrl) throw new Error('Backend не вернул imageUrl для выбранного фото');
      setIconImageAt(selectedIconImage, result.imageUrl);
      const modelNote = result.imageModel ? ` · модель: ${result.imageModel}` : '';
      setMessage(`Выбранное фото ${selectedIconImage + 1} перегенерировано (${selectedPhotoAspect})${modelNote}`);
    } catch (aiError) {
      setError(aiError instanceof Error ? aiError.message : 'Gemini не смог перегенерировать выбранное фото');
    } finally {
      setAiSelectedImageLoading(false);
    }
  }

  async function generateSelectedIconPhotoCount() {
    const count = Math.min(iconPhotoGenerateCount, ICON_IMAGE_MAX - editingImages.length + 1);
    if (count <= 1) {
      await generatePhotoWithGemini(false);
      return;
    }
    for (let index = 0; index < count; index += 1) {
      await generatePhotoWithGemini(index > 0);
    }
  }

  return (
    <section className="ops-page icons-cms-page">
      <AdminPageHeader
        eyebrow="Svet Ikony CMS"
        title="Сайт православных икон"
        subtitle="Контент хранится в backend: иконы, молитвы, святые, Евангелие дня, QR-страницы, SEO и страницы храмов."
        icon="cms"
        actions={
          <>
            {message ? <small className="page-muted">{message}</small> : null}
          <button className="btn btn-quiet" type="button" onClick={() => void loadContent()} disabled={loading}><AppIcon name="refresh" />{loading ? 'Загрузка...' : 'Обновить'}</button>
          {sectionIsEditable ? <button className="btn btn-ai" type="button" onClick={createWithGemini} disabled={aiCreateLoading}><AppIcon name="bot" />{aiCreateLoading ? 'Gemini...' : 'AI создать'}</button> : null}
          <button className="btn btn-primary" type="button" onClick={saveContent} disabled={saving}>{saving ? 'Сохранение...' : 'Сохранить'}</button>
          </>
        }
      />

      {error ? <p className="almabuild-alert error">{error}</p> : null}

      <nav className="icons-section-switcher" aria-label="Svet Ikony content sections">
        {sections.map((section) => (
          <button
            key={section.key}
            type="button"
            className={activeSection === section.key ? 'active' : ''}
            onClick={() => switchSection(section.key)}
          >
            <AppIcon name={section.icon} />
            <span>{section.label}</span>
            <small>{section.note}</small>
          </button>
        ))}
      </nav>

      {activeSection === 'calendar' ? (
        <CalendarMonthCards
          year={activeCalendarYear}
          days={content.calendar.days}
          selectedMonth={activeCalendarMonth}
          onYearChange={changeCalendarYear}
          onMonthChange={changeCalendarMonth}
        />
      ) : activeSection === 'icons' ? (
        <section className="ops-panel icons-workbench">
          <div className="icons-workbench-head">
            <div>
              <span><AppIcon name="cms" />Иконы</span>
              <strong>{iconStats.total} материалов</strong>
            </div>
            <div className="icons-workbench-stats">
              <span><i />{iconStats.published} опубликовано</span>
              <span><i />{iconStats.draft} черновиков</span>
              <span><i />{iconStats.ready} почти готово</span>
            </div>
          </div>

          <div className="icons-workbench-toolbar">
            <label className="icons-search-field">
              <AppIcon name="search" />
              <input value={resourceQuery} onChange={(event) => setResourceQuery(event.target.value)} placeholder="Поиск по названию, slug, святому или SEO" />
            </label>
            <label className="icons-filter-field">
              <span>Статус</span>
              <select value={resourceStatus} onChange={(event) => setResourceStatus(event.target.value as typeof resourceStatus)}>
                <option value="all">Все</option>
                <option value="draft">Черновики</option>
                <option value="published">Опубликовано</option>
              </select>
            </label>
            <button className="btn btn-primary" type="button" onClick={createMaterial}>
              <AppIcon name="cms" />Новая икона
            </button>
            <button className="btn btn-ai" type="button" onClick={createWithGemini} disabled={aiCreateLoading}>
              <AppIcon name="bot" />{aiCreateLoading ? 'Gemini...' : 'AI draft'}
            </button>
          </div>

          <div className="icons-card-grid">
            {visibleItems.map((item) => {
              if (!('qrCodeUrl' in item)) return null;
              const readiness = iconReadiness(item);
              const missing = iconMissingFields(item);
              const image = getItemPreviewImage(item);
              return (
                <article className="icon-resource-card" key={item.id}>
                  <button className="icon-resource-image" type="button" onClick={() => openMaterialEditor(item, image ? 'photo-ai' : 'main')}>
                    {image ? <img src={image} alt={item.title} loading="lazy" /> : <span><AppIcon name="image" />Добавить фото</span>}
                  </button>
                  <div className="icon-resource-body">
                    <div className="icon-resource-title">
                      <span className={'status-pill ' + statusClass(item.status)}><i />{item.status === 'published' ? 'опубликовано' : 'черновик'}</span>
                      <strong>{item.title}</strong>
                      <small>/icons/{item.slug || item.id}</small>
                    </div>
                    <p>{item.shortDescription || item.seoDescription || 'Добавьте краткое описание, чтобы карточка была понятна на сайте.'}</p>
                    <div className="icon-readiness">
                      <span style={{ width: `${readiness.percent}%` }} />
                    </div>
                    <div className="icon-resource-meta">
                      <span>{readiness.complete}/{readiness.total} готово</span>
                      <span>{item.category || 'без категории'}</span>
                      <span>{item.calendarDate || 'без даты'}</span>
                    </div>
                    {missing.length ? <small className="icon-missing">Нужно: {missing.join(', ')}</small> : <small className="icon-missing done">Материал готов к публикации</small>}
                    <div className="icon-card-actions">
                      <button className="table-action" type="button" onClick={() => openMaterialEditor(item, 'main')}>Основное</button>
                      <button className="table-action" type="button" onClick={() => openMaterialEditor(item, 'texts')}>Тексты</button>
                      <button className="table-action" type="button" onClick={() => openMaterialEditor(item, 'seo')}>SEO</button>
                      <button className="table-action" type="button" onClick={() => openMaterialEditor(item, 'photo-ai')}>Фото</button>
                    </div>
                  </div>
                </article>
              );
            })}
          </div>

          {!visibleItems.length ? (
            <div className="icons-empty-state">
              <AppIcon name="cms" />
              <strong>Иконы не найдены</strong>
              <span>Сбросьте фильтры или создайте новую икону.</span>
              <button className="btn btn-primary" type="button" onClick={createMaterial}>Новая икона</button>
            </div>
          ) : null}
        </section>
      ) : (
        <section className="ops-panel icons-materials-panel">
          <div className="panel-title">
            <span><AppIcon name={activeSectionMeta.icon} />{activeSectionMeta.label}</span>
            {sectionIsEditable ? <button className="table-action" type="button" onClick={createMaterial}>+ Добавить</button> : <small>Просмотр: данные собираются из редактора «Иконы»</small>}
          </div>
          <SiteSectionVisualPreview section={activeSection} content={content} items={visibleItems} />
          <div className="table-scroll">
            <table className="ops-table">
              <thead><tr><th>Фото</th><th>Материал</th><th>Тип</th><th>Статус</th><th /></tr></thead>
              <tbody>{visibleItems.map((item) => (
                <tr key={getItemId(item)}>
                  <td>{getItemPreviewImage(item) ? <img className="icons-material-thumb" src={getItemPreviewImage(item)} alt={getItemTitle(item)} loading="lazy" /> : <span className="icons-material-thumb empty"><AppIcon name="image" size={18} /></span>}</td>
                  <td><strong>{getItemTitle(item)}</strong><small>{getItemSubline(activeSection, item)}</small></td>
                  <td>{activeSection}</td>
                  <td>{'status' in item ? <span className={'status-pill ' + statusClass(item.status)}><i />{item.status === 'published' ? 'опубликовано' : 'черновик'}</span> : <span className="status-pill good"><i />активно</span>}</td>
                  <td>{sectionIsEditable ? <button className="table-action" type="button" onClick={() => openMaterialEditor(item)}>Редактировать</button> : <span className="page-muted">из икон</span>}</td>
                </tr>
              ))}</tbody>
            </table>
          </div>
        </section>
      )}

      {editorOpen && editing && sectionIsEditable ? (
        <div className="modal-overlay content-editor-overlay" onMouseDown={() => setEditorOpen(false)}>
          <section className="editor-modal content-editor-modal" role="dialog" aria-modal="true" aria-label="Редактирование сайта икон" onMouseDown={(event) => event.stopPropagation()}>
            <div className="editor-modal-head">
              <div className="editor-titlebar">
                <button className="editor-back-button" type="button" onClick={() => setEditorOpen(false)} aria-label="Закрыть редактор">
                  <AppIcon name="chevron-left" size={22} />
                </button>
                <div>
                  <p className="eyebrow">Редактор материала</p>
                  <h2>{getItemTitle(editing)}</h2>
                </div>
              </div>
              <div className="editor-actions">
                <button className="btn btn-quiet" type="button" onClick={() => setEditorOpen(false)}>Закрыть</button>
                <button className="btn btn-primary" type="button" onClick={saveContent} disabled={saving}><AppIcon name="save" />{saving ? 'Сохранение...' : 'Сохранить'}</button>
              </div>
            </div>

            {'qrCodeUrl' in editing ? (
              <IconEditorOverview
                item={editing}
                image={getItemPreviewImage(editing)}
                onPatch={patchEditing}
                backendAudit={backendAudits[editing.id]}
                backendChecking={backendAuditLoadingId === editing.id}
                onBackendCheck={() => void runBackendIconAudit(editing)}
              />
            ) : null}

            <nav className="editor-tabbar" aria-label="Разделы редактора">
              {editorTabs.map((tab) => (
                <button key={tab.key} className={activeEditorTab === tab.key ? 'active' : ''} type="button" onClick={() => setActiveEditorTab(tab.key)}>
                  <AppIcon name={tab.icon} />
                  <span>{tab.label}</span>
                </button>
              ))}
            </nav>

            <div className="editor-tabstage">
              {activeEditorTab === 'main' ? <EditorFields section={activeSection} item={editing} patch={patchEditing} view="main" /> : null}
              {activeEditorTab === 'texts' && 'qrCodeUrl' in editing ? (
                <IconTextTabs
                  item={editing}
                  activeScope={activeIconTextScope}
                  onScopeChange={setActiveIconTextScope}
                  activeLanguage={activeIconTextLanguage}
                  onLanguageChange={setActiveIconTextLanguage}
                  activeTab={activeIconTextTab}
                  onTabChange={setActiveIconTextTab}
                  patch={patchEditing}
                  onGenerate={runAiTextTabDraft}
                  onGenerateAll={runAiTextScopeDraft}
                  generating={aiTextLoading}
                />
              ) : null}
              {activeEditorTab === 'texts' && !('qrCodeUrl' in editing) ? <EditorFields section={activeSection} item={editing} patch={patchEditing} view="texts" /> : null}
              {activeEditorTab === 'seo' ? <EditorFields section={activeSection} item={editing} patch={patchEditing} view="seo" /> : null}
              {activeEditorTab === 'calendar' && 'qrCodeUrl' in editing ? (
                <EditorSection title="Календарь" note="Укажи гражданскую дату. На сайте в ячейку календаря подтянутся название материала, краткий текст и первое фото из галереи.">
                  <div className="editor-section-grid">
                    <div className="settings-matrix">
                      <label className="editor-field">
                        <span>Дата в календаре</span>
                        <input
                          type="date"
                          value={editing.calendarDate || ''}
                          onChange={(event) => patchEditing({ calendarDate: event.target.value || undefined })}
                        />
                        <small>Если дата пустая, материал не показывается в календаре.</small>
                      </label>
                      <label className="editor-field">
                        <span>Страница</span>
                        <input value={`/icons/${editing.slug || editing.id}`} readOnly onChange={() => undefined} />
                      </label>
                    </div>
                    <div className="calendar-icon-preview">
                      {editingImages[0] ? <img src={editingImages[0]} alt={editing.title} /> : <span><AppIcon name="image" />Первое фото не выбрано</span>}
                      <div>
                        <strong>{editing.title || 'Название материала'}</strong>
                        <small>{editing.saintName || editing.category || 'Подпись календаря'}</small>
                        <p>{editing.shortDescription || editing.seoDescription || 'Краткое описание будет показано в ячейке календаря.'}</p>
                      </div>
                    </div>
                  </div>
                </EditorSection>
              ) : null}
              {activeEditorTab === 'qr' && 'qrCodeUrl' in editing ? <QrCodePreview item={editing} onGenerate={ensureQrPageForEditing} /> : null}
              {activeEditorTab === 'qr' && 'qrId' in editing ? <QrCodePreview item={editing} /> : null}
              {activeEditorTab === 'publish' ? (
                <EditorSection title="Публикация" note="Финальные действия с материалом. Сохранение обновляет данные сайта.">
                  {'status' in editing ? <SelectField label="Статус на сайте" value={editing.status} options={['draft', 'published']} onChange={(value) => patchEditing({ status: value })} /> : null}
                  {'active' in editing ? <SelectField label="QR активен" value={String(editing.active)} options={['true', 'false']} onChange={(value) => patchEditing({ active: value === 'true' })} /> : null}
                  <div className="editor-actions">
                    <button className="btn btn-danger" type="button" onClick={deleteEditing}>Удалить материал</button>
                  </div>
                </EditorSection>
              ) : null}
            </div>

            {activeEditorTab === 'photo-ai' && 'day' in editing ? (
              <section className="calendar-day-ai-panel">
                <div className="calendar-day-ai-preview">
                  {editing.imageUrl ? <img src={editing.imageUrl} alt={editing.label || `День ${editing.day}`} /> : <span><AppIcon name="image" size={34} />Фото дня еще не создано</span>}
                </div>
                <div className="content-photo-tools">
                  <EditorSection title="Материал дня" note="Одно действие: Gemini проверит церковную дату, напишет короткое описание и создаст изображение для ячейки.">
                    <div className="calendar-day-ai-meta">
                      <span>{editing.gregorianDate || `День ${editing.day}`}</span>
                      <strong>{editing.julianDate || editing.julianDay || 'Юлианская дата'}</strong>
                    </div>
                    <label className="editor-field">
                      <span>Уточнение для Gemini</span>
                      <textarea value={calendarAiPrompt} onChange={(event) => setCalendarAiPrompt(event.target.value)} placeholder="Например: проверить святого дня по старому стилю, дать библейскую связь, сделать календарную иллюстрацию без текста" />
                    </label>
                    <label className="editor-field">
                      <span>Фото дня URL</span>
                      <input value={editing.imageUrl || ''} onChange={(event) => patchEditing({ imageUrl: event.target.value })} placeholder="https://..." />
                    </label>
                    <div className="editor-actions">
                      <button className="btn btn-ai" type="button" onClick={() => void generateCalendarDayMaterial()} disabled={aiCalendarLoading}>
                        <AppIcon name="bot" />{aiCalendarLoading ? 'Gemini готовит...' : 'Gemini: день + фото'}
                      </button>
                    </div>
                  </EditorSection>
                </div>
              </section>
            ) : null}

            {activeEditorTab === 'photo-ai' && supportsDirectPhoto() ? (
              <section className="content-photo-panel">
                <button className="content-photo-preview icon-gallery-preview" type="button" onClick={() => editingImages[selectedIconImage] ? setFullscreenIconImage(selectedIconImage) : undefined}>
                  {editingImages[selectedIconImage] ? <img src={editingImages[selectedIconImage]} alt={getItemTitle(editing)} /> : <span><AppIcon name="cms" size={34} />Фото {selectedIconImage + 1} не выбрано</span>}
                </button>
                <div className="content-photo-tools">
                  <div className="gemini-bar">
                    <label>
                      <span>Тема для Gemini</span>
                      <textarea value={aiPrompt} onChange={(event) => setAiPrompt(event.target.value)} placeholder="Например: Икона святителя Николая, молитва, история образа, QR-страница для храма" />
                    </label>
                    <button className="btn btn-ai" type="button" onClick={runAiDraft} disabled={aiTextLoading}><AppIcon name="bot" />{aiTextLoading ? 'Gemini пишет...' : 'AI: текст + SEO'}</button>
                  </div>
                  <label className="editor-field">
                    <span>Фото URL {selectedIconImage + 1}</span>
                    <input value={editingImages[selectedIconImage] || ''} onChange={(event) => setIconImageAt(selectedIconImage, event.target.value)} placeholder="https://..." />
                  </label>
                  <div className="content-image-strip">
                    {Array.from({ length: Math.max(1, editingImages.length) }).map((_, index) => (
                      <button key={index} className={selectedIconImage === index ? 'active' : ''} type="button" onClick={() => setSelectedIconImage(index)}>
                        {editingImages[index] ? <img src={editingImages[index]} alt={`Фото ${index + 1}`} /> : <span>{index + 1}</span>}
                        <small>{iconImageLabel(editingImages[index], index)}</small>
                      </button>
                    ))}
                    {editingImages.length < ICON_IMAGE_MAX ? (
                      <button className="content-image-add" type="button" onClick={() => addIconImage()}>
                        <span>+</span>
                        <small>Фото</small>
                      </button>
                    ) : null}
                  </div>
                  <label className="editor-field">
                    <span>Промпт фото</span>
                    <textarea value={aiPrompt} onChange={(event) => setAiPrompt(event.target.value)} placeholder="Orthodox icon of ..., preserve exact iconographic composition, restore details, gold leaf, no text, no watermark" />
                  </label>
                  <AiReferenceUpload
                    title="Референсы макета"
                    hint="Фото 1: макет с рамкой. Фото 2: оригинальная икона, которую вставляем внутрь рамки."
                    images={iconReferenceImages}
                    busy={iconReferenceBusy}
                    onAdd={(files) => void addIconReferenceImages(files)}
                    onRemove={removeIconReference}
                  />
                  <div className="editor-actions">
                    <label className="btn btn-secondary">
                      <input className="visually-hidden" type="file" accept="image/*" multiple disabled={photoUploadLoading} onChange={(event) => void uploadPhotosFromPc(event.target.files)} />
                      {photoUploadLoading ? 'Загрузка...' : 'Загрузить с ПК'}
                    </label>
                    <select className="btn btn-quiet" value={iconPhotoGenerateCount} onChange={(event) => setIconPhotoGenerateCount(Number(event.target.value))}>
                      <option value={1}>1 фото</option>
                      <option value={2}>2 фото</option>
                      <option value={4}>4 фото</option>
                    </select>
                    <button className="btn btn-ai" type="button" onClick={() => void generateSelectedIconPhotoCount()} disabled={aiImageLoading || aiImagePlusLoading || aiSelectedImageLoading}><AppIcon name="bot" />{aiImageLoading || aiImagePlusLoading ? 'Gemini фото...' : 'Генерировать макет'}</button>
                    <select className="btn btn-quiet" value={selectedPhotoAspect} onChange={(event) => setSelectedPhotoAspect(event.target.value as IconPhotoAspect)} title="Размер перегенерации выбранного фото">
                      <option value="source">Как исходник</option>
                      <option value="square">1:1 квадрат</option>
                      <option value="landscape">4:3 горизонт</option>
                      <option value="portrait">3:4 вертикаль</option>
                      <option value="wide">16:9 широкий</option>
                    </select>
                    <button className="btn btn-secondary" type="button" onClick={() => void regenerateSelectedIconPhoto()} disabled={!editingImages[selectedIconImage] || isGeneratedQrImage(editingImages[selectedIconImage] || '') || aiImageLoading || aiImagePlusLoading || aiSelectedImageLoading}><AppIcon name="sparkles" />{aiSelectedImageLoading ? 'Перегенерация...' : 'Перегенерировать выбранное'}</button>
                    <button className="btn btn-quiet" type="button" disabled={!editingImages[selectedIconImage] || selectedIconImage === 0} onClick={() => moveIconImage(selectedIconImage, -1)}>Влево</button>
                    <button className="btn btn-quiet" type="button" disabled={!editingImages[selectedIconImage] || selectedIconImage >= editingImages.length - 1} onClick={() => moveIconImage(selectedIconImage, 1)}>Вправо</button>
                    <button className="btn btn-quiet" type="button" disabled={!editingImages[selectedIconImage] || selectedIconImage === 0} onClick={() => promoteIconImage(selectedIconImage)}>Сделать главной</button>
                    <button className="btn btn-quiet" type="button" disabled={!editingImages[selectedIconImage]} onClick={() => setFullscreenIconImage(selectedIconImage)}>На весь экран</button>
                    <button className="btn btn-quiet" type="button" disabled={!editingImages[selectedIconImage]} onClick={() => removeIconImage(selectedIconImage)}>Убрать</button>
                  </div>
                </div>
              </section>
            ) : null}
            {fullscreenIconImage !== null && supportsDirectPhoto() ? (
              <div className="content-fullscreen-viewer" onMouseDown={() => setFullscreenIconImage(null)}>
                <div className="content-fullscreen-head">
                  <button className="btn btn-quiet" type="button" onClick={(event) => { event.stopPropagation(); setFullscreenIconImage(Math.max(0, fullscreenIconImage - 1)); }}>Назад</button>
                  <strong>{fullscreenIconImage === 0 ? 'Оригинал / обложка' : `Икона ${fullscreenIconImage + 1}`}</strong>
                  <button className="btn btn-quiet" type="button" onClick={(event) => { event.stopPropagation(); setFullscreenIconImage(Math.min(editingImages.length - 1, fullscreenIconImage + 1)); }}>Вперед</button>
                  <button className="btn btn-primary" type="button" onClick={(event) => { event.stopPropagation(); setFullscreenIconImage(null); }}>Закрыть</button>
                </div>
                <div className="content-fullscreen-image" onMouseDown={(event) => event.stopPropagation()}>
                  {editingImages[fullscreenIconImage] ? <img src={editingImages[fullscreenIconImage]} alt={`Фото ${fullscreenIconImage + 1}`} /> : <span>Фото не выбрано</span>}
                </div>
              </div>
            ) : null}
          </section>
        </div>
      ) : null}
    </section>
  );
}
