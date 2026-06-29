import { useEffect, useMemo, useState } from 'react';
import { ActionButton } from './ActionButton';
import { AdminPanel } from './AdminPanel';
import { useAdminToast } from './useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import {
  getSitePublishingDraft,
  saveSiteFooter,
  saveSitePage,
  saveSiteSections,
  type SiteFooterDraft,
  type SitePageDraft,
  type SitePageKey,
  type SitePublishingDraft,
  type SiteSectionKey
} from '../../services/admin/sitePublishingService';
import type { SiteId } from '../../types/admin';
import type { LocalizedAdminTextDto } from '../../types/adminApi';
import { FieldError, LanguageTabs, isValidSlug, isValidUrl, useLangTab, type FormErrors } from './forms/formUtils';

const pageLabels: Record<SitePageKey, string> = {
  home: 'Home page',
  about: 'About page'
};

const sectionLabels: Record<SiteSectionKey, string> = {
  services: 'Services sections',
  products: 'Products sections'
};

function localized(value: LocalizedAdminTextDto, lang: 'uk' | 'ru' | 'en') {
  return value[lang] || value.uk || value.ru || value.en || '';
}

function setLocalized(value: LocalizedAdminTextDto, lang: 'uk' | 'ru' | 'en', nextValue: string): LocalizedAdminTextDto {
  return { ...value, [lang]: nextValue };
}

function itemsToText(items: LocalizedAdminTextDto[], lang: 'uk' | 'ru' | 'en') {
  return items.map((item) => localized(item, lang)).join('\n');
}

function textToItems(value: string, lang: 'uk' | 'ru' | 'en', current: LocalizedAdminTextDto[]) {
  return value.split('\n').map((line, index) => setLocalized(current[index] || {}, lang, line)).filter((item) => localized(item, lang).trim());
}

