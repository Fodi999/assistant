import { useEffect, useMemo, useState, type MouseEvent, type TouchEvent, type WheelEvent } from 'react';
import {
  aiCreateArticleDraft,
  aiGenerateArticleImage,
  createArticle,
  deleteArticle,
  listArticleCategories,
  updateArticle,
  uploadCmsReference,
  type AiArticleDraft,
  type CmsArticleCategory,
  type CmsImageScaleSettings
} from '../api/cms';
import { revalidateSite } from '../api/revalidate';
import type { CmsArticle } from '../types/admin';
import { AiPhotoGallery } from '../components/AiPhotoGallery';
import { AiReferenceUpload, type AiReferenceImage } from '../components/AiReferenceUpload';

type ArticleLang = 'ru' | 'en' | 'pl' | 'uk';

interface CmsPageProps {
  articles: CmsArticle[];
  loading: boolean;
  error: string | null;
  onLoadCms: () => Promise<void>;
}

const TEXT_LENGTHS = [2000, 3500, 5000, 7500, 10000];
const PHOTO_COUNTS = [1, 2, 4, 6, 8, 10, 12];
const IMAGE_MODELS = [
  { value: 'flash', label: 'Flash', hint: 'Быстро · серия', model: 'Gemini 3.1 Flash Image' },
  { value: 'pro', label: 'Pro', hint: 'Премиум · медленнее', model: 'Gemini 3 Pro Image' }
] as const;
const SCENE_PRESETS = [
  { value: 'editorial', label: 'Editorial', hint: 'Универсальный журнал' },
  { value: 'product-white', label: 'Продукт', hint: 'Белый фон' },
  { value: 'delivery-product', label: 'Доставка', hint: 'Точный товар без людей' },
  { value: 'recipe-table', label: 'Рецепт', hint: 'Подача на столе' },
  { value: 'cooking-process', label: 'Процесс', hint: 'Пошаговое действие' },
  { value: 'restaurant-plating', label: 'Ресторан', hint: 'Fine dining' },
  { value: 'home-interior', label: 'Дом', hint: 'Домашний интерьер' },
  { value: 'object-interior', label: 'Предмет', hint: 'Предмет в интерьере' }
] as const;
const PHOTO_SCENARIOS = [
  { value: 'white-background', label: 'Белый фон' },
  { value: 'kitchen-countertop', label: 'На столешнице' },
  { value: 'restaurant-kitchen', label: 'В ресторанной кухне' },
  { value: 'beside-adult-hand', label: 'Рядом с рукой' },
  { value: 'beside-tableware', label: 'Рядом с посудой' },
  { value: 'lifestyle', label: 'Lifestyle' }
] as const;
const SCALE_REFERENCES = [
  { value: 'none', label: 'Без ориентира' },
  { value: 'standard 60 cm kitchen countertop depth', label: 'Столешница 60 см' },
  { value: 'standard 24 cm dinner plate', label: 'Тарелка 24 см' },
  { value: 'adult human hand', label: 'Рука взрослого' },
  { value: 'standard 1 liter bottle', label: 'Бутылка 1 л' },
  { value: 'custom', label: 'Указать вручную' }
] as const;
function formatCmsDate(value?: string | number[]): string {
  if (!value) return '-';
  const date = Array.isArray(value)
    ? new Date(value[0], (value[1] || 1) - 1, value[2] || 1, value[3] || 0, value[4] || 0)
    : new Date(value);
  return Number.isNaN(date.getTime()) ? '-' : date.toLocaleString('ru-RU');
}

function extractMarkdownImages(content?: string | null): string[] {
  return Array.from(String(content || '').matchAll(/!\[[^\]]*\]\((https?:\/\/[^)\s]+)\)/g)).map((match) => match[1]);
}

