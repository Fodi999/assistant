import { useEffect, useMemo, useState } from 'react';
import {
  aiCreateArticleDraft,
  aiGenerateArticleImage,
  createArticle,
  deleteArticle,
  listArticleCategories,
  listArticles,
  updateArticle,
  uploadCmsReference,
  type CmsArticleCategory
} from '../api/cms';
import { DataSourceBadge, type DataSource } from '../components/DataSourceBadge';
import { contentArticles, siteLabel } from '../lib/mockData';
import type { CmsArticle, ContentArticle, ContentType, PublishStatus, SiteKey } from '../types/admin';
import { AppIcon } from '../components/AppIcon';
import { contentTypeLabels, publishStatusLabels } from '../lib/labels';
import { revalidateSite } from '../api/revalidate';

type Language = 'ru' | 'pl' | 'en' | 'uk';

const languages: Array<{ key: Language; label: string }> = [
  { key: 'ru', label: 'RU' },
  { key: 'pl', label: 'PL' },
  { key: 'en', label: 'EN' },
  { key: 'uk', label: 'UK' }
];

const ARTICLE_IMAGE_COUNT = 4;
const ARTICLE_IMAGE_MAX = 12;
const ARTICLE_REFERENCE_MAX = 4;

type ArticleDraft = {
  id?: string;
  slug: string;
  category: string;
  image_url: string;
  published: boolean;
  title_ru: string;
  title_pl: string;
  title_en: string;
  title_uk: string;
  content_ru: string;
  content_pl: string;
  content_en: string;
  content_uk: string;
  seo_title: string;
  seo_description: string;
  seo_title_ru: string;
  seo_title_pl: string;
  seo_title_en: string;
  seo_title_uk: string;
  seo_description_ru: string;
  seo_description_pl: string;
  seo_description_en: string;
  seo_description_uk: string;
};

function articleFromApi(article: CmsArticle): ContentArticle {
  return {
    id: article.id,
    site: 'culinary',
    type: 'article',
    title: {
      ru: article.title_ru || article.title_en || article.slug,
      pl: article.title_pl || article.title_en || article.slug,
      en: article.title_en || article.title_ru || article.slug,
      kk: article.title_ru || article.title_en || article.slug
    },
    slug: article.slug,
    excerpt: {
      ru: article.seo_description_ru || article.seo_description || '',
      pl: article.seo_description_pl || article.seo_description || '',
      en: article.seo_description_en || article.seo_description || '',
      kk: article.seo_description_ru || article.seo_description || ''
    },
    status: article.published ? 'published' : 'draft',
    languages: ['ru', 'pl', 'en'],
    affiliateProductIds: [],
    seoTitle: {
      ru: article.seo_title_ru || article.seo_title || '',
      pl: article.seo_title_pl || article.seo_title || '',
      en: article.seo_title_en || article.seo_title || '',
      kk: article.seo_title_ru || article.seo_title || ''
    },
    seoDescription: {
      ru: article.seo_description_ru || article.seo_description || '',
      pl: article.seo_description_pl || article.seo_description || '',
      en: article.seo_description_en || article.seo_description || '',
      kk: article.seo_description_ru || article.seo_description || ''
    },
    publishedAt: typeof article.published_at === 'string' ? article.published_at : undefined
  };
}

function draftFromArticle(article?: CmsArticle): ArticleDraft {
  return {
    id: article?.id,
    slug: article?.slug ?? '',
    category: article?.category ?? 'general',
    image_url: article?.image_url ?? '',
    published: Boolean(article?.published),
    title_ru: article?.title_ru ?? '',
    title_pl: article?.title_pl ?? '',
    title_en: article?.title_en ?? '',
    title_uk: article?.title_uk ?? '',
    content_ru: stripVisualStory(article?.content_ru),
    content_pl: stripVisualStory(article?.content_pl),
    content_en: stripVisualStory(article?.content_en),
    content_uk: stripVisualStory(article?.content_uk),
    seo_title: article?.seo_title ?? '',
    seo_description: article?.seo_description ?? '',
    seo_title_ru: article?.seo_title_ru ?? '',
    seo_title_pl: article?.seo_title_pl ?? '',
    seo_title_en: article?.seo_title_en ?? '',
    seo_title_uk: article?.seo_title_uk ?? '',
    seo_description_ru: article?.seo_description_ru ?? '',
    seo_description_pl: article?.seo_description_pl ?? '',
    seo_description_en: article?.seo_description_en ?? '',
    seo_description_uk: article?.seo_description_uk ?? ''
  };
}