export function SitePagesEditor() {
  const { activeSiteId, activeSite } = useActiveSite();
  const toast = useAdminToast();
  const [lang, setLang] = useLangTab();
  const [draft, setDraft] = useState<SitePublishingDraft | null>(null);
  const [activePage, setActivePage] = useState<SitePageKey>('home');
  const [activeSection, setActiveSection] = useState<SiteSectionKey>('services');
  const [errors, setErrors] = useState<FormErrors>({});
  const siteId = activeSiteId as SiteId;

  useEffect(() => {
    void getSitePublishingDraft(siteId).then(setDraft);
  }, [siteId]);

  const page = draft?.pages[activePage] || null;
  const previewTitle = useMemo(() => page ? localized(page.title, lang) : '', [lang, page]);

  function updatePage(nextPage: SitePageDraft) {
    if (!draft) return;
    setDraft({ ...draft, pages: { ...draft.pages, [nextPage.key]: nextPage } });
  }

  function updateFooter(nextFooter: SiteFooterDraft) {
    if (!draft) return;
    setDraft({ ...draft, footer: nextFooter });
  }

  function validatePage(current: SitePageDraft) {
    const next: FormErrors = {};
    if (!localized(current.title, lang).trim()) next.title = 'Заполните заголовок для активного языка.';
    if (!isValidSlug(current.slug)) next.slug = 'Slug: lowercase letters, цифры и дефисы.';
    if (!isValidUrl(current.imageUrl)) next.imageUrl = 'Введите корректный URL изображения.';
    setErrors(next);
    return !Object.keys(next).length;
  }

  async function handleSavePage(status?: SitePageDraft['status']) {
    if (!page) return;
    const nextPage = { ...page, status: status || page.status };
    if (!validatePage(nextPage)) return;
    const nextDraft = await saveSitePage(siteId, nextPage);
    setDraft(nextDraft);
    toast.success(status === 'published' ? 'Page published.' : status === 'draft' ? 'Page unpublished.' : 'Page saved.');
  }

  async function handleSaveSections() {
    if (!draft) return;
    const nextDraft = await saveSiteSections(siteId, draft.sections);
    setDraft(nextDraft);
    toast.success('Sections saved.');
  }

  async function handleSaveFooter() {
    if (!draft) return;
    const nextDraft = await saveSiteFooter(siteId, draft.footer);
    setDraft(nextDraft);
    toast.success('Footer/contact blocks saved.');
  }

  if (!draft || !page) {
    return null;
  }

  const section = draft.sections[activeSection];

  return (
    <div className="site-publishing-grid">
      <AdminPanel title="Site pages editor" icon="home" meta={activeSite.id}>
        <div className="admin-form-grid">
          <div className="admin-form-tabs">
            {(Object.keys(pageLabels) as SitePageKey[]).map((key) => (
              <button key={key} type="button" className={activePage === key ? 'active' : ''} onClick={() => setActivePage(key)}>
                {pageLabels[key]}
              </button>
            ))}
          </div>
          <LanguageTabs active={lang} onChange={setLang} />

          <label>
            <span>Title {lang.toUpperCase()}</span>
            <input value={localized(page.title, lang)} onChange={(event) => updatePage({ ...page, title: setLocalized(page.title, lang, event.target.value) })} />
            <FieldError message={errors.title} />
          </label>
          <label>
            <span>Content {lang.toUpperCase()}</span>
            <textarea value={localized(page.content, lang)} onChange={(event) => updatePage({ ...page, content: setLocalized(page.content, lang, event.target.value) })} />
          </label>
          <label>
            <span>Slug</span>
            <input value={page.slug} onChange={(event) => updatePage({ ...page, slug: event.target.value })} />
            <FieldError message={errors.slug} />
          </label>
          <label>
            <span>Image URL</span>
            <input value={page.imageUrl} onChange={(event) => updatePage({ ...page, imageUrl: event.target.value })} />
            <FieldError message={errors.imageUrl} />
          </label>
          <label>
            <span>SEO title {lang.toUpperCase()}</span>
            <input value={localized(page.seoTitle, lang)} onChange={(event) => updatePage({ ...page, seoTitle: setLocalized(page.seoTitle, lang, event.target.value) })} />
          </label>
          <label>
            <span>SEO description {lang.toUpperCase()}</span>
            <input value={localized(page.seoDescription, lang)} onChange={(event) => updatePage({ ...page, seoDescription: setLocalized(page.seoDescription, lang, event.target.value) })} />
          </label>
          <div className="admin-panel-actions">
            <ActionButton icon="save" onClick={() => void handleSavePage()}>Save</ActionButton>
            <ActionButton tone="primary" icon="deploy" onClick={() => void handleSavePage('published')}>Publish</ActionButton>
            <ActionButton icon="refresh" onClick={() => void handleSavePage('draft')}>Unpublish</ActionButton>
          </div>
        </div>
      </AdminPanel>

      <AdminPanel title="Preview" icon="external" meta={page.status}>
        <article className="publishing-preview">
          {page.imageUrl ? <img src={page.imageUrl} alt="" /> : null}
          <p>{page.slug}</p>
          <h3>{previewTitle || 'Untitled page'}</h3>
          <span>{localized(page.content, lang) || 'No content yet.'}</span>
          <small>{localized(page.seoTitle, lang) || 'No SEO title'}</small>
        </article>
      </AdminPanel>

      <AdminPanel title="Sections" icon="sliders">
        <div className="admin-form-grid">
          <div className="admin-form-tabs">
            {(Object.keys(sectionLabels) as SiteSectionKey[]).map((key) => (
              <button key={key} type="button" className={activeSection === key ? 'active' : ''} onClick={() => setActiveSection(key)}>
                {sectionLabels[key]}
              </button>
            ))}
          </div>
          <label>
            <span>Section title {lang.toUpperCase()}</span>
            <input value={localized(section.title, lang)} onChange={(event) => setDraft({ ...draft, sections: { ...draft.sections, [activeSection]: { ...section, title: setLocalized(section.title, lang, event.target.value) } } })} />
          </label>
          <label>
            <span>Items {lang.toUpperCase()}</span>
            <textarea value={itemsToText(section.items, lang)} onChange={(event) => setDraft({ ...draft, sections: { ...draft.sections, [activeSection]: { ...section, items: textToItems(event.target.value, lang, section.items) } } })} />
          </label>
          <ActionButton icon="save" onClick={() => void handleSaveSections()}>Save sections</ActionButton>
        </div>
      </AdminPanel>

      <AdminPanel title="Footer/contact blocks" icon="cms">
        <div className="admin-form-grid">
          <label><span>Contact title {lang.toUpperCase()}</span><input value={localized(draft.footer.contactTitle, lang)} onChange={(event) => updateFooter({ ...draft.footer, contactTitle: setLocalized(draft.footer.contactTitle, lang, event.target.value) })} /></label>
          <label><span>Contact text {lang.toUpperCase()}</span><textarea value={localized(draft.footer.contactText, lang)} onChange={(event) => updateFooter({ ...draft.footer, contactText: setLocalized(draft.footer.contactText, lang, event.target.value) })} /></label>
          <label><span>Email</span><input value={draft.footer.email} onChange={(event) => updateFooter({ ...draft.footer, email: event.target.value })} /></label>
          <label><span>Phone</span><input value={draft.footer.phone} onChange={(event) => updateFooter({ ...draft.footer, phone: event.target.value })} /></label>
          <label><span>Address</span><input value={draft.footer.address} onChange={(event) => updateFooter({ ...draft.footer, address: event.target.value })} /></label>
          <ActionButton icon="save" onClick={() => void handleSaveFooter()}>Save footer</ActionButton>
        </div>
      </AdminPanel>
    </div>
  );
}
