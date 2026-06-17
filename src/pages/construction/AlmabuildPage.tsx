import { useEffect, useMemo, useState } from 'react';
import { ExternalLink, Plus, RefreshCw, Save, Trash2 } from 'lucide-react';
import { generateAiImage } from '../../api/ai';
import {
  aiEditAlmabuildItem,
  almabuildSiteUrl,
  getAlmabuildContent,
  saveAlmabuildContent,
  type AlmabuildContent,
  type Kit,
  type MaterialCategory,
  type Product,
  type Project
} from '../../api/almabuild';
import { uploadCmsReference } from '../../api/cms';
import { AppIcon } from '../../components/AppIcon';
import type { AlmabuildSection } from '../../types/admin';

type AlmabuildLanguage = 'ru' | 'kk' | 'en';
type LocalizedStringKey = 'title' | 'text' | 'category' | 'spec' | 'meta' | 'seoTitle' | 'seoDescription' | 'pageTitle' | 'pageText';
type LocalizedListKey = 'bullets' | 'items';
const PROJECT_IMAGE_COUNT = 4;
const PROJECT_IMAGE_MAX = 12;
const PROJECT_REFERENCE_MAX = 4;

const almabuildLanguages: Array<{ key: AlmabuildLanguage; label: string; name: string }> = [
  { key: 'ru', label: 'RU', name: 'Русский' },
  { key: 'kk', label: 'KZ', name: 'Қазақша' },
  { key: 'en', label: 'EN', name: 'English' }
];

const emptyContent: AlmabuildContent = {
  materialCategories: [],
  products: [],
  kits: [],
  projects: []
};

function splitList(value: string) {
  return value.split('\n').map((item) => item.trim()).filter(Boolean);
}

function joinList(value: string[]) {
  return value.join('\n');
}

const slugTranslit: Record<string, string> = {
  а: 'a', ә: 'a', б: 'b', в: 'v', г: 'g', ғ: 'g', д: 'd', е: 'e', ё: 'e', ж: 'zh', з: 'z',
  и: 'i', й: 'y', к: 'k', қ: 'k', л: 'l', м: 'm', н: 'n', ң: 'n', о: 'o', ө: 'o', п: 'p',
  р: 'r', с: 's', т: 't', у: 'u', ұ: 'u', ү: 'u', ф: 'f', х: 'h', һ: 'h', ц: 'ts', ч: 'ch',
  ш: 'sh', щ: 'sch', ы: 'y', і: 'i', э: 'e', ю: 'yu', я: 'ya', ь: '', ъ: ''
};

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .split('')
    .map((char) => slugTranslit[char] ?? char)
    .join('')
    .replace(/[^a-z0-9]+/gi, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 88);
}

function langSuffix(lang: AlmabuildLanguage) {
  if (lang === 'kk') return 'Kk';
  if (lang === 'en') return 'En';
  return 'Ru';
}

function localizedString<T extends Record<string, unknown>>(item: T, key: LocalizedStringKey, lang: AlmabuildLanguage): string {
  const localized = item[`${key}${langSuffix(lang)}`] as string | undefined;
  if (localized) return localized;
  return lang === 'ru' ? String(item[key] || '') : '';
}

function localizedList<T extends Record<string, unknown>>(item: T, key: LocalizedListKey, lang: AlmabuildLanguage): string[] {
  const localized = item[`${key}${langSuffix(lang)}`] as string[] | undefined;
  if (Array.isArray(localized) && localized.length) return localized;
  return lang === 'ru' && Array.isArray(item[key]) ? item[key] as string[] : [];
}

function patchLocalizedString<T extends Record<string, unknown>>(item: T, key: LocalizedStringKey, lang: AlmabuildLanguage, value: string): Partial<T> {
  const patch = { [`${key}${langSuffix(lang)}`]: value } as Partial<T>;
  if (lang === 'ru') return { ...patch, [key]: value } as Partial<T>;
  return patch;
}

function patchLocalizedList<T extends Record<string, unknown>>(item: T, key: LocalizedListKey, lang: AlmabuildLanguage, value: string[]): Partial<T> {
  const patch = { [`${key}${langSuffix(lang)}`]: value } as Partial<T>;
  if (lang === 'ru') return { ...patch, [key]: value } as Partial<T>;
  return patch;
}

function categoryTemplate(index: number): MaterialCategory {
  return {
    index: '',
    slug: '',
    title: '',
    text: '',
    bullets: [],
    photo: ''
  };
}

function productTemplate(categorySlug = ''): Product {
  return {
    categorySlug,
    category: '',
    title: '',
    spec: '',
    photo: ''
  };
}

function kitTemplate(): Kit {
  return {
    title: '',
    text: '',
    items: []
  };
}

function projectTemplate(): Project {
  return {
    slug: '',
    title: '',
    meta: '',
    seoTitle: '',
    seoDescription: '',
    pageTitle: '',
    pageText: '',
    photo: ''
  };
}

function isImageUrl(value?: string) {
  return /^https?:\/\//i.test(String(value || '').trim());
}

function projectImageSeries(project?: Project): string[] {
  const unique = [project?.photo || '', ...(project?.imageUrls || [])]
    .map((url) => url.trim())
    .filter(Boolean)
    .filter((url) => isImageUrl(url))
    .filter((url, index, list) => list.indexOf(url) === index);
  const targetCount = Math.max(PROJECT_IMAGE_COUNT, Math.min(PROJECT_IMAGE_MAX, unique.length));
  return unique.slice(0, PROJECT_IMAGE_MAX).concat(Array(targetCount).fill('')).slice(0, targetCount);
}

function projectWithImages(project: Project, images: string[]): Project {
  const cleanImages = images.map((url) => url.trim()).filter(Boolean);
  const visualClass = isImageUrl(project.photo) ? '' : project.photo || '';
  return {
    ...project,
    photo: cleanImages[0] || visualClass,
    imageUrls: cleanImages
  };
}