function extractMarkdownImages(content?: string | null): string[] {
  return Array.from(String(content || '').matchAll(/!\[[^\]]*]\((https?:\/\/[^)\s]+)\)/g)).map((match) => match[1]);
}

function stripVisualStory(content?: string | null): string {
  return String(content || '').replace(/\n{0,2}##\s+Visual story[\s\S]*$/i, '').trim();
}

function articleImageSeries(article?: CmsArticle): string[] {
  const storyImages = extractMarkdownImages([article?.content_ru, article?.content_en, article?.content_pl, article?.content_uk].filter(Boolean).join('\n'));
  const unique = [article?.image_url || '', ...storyImages]
    .map((url) => url.trim())
    .filter(Boolean)
    .filter((url, index, list) => list.indexOf(url) === index);
  const targetCount = Math.max(ARTICLE_IMAGE_COUNT, Math.min(ARTICLE_IMAGE_MAX, unique.length));
  return unique.slice(0, ARTICLE_IMAGE_MAX).concat(Array(targetCount).fill('')).slice(0, targetCount);
}

function withVisualStory(content: string, images: string[]): string {
  const cleanContent = stripVisualStory(content);
  const story = images.slice(1).filter(Boolean);
  if (!story.length) return cleanContent;
  return `${cleanContent}\n\n## Visual story\n${story.map((url, index) => `![Step ${index + 1}](${url})`).join('\n\n')}`;
}

function payloadFromDraft(draft: ArticleDraft, images: string[]): Omit<CmsArticle, 'id' | 'updated_at'> {
  const cleanImages = images.map((url) => url.trim()).filter(Boolean);
  return {
    slug: draft.slug,
    category: draft.category,
    image_url: cleanImages[0] || draft.image_url || null,
    published: draft.published,
    title_ru: draft.title_ru || null,
    title_pl: draft.title_pl || null,
    title_en: draft.title_en || null,
    title_uk: draft.title_uk || null,
    content_ru: withVisualStory(draft.content_ru, cleanImages) || null,
    content_pl: withVisualStory(draft.content_pl, cleanImages) || null,
    content_en: withVisualStory(draft.content_en, cleanImages) || null,
    content_uk: withVisualStory(draft.content_uk, cleanImages) || null,
    seo_title: draft.seo_title || null,
    seo_description: draft.seo_description || null,
    seo_title_ru: draft.seo_title_ru || null,
    seo_title_pl: draft.seo_title_pl || null,
    seo_title_en: draft.seo_title_en || null,
    seo_title_uk: draft.seo_title_uk || null,
    seo_description_ru: draft.seo_description_ru || null,
    seo_description_pl: draft.seo_description_pl || null,
    seo_description_en: draft.seo_description_en || null,
    seo_description_uk: draft.seo_description_uk || null,
    published_at: draft.published ? new Date().toISOString() : null
  };
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9а-яёіїєґ]+/gi, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 88);
}

function articleTitle(article: CmsArticle): string {
  return article.title_ru || article.title_pl || article.title_en || article.title_uk || article.slug;
}

