import type {
  ChurchCalendarType,
  ChurchContentStatus,
  ChurchDayType,
  ChurchLanguage,
  ChurchPrayerType,
  ChurchCalendarDay,
  ChurchIcon,
  ChurchPrayer,
  ChurchSaint,
  ChurchArticle,
  ChurchGospel
} from '../../../api/churchContent';

export type DetailTab = 'overview' | 'saints' | 'icons' | 'prayers' | 'articles' | 'gospel' | 'publish';
export type CalendarViewMode = 'month' | 'week' | 'list';
export type CalendarDetailMode = 'idle' | 'create' | 'view' | 'edit';
export type WorkflowStepStatus = 'empty' | 'draft' | 'completed' | 'warning';
export type LanguageStatus = 'empty' | 'draft' | 'completed' | 'needs-translation';
export type CmsLanguage = ChurchLanguage;
export type CalendarDateBasis = 'new' | 'old';
export type IconPhotoRole = 'cover' | 'icon' | 'angleOne' | 'angleTwo';
export type IconExtraPhotoRole = Exclude<IconPhotoRole, 'cover'>;

export type WorkflowStepIcon = 'dashboard' | 'qr' | 'sparkles' | 'cms' | 'seo' | 'globe';

export type WorkflowStep = {
  id: DetailTab;
  index: string;
  label: string;
  status: WorkflowStepStatus;
  disabled: boolean;
  hint: string;
  icon: WorkflowStepIcon;
};

export type LanguageFieldStatus = Record<CmsLanguage, LanguageStatus>;

export type CalendarSlot = {
  key: string;
  date: string;
  label: string;
  weekday: string;
  day: ChurchCalendarDay | null;
  isToday: boolean;
  isSelected: boolean;
};

export type CalendarMonthSlot = {
  key: string;
  date: string;
  label: string;
  filled: number;
  total: number;
};

export type DrawerState = { type: 'ai' } | null;

export type ChurchDayContent = {
  saints: ChurchSaint[];
  icons: ChurchIcon[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  gospel: ChurchGospel[];
};

export const detailTabs: Array<{ id: DetailTab; label: string; icon: WorkflowStepIcon }> = [
  { id: 'overview', label: 'Святой / праздник', icon: 'dashboard' },
  { id: 'saints', label: 'Святой', icon: 'sparkles' },
  { id: 'icons', label: 'Икона', icon: 'qr' },
  { id: 'prayers', label: 'Молитвы', icon: 'sparkles' },
  { id: 'articles', label: 'Статья', icon: 'cms' },
  { id: 'gospel', label: 'Евангелие', icon: 'globe' },
  { id: 'publish', label: 'SEO и публикация', icon: 'seo' }
];

export const churchNavTabByKey: Record<string, DetailTab> = {
  'church-home': 'overview',
  'church-calendar': 'overview',
  'church-icons': 'icons',
  'church-prayers': 'prayers',
  'church-saints': 'saints',
  'church-gospel': 'gospel',
  'church-churches': 'articles',
  'church-qr': 'publish',
  'church-seo': 'publish'
};

export const dayTypes: ChurchDayType[] = ['feast', 'saint', 'fasting', 'memorial', 'gospel', 'quiet'];
export const statuses: ChurchContentStatus[] = ['draft', 'published', 'archived'];

export const dayTypeLabels: Record<ChurchDayType, string> = {
  feast: 'Праздник',
  saint: 'Святой',
  fasting: 'Постный день',
  memorial: 'Память',
  gospel: 'Евангелие',
  quiet: 'Обычный день'
};

export const calendarTypeLabels: Record<ChurchCalendarType, string> = {
  both: 'Оба стиля',
  old_style: 'Только старый стиль',
  new_style: 'Только новый стиль'
};

export const statusLabels: Record<ChurchContentStatus, string> = {
  draft: 'Черновик',
  published: 'Опубликовано',
  archived: 'Архив'
};

export const iconPhotoSlots: Array<{ role: IconPhotoRole; index: string; title: string; description: string }> = [
  { role: 'cover', index: '01', title: 'Основное фото / обложка', description: 'Идет на сайт как главное фото иконы.' },
  { role: 'icon', index: '02', title: 'Сама икона', description: 'Чистый фронтальный снимок иконы.' },
  { role: 'angleOne', index: '03', title: 'Ракурс иконы 1', description: 'Боковой или детальный ракурс.' },
  { role: 'angleTwo', index: '04', title: 'Ракурс иконы 2', description: 'Второй дополнительный ракурс.' }
];

export const languages: ChurchLanguage[] = ['uk', 'ru', 'en'];

export const cmsLanguages: Array<{ id: CmsLanguage; short: string; label: string }> = [
  { id: 'uk', short: 'UK', label: 'Українська' },
  { id: 'ru', short: 'RU', label: 'Русский' },
  { id: 'en', short: 'EN', label: 'English' }
];

export const prayerTypes: ChurchPrayerType[] = ['prayer', 'akathist', 'troparion', 'kontakion', 'velichanie', 'modern'];

export const prayerTypeLabels: Record<ChurchPrayerType, string> = {
  prayer: 'Каноническая молитва',
  akathist: 'Акафист',
  troparion: 'Тропарь',
  kontakion: 'Кондак',
  velichanie: 'Величание',
  modern: 'Современная молитва'
};

export const churchPublicUrl = (import.meta.env.VITE_CHURCH_PUBLIC_URL || 'https://ikona.link').replace(/\/+$/, '');
export const churchPreviewToken = import.meta.env.VITE_CHURCH_PREVIEW_TOKEN || '';

export const calendarViewModes: Array<{ value: CalendarViewMode; label: string }> = [
  { value: 'month', label: 'Месяц' },
  { value: 'week', label: 'Неделя' },
  { value: 'list', label: 'Список' }
];