function LanguageTabs({ active, onChange }: { active: AlmabuildLanguage; onChange: (lang: AlmabuildLanguage) => void }) {
  return (
    <div className="almabuild-language-tabs" aria-label="Язык редактирования">
      <span>Язык редактирования</span>
      <div>
        {almabuildLanguages.map((lang) => (
          <button key={lang.key} className={active === lang.key ? 'active' : ''} type="button" onClick={() => onChange(lang.key)}>
            {lang.label}
            <small>{lang.name}</small>
          </button>
        ))}
      </div>
    </div>
  );
}

function Field({
  label,
  help,
  children,
  wide = false
}: {
  label: string;
  help: string;
  children: React.ReactNode;
  wide?: boolean;
}) {
  return (
    <label className={'almabuild-field' + (wide ? ' wide' : '')}>
      <span>{label}</span>
      {children}
      <small>{help}</small>
    </label>
  );
}

function StaticSectionNotice({ title, text }: { title: string; text: string }) {
  return (
    <article className="almabuild-panel">
      <div className="section-head">
        <div>
          <span className="eyebrow">Раздел сайта</span>
          <h2>{title}</h2>
          <p className="section-note">{text}</p>
        </div>
      </div>
      <div className="site-preview">
        <span>Следующий шаг</span>
        <strong>Подключить этот блок к backend-контенту</strong>
        <p>Сейчас этот раздел хранится в коде публичного сайта Kazaxbud. Материалы, каталог, проекты и смета уже редактируются из CMS.</p>
      </div>
    </article>
  );
}