export function ContentPage({ activeSite }: { activeSite: SiteKey }) {
  const [site, setSite] = useState<SiteKey | 'all'>(activeSite);
  const [type, setType] = useState<ContentType | 'all'>('all');
  const [apiArticles, setApiArticles] = useState<CmsArticle[]>([]);
  const [items, setItems] = useState<ContentArticle[]>(contentArticles);
  const [categories, setCategories] = useState<CmsArticleCategory[]>([]);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const [editorOpen, setEditorOpen] = useState(false);
  const [draft, setDraft] = useState<ArticleDraft>(() => draftFromArticle());
  const [images, setImages] = useState<string[]>(() => articleImageSeries());
  const [selectedImage, setSelectedImage] = useState(0);
  const [fullscreenImage, setFullscreenImage] = useState<number | null>(null);
  const [activeLang, setActiveLang] = useState<Language>('ru');
  const [aiTopic, setAiTopic] = useState('');
  const [aiImagePrompt, setAiImagePrompt] = useState('');
  const [referenceUrls, setReferenceUrls] = useState<string[]>([]);
  const [referenceUrlInput, setReferenceUrlInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [busyLabel, setBusyLabel] = useState('');
  const [imageBusy, setImageBusy] = useState(false);
  const [imageBusyLabel, setImageBusyLabel] = useState('');
  const [referenceBusy, setReferenceBusy] = useState(false);
  const [referenceBusyLabel, setReferenceBusyLabel] = useState('');
  const [message, setMessage] = useState<string | undefined>();
  const rows = useMemo(() => items.filter((item) => (site === 'all' || item.site === site) && (type === 'all' || item.type === type)), [items, site, type]);
  const articleImages = useMemo(() => new Map(apiArticles.map((article) => [article.id, article.image_url || ''])), [apiArticles]);
  const articleImageCounts = useMemo(() => new Map(apiArticles.map((article) => [article.id, articleImageSeries(article).filter(Boolean).length])), [apiArticles]);

  const refresh = () => listArticles().then((articles) => {
    setApiArticles(articles);
    setItems(articles.map(articleFromApi));
    setSource('api');
    setSourceError(undefined);
  });

  useEffect(() => {
    void refresh()
      .catch((error) => {
        setItems(contentArticles);
        setSource('mock');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, []);

  useEffect(() => {
    void listArticleCategories().then(setCategories).catch(() => setCategories([]));
  }, []);

  const openNew = () => {
    setDraft(draftFromArticle());
    setImages(articleImageSeries());
    setSelectedImage(0);
    setFullscreenImage(null);
    setAiTopic('');
    setAiImagePrompt('');
    setReferenceUrls([]);
    setReferenceUrlInput('');
    setActiveLang('ru');
    setMessage(undefined);
    setEditorOpen(true);
  };

  const openEdit = (id: string) => {
    const article = apiArticles.find((item) => item.id === id);
    if (!article) return;
    setDraft(draftFromArticle(article));
    setImages(articleImageSeries(article));
    setSelectedImage(0);
    setFullscreenImage(null);
    setAiTopic(articleTitle(article));
    setAiImagePrompt(`Editorial food photo for article: ${articleTitle(article)}`);
    setReferenceUrls([]);
    setReferenceUrlInput('');
    setActiveLang('ru');
    setMessage(undefined);
    setEditorOpen(true);
  };

  const patchDraft = (patch: Partial<ArticleDraft>) => setDraft((current) => ({ ...current, ...patch }));

  const generateAi = async () => {
    const topic = aiTopic.trim() || draft.title_ru || draft.title_pl || draft.title_en;
    if (!topic) {
      setMessage('Напиши тему для AI черновика.');
      return;
    }
    setBusy(true);
    setBusyLabel('Gemini пишет...');
    setMessage(undefined);
    try {
      const result = await aiCreateArticleDraft(topic, 4500, 3);
      setDraft((current) => ({
        ...current,
        slug: current.slug || result.slug || slugify(topic),
        category: result.category || current.category,
        title_en: result.title_en || current.title_en,
        title_ru: result.title_ru || current.title_ru,
        title_pl: result.title_pl || current.title_pl,
        title_uk: result.title_uk || current.title_uk,
        content_en: result.content_en || current.content_en,
        content_ru: result.content_ru || current.content_ru,
        content_pl: result.content_pl || current.content_pl,
        content_uk: result.content_uk || current.content_uk,
        seo_title: result.seo_title || current.seo_title,
        seo_description: result.seo_description || current.seo_description,
        seo_title_en: result.seo_title_en || result.seo_title || current.seo_title_en,
        seo_title_ru: result.seo_title_ru || result.seo_title || current.seo_title_ru,
        seo_title_pl: result.seo_title_pl || result.seo_title || current.seo_title_pl,
        seo_title_uk: result.seo_title_uk || result.seo_title || current.seo_title_uk,
        seo_description_en: result.seo_description_en || result.seo_description || current.seo_description_en,
        seo_description_ru: result.seo_description_ru || result.seo_description || current.seo_description_ru,
        seo_description_pl: result.seo_description_pl || result.seo_description || current.seo_description_pl,
        seo_description_uk: result.seo_description_uk || result.seo_description || current.seo_description_uk
      }));
      setImages(articleImageSeries({ image_url: '', content_ru: '', content_en: '', content_pl: '', content_uk: '' } as CmsArticle));
      setAiImagePrompt(result.image_prompts[0] || `Editorial food photo for article: ${topic}`);
      setMessage(`AI создал черновик. Промптов фото: ${result.image_prompts.length}.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не вернул черновик.');
    } finally {
      setBusy(false);
      setBusyLabel('');
    }
  };

  const saveDraft = async () => {
    const title = draft.title_ru || draft.title_pl || draft.title_en || draft.title_uk;
    if (!title.trim()) {
      setMessage('Нужен хотя бы один заголовок.');
      return;
    }
    const nextDraft = { ...draft, slug: draft.slug || slugify(title) };
    setDraft(nextDraft);
    setBusy(true);
    setBusyLabel('Сохраняем...');
    setMessage(undefined);
    try {
      const previousSlug = nextDraft.id ? apiArticles.find((article) => article.id === nextDraft.id)?.slug : undefined;
      let saved: CmsArticle;
      if (nextDraft.id) {
        saved = await updateArticle(nextDraft.id, payloadFromDraft(nextDraft, images));
      } else {
        saved = await createArticle(payloadFromDraft(nextDraft, images));
      }
      await revalidateSite({ type: 'article', slug: saved.slug || nextDraft.slug });
      if (previousSlug && previousSlug !== saved.slug) {
        await revalidateSite({ type: 'article', slug: previousSlug });
      }
      await refresh();
      setEditorOpen(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить материал.');
    } finally {
      setBusy(false);
      setBusyLabel('');
    }
  };

  const uploadPhoto = async (file: File | null) => {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setMessage('Файл должен быть изображением.');
      return;
    }
    setImageBusy(true);
    setImageBusyLabel('Загружаем фото...');
    setMessage(undefined);
    try {
      const url = await uploadCmsReference(file);
      setImages((current) => current.map((item, index) => index === selectedImage ? url : item));
      if (selectedImage === 0) patchDraft({ image_url: url });
      setMessage('Фото загружено в R2 и добавлено в материал.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото.');
    } finally {
      setImageBusy(false);
      setImageBusyLabel('');
    }
  };

  const generatePhoto = async () => {
    const title = draft.title_en || draft.title_ru || draft.title_pl || draft.title_uk || aiTopic;
    if (!title.trim()) {
      setMessage('Нужен заголовок или тема для генерации фото.');
      return;
    }
    setImageBusy(true);
    setImageBusyLabel(`Gemini фото ${selectedImage + 1}...`);
    setMessage(undefined);
    try {
      const result = await aiGenerateArticleImage(title, aiImagePrompt || undefined, selectedImage, false, referenceUrls, 'flash', 'editorial', { photoScenarios: [] });
      setImages((current) => current.map((item, index) => index === selectedImage ? result.image_url : item));
      if (selectedImage === 0) patchDraft({ image_url: result.image_url });
      setMessage(`AI фото ${selectedImage + 1} создано.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не создал фото.');
    } finally {
      setImageBusy(false);
      setImageBusyLabel('');
    }
  };

  const generatePhotoSeries = async () => {
    const title = draft.title_en || draft.title_ru || draft.title_pl || draft.title_uk || aiTopic;
    if (!title.trim()) {
      setMessage('Нужен заголовок или тема для генерации фото.');
      return;
    }
    setImageBusy(true);
    setImageBusyLabel(`Gemini 1/${ARTICLE_IMAGE_COUNT}...`);
    setMessage(undefined);
    try {
      const next: string[] = [];
      for (let index = 0; index < ARTICLE_IMAGE_COUNT; index += 1) {
        const prompt = index === 0
          ? aiImagePrompt || `Editorial food cover photo for article: ${title}`
          : `${aiImagePrompt || title}. Step ${index}: detailed supporting article photo, no text, realistic food editorial.`;
        const result = await aiGenerateArticleImage(title, prompt, index, false, referenceUrls, 'flash', 'editorial', { photoScenarios: [] });
        next.push(result.image_url);
        setImageBusyLabel(`Gemini ${Math.min(index + 2, ARTICLE_IMAGE_COUNT)}/${ARTICLE_IMAGE_COUNT}...`);
        setImages([...next, ...Array(ARTICLE_IMAGE_COUNT - next.length).fill('')]);
      }
      patchDraft({ image_url: next[0] || '' });
      setSelectedImage(0);
      setMessage('AI создал серию из 4 фото.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не создал 4 фото.');
    } finally {
      setImageBusy(false);
      setImageBusyLabel('');
    }
  };

  const generateAdditionalPhoto = async () => {
    if (images.length >= ARTICLE_IMAGE_MAX) {
      setMessage(`Максимум ${ARTICLE_IMAGE_MAX} фото в галерее.`);
      return;
    }
    const title = draft.title_en || draft.title_ru || draft.title_pl || draft.title_uk || aiTopic;
    if (!title.trim()) {
      setMessage('Нужен заголовок или тема для генерации фото.');
      return;
    }
    const index = images.length;
    setImages((current) => [...current, '']);
    setSelectedImage(index);
    setImageBusy(true);
    setImageBusyLabel(`Gemini + фото ${index + 1}...`);
    setMessage(undefined);
    try {
      const prompt = `${aiImagePrompt || title}. Additional gallery photo ${index + 1}, realistic editorial food image, no text.`;
      const result = await aiGenerateArticleImage(title, prompt, index, false, referenceUrls, 'flash', 'editorial', { photoScenarios: [] });
      setImages((current) => current.map((item, itemIndex) => itemIndex === index ? result.image_url : item));
      setMessage(`AI добавил фото ${index + 1}.`);
    } catch (error) {
      setImages((current) => current.filter((_, itemIndex) => itemIndex !== index));
      setSelectedImage(Math.max(0, index - 1));
      setMessage(error instanceof Error ? error.message : 'AI не создал дополнительное фото.');
    } finally {
      setImageBusy(false);
      setImageBusyLabel('');
    }
  };

  const setImageAt = (index: number, url: string) => {
    setImages((current) => current.map((item, itemIndex) => itemIndex === index ? url : item));
    if (index === 0) patchDraft({ image_url: url });
  };

  const addReferenceFiles = async (files: FileList | null) => {
    if (!files) return;
    const slots = ARTICLE_REFERENCE_MAX - referenceUrls.length;
    const selected = Array.from(files).filter((file) => file.type.startsWith('image/')).slice(0, slots);
    if (!selected.length) {
      setMessage(`Можно добавить максимум ${ARTICLE_REFERENCE_MAX} референса.`);
      return;
    }
    setReferenceBusy(true);
    setReferenceBusyLabel('Загружаем референсы...');
    setMessage(undefined);
    try {
      const uploaded: string[] = [];
      for (const file of selected) {
        if (file.size > 10 * 1024 * 1024) throw new Error('Каждый референс должен быть меньше 10 MB');
        uploaded.push(await uploadCmsReference(file));
      }
      setReferenceUrls((current) => [...current, ...uploaded].slice(0, ARTICLE_REFERENCE_MAX));
      setMessage(`Добавлено референсов: ${uploaded.length}.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить референсы.');
    } finally {
      setReferenceBusy(false);
      setReferenceBusyLabel('');
    }
  };

  const addReferenceUrl = () => {
    const url = referenceUrlInput.trim();
    if (!url) return;
    if (!/^https?:\/\//i.test(url)) {
      setMessage('URL референса должен начинаться с http:// или https://');
      return;
    }
    setReferenceUrls((current) => {
      if (current.includes(url)) return current;
      return [...current, url].slice(0, ARTICLE_REFERENCE_MAX);
    });
    setReferenceUrlInput('');
  };

  const removeReferenceUrl = (index: number) => {
    setReferenceUrls((current) => current.filter((_, itemIndex) => itemIndex !== index));
  };

  const addImageSlot = () => {
    setImages((current) => {
      if (current.length >= ARTICLE_IMAGE_MAX) return current;
      setSelectedImage(current.length);
      return [...current, ''];
    });
  };

  const removeSelectedImage = () => {
    setImages((current) => {
      if (current.length <= ARTICLE_IMAGE_COUNT) {
        const next = current.map((item, index) => index === selectedImage ? '' : item);
        if (selectedImage === 0) patchDraft({ image_url: '' });
        return next;
      }
      const next = current.filter((_, index) => index !== selectedImage);
      const nextSelected = Math.min(selectedImage, next.length - 1);
      setSelectedImage(nextSelected);
      if (selectedImage === 0) patchDraft({ image_url: next[0] || '' });
      return next;
    });
  };

  const removeDraft = async () => {
    if (!draft.id) return;
    setBusy(true);
    setBusyLabel('Удаляем...');
    setMessage(undefined);
    try {
      await deleteArticle(draft.id);
      await revalidateSite({ type: 'article', slug: draft.slug });
      await refresh();
      setEditorOpen(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось удалить материал.');
    } finally {
      setBusy(false);
      setBusyLabel('');
    }
  };

  const titleKey = `title_${activeLang}` as keyof ArticleDraft;
  const contentKey = `content_${activeLang}` as keyof ArticleDraft;
  const seoTitleKey = `seo_title_${activeLang}` as keyof ArticleDraft;
  const seoDescriptionKey = `seo_description_${activeLang}` as keyof ArticleDraft;

  return (
    <section className="ops-page">
      <Header title="Контент" subtitle="Статьи, обзоры, сравнения, подборки и связка с партнерскими товарами." icon="cms" source={source} onCreate={openNew} />
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул контент: {sourceError}. Показаны mock-данные.</p> : null}
      <div className="filter-bar"><select value={site} onChange={(event) => setSite(event.target.value as SiteKey | 'all')}><option value="all">Все сайты</option><option value="culinary">Кулинарный</option><option value="construction">Строительный</option></select><select value={type} onChange={(event) => setType(event.target.value as ContentType | 'all')}><option value="all">Все типы</option><option value="article">статья</option><option value="review">обзор</option><option value="comparison">сравнение</option><option value="roundup">подборка</option><option value="recipe">рецепт</option></select></div>
      <section className="ops-panel"><table className="ops-table"><thead><tr><th>Фото</th><th>Материал</th><th>Сайт</th><th>Тип</th><th>Партнерские товары</th><th>Статус</th><th /></tr></thead><tbody>{rows.map((item) => {
        const imageUrl = articleImages.get(item.id);
        return <tr key={item.id}><td>{imageUrl ? <img className="catalog-product-thumb" src={imageUrl} alt={item.title.ru} loading="lazy" /> : <span className="catalog-product-thumb empty"><AppIcon name="cms" size={18} /></span>}</td><td><strong>{item.title.ru}</strong><small>{item.slug}</small><small>Фото: {articleImageCounts.get(item.id) ?? 0}/{Math.max(ARTICLE_IMAGE_COUNT, articleImageCounts.get(item.id) ?? 0)}</small></td><td>{siteLabel(item.site)}</td><td>{contentTypeLabels[item.type]}</td><td>{item.affiliateProductIds.join(', ') || 'нет'}</td><td><span className="status-pill info"><i />{publishStatusLabels[item.status as PublishStatus]}</span></td><td><button className="table-action" type="button" onClick={() => openEdit(item.id)}>Редактировать</button></td></tr>;
      })}</tbody></table></section>
      {editorOpen ? (
        <div className="modal-overlay">
          <div className="editor-modal content-editor-modal">
            <div className="editor-modal-head">
              <div><p className="eyebrow">{draft.id ? 'Редактирование' : 'Новый материал'}</p><h2>{draft.id ? draft.title_ru || draft.slug : 'AI контент'}</h2></div>
              <div className="editor-actions">
                <button className="btn btn-quiet" type="button" onClick={() => setEditorOpen(false)}>Закрыть</button>
                <button className="btn btn-primary" type="button" disabled={busy} onClick={saveDraft}>{busyLabel || 'Сохранить'}</button>
              </div>
            </div>

            <div className="gemini-bar">
              <label><span>Тема для AI</span><textarea value={aiTopic} onChange={(event) => setAiTopic(event.target.value)} placeholder="Например: Лучшие ножи для рыбы: обзор, сравнение, affiliate блоки и SEO" /></label>
              <button className="btn btn-ai" type="button" disabled={busy} onClick={generateAi}><AppIcon name="bot" />{busyLabel || 'AI черновик'}</button>
            </div>

            <div className="editor-grid">
              <label className="editor-field"><span>Slug</span><input value={draft.slug} onChange={(event) => patchDraft({ slug: slugify(event.target.value) })} placeholder="how-to-choose-fish-knife" /></label>
              <label className="editor-field"><span>Категория</span><select value={draft.category} onChange={(event) => patchDraft({ category: event.target.value })}>{categories.length ? categories.map((category) => <option key={category.slug} value={category.slug}>{category.title_ru || category.title_pl || category.slug}</option>) : <><option value="general">general</option><option value="fish">fish</option><option value="tools">tools</option><option value="review">review</option></>}</select></label>
              <label className="editor-field"><span>Фото URL {selectedImage + 1}</span><input value={images[selectedImage] || ''} onChange={(event) => setImageAt(selectedImage, event.target.value)} placeholder="https://..." /></label>
              <label className="editor-check"><input type="checkbox" checked={draft.published} onChange={(event) => patchDraft({ published: event.target.checked })} />Опубликовано</label>
            </div>

            <section className="content-photo-panel">
              <button className="content-photo-preview" type="button" onClick={() => images[selectedImage] ? setFullscreenImage(selectedImage) : undefined}>
                {images[selectedImage] ? <img src={images[selectedImage]} alt={draft.title_ru || draft.slug || 'Фото материала'} onError={() => setMessage('Фото URL не загрузился в браузере. Проверь, что R2 public URL доступен без авторизации.')} /> : <span><AppIcon name="cms" size={34} />Фото {selectedImage + 1} не выбрано</span>}
              </button>
              <div className="content-photo-tools">
                <div className="content-image-strip">
                  {images.map((url, index) => <button key={index} className={selectedImage === index ? 'active' : ''} type="button" onClick={() => setSelectedImage(index)}>{url ? <img src={url} alt={`Фото ${index + 1}`} /> : <span>{index + 1}</span>}<small>{index === 0 ? 'Обложка' : `Фото ${index + 1}`}</small></button>)}
                  {images.length < ARTICLE_IMAGE_MAX ? <button className="content-image-add" type="button" onClick={addImageSlot}><span>+</span><small>Фото</small></button> : null}
                </div>
                <label className="editor-field"><span>Промпт фото</span><textarea value={aiImagePrompt} onChange={(event) => setAiImagePrompt(event.target.value)} placeholder="Editorial food photo, crispy fish skin, professional kitchen lighting" /></label>
                <section className="content-reference-panel">
                  <div className="panel-title compact"><span><AppIcon name="external" />Референсы для AI</span><small>{referenceUrls.length}/{ARTICLE_REFERENCE_MAX}</small></div>
                  <div className="content-reference-actions">
                    <label className="btn btn-secondary"><input className="visually-hidden" type="file" accept="image/*" multiple disabled={referenceBusy || referenceUrls.length >= ARTICLE_REFERENCE_MAX} onChange={(event) => void addReferenceFiles(event.target.files)} />{referenceBusyLabel || 'Загрузить с ПК'}</label>
                    <div className="content-reference-url"><input value={referenceUrlInput} onChange={(event) => setReferenceUrlInput(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') addReferenceUrl(); }} placeholder="https://... фото-референс" /><button className="btn btn-quiet" type="button" disabled={referenceUrls.length >= ARTICLE_REFERENCE_MAX} onClick={addReferenceUrl}>URL</button></div>
                  </div>
                  {referenceUrls.length ? <div className="content-reference-strip">{referenceUrls.map((url, index) => <div key={`${url}-${index}`} className="content-reference-thumb"><img src={url} alt={`Референс ${index + 1}`} /><button type="button" onClick={() => removeReferenceUrl(index)}>×</button></div>)}</div> : <p className="editor-message">AI будет генерировать без визуального референса.</p>}
                </section>
                <div className="editor-actions">
                  <label className="btn btn-secondary"><input className="visually-hidden" type="file" accept="image/*" disabled={imageBusy} onChange={(event) => void uploadPhoto(event.target.files?.[0] ?? null)} />{imageBusyLabel === 'Загружаем фото...' ? imageBusyLabel : 'Загрузить'}</label>
                  <button className="btn btn-ai" type="button" disabled={imageBusy} onClick={generatePhoto}><AppIcon name="bot" />{imageBusyLabel.startsWith('Gemini фото') ? imageBusyLabel : 'AI фото'}</button>
                  <button className="btn btn-ai" type="button" disabled={imageBusy} onClick={generatePhotoSeries}><AppIcon name="bot" />{imageBusyLabel.startsWith('Gemini ') && imageBusyLabel.includes('/') ? imageBusyLabel : 'AI 4 фото'}</button>
                  <button className="btn btn-ai" type="button" disabled={imageBusy || images.length >= ARTICLE_IMAGE_MAX} onClick={generateAdditionalPhoto}><AppIcon name="bot" />{imageBusyLabel.startsWith('Gemini +') ? imageBusyLabel : 'AI + фото'}</button>
                  <button className="btn btn-quiet" type="button" disabled={imageBusy || !images[selectedImage]} onClick={() => setFullscreenImage(selectedImage)}>На весь экран</button>
                  <button className="btn btn-quiet" type="button" disabled={imageBusy || (!images[selectedImage] && images.length <= ARTICLE_IMAGE_COUNT)} onClick={removeSelectedImage}>Убрать</button>
                </div>
              </div>
            </section>

            <div className="analytics-mode-switcher content-lang-tabs">
              {languages.map((language) => <button key={language.key} className={activeLang === language.key ? 'analytics-mode-button active' : 'analytics-mode-button'} type="button" onClick={() => setActiveLang(language.key)}>{language.label}</button>)}
            </div>

            <div className="editor-grid">
              <label className="editor-field"><span>Заголовок {activeLang.toUpperCase()}</span><input value={String(draft[titleKey] ?? '')} onChange={(event) => patchDraft({ [titleKey]: event.target.value })} /></label>
              <label className="editor-field"><span>SEO title {activeLang.toUpperCase()}</span><input value={String(draft[seoTitleKey] ?? '')} onChange={(event) => patchDraft({ [seoTitleKey]: event.target.value })} /></label>
            </div>
            <label className="editor-field"><span>SEO description {activeLang.toUpperCase()}</span><textarea value={String(draft[seoDescriptionKey] ?? '')} onChange={(event) => patchDraft({ [seoDescriptionKey]: event.target.value })} /></label>
            <label className="editor-field"><span>Текст / Markdown {activeLang.toUpperCase()}</span><textarea className="content-body-editor" value={String(draft[contentKey] ?? '')} onChange={(event) => patchDraft({ [contentKey]: event.target.value })} /></label>

            {message ? <p className="editor-message">{message}</p> : null}
            <div className="editor-actions">
              {draft.id ? <button className="btn btn-danger" type="button" disabled={busy} onClick={removeDraft}>{busyLabel === 'Удаляем...' ? busyLabel : 'Удалить'}</button> : null}
              <button className="btn btn-secondary" type="button" disabled={busy} onClick={() => patchDraft({ slug: slugify(draft.title_ru || draft.title_pl || draft.title_en || draft.title_uk) })}>Сделать slug</button>
              <button className="btn btn-primary" type="button" disabled={busy} onClick={saveDraft}>{busyLabel || 'Сохранить'}</button>
            </div>
          </div>
          {fullscreenImage !== null ? (
            <div className="image-lightbox" onClick={() => setFullscreenImage(null)}>
              <div className="image-lightbox-panel" onClick={(event) => event.stopPropagation()}>
                <div className="image-lightbox-head">
                  <strong>{fullscreenImage === 0 ? 'Обложка' : `Фото ${fullscreenImage + 1}`}</strong>
                  <div className="editor-actions">
                    <button className="btn btn-quiet" type="button" onClick={() => setFullscreenImage((current) => current === null ? null : Math.max(0, current - 1))}>Назад</button>
                    <button className="btn btn-quiet" type="button" onClick={() => setFullscreenImage((current) => current === null ? null : Math.min(images.length - 1, current + 1))}>Вперед</button>
                    <button className="btn btn-primary" type="button" onClick={() => setFullscreenImage(null)}>Закрыть</button>
                  </div>
                </div>
                <div className="image-lightbox-stage">
                  {images[fullscreenImage] ? <img src={images[fullscreenImage]} alt={`Фото ${fullscreenImage + 1}`} /> : <span>Фото не выбрано</span>}
                </div>
                <div className="content-image-strip lightbox-strip">
                  {images.map((url, index) => <button key={index} className={fullscreenImage === index ? 'active' : ''} type="button" onClick={() => setFullscreenImage(index)}>{url ? <img src={url} alt={`Фото ${index + 1}`} /> : <span>{index + 1}</span>}<small>{index === 0 ? 'Обложка' : `Фото ${index + 1}`}</small></button>)}
                </div>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function Header({ title, subtitle, icon, source, onCreate }: { title: string; subtitle: string; icon: 'cms'; source: DataSource; onCreate: () => void }) {
  return <div className="ops-header"><div className="ops-header-icon"><AppIcon name={icon} /></div><div><p className="eyebrow">Публикации</p><h2>{title}</h2><p>{subtitle}</p></div><div className="ops-header-actions"><button className="btn btn-ai" type="button" onClick={onCreate}><AppIcon name="bot" />Создать AI</button><button className="btn btn-primary" type="button" onClick={onCreate}>Новый</button><DataSourceBadge source={source} label="Контент" /></div></div>;
}
