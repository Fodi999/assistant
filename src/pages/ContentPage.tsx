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

type Language = 'ru' | 'pl' | 'en' | 'uk';

const languages: Array<{ key: Language; label: string }> = [
  { key: 'ru', label: 'RU' },
  { key: 'pl', label: 'PL' },
  { key: 'en', label: 'EN' },
  { key: 'uk', label: 'UK' }
];

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
    content_ru: article?.content_ru ?? '',
    content_pl: article?.content_pl ?? '',
    content_en: article?.content_en ?? '',
    content_uk: article?.content_uk ?? '',
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

function payloadFromDraft(draft: ArticleDraft): Omit<CmsArticle, 'id' | 'updated_at'> {
  return {
    slug: draft.slug,
    category: draft.category,
    image_url: draft.image_url || null,
    published: draft.published,
    title_ru: draft.title_ru || null,
    title_pl: draft.title_pl || null,
    title_en: draft.title_en || null,
    title_uk: draft.title_uk || null,
    content_ru: draft.content_ru || null,
    content_pl: draft.content_pl || null,
    content_en: draft.content_en || null,
    content_uk: draft.content_uk || null,
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
  const [activeLang, setActiveLang] = useState<Language>('ru');
  const [aiTopic, setAiTopic] = useState('');
  const [aiImagePrompt, setAiImagePrompt] = useState('');
  const [busy, setBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [message, setMessage] = useState<string | undefined>();
  const rows = useMemo(() => items.filter((item) => (site === 'all' || item.site === site) && (type === 'all' || item.type === type)), [items, site, type]);
  const articleImages = useMemo(() => new Map(apiArticles.map((article) => [article.id, article.image_url || ''])), [apiArticles]);

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
    setAiTopic('');
    setAiImagePrompt('');
    setActiveLang('ru');
    setMessage(undefined);
    setEditorOpen(true);
  };

  const openEdit = (id: string) => {
    const article = apiArticles.find((item) => item.id === id);
    if (!article) return;
    setDraft(draftFromArticle(article));
    setAiTopic(articleTitle(article));
    setAiImagePrompt(`Editorial food photo for article: ${articleTitle(article)}`);
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
      setAiImagePrompt(result.image_prompts[0] || `Editorial food photo for article: ${topic}`);
      setMessage(`AI создал черновик. Промптов фото: ${result.image_prompts.length}.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не вернул черновик.');
    } finally {
      setBusy(false);
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
    setMessage(undefined);
    try {
      if (nextDraft.id) {
        await updateArticle(nextDraft.id, payloadFromDraft(nextDraft));
      } else {
        await createArticle(payloadFromDraft(nextDraft));
      }
      await refresh();
      setEditorOpen(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить материал.');
    } finally {
      setBusy(false);
    }
  };

  const uploadPhoto = async (file: File | null) => {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setMessage('Файл должен быть изображением.');
      return;
    }
    setImageBusy(true);
    setMessage(undefined);
    try {
      const url = await uploadCmsReference(file);
      patchDraft({ image_url: url });
      setMessage('Фото загружено в R2 и добавлено в материал.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото.');
    } finally {
      setImageBusy(false);
    }
  };

  const generatePhoto = async () => {
    const title = draft.title_en || draft.title_ru || draft.title_pl || draft.title_uk || aiTopic;
    if (!title.trim()) {
      setMessage('Нужен заголовок или тема для генерации фото.');
      return;
    }
    setImageBusy(true);
    setMessage(undefined);
    try {
      const result = await aiGenerateArticleImage(title, aiImagePrompt || undefined, 0, false, [], 'flash', 'editorial', { photoScenarios: [] });
      patchDraft({ image_url: result.image_url });
      setMessage('AI фото создано и поставлено как обложка.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'AI не создал фото.');
    } finally {
      setImageBusy(false);
    }
  };

  const removeDraft = async () => {
    if (!draft.id) return;
    setBusy(true);
    setMessage(undefined);
    try {
      await deleteArticle(draft.id);
      await refresh();
      setEditorOpen(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось удалить материал.');
    } finally {
      setBusy(false);
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
        return <tr key={item.id}><td>{imageUrl ? <img className="catalog-product-thumb" src={imageUrl} alt={item.title.ru} loading="lazy" /> : <span className="catalog-product-thumb empty"><AppIcon name="cms" size={18} /></span>}</td><td><strong>{item.title.ru}</strong><small>{item.slug}</small></td><td>{siteLabel(item.site)}</td><td>{contentTypeLabels[item.type]}</td><td>{item.affiliateProductIds.join(', ') || 'нет'}</td><td><span className="status-pill info"><i />{publishStatusLabels[item.status as PublishStatus]}</span></td><td><button className="table-action" type="button" onClick={() => openEdit(item.id)}>Редактировать</button></td></tr>;
      })}</tbody></table></section>
      {editorOpen ? (
        <div className="modal-overlay">
          <div className="editor-modal content-editor-modal">
            <div className="editor-modal-head">
              <div><p className="eyebrow">{draft.id ? 'Редактирование' : 'Новый материал'}</p><h2>{draft.id ? draft.title_ru || draft.slug : 'AI контент'}</h2></div>
              <div className="editor-actions">
                <button className="btn btn-quiet" type="button" onClick={() => setEditorOpen(false)}>Закрыть</button>
                <button className="btn btn-primary" type="button" disabled={busy} onClick={saveDraft}>Сохранить</button>
              </div>
            </div>

            <div className="gemini-bar">
              <label><span>Тема для AI</span><textarea value={aiTopic} onChange={(event) => setAiTopic(event.target.value)} placeholder="Например: Лучшие ножи для рыбы: обзор, сравнение, affiliate блоки и SEO" /></label>
              <button className="btn btn-ai" type="button" disabled={busy} onClick={generateAi}><AppIcon name="bot" />AI черновик</button>
            </div>

            <div className="editor-grid">
              <label className="editor-field"><span>Slug</span><input value={draft.slug} onChange={(event) => patchDraft({ slug: slugify(event.target.value) })} placeholder="how-to-choose-fish-knife" /></label>
              <label className="editor-field"><span>Категория</span><select value={draft.category} onChange={(event) => patchDraft({ category: event.target.value })}>{categories.length ? categories.map((category) => <option key={category.slug} value={category.slug}>{category.title_ru || category.title_pl || category.slug}</option>) : <><option value="general">general</option><option value="fish">fish</option><option value="tools">tools</option><option value="review">review</option></>}</select></label>
              <label className="editor-field"><span>Фото URL</span><input value={draft.image_url} onChange={(event) => patchDraft({ image_url: event.target.value })} placeholder="https://..." /></label>
              <label className="editor-check"><input type="checkbox" checked={draft.published} onChange={(event) => patchDraft({ published: event.target.checked })} />Опубликовано</label>
            </div>

            <section className="content-photo-panel">
              <div className="content-photo-preview">
                {draft.image_url ? <img src={draft.image_url} alt={draft.title_ru || draft.slug || 'Фото материала'} onError={() => setMessage('Фото URL не загрузился в браузере. Проверь, что R2 public URL доступен без авторизации.')} /> : <span><AppIcon name="cms" size={34} />Фото не выбрано</span>}
              </div>
              <div className="content-photo-tools">
                <label className="editor-field"><span>Промпт фото</span><textarea value={aiImagePrompt} onChange={(event) => setAiImagePrompt(event.target.value)} placeholder="Editorial food photo, crispy fish skin, professional kitchen lighting" /></label>
                <div className="editor-actions">
                  <label className="btn btn-secondary"><input className="visually-hidden" type="file" accept="image/*" onChange={(event) => void uploadPhoto(event.target.files?.[0] ?? null)} />Загрузить</label>
                  <button className="btn btn-ai" type="button" disabled={imageBusy} onClick={generatePhoto}><AppIcon name="bot" />AI фото</button>
                  <button className="btn btn-quiet" type="button" disabled={imageBusy || !draft.image_url} onClick={() => patchDraft({ image_url: '' })}>Убрать</button>
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
              {draft.id ? <button className="btn btn-danger" type="button" disabled={busy} onClick={removeDraft}>Удалить</button> : null}
              <button className="btn btn-secondary" type="button" disabled={busy} onClick={() => patchDraft({ slug: slugify(draft.title_ru || draft.title_pl || draft.title_en || draft.title_uk) })}>Сделать slug</button>
              <button className="btn btn-primary" type="button" disabled={busy} onClick={saveDraft}>Сохранить</button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function Header({ title, subtitle, icon, source, onCreate }: { title: string; subtitle: string; icon: 'cms'; source: DataSource; onCreate: () => void }) {
  return <div className="ops-header"><div className="ops-header-icon"><AppIcon name={icon} /></div><div><p className="eyebrow">Публикации</p><h2>{title}</h2><p>{subtitle}</p></div><div className="ops-header-actions"><button className="btn btn-ai" type="button" onClick={onCreate}><AppIcon name="bot" />Создать AI</button><button className="btn btn-primary" type="button" onClick={onCreate}>Новый</button><DataSourceBadge source={source} label="Контент" /></div></div>;
}