export function AlmabuildPage({ activeSection }: { activeSection: AlmabuildSection }) {
  const [content, setContent] = useState<AlmabuildContent>(emptyContent);
  const [activeLang, setActiveLang] = useState<AlmabuildLanguage>('ru');
  const [projectEditorOpen, setProjectEditorOpen] = useState(false);
  const [projectDraft, setProjectDraft] = useState<Project>(() => projectTemplate());
  const [projectImages, setProjectImages] = useState<string[]>(() => projectImageSeries());
  const [selectedProjectImage, setSelectedProjectImage] = useState(0);
  const [fullscreenProjectImage, setFullscreenProjectImage] = useState<number | null>(null);
  const [editingProjectIndex, setEditingProjectIndex] = useState<number | null>(null);
  const [projectAiTopic, setProjectAiTopic] = useState('');
  const [projectPhotoPrompt, setProjectPhotoPrompt] = useState('');
  const [projectReferenceUrls, setProjectReferenceUrls] = useState<string[]>([]);
  const [projectReferenceUrlInput, setProjectReferenceUrlInput] = useState('');
  const [projectReferenceBusy, setProjectReferenceBusy] = useState(false);
  const [projectReferenceBusyLabel, setProjectReferenceBusyLabel] = useState('');
  const [projectBusy, setProjectBusy] = useState(false);
  const [projectBusyLabel, setProjectBusyLabel] = useState('');
  const [projectImageBusy, setProjectImageBusy] = useState(false);
  const [projectImageBusyLabel, setProjectImageBusyLabel] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const stats = useMemo(() => [
    { label: 'Категории', value: String(content.materialCategories.length), note: 'Блок «Материалы» + фильтр каталога' },
    { label: 'Товары', value: String(content.products.length), note: 'Карточки и страница каталога' },
    { label: 'Комплекты', value: String(content.kits.length), note: 'Блок готовых наборов' },
    { label: 'Проекты', value: String(content.projects.length), note: 'Кейсы на главной' }
  ], [content]);

  async function loadContent() {
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      setContent(await getAlmabuildContent());
      setMessage('Контент ALMABUILD загружен из backend');
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : 'Не удалось загрузить ALMABUILD');
    } finally {
      setLoading(false);
    }
  }

  async function publishContent() {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      setContent(await saveAlmabuildContent(content));
      setMessage('Опубликовано. Сайт kazaxbud читает эти данные через backend.');
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Не удалось сохранить ALMABUILD');
    } finally {
      setSaving(false);
    }
  }

  useEffect(() => {
    void loadContent();
  }, []);

  useEffect(() => {
    if (activeSection !== 'projects') setProjectEditorOpen(false);
  }, [activeSection]);

  function updateCategory(index: number, patch: Partial<MaterialCategory>) {
    setContent((current) => ({
      ...current,
      materialCategories: current.materialCategories.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function updateProduct(index: number, patch: Partial<Product>) {
    setContent((current) => ({
      ...current,
      products: current.products.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function updateKit(index: number, patch: Partial<Kit>) {
    setContent((current) => ({
      ...current,
      kits: current.kits.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function openNewProject() {
    const draft = projectTemplate();
    setProjectDraft(draft);
    setProjectImages(projectImageSeries(draft));
    setSelectedProjectImage(0);
    setFullscreenProjectImage(null);
    setEditingProjectIndex(null);
    setProjectAiTopic('');
    setProjectPhotoPrompt('Real commercial renovation project photo, finished interior, construction materials, architectural editorial lighting, no people');
    setProjectReferenceUrls([]);
    setProjectReferenceUrlInput('');
    setProjectEditorOpen(true);
    setMessage(null);
  }

  function openEditProject(index: number) {
    const draft = { ...content.projects[index] };
    setProjectDraft(draft);
    setProjectImages(projectImageSeries(draft));
    setSelectedProjectImage(0);
    setFullscreenProjectImage(null);
    setEditingProjectIndex(index);
    setProjectAiTopic(localizedString(draft, 'title', activeLang) || draft.title);
    setProjectPhotoPrompt(`Commercial construction case photo: ${localizedString(draft, 'title', activeLang) || draft.title}. ${localizedString(draft, 'meta', activeLang) || draft.meta}. Finished interior, realistic architectural lighting, no people`);
    setProjectReferenceUrls([]);
    setProjectReferenceUrlInput('');
    setProjectEditorOpen(true);
    setMessage(null);
  }

  function patchProjectDraft(patch: Partial<Project>) {
    setProjectDraft((current) => ({ ...current, ...patch }));
  }

  function setProjectImageAt(index: number, url: string) {
    setProjectImages((current) => {
      const next = current.map((item, itemIndex) => itemIndex === index ? url : item);
      setProjectDraft((draft) => projectWithImages(draft, next));
      return next;
    });
  }

  function addProjectImageSlot() {
    setProjectImages((current) => {
      if (current.length >= PROJECT_IMAGE_MAX) return current;
      setSelectedProjectImage(current.length);
      return [...current, ''];
    });
  }

  function removeSelectedProjectImage() {
    setProjectImages((current) => {
      if (current.length <= PROJECT_IMAGE_COUNT) {
        const next = current.map((item, index) => index === selectedProjectImage ? '' : item);
        setProjectDraft((draft) => projectWithImages(draft, next));
        return next;
      }
      const next = current.filter((_, index) => index !== selectedProjectImage);
      setSelectedProjectImage(Math.min(selectedProjectImage, next.length - 1));
      setProjectDraft((draft) => projectWithImages(draft, next));
      return next;
    });
  }

  function saveProjectDraft() {
    const projectTitle = localizedString(projectDraft, 'title', activeLang) || projectDraft.title || projectAiTopic;
    const nextProject = projectWithImages({
      ...projectDraft,
      slug: projectDraft.slug || slugify(projectTitle),
      seoTitle: projectDraft.seoTitle || localizedString(projectDraft, 'seoTitle', 'ru') || `${projectTitle} | ALMABUILD PRO`,
      seoDescription: projectDraft.seoDescription || localizedString(projectDraft, 'seoDescription', 'ru') || localizedString(projectDraft, 'meta', 'ru') || projectDraft.meta,
      pageTitle: projectDraft.pageTitle || localizedString(projectDraft, 'pageTitle', 'ru') || projectTitle,
      pageText: projectDraft.pageText || localizedString(projectDraft, 'pageText', 'ru') || localizedString(projectDraft, 'meta', 'ru') || projectDraft.meta
    }, projectImages);
    setProjectDraft(nextProject);
    setContent((current) => {
      if (editingProjectIndex === null) {
        return { ...current, projects: [nextProject, ...current.projects] };
      }
      return {
        ...current,
        projects: current.projects.map((item, index) => index === editingProjectIndex ? nextProject : item)
      };
    });
    setProjectEditorOpen(false);
    setMessage('Проект сохранен в черновике CMS. Нажми «Опубликовать», чтобы отправить изменения в backend.');
  }

  function deleteProjectDraft() {
    if (editingProjectIndex === null) {
      setProjectEditorOpen(false);
      return;
    }
    setContent((current) => ({
      ...current,
      projects: current.projects.filter((_, index) => index !== editingProjectIndex)
    }));
    setProjectEditorOpen(false);
    setMessage('Проект удален из черновика CMS. Нажми «Опубликовать», чтобы сохранить удаление в backend.');
  }

  async function generateProjectDraft() {
    const instruction = projectAiTopic.trim() || localizedString(projectDraft, 'title', activeLang) || projectDraft.title;
    if (!instruction) {
      setMessage('Напиши тему для AI проекта.');
      return;
    }
    setProjectBusy(true);
    setProjectBusyLabel('Gemini пишет...');
    setMessage(null);
    try {
      const result = await aiEditAlmabuildItem<Project>('project', [
        instruction,
        '',
        'Создай полноценную SEO-карточку проекта для строительного сайта в Алматы.',
        'Обязательно заполни: slug латиницей через дефис, seoTitle, seoDescription, pageTitle, pageText.',
        'Обязательно переведи и заполни ВСЕ языки: titleRu/titleKk/titleEn, metaRu/metaKk/metaEn, seoTitleRu/seoTitleKk/seoTitleEn, seoDescriptionRu/seoDescriptionKk/seoDescriptionEn, pageTitleRu/pageTitleKk/pageTitleEn, pageTextRu/pageTextKk/pageTextEn.',
        'RU пиши по-русски, KZ пиши на казахском языке, EN пиши по-английски. Не оставляй KZ/EN русским текстом.',
        'pageText сделай 2-4 абзаца: задача объекта, материалы/работы, сроки/контроль, выгода для клиента.',
        'title/meta оставь короткими для карточки на главной.'
      ].join('\n'), projectDraft);
      setProjectDraft((current) => ({
        ...current,
        ...result,
        slug: result.slug || current.slug || slugify(localizedString(result, 'title', activeLang) || result.title || instruction),
        ...patchLocalizedString(result, 'title', activeLang, localizedString(result, 'title', activeLang) || result.title || localizedString(current, 'title', activeLang)),
        ...patchLocalizedString(result, 'meta', activeLang, localizedString(result, 'meta', activeLang) || result.meta || localizedString(current, 'meta', activeLang)),
        ...patchLocalizedString(result, 'seoTitle', activeLang, localizedString(result, 'seoTitle', activeLang) || result.seoTitle || localizedString(current, 'seoTitle', activeLang)),
        ...patchLocalizedString(result, 'seoDescription', activeLang, localizedString(result, 'seoDescription', activeLang) || result.seoDescription || localizedString(current, 'seoDescription', activeLang)),
        ...patchLocalizedString(result, 'pageTitle', activeLang, localizedString(result, 'pageTitle', activeLang) || result.pageTitle || localizedString(current, 'pageTitle', activeLang)),
        ...patchLocalizedString(result, 'pageText', activeLang, localizedString(result, 'pageText', activeLang) || result.pageText || localizedString(current, 'pageText', activeLang))
      }));
      setProjectPhotoPrompt(`Commercial construction case photo: ${result.title || instruction}. ${result.meta || ''}. Finished interior, realistic architectural lighting, no people`);
      setMessage('Gemini подготовил текст проекта. Проверь и нажми «Сохранить».');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Gemini не создал проект.');
    } finally {
      setProjectBusy(false);
      setProjectBusyLabel('');
    }
  }

  async function uploadProjectPhoto(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setMessage('Файл должен быть изображением.');
      return;
    }
    setProjectImageBusy(true);
    setProjectImageBusyLabel('Загружаем фото...');
    setMessage(null);
    try {
      const url = await uploadCmsReference(file);
      setProjectImageAt(selectedProjectImage, url);
      setMessage('Фото загружено в R2 и добавлено в проект.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото проекта.');
    } finally {
      setProjectImageBusy(false);
      setProjectImageBusyLabel('');
    }
  }

  async function addProjectReferenceFiles(files: FileList | null) {
    if (!files) return;
    const slots = PROJECT_REFERENCE_MAX - projectReferenceUrls.length;
    const selected = Array.from(files).filter((file) => file.type.startsWith('image/')).slice(0, slots);
    if (!selected.length) {
      setMessage(`Можно добавить максимум ${PROJECT_REFERENCE_MAX} референса.`);
      return;
    }
    setProjectReferenceBusy(true);
    setProjectReferenceBusyLabel('Загружаем референсы...');
    setMessage(null);
    try {
      const uploaded: string[] = [];
      for (const file of selected) {
        if (file.size > 10 * 1024 * 1024) throw new Error('Каждый референс должен быть меньше 10 MB');
        uploaded.push(await uploadCmsReference(file));
      }
      setProjectReferenceUrls((current) => [...current, ...uploaded].slice(0, PROJECT_REFERENCE_MAX));
      setMessage(`Добавлено референсов: ${uploaded.length}. Gemini будет учитывать их при генерации.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить референсы проекта.');
    } finally {
      setProjectReferenceBusy(false);
      setProjectReferenceBusyLabel('');
    }
  }

  function addProjectReferenceUrl() {
    const url = projectReferenceUrlInput.trim();
    if (!url) return;
    if (!/^https?:\/\//i.test(url)) {
      setMessage('URL референса должен начинаться с http:// или https://');
      return;
    }
    setProjectReferenceUrls((current) => {
      if (current.includes(url)) return current;
      return [...current, url].slice(0, PROJECT_REFERENCE_MAX);
    });
    setProjectReferenceUrlInput('');
  }

  function removeProjectReferenceUrl(index: number) {
    setProjectReferenceUrls((current) => current.filter((_, itemIndex) => itemIndex !== index));
  }

  async function generateProjectPhoto() {
    const title = localizedString(projectDraft, 'title', activeLang) || projectDraft.title || projectAiTopic;
    if (!title.trim()) {
      setMessage('Нужна тема или название проекта для генерации фото.');
      return;
    }
    setProjectImageBusy(true);
    setProjectImageBusyLabel('Gemini фото...');
    setMessage(null);
    try {
      const result = await generateAiImage({
        site: 'construction',
        title,
        description: localizedString(projectDraft, 'meta', activeLang) || projectDraft.meta,
        scene: projectPhotoPrompt || `Commercial construction project photo: ${title}, finished interior, realistic architectural lighting, no people`,
        imageType: 'construction',
        referenceUrls: projectReferenceUrls,
        variant: selectedProjectImage
      });
      if (!result.imageUrl) throw new Error('Backend не вернул URL фото.');
      setProjectImageAt(selectedProjectImage, result.imageUrl);
      setMessage(`AI фото проекта ${selectedProjectImage + 1} создано.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не создал фото проекта.');
    } finally {
      setProjectImageBusy(false);
      setProjectImageBusyLabel('');
    }
  }

  async function generateProjectPhotoSeries() {
    const title = localizedString(projectDraft, 'title', activeLang) || projectDraft.title || projectAiTopic;
    if (!title.trim()) {
      setMessage('Нужна тема или название проекта для генерации фото.');
      return;
    }
    setProjectImageBusy(true);
    setProjectImageBusyLabel(`Gemini 1/${PROJECT_IMAGE_COUNT}...`);
    setMessage(null);
    try {
      const next: string[] = [];
      for (let index = 0; index < PROJECT_IMAGE_COUNT; index += 1) {
        const scene = index === 0
          ? projectPhotoPrompt || `Commercial construction case hero photo: ${title}, finished interior, realistic architectural lighting, no people`
          : `${projectPhotoPrompt || title}. Gallery variant ${index + 1}: different angle, material details, lighting and fit-out quality, no people`;
        const result = await generateAiImage({
          site: 'construction',
          title,
          description: localizedString(projectDraft, 'meta', activeLang) || projectDraft.meta,
          scene,
          imageType: 'construction',
          referenceUrls: projectReferenceUrls,
          variant: index
        });
        if (!result.imageUrl) throw new Error('Backend не вернул URL фото.');
        next.push(result.imageUrl);
        setProjectImageBusyLabel(`Gemini ${Math.min(index + 2, PROJECT_IMAGE_COUNT)}/${PROJECT_IMAGE_COUNT}...`);
        setProjectImages([...next, ...Array(PROJECT_IMAGE_COUNT - next.length).fill('')]);
      }
      setProjectDraft((draft) => projectWithImages(draft, next));
      setSelectedProjectImage(0);
      setMessage(projectReferenceUrls.length ? 'AI создал 4 фото по референсам.' : 'AI создал 4 фото без визуальных референсов.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не создал серию фото проекта.');
    } finally {
      setProjectImageBusy(false);
      setProjectImageBusyLabel('');
    }
  }

  async function generateAdditionalProjectPhoto() {
    if (projectImages.length >= PROJECT_IMAGE_MAX) {
      setMessage(`Максимум ${PROJECT_IMAGE_MAX} фото в галерее проекта.`);
      return;
    }
    const title = localizedString(projectDraft, 'title', activeLang) || projectDraft.title || projectAiTopic;
    if (!title.trim()) {
      setMessage('Нужна тема или название проекта для генерации фото.');
      return;
    }
    const index = projectImages.length;
    setProjectImages((current) => [...current, '']);
    setSelectedProjectImage(index);
    setProjectImageBusy(true);
    setProjectImageBusyLabel(`Gemini + фото ${index + 1}...`);
    setMessage(null);
    try {
      const result = await generateAiImage({
        site: 'construction',
        title,
        description: localizedString(projectDraft, 'meta', activeLang) || projectDraft.meta,
        scene: `${projectPhotoPrompt || title}. Additional gallery photo ${index + 1}, finished commercial renovation, construction details, no people`,
        imageType: 'construction',
        referenceUrls: projectReferenceUrls,
        variant: index
      });
      if (!result.imageUrl) throw new Error('Backend не вернул URL фото.');
      setProjectImageAt(index, result.imageUrl);
      setMessage(`AI добавил фото проекта ${index + 1}.`);
    } catch (error) {
      setProjectImages((current) => current.filter((_, itemIndex) => itemIndex !== index));
      setSelectedProjectImage(Math.max(0, index - 1));
      setMessage(error instanceof Error ? error.message : 'AI не создал дополнительное фото проекта.');
    } finally {
      setProjectImageBusy(false);
      setProjectImageBusyLabel('');
    }
  }

  return (
    <section className="almabuild-page">
      <header className="almabuild-hero" id="almabuild-overview">
        <div>
          <span className="eyebrow">ALMABUILD PRO CMS</span>
          <h2>Управление сайтом kazaxbud</h2>
          <p>Каждый блок ниже подписан так же, как он выглядит на сайте. Изменения сохраняются в backend и появляются на публичных страницах после публикации.</p>
        </div>
        <div className="almabuild-actions">
          <a className="btn btn-quiet" href={almabuildSiteUrl} target="_blank" rel="noreferrer"><ExternalLink size={16} />Открыть сайт</a>
          <button className="btn btn-quiet" type="button" onClick={loadContent} disabled={loading || saving}><RefreshCw size={16} />Обновить</button>
          <button className="btn btn-primary" type="button" onClick={publishContent} disabled={loading || saving}><Save size={16} />{saving ? 'Публикуем...' : 'Опубликовать'}</button>
        </div>
      </header>

      {message && <p className="almabuild-alert">{message}</p>}
      {error && <p className="almabuild-alert error">{error}</p>}

      <LanguageTabs active={activeLang} onChange={setActiveLang} />

      <div className="almabuild-section-workspace">
          <div className="metrics-grid">
            {stats.map((item) => (
              <article className="metric-card" key={item.label}>
                <span className="metric-label">{item.label}</span>
                <strong className="metric-value">{item.value}</strong>
                <p className="metric-note">{item.note}</p>
              </article>
            ))}
          </div>

      {activeSection === 'services' ? (
        <StaticSectionNotice title="Услуги" text="Верхний раздел сайта: услуги, подход и оффер. Пока это статический блок публичного сайта, вынесли его в боковую панель, чтобы структура редактирования совпадала с сайтом." />
      ) : null}

      {activeSection === 'materials' ? <article className="almabuild-panel" id="almabuild-categories">
        <div className="section-head">
          <div>
            <span className="eyebrow">Блок сайта: «Материалы для коммерческой отделки»</span>
            <h2>Категории материалов</h2>
            <p className="section-note">Эти карточки видны на главной в блоке «Материалы» и как фильтры на странице каталога.</p>
          </div>
          <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, materialCategories: [...current.materialCategories, categoryTemplate(current.materialCategories.length)] }))}><Plus size={16} />Добавить категорию</button>
        </div>
        <div className="almabuild-card-list">
          {content.materialCategories.map((category, index) => (
            <article className="almabuild-edit-card" key={category.slug + '-' + index}>
              <div className="edit-card-head">
                <div>
                  <span>Категория на сайте #{index + 1}</span>
                  <h3>{localizedString(category, 'title', activeLang) || category.title || 'Без названия'}</h3>
                </div>
                <button className="icon-danger" type="button" aria-label="Удалить категорию" onClick={() => setContent((current) => ({ ...current, materialCategories: current.materialCategories.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button>
              </div>
              <div className="almabuild-form-grid categories">
                <Field label={`Название карточки ${activeLang.toUpperCase()}`} help="Крупный заголовок в блоке «Материалы» и название фильтра в каталоге."><input value={localizedString(category, 'title', activeLang)} placeholder={category.title} onChange={(event) => updateCategory(index, patchLocalizedString(category, 'title', activeLang, event.target.value))} /></Field>
                <Field label="URL slug" help="Адрес категории: /catalog/slug. Лучше латиница без пробелов."><input value={category.slug} onChange={(event) => updateCategory(index, { slug: event.target.value })} /></Field>
                <Field label="Номер" help="Маленький индекс на карточке, например [0:1]."><input value={category.index} onChange={(event) => updateCategory(index, { index: event.target.value })} /></Field>
                <Field label="Визуальный класс" help="Технический ключ фоновой картинки/стиля."><input value={category.photo} onChange={(event) => updateCategory(index, { photo: event.target.value })} /></Field>
                <Field label={`Описание карточки ${activeLang.toUpperCase()}`} help="Текст под названием категории на главной." wide><textarea value={localizedString(category, 'text', activeLang)} placeholder={category.text} onChange={(event) => updateCategory(index, patchLocalizedString(category, 'text', activeLang, event.target.value))} /></Field>
                <Field label={`Список внутри категории ${activeLang.toUpperCase()}`} help="Показывается в деталях/админке: каждый пункт с новой строки." wide><textarea value={joinList(localizedList(category, 'bullets', activeLang))} placeholder={joinList(category.bullets)} onChange={(event) => updateCategory(index, patchLocalizedList(category, 'bullets', activeLang, splitList(event.target.value)))} /></Field>
              </div>
              <div className="site-preview">
                <span>Как это читается на сайте</span>
                <strong>{category.index} · {localizedString(category, 'title', activeLang) || category.title}</strong>
                <p>{localizedString(category, 'text', activeLang) || category.text}</p>
              </div>
            </article>
          ))}
        </div>
      </article> : null}

      {activeSection === 'catalog' ? <article className="almabuild-panel" id="almabuild-products">
        <div className="section-head">
          <div>
            <span className="eyebrow">Блок сайта: «Сопутствующие товары» + страница каталога</span>
            <h2>Товары и материалы</h2>
            <p className="section-note">Эти позиции отображаются карточками на главной и в каталоге. Категория связывает товар с фильтром.</p>
          </div>
          <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, products: [productTemplate(current.materialCategories[0]?.slug), ...current.products] }))}><Plus size={16} />Добавить товар</button>
        </div>
        <div className="almabuild-card-list compact">
          {content.products.map((product, index) => (
            <article className="almabuild-edit-card product" key={product.title + '-' + index}>
              <div className="edit-card-head">
                <div><span>Карточка товара #{index + 1}</span><h3>{localizedString(product, 'title', activeLang) || product.title || 'Без названия'}</h3></div>
                <button className="icon-danger" type="button" aria-label="Удалить товар" onClick={() => setContent((current) => ({ ...current, products: current.products.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button>
              </div>
              <div className="almabuild-form-grid products">
                <Field label={`Название товара ${activeLang.toUpperCase()}`} help="Главный текст на карточке товара."><input value={localizedString(product, 'title', activeLang)} placeholder={product.title} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'title', activeLang, event.target.value))} /></Field>
                <Field label={`Метка категории ${activeLang.toUpperCase()}`} help="Короткая подпись над названием: ГКЛ, Профили, Свет."><input value={localizedString(product, 'category', activeLang)} placeholder={product.category} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'category', activeLang, event.target.value))} /></Field>
                <Field label="Раздел каталога" help="Определяет, в каком фильтре каталога появится товар."><select value={product.categorySlug} onChange={(event) => updateProduct(index, { categorySlug: event.target.value })}>{content.materialCategories.map((category) => <option key={category.slug} value={category.slug}>{category.title}</option>)}</select></Field>
                <Field label={`Характеристики ${activeLang.toUpperCase()}`} help="Краткий размер, класс или назначение."><input value={localizedString(product, 'spec', activeLang)} placeholder={product.spec} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'spec', activeLang, event.target.value))} /></Field>
                <Field label="Визуальный класс" help="Технический ключ оформления карточки."><input value={product.photo} onChange={(event) => updateProduct(index, { photo: event.target.value })} /></Field>
              </div>
              <div className="site-preview small"><span>Карточка</span><strong>{localizedString(product, 'category', activeLang) || product.category}</strong><p>{localizedString(product, 'title', activeLang) || product.title} · {localizedString(product, 'spec', activeLang) || product.spec}</p></div>
            </article>
          ))}
        </div>
      </article> : null}

      {activeSection === 'estimate' ? (
        <article className="almabuild-panel" id="almabuild-kits">
          <div className="section-head">
            <div>
              <span className="eyebrow">Блок сайта: «Готовые наборы под объект»</span>
              <h2>Комплекты</h2>
              <p className="section-note">Наборы материалов, которые можно быстро добавить в смету.</p>
            </div>
            <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, kits: [kitTemplate(), ...current.kits] }))}><Plus size={16} />Добавить</button>
          </div>
          <div className="almabuild-card-list">
            {content.kits.map((kit, index) => (
              <article className="almabuild-edit-card" key={kit.title + '-' + index}>
                <div className="edit-card-head"><div><span>Комплект #{index + 1}</span><h3>{localizedString(kit, 'title', activeLang) || kit.title || 'Без названия'}</h3></div><button className="icon-danger" type="button" aria-label="Удалить комплект" onClick={() => setContent((current) => ({ ...current, kits: current.kits.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button></div>
                <div className="almabuild-form-grid one">
                  <Field label={`Название комплекта ${activeLang.toUpperCase()}`} help="Заголовок карточки в блоке комплектов."><input value={localizedString(kit, 'title', activeLang)} placeholder={kit.title} onChange={(event) => updateKit(index, patchLocalizedString(kit, 'title', activeLang, event.target.value))} /></Field>
                  <Field label={`Описание ${activeLang.toUpperCase()}`} help="Одна строка под названием комплекта."><textarea value={localizedString(kit, 'text', activeLang)} placeholder={kit.text} onChange={(event) => updateKit(index, patchLocalizedString(kit, 'text', activeLang, event.target.value))} /></Field>
                  <Field label={`Состав комплекта ${activeLang.toUpperCase()}`} help="Каждый пункт с новой строки, показывается списком."><textarea value={joinList(localizedList(kit, 'items', activeLang))} placeholder={joinList(kit.items)} onChange={(event) => updateKit(index, patchLocalizedList(kit, 'items', activeLang, splitList(event.target.value)))} /></Field>
                </div>
              </article>
            ))}
          </div>
        </article>
      ) : null}

      {activeSection === 'projects' ? (
        <article className="almabuild-panel" id="almabuild-projects">
          <div className="section-head">
            <div>
              <span className="eyebrow">Блок сайта: «Коммерческие пространства»</span>
              <h2>Проекты</h2>
              <p className="section-note">Кейсы на главной: название объекта, формат, площадь и сроки. Редактирование открывается отдельно, как в блогах кулинарного сайта.</p>
            </div>
            <button className="btn btn-primary" type="button" onClick={openNewProject}><Plus size={16} />Новый проект</button>
          </div>
          <section className="ops-panel almabuild-project-list">
            <table className="ops-table">
              <thead>
                <tr><th>Проект</th><th>Описание</th><th>Визуал</th><th>Языки</th><th /></tr>
              </thead>
              <tbody>
                {content.projects.map((project, index) => (
                  <tr key={`${project.title}-${index}`}>
                    <td><strong>{localizedString(project, 'title', activeLang) || project.title || 'Без названия'}</strong><small>Проект #{index + 1}</small></td>
                    <td>{localizedString(project, 'meta', activeLang) || project.meta || 'Нет описания'}</td>
                    <td>{isImageUrl(project.photo) ? <img className="catalog-product-thumb" src={project.photo} alt={project.title || 'Проект'} loading="lazy" /> : <code>{project.photo || 'photo-project'}</code>}</td>
                    <td>{['ru', 'kk', 'en'].filter((lang) => localizedString(project, 'title', lang as AlmabuildLanguage)).map((lang) => lang.toUpperCase()).join(' / ') || 'RU'}</td>
                    <td><button className="table-action" type="button" onClick={() => openEditProject(index)}>Редактировать</button></td>
                  </tr>
                ))}
                {!content.projects.length ? <tr><td colSpan={5}>Проектов пока нет. Нажми «Новый проект», чтобы добавить первый кейс.</td></tr> : null}
              </tbody>
            </table>
          </section>
        </article>
      ) : null}

      {activeSection === 'contact' ? (
        <StaticSectionNotice title="Контакты" text="Форма заявки уже отправляет лиды в backend. Тексты контактов и телефон пока статические на публичном сайте; теперь у раздела есть отдельное место в редакторе." />
      ) : null}
      </div>

      {projectEditorOpen ? (
        <div className="modal-overlay">
          <div className="editor-modal almabuild-project-modal">
            <div className="editor-modal-head">
              <div>
                <p className="eyebrow">{editingProjectIndex === null ? 'Новый проект' : 'Редактирование проекта'}</p>
                <h2>{localizedString(projectDraft, 'title', activeLang) || projectDraft.title || 'Коммерческий проект'}</h2>
              </div>
              <div className="editor-actions">
                <button className="btn btn-quiet" type="button" onClick={() => setProjectEditorOpen(false)}>Закрыть</button>
                <button className="btn btn-primary" type="button" onClick={saveProjectDraft}>Сохранить</button>
              </div>
            </div>

            <div className="gemini-bar">
              <label>
                <span>Тема для AI</span>
                <textarea value={projectAiTopic} onChange={(event) => setProjectAiTopic(event.target.value)} placeholder="Например: шоурум плитки 180 м² в Алматы, быстрый ремонт, материалы под ключ" />
              </label>
              <button className="btn btn-ai" type="button" disabled={projectBusy} onClick={() => void generateProjectDraft()}><AppIcon name="bot" />{projectBusyLabel || 'AI черновик'}</button>
            </div>

            <div className="analytics-mode-switcher content-lang-tabs">
              {almabuildLanguages.map((language) => (
                <button key={language.key} className={activeLang === language.key ? 'analytics-mode-button active' : 'analytics-mode-button'} type="button" onClick={() => setActiveLang(language.key)}>
                  {language.label}
                </button>
              ))}
            </div>

            <div className="editor-grid">
              <label className="editor-field">
                <span>Название проекта {activeLang.toUpperCase()}</span>
                <input value={localizedString(projectDraft, 'title', activeLang)} placeholder={projectDraft.title || 'BUTIK KZ'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'title', activeLang, event.target.value))} />
              </label>
              <label className="editor-field">
                <span>URL slug</span>
                <input value={projectDraft.slug || ''} onChange={(event) => patchProjectDraft({ slug: slugify(event.target.value) })} placeholder="butik-kz-commercial-fitout" />
              </label>
              <label className="editor-field">
                <span>Фото URL {selectedProjectImage + 1}</span>
                <input value={projectImages[selectedProjectImage] || ''} onChange={(event) => setProjectImageAt(selectedProjectImage, event.target.value)} placeholder="https://..." />
              </label>
            </div>

            <label className="editor-field">
              <span>Описание проекта {activeLang.toUpperCase()}</span>
              <textarea value={localizedString(projectDraft, 'meta', activeLang)} placeholder={projectDraft.meta || 'Магазин одежды · 320 м² · 28 дней'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'meta', activeLang, event.target.value))} />
            </label>

            <div className="editor-grid">
              <label className="editor-field">
                <span>SEO title {activeLang.toUpperCase()}</span>
                <input value={localizedString(projectDraft, 'seoTitle', activeLang)} placeholder={projectDraft.seoTitle || 'Отделка магазина под ключ в Алматы'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'seoTitle', activeLang, event.target.value))} />
              </label>
              <label className="editor-field">
                <span>Заголовок страницы {activeLang.toUpperCase()}</span>
                <input value={localizedString(projectDraft, 'pageTitle', activeLang)} placeholder={projectDraft.pageTitle || localizedString(projectDraft, 'title', activeLang)} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'pageTitle', activeLang, event.target.value))} />
              </label>
            </div>

            <label className="editor-field">
              <span>SEO description {activeLang.toUpperCase()}</span>
              <textarea value={localizedString(projectDraft, 'seoDescription', activeLang)} placeholder={projectDraft.seoDescription || 'Кейс коммерческой отделки: материалы, сроки, комплектация и контроль работ в Алматы.'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'seoDescription', activeLang, event.target.value))} />
            </label>

            <label className="editor-field">
              <span>Текст SEO-страницы {activeLang.toUpperCase()}</span>
              <textarea className="content-body-editor almabuild-page-text-editor" value={localizedString(projectDraft, 'pageText', activeLang)} placeholder={projectDraft.pageText || 'Опишите задачу объекта, материалы, этапы работ, сроки и пользу для клиента.'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'pageText', activeLang, event.target.value))} />
            </label>

            <section className="content-photo-panel almabuild-project-photo-panel">
              <button className="content-photo-preview" type="button" onClick={() => projectImages[selectedProjectImage] ? setFullscreenProjectImage(selectedProjectImage) : undefined}>
                {projectImages[selectedProjectImage] ? <img src={projectImages[selectedProjectImage]} alt={localizedString(projectDraft, 'title', activeLang) || projectDraft.title || 'Фото проекта'} /> : <span><AppIcon name="cms" size={34} />Фото {selectedProjectImage + 1} не выбрано</span>}
              </button>
              <div className="content-photo-tools">
                <div className="content-image-strip">
                  {projectImages.map((url, index) => (
                    <button key={index} className={selectedProjectImage === index ? 'active' : ''} type="button" onClick={() => setSelectedProjectImage(index)}>
                      {url ? <img src={url} alt={`Фото проекта ${index + 1}`} /> : <span>{index + 1}</span>}
                      <small>{index === 0 ? 'Обложка' : `Фото ${index + 1}`}</small>
                    </button>
                  ))}
                  {projectImages.length < PROJECT_IMAGE_MAX ? <button className="content-image-add" type="button" onClick={addProjectImageSlot}><span>+</span><small>Фото</small></button> : null}
                </div>
                <label className="editor-field">
                  <span>Промпт фото</span>
                  <textarea value={projectPhotoPrompt} onChange={(event) => setProjectPhotoPrompt(event.target.value)} placeholder="Commercial construction project photo, finished retail interior, premium materials, no people" />
                </label>
                <section className="content-reference-panel">
                  <div className="panel-title compact"><span><AppIcon name="external" />Референсы для AI</span><small>{projectReferenceUrls.length}/{PROJECT_REFERENCE_MAX}</small></div>
                  <div className="content-reference-actions">
                    <label className="btn btn-secondary">
                      <input className="visually-hidden" type="file" accept="image/*" multiple disabled={projectReferenceBusy || projectReferenceUrls.length >= PROJECT_REFERENCE_MAX} onChange={(event) => void addProjectReferenceFiles(event.target.files)} />
                      {projectReferenceBusyLabel || 'Загрузить с ПК'}
                    </label>
                    <div className="content-reference-url">
                      <input value={projectReferenceUrlInput} onChange={(event) => setProjectReferenceUrlInput(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') addProjectReferenceUrl(); }} placeholder="https://... фото-референс" />
                      <button className="btn btn-quiet" type="button" disabled={projectReferenceUrls.length >= PROJECT_REFERENCE_MAX} onClick={addProjectReferenceUrl}>URL</button>
                    </div>
                  </div>
                  {projectReferenceUrls.length ? <div className="content-reference-strip">{projectReferenceUrls.map((url, index) => <div key={`${url}-${index}`} className="content-reference-thumb"><img src={url} alt={`Референс проекта ${index + 1}`} /><button type="button" onClick={() => removeProjectReferenceUrl(index)}>×</button></div>)}</div> : <p className="editor-message">AI будет генерировать без визуального референса.</p>}
                </section>
                <div className="editor-actions">
                  <label className="btn btn-secondary">
                    <input className="visually-hidden" type="file" accept="image/*" disabled={projectImageBusy} onChange={(event) => void uploadProjectPhoto(event.target.files?.[0] ?? null)} />
                    {projectImageBusyLabel === 'Загружаем фото...' ? projectImageBusyLabel : 'Загрузить'}
                  </label>
                  <button className="btn btn-ai" type="button" disabled={projectImageBusy} onClick={() => void generateProjectPhoto()}><AppIcon name="bot" />{projectImageBusyLabel === 'Gemini фото...' ? projectImageBusyLabel : 'AI фото'}</button>
                  <button className="btn btn-ai" type="button" disabled={projectImageBusy} onClick={() => void generateProjectPhotoSeries()}><AppIcon name="bot" />{projectImageBusyLabel.startsWith('Gemini ') && projectImageBusyLabel.includes('/') ? projectImageBusyLabel : 'AI 4 фото'}</button>
                  <button className="btn btn-ai" type="button" disabled={projectImageBusy || projectImages.length >= PROJECT_IMAGE_MAX} onClick={() => void generateAdditionalProjectPhoto()}><AppIcon name="bot" />{projectImageBusyLabel.startsWith('Gemini +') ? projectImageBusyLabel : 'AI + фото'}</button>
                  <button className="btn btn-quiet" type="button" disabled={projectImageBusy || !projectImages[selectedProjectImage]} onClick={() => setFullscreenProjectImage(selectedProjectImage)}>На весь экран</button>
                  <button className="btn btn-quiet" type="button" disabled={projectImageBusy || (!projectImages[selectedProjectImage] && projectImages.length <= PROJECT_IMAGE_COUNT)} onClick={removeSelectedProjectImage}>Убрать</button>
                </div>
              </div>
            </section>

            <div className="site-preview">
              <span>Как это читается на сайте</span>
              <strong>{localizedString(projectDraft, 'title', activeLang) || projectDraft.title || 'Название проекта'}</strong>
              <p>{localizedString(projectDraft, 'meta', activeLang) || projectDraft.meta || 'Формат · площадь · срок'}</p>
              <p>{projectDraft.slug ? `/projects/${projectDraft.slug}` : 'URL появится после AI или заполнения slug'}</p>
            </div>

            <div className="editor-actions">
              <button className="btn btn-danger" type="button" onClick={deleteProjectDraft}>{editingProjectIndex === null ? 'Отменить' : 'Удалить'}</button>
              <button className="btn btn-secondary" type="button" onClick={() => patchProjectDraft({ slug: slugify(localizedString(projectDraft, 'pageTitle', activeLang) || localizedString(projectDraft, 'title', activeLang) || projectAiTopic) })}>Сделать URL</button>
              <button className="btn btn-primary" type="button" onClick={saveProjectDraft}>Сохранить</button>
            </div>
          </div>
          {fullscreenProjectImage !== null ? (
            <div className="image-lightbox" onClick={() => setFullscreenProjectImage(null)}>
              <div className="image-lightbox-panel" onClick={(event) => event.stopPropagation()}>
                <div className="image-lightbox-head">
                  <strong>{fullscreenProjectImage === 0 ? 'Обложка' : `Фото ${fullscreenProjectImage + 1}`}</strong>
                  <div className="editor-actions">
                    <button className="btn btn-quiet" type="button" onClick={() => setFullscreenProjectImage((current) => current === null ? null : Math.max(0, current - 1))}>Назад</button>
                    <button className="btn btn-quiet" type="button" onClick={() => setFullscreenProjectImage((current) => current === null ? null : Math.min(projectImages.length - 1, current + 1))}>Вперед</button>
                    <button className="btn btn-primary" type="button" onClick={() => setFullscreenProjectImage(null)}>Закрыть</button>
                  </div>
                </div>
                <div className="image-lightbox-stage">
                  {projectImages[fullscreenProjectImage] ? <img src={projectImages[fullscreenProjectImage]} alt={`Фото проекта ${fullscreenProjectImage + 1}`} /> : <span>Фото не выбрано</span>}
                </div>
                <div className="content-image-strip lightbox-strip">
                  {projectImages.map((url, index) => (
                    <button key={index} className={fullscreenProjectImage === index ? 'active' : ''} type="button" onClick={() => setFullscreenProjectImage(index)}>
                      {url ? <img src={url} alt={`Фото проекта ${index + 1}`} /> : <span>{index + 1}</span>}
                      <small>{index === 0 ? 'Обложка' : `Фото ${index + 1}`}</small>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