function stripVisualStory(content?: string | null): string {
  return String(content || '').replace(/\n{0,2}##\s+Visual story[\s\S]*$/i, '').trim();
}

function articleToDraft(article: CmsArticle): AiArticleDraft {
  return {
    slug: article.slug || '',
    category: article.category || 'food',
    title_en: article.title_en || '',
    title_ru: article.title_ru || '',
    title_pl: article.title_pl || '',
    title_uk: article.title_uk || '',
    content_en: stripVisualStory(article.content_en),
    content_ru: stripVisualStory(article.content_ru),
    content_pl: stripVisualStory(article.content_pl),
    content_uk: stripVisualStory(article.content_uk),
    seo_title: article.seo_title || article.title_en || article.title_ru || '',
    seo_description: article.seo_description || '',
    seo_title_en: article.seo_title_en || article.seo_title || article.title_en || '',
    seo_title_ru: article.seo_title_ru || article.seo_title || article.title_ru || article.title_en || '',
    seo_title_pl: article.seo_title_pl || article.seo_title || article.title_pl || article.title_en || '',
    seo_title_uk: article.seo_title_uk || article.seo_title || article.title_uk || article.title_en || '',
    seo_description_en: article.seo_description_en || article.seo_description || '',
    seo_description_ru: article.seo_description_ru || article.seo_description || '',
    seo_description_pl: article.seo_description_pl || article.seo_description || '',
    seo_description_uk: article.seo_description_uk || article.seo_description || '',
    image_prompts: []
  };
}

function normalizeSeoDraft(nextDraft: AiArticleDraft): AiArticleDraft {
  return {
    ...nextDraft,
    seo_title_en: nextDraft.seo_title_en || nextDraft.seo_title || nextDraft.title_en || '',
    seo_title_ru: nextDraft.seo_title_ru || nextDraft.seo_title || nextDraft.title_ru || nextDraft.title_en || '',
    seo_title_pl: nextDraft.seo_title_pl || nextDraft.seo_title || nextDraft.title_pl || nextDraft.title_en || '',
    seo_title_uk: nextDraft.seo_title_uk || nextDraft.seo_title || nextDraft.title_uk || nextDraft.title_en || '',
    seo_description_en: nextDraft.seo_description_en || nextDraft.seo_description || '',
    seo_description_ru: nextDraft.seo_description_ru || nextDraft.seo_description || '',
    seo_description_pl: nextDraft.seo_description_pl || nextDraft.seo_description || '',
    seo_description_uk: nextDraft.seo_description_uk || nextDraft.seo_description || ''
  };
}

function parseAvatarPosition(value: string): { x: number; y: number; scale: number } {
  const match = value.match(/^(\d+(?:\.\d+)?)%\s+(\d+(?:\.\d+)?)(?:%\s+(\d+(?:\.\d+)?))?$/);
  if (match) {
    return {
      x: Number(match[1]),
      y: Number(match[2]),
      scale: Number(match[3] || 1.3)
    };
  }
  if (value === 'top') return { x: 50, y: 0, scale: 1.3 };
  if (value === 'bottom') return { x: 50, y: 100, scale: 1.3 };
  if (value === 'left') return { x: 0, y: 50, scale: 1.3 };
  if (value === 'right') return { x: 100, y: 50, scale: 1.3 };
  return { x: 50, y: 50, scale: 1.3 };
}

function avatarPanStyle(position: string) {
  const { x, y, scale } = parseAvatarPosition(position);
  return {
    left: `${x}%`,
    top: `${y}%`,
    width: `${scale * 100}%`,
    height: `${scale * 100}%`,
    transform: `translate(-${x}%, -${y}%)`
  };
}

function avatarPositionFromClient(target: HTMLElement, clientX: number, clientY: number): string {
  const current = parseAvatarPosition(target.dataset.position || '50% 50% 1.3');
  const rect = target.getBoundingClientRect();
  const x = Math.min(100, Math.max(0, ((clientX - rect.left) / rect.width) * 100));
  const y = Math.min(100, Math.max(0, ((clientY - rect.top) / rect.height) * 100));
  return `${Math.round(x)}% ${Math.round(y)}% ${current.scale.toFixed(2)}`;
}

export function CmsPage({ articles, loading, error, onLoadCms }: CmsPageProps) {
  const [topic, setTopic] = useState('');
  const [targetChars, setTargetChars] = useState(3500);
  const [photoCount, setPhotoCount] = useState(4);
  const [imageModel, setImageModel] = useState<'flash' | 'pro'>('flash');
  const [scenePreset, setScenePreset] = useState('editorial');
  const [objectDimensions, setObjectDimensions] = useState({ width: '', height: '', depth: '', weight: '' });
  const [photoScenarios, setPhotoScenarios] = useState<string[]>([]);
  const [scaleReference, setScaleReference] = useState('none');
  const [customScaleReference, setCustomScaleReference] = useState('');
  const [draft, setDraft] = useState<AiArticleDraft | null>(null);
  const [editingArticle, setEditingArticle] = useState<CmsArticle | null>(null);
  const [authorName, setAuthorName] = useState('Szef Kuchni');
  const [authorAvatarUrl, setAuthorAvatarUrl] = useState('');
  const [authorAvatarPosition, setAuthorAvatarPosition] = useState('center');
  const [authorAvatarDragging, setAuthorAvatarDragging] = useState(false);
  const [authorAvatarBusy, setAuthorAvatarBusy] = useState(false);
  const [categories, setCategories] = useState<CmsArticleCategory[]>([]);
  const [listCategoryFilter, setListCategoryFilter] = useState('all');
  const [images, setImages] = useState<string[]>([]);
  const [selectedImage, setSelectedImage] = useState(0);
  const [language, setLanguage] = useState<ArticleLang>('ru');
  const [aiBusy, setAiBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [singleImageBusy, setSingleImageBusy] = useState(false);
  const [enhancedBusy, setEnhancedBusy] = useState(false);
  const [imageProgress, setImageProgress] = useState(0);
  const [referenceImages, setReferenceImages] = useState<AiReferenceImage[]>([]);
  const [referenceBusy, setReferenceBusy] = useState(false);
  const [saveBusy, setSaveBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  async function toggleArticle(article: CmsArticle) {
    setLocalError(null);
    try {
      const updated = await updateArticle(article.id, { published: !article.published });
      await revalidateSite({ type: 'article', slug: updated.slug || article.slug });
      setMessage(article.published ? 'Статья снята с публикации' : 'Статья опубликована на сайте');
      await onLoadCms();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось изменить публикацию');
    }
  }

  async function removeArticle(article: CmsArticle) {
    if (!window.confirm(`Удалить статью «${article.title_ru || article.title_en}»?`)) return;
    try {
      await deleteArticle(article.id);
      await revalidateSite({ type: 'article', slug: article.slug });
      setMessage('Статья удалена');
      await onLoadCms();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось удалить статью');
    }
  }

  useEffect(() => {
    void listArticleCategories()
      .then((rows) => setCategories(rows))
      .catch(() => setCategories([]));
  }, []);

  const categoryOptions = useMemo(() => {
    if (categories.length > 0) {
      return categories.map((category) => ({
        slug: category.slug,
        title: category.title_ru || category.title_en || category.slug
      }));
    }
    return [];
  }, [categories]);

  function normalizeCategory(raw: string): string {
    const value = raw.trim().toLowerCase();
    if (!value) return 'food';

    const aliases: Record<string, string> = {
      ingredient: 'ingredients',
      ingredients: 'ingredients',
      ингридиенты: 'ingredients',
      ингредиенты: 'ingredients',
      ingredienty: 'ingredients',
      technique: 'techniques',
      techniques: 'techniques',
      техника: 'techniques',
      техники: 'techniques',
      recipe: 'recipes',
      recipes: 'recipes',
      'recipe development': 'recipes',
      рецепт: 'recipes',
      рецепты: 'recipes',
      food: 'food',
      еда: 'food',
      оборудование: 'equipment',
      equipment: 'equipment',
      'restaurant management': 'restaurant-management',
      management: 'restaurant-management',
      менеджмент: 'restaurant-management',
      nutrition: 'nutrition',
      питание: 'nutrition',
      beverages: 'beverages',
      напитки: 'beverages'
    };

    if (aliases[value]) return aliases[value];
    return value.replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'food';
  }

  const filteredArticles = useMemo(() => {
    if (listCategoryFilter === 'all') return articles;
    return articles.filter((article) => normalizeCategory(String(article.category || '')) === listCategoryFilter);
  }, [articles, listCategoryFilter]);

  function updateDraft(field: keyof AiArticleDraft, value: string) {
    setDraft((current) => current ? { ...current, [field]: value } : current);
  }

  function startEditArticle(article: CmsArticle) {
    const storyImages = extractMarkdownImages(article.content_ru || article.content_en || article.content_pl || article.content_uk);
    const nextImages = [article.image_url || '', ...storyImages].filter(Boolean);
    setDraft(articleToDraft(article));
    setEditingArticle(article);
    setAuthorName(article.author_name || 'Szef Kuchni');
    setAuthorAvatarUrl(article.author_avatar_url || '');
    setAuthorAvatarPosition(article.author_avatar_position || 'center');
    setImages(nextImages);
    setSelectedImage(0);
    setPhotoCount(Math.max(1, nextImages.length || 4));
    setLanguage('ru');
    setMessage(`Редактируем статью: ${article.title_ru || article.title_en || article.slug}`);
    setLocalError(null);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function closeEditor() {
    setDraft(null);
    setEditingArticle(null);
    setImages([]);
    setSelectedImage(0);
    setAuthorName('Szef Kuchni');
    setAuthorAvatarUrl('');
    setAuthorAvatarPosition('center');
  }

  async function uploadAuthorAvatar(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setLocalError('Аватар должен быть изображением');
      return;
    }
    setAuthorAvatarBusy(true);
    setLocalError(null);
    try {
      const url = await uploadCmsReference(file);
      setAuthorAvatarUrl(url);
      setMessage('Аватар автора загружен');
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось загрузить аватар автора');
    } finally {
      setAuthorAvatarBusy(false);
    }
  }

  function moveAuthorAvatarFromMouse(event: MouseEvent<HTMLDivElement>) {
    if (!authorAvatarUrl) return;
    setAuthorAvatarPosition(avatarPositionFromClient(event.currentTarget, event.clientX, event.clientY));
  }

  function moveAuthorAvatarFromTouch(event: TouchEvent<HTMLDivElement>) {
    if (!authorAvatarUrl || event.touches.length === 0) return;
    event.preventDefault();
    const touch = event.touches[0];
    setAuthorAvatarPosition(avatarPositionFromClient(event.currentTarget, touch.clientX, touch.clientY));
  }

  function zoomAuthorAvatar(event: WheelEvent<HTMLDivElement>) {
    if (!authorAvatarUrl) return;
    event.preventDefault();
    const current = parseAvatarPosition(authorAvatarPosition);
    const nextScale = Math.min(2.2, Math.max(1.05, current.scale + (event.deltaY > 0 ? -0.08 : 0.08)));
    setAuthorAvatarPosition(`${Math.round(current.x)}% ${Math.round(current.y)}% ${nextScale.toFixed(2)}`);
  }

  function imageScaleSettings(): CmsImageScaleSettings {
    const parseOptional = (value: string) => value.trim() ? Number(value) : undefined;
    return {
      widthCm: parseOptional(objectDimensions.width),
      heightCm: parseOptional(objectDimensions.height),
      depthCm: parseOptional(objectDimensions.depth),
      weightKg: parseOptional(objectDimensions.weight),
      photoScenarios,
      scaleReference,
      customScaleReference: scaleReference === 'custom' ? customScaleReference.trim() : undefined
    };
  }

  function togglePhotoScenario(value: string) {
    setPhotoScenarios((current) => current.includes(value)
      ? current.filter((item) => item !== value)
      : [...current, value]);
  }

  async function generateImages(nextDraft: AiArticleDraft) {
    setImageBusy(true);
    setImageProgress(0);
    setImages([]);
    try {
      const nextImages: string[] = [];
      for (let index = 0; index < photoCount; index += 1) {
        const result = await aiGenerateArticleImage(nextDraft.title_en, nextDraft.image_prompts[index], index, imageModel === 'pro', referenceImages.map((item) => item.url), imageModel, scenePreset, imageScaleSettings());
        nextImages.push(result.image_url);
        setImages([...nextImages]);
        setImageProgress(index + 1);
      }
      setSelectedImage(0);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать изображения');
    } finally {
      setImageBusy(false);
    }
  }

  async function generateEnhancedImage() {
    if (!draft) return;
    setEnhancedBusy(true);
    setLocalError(null);
    try {
      const result = await aiGenerateArticleImage(draft.title_en, draft.image_prompts[selectedImage], selectedImage, true, referenceImages.map((item) => item.url), 'pro', scenePreset, imageScaleSettings());
      setImages((current) => {
        const next = [...current];
        next[selectedImage] = result.image_url;
        return next;
      });
      setMessage(`Фото ${selectedImage + 1} улучшено через Gemini 3 Pro Image`);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать улучшенное фото');
    } finally {
      setEnhancedBusy(false);
    }
  }

  async function generateAdditionalImage() {
    if (!draft || images.length >= 12) return;
    const index = images.length;
    setSingleImageBusy(true);
    setLocalError(null);
    try {
      const result = await aiGenerateArticleImage(draft.title_en, draft.image_prompts[index], index, imageModel === 'pro', referenceImages.map((item) => item.url), imageModel, scenePreset, imageScaleSettings());
      setImages((current) => [...current, result.image_url]);
      setPhotoCount((current) => Math.max(current, index + 1));
      setSelectedImage(index);
      setMessage(`Добавлено фото ${index + 1}`);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать дополнительное фото');
    } finally {
      setSingleImageBusy(false);
    }
  }

  function removeArticleImage(index: number) {
    setImages((current) => {
      const next = current.filter((_, itemIndex) => itemIndex !== index);
      setPhotoCount(Math.max(1, next.length));
      setSelectedImage((selected) => {
        if (next.length === 0) return 0;
        if (selected > index) return selected - 1;
        if (selected === index) return Math.min(index, next.length - 1);
        return selected;
      });
      return next;
    });
  }

  async function addReferenceImages(files: FileList | null) {
    if (!files) return;
    const selected = Array.from(files).filter((file) => file.type.startsWith('image/')).slice(0, 2 - referenceImages.length);
    if (selected.length === 0) return;
    setReferenceBusy(true);
    setLocalError(null);
    try {
      const uploaded: AiReferenceImage[] = [];
      for (const file of selected) {
        if (file.size > 10 * 1024 * 1024) throw new Error('Каждый референс должен быть меньше 10 MB');
        const url = await uploadCmsReference(file);
        uploaded.push({ url, preview: URL.createObjectURL(file), name: file.name });
      }
      setReferenceImages((current) => [...current, ...uploaded].slice(0, 2));
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось загрузить референсы');
    } finally {
      setReferenceBusy(false);
    }
  }

  function removeReference(index: number) {
    setReferenceImages((current) => {
      const target = current[index];
      if (target) URL.revokeObjectURL(target.preview);
      return current.filter((_, itemIndex) => itemIndex !== index);
    });
  }

  async function generateDraft() {
    if (!topic.trim()) {
      setLocalError('Введите тему или рабочее название статьи.');
      return;
    }
    setAiBusy(true);
    setLocalError(null);
    setMessage(null);
    setImages([]);
    try {
      const nextDraft = normalizeSeoDraft(await aiCreateArticleDraft(topic.trim(), targetChars, photoCount));
      nextDraft.category = normalizeCategory(nextDraft.category || 'food');
      setDraft(nextDraft);
      setEditingArticle(null);
      setAuthorName('Szef Kuchni');
      setAuthorAvatarUrl('');
      setAuthorAvatarPosition('center');
      setLanguage('ru');
      void generateImages(nextDraft);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось создать статью');
    } finally {
      setAiBusy(false);
    }
  }

  async function regenerateText() {
    if (!draft) return;
    setAiBusy(true);
    setLocalError(null);
    try {
      const nextDraft = normalizeSeoDraft(await aiCreateArticleDraft(topic.trim() || draft.title_en, targetChars, photoCount));
      nextDraft.category = normalizeCategory(nextDraft.category || draft.category);
      setDraft(nextDraft);
      setMessage(`Текст перегенерирован: цель ${targetChars.toLocaleString('ru-RU')} символов на язык`);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось перегенерировать текст');
    } finally {
      setAiBusy(false);
    }
  }

  async function saveArticle() {
    if (!draft) return;
    setSaveBusy(true);
    setLocalError(null);
    try {
      const visualStory = images.length > 1
        ? `\n\n## Visual story\n${images.slice(1).map((url, index) => `![Step ${index + 1}](${url})`).join('\n\n')}`
        : '';
      const payload = {
        ...draft,
        category: normalizeCategory(draft.category),
        content_en: `${draft.content_en}${visualStory}`,
        content_ru: `${draft.content_ru}${visualStory}`,
        content_pl: `${draft.content_pl}${visualStory}`,
        content_uk: `${draft.content_uk}${visualStory}`,
        seo_title: draft.seo_title_en || draft.seo_title || draft.title_en,
        seo_description: draft.seo_description_en || draft.seo_description || '',
        image_url: images[selectedImage],
        author_name: authorName.trim() || 'Szef Kuchni',
        author_avatar_url: authorAvatarUrl || null,
        author_avatar_position: authorAvatarPosition,
        published: editingArticle?.published ?? false
      };
      if (editingArticle) {
        const updated = await updateArticle(editingArticle.id, payload);
        await revalidateSite({ type: 'article', slug: updated.slug || editingArticle.slug });
        setMessage('Статья обновлена');
      } else {
        const created = await createArticle(payload);
        await revalidateSite({ type: 'article', slug: created.slug });
        setMessage('Статья сохранена как черновик');
      }
      closeEditor();
      setTopic('');
      await onLoadCms();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Не удалось сохранить статью');
    } finally {
      setSaveBusy(false);
    }
  }

  const titleField = `title_${language}` as keyof AiArticleDraft;
  const contentField = `content_${language}` as keyof AiArticleDraft;
  const seoTitleField = `seo_title_${language}` as keyof AiArticleDraft;
  const seoDescriptionField = `seo_description_${language}` as keyof AiArticleDraft;

  return (
    <section className="cms-page">
      <header className="cms-hero">
        <div><span className="eyebrow">AI editorial studio</span><h2>CMS статьи</h2><p>Создавайте экспертные статьи, переводы и визуальную историю с Gemini.</p></div>
        <button className="btn btn-quiet" onClick={() => void onLoadCms()} disabled={loading}>Обновить</button>
      </header>

      <section className="cms-ai-studio">
        <div className="cms-ai-copy"><span className="ai-orb">AI</span><div><div className="model-badges"><span>Текст · Gemini 3.5 Flash</span><span>Фото · Gemini 3.1 Flash Image</span><span className="premium">Обложка · Gemini 3 Pro Image</span></div><h3>Новая статья</h3><p>Укажите тему, объём текста и количество изображений для визуальной серии.</p></div></div>
        <div className="cms-ai-controls">
          <div className="cms-ai-command"><input value={topic} onChange={(event) => setTopic(event.target.value)} placeholder="Например: Как правильно выдерживать свежую рыбу перед подачей" onKeyDown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') void generateDraft(); }} /><button className="btn btn-ai" onClick={() => void generateDraft()} disabled={aiBusy}>{aiBusy ? 'Gemini пишет...' : 'Создать AI-черновик'}</button></div>
          <div className="generation-settings">
            <div className="generation-option"><div className="generation-option-head"><span>Объём текста</span><strong>{targetChars.toLocaleString('ru-RU')} <small>символов / язык</small></strong></div><div className="generation-segments">{TEXT_LENGTHS.map((count) => <button type="button" key={count} className={targetChars === count ? 'active' : ''} onClick={() => setTargetChars(count)}>{count >= 1000 ? `${count / 1000}k` : count}</button>)}</div></div>
            <div className="generation-option"><div className="generation-option-head"><span>Серия изображений</span><strong>{photoCount} <small>фото</small></strong></div><div className="generation-segments photo">{PHOTO_COUNTS.map((count) => <button type="button" key={count} className={photoCount === count ? 'active' : ''} onClick={() => setPhotoCount(count)}>{count}</button>)}</div></div>
          </div>
          <div className="visual-settings">
            <div className="visual-setting"><div className="visual-setting-head"><strong>Модель изображений</strong><span>{IMAGE_MODELS.find((item) => item.value === imageModel)?.model}</span></div><div className="visual-model-options">{IMAGE_MODELS.map((item) => <button type="button" key={item.value} className={imageModel === item.value ? 'active' : ''} onClick={() => setImageModel(item.value)}><strong>{item.label}</strong><small>{item.hint}</small></button>)}</div></div>
            <div className="visual-setting"><div className="visual-setting-head"><strong>Категория сцены</strong><span>Определяет композицию и фон</span></div><div className="scene-options">{SCENE_PRESETS.map((item) => <button type="button" key={item.value} className={scenePreset === item.value ? 'active' : ''} onClick={() => setScenePreset(item.value)}><strong>{item.label}</strong><small>{item.hint}</small></button>)}</div></div>
          </div>
          <div className="scale-settings">
            <div className="scale-setting-head"><div><strong>Масштаб и реалистичность</strong><span>Gemini сохранит правдоподобные пропорции объекта в кадре</span></div><span className="scale-status">{Object.values(objectDimensions).some(Boolean) ? 'Размер задан' : 'Необязательно'}</span></div>
            <div className="scale-settings-grid">
              <div className="dimension-setting"><span className="scale-label">Размер товара</span><div className="dimension-inputs">{([['width', 'Ширина', 'см'], ['height', 'Высота', 'см'], ['depth', 'Глубина', 'см'], ['weight', 'Вес', 'кг']] as const).map(([key, label, unit]) => <label key={key}><span>{label}</span><div><input type="number" min="0" step="0.1" value={objectDimensions[key]} onChange={(event) => setObjectDimensions((current) => ({ ...current, [key]: event.target.value }))} placeholder="—" /><b>{unit}</b></div></label>)}</div></div>
              <div className="scenario-setting"><span className="scale-label">Сценарий фото <small>можно несколько</small></span><div className="scenario-options">{PHOTO_SCENARIOS.map((item) => <button type="button" key={item.value} className={photoScenarios.includes(item.value) ? 'active' : ''} onClick={() => togglePhotoScenario(item.value)}><i>{photoScenarios.includes(item.value) ? '✓' : '+'}</i>{item.label}</button>)}</div></div>
              <div className="reference-scale-setting"><span className="scale-label">Масштабный ориентир</span><div className="scale-reference-options">{SCALE_REFERENCES.map((item) => <button type="button" key={item.value} className={scaleReference === item.value ? 'active' : ''} onClick={() => setScaleReference(item.value)}>{item.label}</button>)}</div>{scaleReference === 'custom' && <input className="custom-scale-input" value={customScaleReference} onChange={(event) => setCustomScaleReference(event.target.value)} placeholder="Например: рядом с кофейной чашкой высотой 9 см" />}</div>
            </div>
          </div>
          <AiReferenceUpload title="Визуальные референсы" hint="До 2 фото. Gemini увидит форму, детали и стиль перед генерацией." images={referenceImages} busy={referenceBusy} onAdd={(files) => void addReferenceImages(files)} onRemove={removeReference} />
        </div>
        {localError && <p className="form-error notice">{localError}</p>}
        {message && <p className="form-success notice">{message}</p>}
      </section>

      {draft && <section className="article-review">
        <div className="article-review-head"><div><span className="eyebrow">{editingArticle ? 'Edit article' : 'Review before database'}</span><h3>{draft.title_ru || draft.title_en}</h3><p>{editingArticle ? 'Редактируйте опубликованную или черновую статью так же, как новый AI-черновик.' : 'Статья сохраняется только после подтверждения.'}</p></div><div className="article-review-meta"><span>{draft.category}</span><span>{draft.slug}</span></div></div>

        <div className="article-review-grid">
          <section className="article-images-panel">
            <AiPhotoGallery heading="Визуальная история" subtitle={`${images.length || photoCount} фото · Gemini 3.1 Flash Image`} actions={<><button className="btn btn-quiet" onClick={() => void generateImages(draft)} disabled={imageBusy || singleImageBusy || enhancedBusy}>{imageBusy ? `Создаём ${Math.min(imageProgress + 1, photoCount)} из ${photoCount}...` : 'Перегенерировать серию'}</button><button className="btn btn-pro-image" onClick={() => void generateEnhancedImage()} disabled={imageBusy || singleImageBusy || enhancedBusy || !images[selectedImage]}>{enhancedBusy ? 'Улучшаем...' : 'Улучшить главное фото'}</button></>} images={images} selectedIndex={selectedImage} title={draft.title_ru || draft.title_en} emptyText="Изображения не созданы" busyText={imageBusy ? `Gemini создаёт изображение ${Math.min(imageProgress + 1, photoCount)} из ${photoCount}` : 'Gemini создаёт дополнительное фото'} busy={imageBusy || enhancedBusy} addBusy={singleImageBusy} itemLabel={(index) => index === 0 ? 'Обложка' : `Этап ${index}`} onSelect={setSelectedImage} onAdd={() => void generateAdditionalImage()} onRemove={removeArticleImage} />
          </section>

          <section className="article-editor-panel">
            <div className="article-panel-head"><div><h4>Текст статьи</h4><p>Цель: {targetChars.toLocaleString('ru-RU')} символов на каждый язык</p></div><div className="article-editor-tools"><button className="btn btn-quiet" type="button" onClick={() => void regenerateText()} disabled={aiBusy}>{aiBusy ? 'Gemini пишет...' : 'Перегенерировать текст'}</button><div className="language-tabs">{(['ru', 'en', 'pl', 'uk'] as const).map((item) => <button type="button" key={item} className={language === item ? 'active' : ''} onClick={() => setLanguage(item)}>{item.toUpperCase()}</button>)}</div></div></div>
            <label className="field"><span className="field-label">Заголовок · {language.toUpperCase()}</span><input value={String(draft[titleField])} onChange={(event) => updateDraft(titleField, event.target.value)} /></label>
            <label className="field article-content-field"><span className="field-label">Текст · Markdown</span><textarea value={String(draft[contentField])} onChange={(event) => updateDraft(contentField, event.target.value)} /><span className={Math.abs(String(draft[contentField]).length - targetChars) / targetChars <= .15 ? 'length-ok' : ''}>{String(draft[contentField]).length} / {targetChars.toLocaleString('ru-RU')} символов</span></label>
            <div className="draft-grid two">
              <div className="article-author-editor span-2">
                <div className={`article-author-preview ${authorAvatarUrl ? 'is-draggable' : ''}`} data-position={authorAvatarPosition} onMouseDown={(event) => { setAuthorAvatarDragging(true); moveAuthorAvatarFromMouse(event); }} onMouseMove={(event) => { if (authorAvatarDragging) moveAuthorAvatarFromMouse(event); }} onMouseUp={() => setAuthorAvatarDragging(false)} onMouseLeave={() => setAuthorAvatarDragging(false)} onTouchStart={moveAuthorAvatarFromTouch} onTouchMove={moveAuthorAvatarFromTouch} onWheel={zoomAuthorAvatar} title={authorAvatarUrl ? 'Потяните фото курсором, колесом мыши меняйте масштаб' : undefined}>
                  {authorAvatarUrl ? <img src={authorAvatarUrl} alt={authorName || 'Автор'} style={avatarPanStyle(authorAvatarPosition)} /> : <span>{(authorName || 'SK').slice(0, 2).toUpperCase()}</span>}
                </div>
                <label className="field"><span className="field-label">Имя автора</span><input value={authorName} onChange={(event) => setAuthorName(event.target.value)} placeholder="Например: Dima Fomin" /></label>
                <label className="btn btn-quiet article-author-upload">
                  <input type="file" accept="image/*" onChange={(event) => void uploadAuthorAvatar(event.target.files?.[0] || null)} disabled={authorAvatarBusy} />
                  {authorAvatarBusy ? 'Загружаем...' : authorAvatarUrl ? 'Заменить аватар' : 'Загрузить аватар'}
                </label>
                {authorAvatarUrl && <p className="article-author-position-hint">Потяните фото в круге курсором. Колесо мыши меняет масштаб.</p>}
              </div>
              <label className="field"><span className="field-label">Slug</span><input value={draft.slug} onChange={(event) => updateDraft('slug', event.target.value)} /></label>
              <label className="field"><span className="field-label">Категория</span><select value={normalizeCategory(draft.category)} onChange={(event) => updateDraft('category', event.target.value)}>{categoryOptions.map((item) => <option key={item.slug} value={item.slug}>{item.title}</option>)}</select></label>
              <label className="field"><span className="field-label">SEO title · {language.toUpperCase()}</span><input value={String(draft[seoTitleField] || '')} onChange={(event) => updateDraft(seoTitleField, event.target.value)} maxLength={70} /></label>
              <label className="field"><span className="field-label">SEO description · {language.toUpperCase()}</span><textarea value={String(draft[seoDescriptionField] || '')} onChange={(event) => updateDraft(seoDescriptionField, event.target.value)} maxLength={180} /></label>
            </div>
          </section>
        </div>
        <div className="article-actions"><div><strong>{editingArticle ? 'Сохранить изменения' : 'Статья готова к проверке'}</strong><span>{editingArticle ? 'Статус публикации сохранится без изменений' : 'После сохранения останется черновиком'}</span></div><div><button className="btn btn-quiet" onClick={closeEditor}>Отменить</button><button className="btn btn-primary" onClick={() => void saveArticle()} disabled={saveBusy || imageBusy || singleImageBusy || enhancedBusy}>{saveBusy ? 'Сохраняем...' : editingArticle ? 'Сохранить изменения' : 'Сохранить черновик'}</button></div></div>
      </section>}

      <section className="cms-list page-card">
        <div className="section-head"><div><h3>Все статьи</h3><p className="page-muted">Всего: {filteredArticles.length}</p></div></div>
        <div className="cms-list-toolbar"><label className="field"><span className="field-label">Фильтр по теме</span><select value={listCategoryFilter} onChange={(event) => setListCategoryFilter(event.target.value)}><option value="all">Все категории</option>{categoryOptions.map((item) => <option key={item.slug} value={item.slug}>{item.title}</option>)}</select></label></div>
        {loading && <p className="page-muted">Загружаем статьи...</p>}
        {error && <p className="form-error">{error}</p>}
        {!loading && !error && <div className="orders-table-wrap"><table className="orders-table"><thead><tr><th>Заголовок</th><th>Автор</th><th>Категория</th><th>Slug</th><th>Дата публикации</th><th>Статус</th><th>Управление</th></tr></thead><tbody>{filteredArticles.map((article) => <tr key={article.id}><td><strong>{article.title_ru || article.title_en || '-'}</strong></td><td>{article.author_name || 'Szef Kuchni'}</td><td>{normalizeCategory(String(article.category || ''))}</td><td>{article.slug}</td><td>{formatCmsDate(article.published_at || article.updated_at)}</td><td><span className={`status-badge status-badge-${article.published ? 'ok' : 'neutral'}`}>{article.published ? 'Опубликована' : 'Черновик'}</span></td><td><div className="table-actions"><button className="btn btn-quiet" onClick={() => startEditArticle(article)}>Редактировать</button><button className="btn btn-quiet" onClick={() => void toggleArticle(article)}>{article.published ? 'Снять' : 'Опубликовать'}</button><button className="btn btn-danger" onClick={() => void removeArticle(article)}>Удалить</button></div></td></tr>)}</tbody></table></div>}
      </section>
    </section>
  );
}
