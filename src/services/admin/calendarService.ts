import { isApiMode } from '../../config/adminConfig';
import {
  getIconsSiteContent,
  saveIconsSiteContent,
  type CalendarDay,
  type CalendarDayKind,
  type IconsSiteContent
} from '../../api/iconsSite';
import type { ActiveSiteId } from '../../components/admin/ActiveSiteContext';

export type CalendarDayPayload = Omit<CalendarDay, 'id'> & { id?: string };

const churchSeedDays: CalendarDay[] = [
  {
    id: 'church-calendar-06-29',
    month: '6',
    day: '29',
    gregorianDate: '2026-06-29',
    julianDay: '16',
    julianDate: '16 червня',
    label: 'День апостолів',
    note: 'Літургійна згадка та читання дня.',
    kind: 'feast',
    imageUrl: '',
    iconSlug: 'apostles',
    prayerSlug: 'daily-prayer',
    gospelSlug: 'daily-gospel',
    detailHref: '/calendar/apostles',
    current: true,
    feast: true,
    textOnly: false,
    description: 'Парафіяльний календар готовий до редагування подій, ікон та читань.'
  },
  {
    id: 'church-calendar-07-01',
    month: '7',
    day: '1',
    gregorianDate: '2026-07-01',
    julianDay: '18',
    julianDate: '18 червня',
    label: 'Молитва дня',
    note: 'Короткий молитовний блок для головної сторінки.',
    kind: 'prayer',
    imageUrl: '',
    iconSlug: '',
    prayerSlug: 'morning-prayer',
    gospelSlug: '',
    detailHref: '/prayers/morning-prayer',
    current: false,
    feast: false,
    textOnly: true,
    description: ''
  }
];

const genericSeedDays: CalendarDay[] = [];

export async function listCalendarDays(siteId: ActiveSiteId, year: number, month: number) {
  if (shouldUseChurchApi(siteId)) {
    const content = await getIconsSiteContent({ year, month });
    return filterDays(content.calendar?.days ?? [], month);
  }

  return readLocalDays(siteId, year, month);
}

export async function getCalendarDay(siteId: ActiveSiteId, year: number, month: number, id: string) {
  const days = await listCalendarDays(siteId, year, month);
  return days.find((day) => day.id === id) ?? null;
}

export async function saveCalendarDay(siteId: ActiveSiteId, year: number, month: number, payload: CalendarDayPayload) {
  const day = normalizeDay(payload, month);

  if (shouldUseChurchApi(siteId)) {
    const content = await getIconsSiteContent({ year, month });
    const currentDays = content.calendar?.days ?? [];
    const nextDays = upsertDay(currentDays, day);
    const nextContent: IconsSiteContent = {
      ...content,
      calendar: {
        ...content.calendar,
        days: nextDays
      }
    };
    const saved = await saveIconsSiteContent(nextContent);
    return filterDays(saved.calendar?.days ?? nextDays, month);
  }

  const nextDays = upsertDay(readLocalDays(siteId, year, month), day);
  writeLocalDays(siteId, year, month, nextDays);
  return nextDays;
}

export async function removeCalendarDay(siteId: ActiveSiteId, year: number, month: number, id: string) {
  if (shouldUseChurchApi(siteId)) {
    const content = await getIconsSiteContent({ year, month });
    const nextDays = (content.calendar?.days ?? []).filter((day) => day.id !== id);
    const nextContent: IconsSiteContent = {
      ...content,
      calendar: {
        ...content.calendar,
        days: nextDays
      }
    };
    const saved = await saveIconsSiteContent(nextContent);
    return filterDays(saved.calendar?.days ?? nextDays, month);
  }

  const nextDays = readLocalDays(siteId, year, month).filter((day) => day.id !== id);
  writeLocalDays(siteId, year, month, nextDays);
  return nextDays;
}

export function createBlankCalendarDay(month: number, day = '1'): CalendarDay {
  return normalizeDay({
    month: String(month),
    day,
    gregorianDate: '',
    julianDay: '',
    julianDate: '',
    label: '',
    note: '',
    kind: 'quiet',
    imageUrl: '',
    iconSlug: '',
    prayerSlug: '',
    gospelSlug: '',
    detailHref: '',
    current: false,
    feast: false,
    textOnly: false,
    description: ''
  }, month);
}

export const calendarKindOptions: CalendarDayKind[] = ['feast', 'fast', 'gospel', 'prayer', 'quiet'];

function shouldUseChurchApi(siteId: ActiveSiteId) {
  return isApiMode && siteId === 'church';
}

function filterDays(days: CalendarDay[], month: number) {
  return days
    .filter((day) => !day.month || Number(day.month) === month)
    .map((day) => normalizeDay(day, month))
    .sort((left, right) => Number(left.day) - Number(right.day));
}

function upsertDay(days: CalendarDay[], day: CalendarDay) {
  const exists = days.some((item) => item.id === day.id);
  const nextDays = exists ? days.map((item) => (item.id === day.id ? day : item)) : [...days, day];
  return nextDays.sort((left, right) => Number(left.month ?? 0) - Number(right.month ?? 0) || Number(left.day) - Number(right.day));
}

function normalizeDay(payload: CalendarDayPayload, month: number): CalendarDay {
  const dayNumber = normalizeNumberText(payload.day || '1');
  return {
    id: payload.id || `calendar-${month}-${dayNumber}-${Date.now()}`,
    month: normalizeNumberText(payload.month || String(month)),
    day: dayNumber,
    gregorianDate: payload.gregorianDate?.trim() ?? '',
    julianDay: payload.julianDay?.trim() ?? '',
    julianDate: payload.julianDate?.trim() ?? '',
    label: payload.label.trim(),
    note: payload.note.trim(),
    kind: payload.kind,
    imageUrl: payload.imageUrl.trim(),
    iconSlug: payload.iconSlug.trim(),
    prayerSlug: payload.prayerSlug.trim(),
    gospelSlug: payload.gospelSlug.trim(),
    detailHref: payload.detailHref.trim(),
    current: Boolean(payload.current),
    feast: Boolean(payload.feast),
    textOnly: Boolean(payload.textOnly),
    description: payload.description.trim()
  };
}

function normalizeNumberText(value: string) {
  return String(Number.parseInt(value, 10) || 1);
}

function readLocalDays(siteId: ActiveSiteId, year: number, month: number) {
  const stored = localStorage.getItem(storageKey(siteId, year, month));
  if (stored) {
    try {
      const parsed = JSON.parse(stored) as CalendarDay[];
      return filterDays(parsed, month);
    } catch {
      localStorage.removeItem(storageKey(siteId, year, month));
    }
  }

  const seed = siteId === 'church' ? churchSeedDays : genericSeedDays;
  const days = filterDays(seed, month);
  writeLocalDays(siteId, year, month, days);
  return days;
}

function writeLocalDays(siteId: ActiveSiteId, year: number, month: number, days: CalendarDay[]) {
  localStorage.setItem(storageKey(siteId, year, month), JSON.stringify(days));
}

function storageKey(siteId: ActiveSiteId, year: number, month: number) {
  return `admin_calendar_${siteId}_${year}_${month}`;
}
