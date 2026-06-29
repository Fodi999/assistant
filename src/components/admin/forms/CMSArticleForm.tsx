import { useMemo, useState } from 'react';
import type { AdminResourceRow, ResourceStatus } from '../../../types/admin';
import type { CreateCMSPageDto, LocalizedAdminTextDto } from '../../../types/adminApi';
import { FieldError, LanguageTabs, firstText, isValidSlug, isValidUrl, optionalInteger, useLangTab, type FormErrors } from './formUtils';

type BackendArticle = {
  slug?: string | null; category?: string | null; title_uk?: string | null; title_ru?: string | null; title_en?: string | null;
  content_uk?: string | null; content_ru?: string | null; content_en?: string | null; image_url?: string | null; author_name?: string | null;
  seo_title_uk?: string | null; seo_title_ru?: string | null; seo_title_en?: string | null;
  seo_description_uk?: string | null; seo_description_ru?: string | null; seo_description_en?: string | null;
  published?: boolean; order_index?: number | null;
};

type CMSFormState = {
  title: LocalizedAdminTextDto;
  content: LocalizedAdminTextDto;
  seoTitle: LocalizedAdminTextDto;
  seoDescription: LocalizedAdminTextDto;
  slug: string;
  category: string;
  imageUrl: string;
  authorName: string;
  status: ResourceStatus;
  orderIndex: string;
};

function initialState(row?: AdminResourceRow | null): CMSFormState {
  const backend = (row?.backend || {}) as BackendArticle;
  return {
    title: { uk: backend.title_uk || '', ru: backend.title_ru || row?.title || '', en: backend.title_en || '' },
    content: { uk: backend.content_uk || '', ru: backend.content_ru || '', en: backend.content_en || '' },
    seoTitle: { uk: backend.seo_title_uk || '', ru: backend.seo_title_ru || '', en: backend.seo_title_en || '' },
    seoDescription: { uk: backend.seo_description_uk || '', ru: backend.seo_description_ru || '', en: backend.seo_description_en || '' },
    slug: backend.slug || row?.slug || '',
    category: backend.category || row?.type || '',
    imageUrl: backend.image_url || '',
    authorName: backend.author_name || '',
    status: backend.published ? 'published' : row?.status || 'draft',
    orderIndex: backend.order_index?.toString() || ''
  };
}

export function CMSArticleForm({ formId, row, disabled, onSubmit }: { formId: string; row?: AdminResourceRow | null; disabled?: boolean; onSubmit: (payload: CreateCMSPageDto) => void }) {
  const [lang, setLang] = useLangTab();
  const [form, setForm] = useState<CMSFormState>(() => initialState(row));
  const [errors, setErrors] = useState<FormErrors>({});
  const title = useMemo(() => firstText(form.title), [form.title]);

  function validate() {
    const next: FormErrors = {};
    if (!title.trim()) next.title = 'Введите заголовок хотя бы на одном языке.';
    if (!form.title.en?.trim()) next.titleEn = 'Backend требует английский заголовок. Заполните EN.';
    if (!isValidSlug(form.slug)) next.slug = 'Slug: только lowercase letters, цифры и дефисы.';
    if (!isValidUrl(form.imageUrl)) next.imageUrl = 'Введите корректный URL изображения.';
    if (form.orderIndex && Number.isNaN(Number(form.orderIndex))) next.orderIndex = 'Введите целое число.';
    setErrors(next);
    return !Object.keys(next).length;
  }

  return (
    <form id={formId} className="admin-form-grid" onSubmit={(event) => {
      event.preventDefault();
      if (!validate()) return;
      onSubmit({
        siteId: row?.siteId || 'construction',
        title,
        localizedTitle: form.title,
        slug: form.slug.trim() || undefined,
        type: form.category.trim() || undefined,
        contentLocalized: form.content,
        imageUrl: form.imageUrl.trim() || undefined,
        authorName: form.authorName.trim() || undefined,
        status: form.status,
        seoTitle: form.seoTitle,
        seoDescription: form.seoDescription,
        orderIndex: optionalInteger(form.orderIndex)
      });
    }}>
      <LanguageTabs active={lang} onChange={setLang} />
      <label><span>Title {lang.toUpperCase()}</span><input disabled={disabled} value={form.title[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, title: { ...current.title, [lang]: event.target.value } }))} /><FieldError message={errors.title || (lang === 'en' ? errors.titleEn : undefined)} /></label>
      <label><span>Content {lang.toUpperCase()}</span><textarea disabled={disabled} value={form.content[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, content: { ...current.content, [lang]: event.target.value } }))} /></label>
      <label><span>SEO title {lang.toUpperCase()}</span><input disabled={disabled} value={form.seoTitle[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, seoTitle: { ...current.seoTitle, [lang]: event.target.value } }))} /></label>
      <label><span>SEO description {lang.toUpperCase()}</span><input disabled={disabled} value={form.seoDescription[lang] || ''} onChange={(event) => setForm((current) => ({ ...current, seoDescription: { ...current.seoDescription, [lang]: event.target.value } }))} /></label>
      <label><span>Slug</span><input disabled={disabled} value={form.slug} onChange={(event) => setForm((current) => ({ ...current, slug: event.target.value }))} /><FieldError message={errors.slug} /></label>
      <label><span>Category</span><input disabled={disabled} value={form.category} onChange={(event) => setForm((current) => ({ ...current, category: event.target.value }))} /></label>
      <label><span>Status</span><select disabled={disabled} value={form.status} onChange={(event) => setForm((current) => ({ ...current, status: event.target.value as ResourceStatus }))}><option value="draft">draft</option><option value="published">published</option></select></label>
      <label><span>Image URL</span><input disabled={disabled} value={form.imageUrl} onChange={(event) => setForm((current) => ({ ...current, imageUrl: event.target.value }))} /><FieldError message={errors.imageUrl} /></label>
      <label><span>Author</span><input disabled={disabled} value={form.authorName} onChange={(event) => setForm((current) => ({ ...current, authorName: event.target.value }))} /></label>
      <label><span>Order index</span><input disabled={disabled} value={form.orderIndex} onChange={(event) => setForm((current) => ({ ...current, orderIndex: event.target.value }))} /><FieldError message={errors.orderIndex} /></label>
      <article className="publishing-preview cms-article-preview">
        {form.imageUrl && isValidUrl(form.imageUrl) ? <img src={form.imageUrl} alt="" /> : null}
        <p>{form.status} / {form.category || 'Article'} / {form.slug || 'no-slug'}</p>
        <h3>{form.title[lang] || title || 'Untitled article'}</h3>
        <span>{form.content[lang] || 'No content for this language yet.'}</span>
        <small>{form.seoTitle[lang] || 'No SEO title'}</small>
      </article>
    </form>
  );
}
