import { useEffect, useMemo, useState } from 'react';
import {
  createExperience,
  createExpertise,
  createGalleryItem,
  deleteExperience,
  deleteExpertise,
  deleteGalleryItem,
  getAboutPage,
  listExperience,
  listExpertise,
  listGallery,
  listGalleryCategories,
  updateAboutPage,
  updateExperience,
  updateExpertise,
  updateGalleryItem,
  uploadAboutPhoto,
  uploadGalleryPhoto,
  type AboutPageContent,
  type ExperienceItem,
  type ExpertiseItem,
  type GalleryCategory,
  type GalleryItem
} from '../../api/cms';
import { revalidateSite } from '../../api/revalidate';
import { AppIcon } from '../AppIcon';
import { ActionButton } from './ActionButton';
import { AdminPanel } from './AdminPanel';
import { FieldError, LanguageTabs, isValidUrl, useLangTab, type FormErrors, type LangTab } from './forms/formUtils';

const emptyText = { title_en: '', title_pl: '', title_ru: '', title_uk: '' };
const emptyLongText = { description_en: '', description_pl: '', description_ru: '', description_uk: '' };

function localized(value: unknown, key: string, lang: LangTab) {
  const record = value as Record<string, string | null | undefined>;
  return record[`${key}_${lang}`] || record[`${key}_ru`] || record[`${key}_pl`] || record[`${key}_en`] || record[`${key}_uk`] || '';
}

function patchLocalized<T>(value: T, key: string, lang: LangTab, nextValue: string): T {
  return { ...value, [`${key}_${lang}`]: nextValue };
}

function newGalleryItem(category?: GalleryCategory): GalleryItem {
  return {
    id: '',
    image_url: '',
    category_id: category?.id ?? null,
    category_slug: category?.slug ?? 'kitchen',
    slug: '',
    status: 'published',
    ...emptyText,
    ...emptyLongText,
    alt_en: '',
    alt_pl: '',
    alt_ru: '',
    alt_uk: '',
    order_index: 0
  };
}

function newExpertiseItem(): ExpertiseItem {
  return {
    id: '',
    icon: '',
    ...emptyText,
    order_index: 0
  };
}

function newExperienceItem(): ExperienceItem {
  return {
    id: '',
    restaurant: '',
    country: '',
    position: '',
    start_year: null,
    end_year: null,
    ...emptyLongText,
    order_index: 0
  };
}

function numberOrNull(value: string | number | null | undefined) {
  if (value === null || value === undefined || value === '') return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function isImageUrl(value?: string | null) {
  return /^https?:\/\//i.test(String(value || ''));
}

export function KitchenCmsSections() {
  const [lang, setLang] = useLangTab();
  const [about, setAbout] = useState<AboutPageContent | null>(null);
  const [gallery, setGallery] = useState<GalleryItem[]>([]);
  const [galleryCategories, setGalleryCategories] = useState<GalleryCategory[]>([]);
  const [galleryDraft, setGalleryDraft] = useState<GalleryItem | null>(null);
  const [experience, setExperience] = useState<ExperienceItem[]>([]);
  const [experienceDraft, setExperienceDraft] = useState<ExperienceItem | null>(null);
  const [expertise, setExpertise] = useState<ExpertiseItem[]>([]);
  const [expertiseDraft, setExpertiseDraft] = useState<ExpertiseItem | null>(null);
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const kitchenCategory = useMemo(() => galleryCategories.find((category) => category.slug === 'kitchen') || galleryCategories[0], [galleryCategories]);

  async function load() {
    setLoadError(null);
    try {
      const [nextAbout, nextGallery, nextGalleryCategories, nextExperience, nextExpertise] = await Promise.all([
        getAboutPage(),
        listGallery(),
        listGalleryCategories(),
        listExperience(),
        listExpertise()
      ]);
      setAbout(nextAbout);
      setGallery(nextGallery);
      setGalleryCategories(nextGalleryCategories);
      setExperience(nextExperience);
      setExpertise(nextExpertise);
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : 'Не удалось загрузить Kitchen CMS.');
    }
  }

  useEffect(() => {
    void load();
  }, []);

  async function saveAbout() {
    if (!about) return;
    const nextErrors: FormErrors = {};
    if (!localized(about, 'title', lang).trim()) nextErrors.aboutTitle = 'Заполните заголовок.';
    if (!isValidUrl(about.image_url || '')) nextErrors.aboutImage = 'Некорректный URL фото.';
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) return;

    setBusy('about');
    setMessage(null);
    try {
      const saved = await updateAboutPage(about);
      await revalidateSite({ type: 'about' });
      setAbout(saved);
      setMessage('About сохранён и отправлен на сайт.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить About.');
    } finally {
      setBusy('');
    }
  }

  async function uploadAbout(file: File | null) {
    if (!file || !about) return;
    setBusy('about-photo');
    try {
      const url = await uploadAboutPhoto(file);
      setAbout({ ...about, image_url: url });
      setMessage('Фото About загружено. Нажми Save About, чтобы закрепить его в CMS.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото.');
    } finally {
      setBusy('');
    }
  }

  async function saveGalleryDraft() {
    if (!galleryDraft) return;
    const nextErrors: FormErrors = {};
    if (!galleryDraft.image_url.trim()) nextErrors.galleryImage = 'Нужен URL фото.';
    if (!isValidUrl(galleryDraft.image_url)) nextErrors.galleryImage = 'Некорректный URL фото.';
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) return;

    setBusy('gallery');
    setMessage(null);
    try {
      const payload = {
        ...galleryDraft,
        title_en: galleryDraft.title_en || galleryDraft.title_ru || galleryDraft.slug || 'Gallery item',
        title_pl: galleryDraft.title_pl || galleryDraft.title_ru || galleryDraft.title_en,
        title_ru: galleryDraft.title_ru || galleryDraft.title_pl || galleryDraft.title_en,
        title_uk: galleryDraft.title_uk || galleryDraft.title_ru || galleryDraft.title_en,
        alt_en: galleryDraft.alt_en || galleryDraft.title_en || galleryDraft.title_ru,
        alt_pl: galleryDraft.alt_pl || galleryDraft.title_pl || galleryDraft.title_ru,
        alt_ru: galleryDraft.alt_ru || galleryDraft.title_ru || galleryDraft.title_en,
        alt_uk: galleryDraft.alt_uk || galleryDraft.title_uk || galleryDraft.title_ru,
        order_index: Number(galleryDraft.order_index) || 0
      };
      const { id, updated_at, category_slug, ...galleryPayload } = payload;
      const saved = galleryDraft.id
        ? await updateGalleryItem(galleryDraft.id, galleryPayload)
        : await createGalleryItem(galleryPayload);
      await revalidateSite({ type: 'gallery' });
      setGallery((current) => galleryDraft.id ? current.map((item) => item.id === saved.id ? saved : item) : [...current, saved]);
      setGalleryDraft(saved);
      setMessage('Избранная работа сохранена.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить работу.');
    } finally {
      setBusy('');
    }
  }

  async function uploadGallery(file: File | null) {
    if (!file || !galleryDraft) return;
    setBusy('gallery-photo');
    try {
      const url = await uploadGalleryPhoto(file);
      setGalleryDraft({ ...galleryDraft, image_url: url });
      setMessage('Фото работы загружено. Нажми Save work, чтобы сохранить запись.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото.');
    } finally {
      setBusy('');
    }
  }

  async function removeGallery(item: GalleryItem) {
    if (!window.confirm(`Удалить работу "${item.title_ru || item.title_en || item.slug}"?`)) return;
    setBusy(`gallery-${item.id}`);
    try {
      await deleteGalleryItem(item.id);
      await revalidateSite({ type: 'gallery' });
      setGallery((current) => current.filter((row) => row.id !== item.id));
      if (galleryDraft?.id === item.id) setGalleryDraft(null);
      setMessage('Работа удалена.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось удалить работу.');
    } finally {
      setBusy('');
    }
  }

  async function saveExpertiseDraft() {
    if (!expertiseDraft) return;
    setBusy('expertise');
    setMessage(null);
    try {
      const payload = {
        ...expertiseDraft,
        title_en: expertiseDraft.title_en || expertiseDraft.title_ru || 'Expertise',
        title_pl: expertiseDraft.title_pl || expertiseDraft.title_ru || expertiseDraft.title_en,
        title_ru: expertiseDraft.title_ru || expertiseDraft.title_pl || expertiseDraft.title_en,
        title_uk: expertiseDraft.title_uk || expertiseDraft.title_ru || expertiseDraft.title_en,
        order_index: Number(expertiseDraft.order_index) || 0
      };
      const { id, created_at, updated_at, ...expertisePayload } = payload;
      const saved = expertiseDraft.id
        ? await updateExpertise(expertiseDraft.id, expertisePayload)
        : await createExpertise(expertisePayload);
      await revalidateSite({ type: 'expertise' });
      setExpertise((current) => expertiseDraft.id ? current.map((item) => item.id === saved.id ? saved : item) : [...current, saved]);
      setExpertiseDraft(saved);
      setMessage('Expertise сохранён.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить Expertise.');
    } finally {
      setBusy('');
    }
  }

  async function uploadExpertiseIcon(file: File | null) {
    if (!file || !expertiseDraft) return;
    if (!file.type.startsWith('image/')) {
      setMessage('Файл должен быть изображением.');
      return;
    }
    setBusy('expertise-photo');
    setMessage(null);
    try {
      const url = await uploadGalleryPhoto(file);
      setExpertiseDraft({ ...expertiseDraft, icon: url });
      setMessage('Фото expertise загружено. Нажми Save expertise, чтобы сохранить его.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото expertise.');
    } finally {
      setBusy('');
    }
  }

  async function uploadExpertiseIconForItem(file: File | null, item: ExpertiseItem) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setMessage('Файл должен быть изображением.');
      return;
    }
    setBusy(`expertise-photo-${item.id}`);
    setMessage(null);
    try {
      const url = await uploadGalleryPhoto(file);
      const payload = { ...item, icon: url };
      const { id, created_at, updated_at, ...expertisePayload } = payload;
      const saved = await updateExpertise(item.id, expertisePayload);
      await revalidateSite({ type: 'expertise' });
      setExpertise((current) => current.map((row) => row.id === saved.id ? saved : row));
      if (expertiseDraft?.id === saved.id) setExpertiseDraft(saved);
      setMessage('Фото expertise загружено и сохранено.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось загрузить фото expertise.');
    } finally {
      setBusy('');
    }
  }

  async function removeExpertise(item: ExpertiseItem) {
    if (!window.confirm(`Удалить expertise "${item.title_ru || item.title_en}"?`)) return;
    setBusy(`expertise-${item.id}`);
    try {
      await deleteExpertise(item.id);
      await revalidateSite({ type: 'expertise' });
      setExpertise((current) => current.filter((row) => row.id !== item.id));
      if (expertiseDraft?.id === item.id) setExpertiseDraft(null);
      setMessage('Expertise удалён.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось удалить Expertise.');
    } finally {
      setBusy('');
    }
  }

  async function saveExperienceDraft() {
    if (!experienceDraft) return;
    setBusy('experience');
    setMessage(null);
    try {
      const payload = {
        ...experienceDraft,
        start_year: numberOrNull(experienceDraft.start_year),
        end_year: numberOrNull(experienceDraft.end_year),
        order_index: Number(experienceDraft.order_index) || 0
      };
      const { id, created_at, updated_at, ...experiencePayload } = payload;
      const saved = experienceDraft.id
        ? await updateExperience(experienceDraft.id, experiencePayload)
        : await createExperience(experiencePayload);
      await revalidateSite({ type: 'experience' });
      setExperience((current) => experienceDraft.id ? current.map((item) => item.id === saved.id ? saved : item) : [...current, saved]);
      setExperienceDraft(saved);
      setMessage('Моя работа сохранена.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось сохранить опыт.');
    } finally {
      setBusy('');
    }
  }

  async function removeExperience(item: ExperienceItem) {
    if (!window.confirm(`Удалить опыт "${item.restaurant}"?`)) return;
    setBusy(`experience-${item.id}`);
    try {
      await deleteExperience(item.id);
      await revalidateSite({ type: 'experience' });
      setExperience((current) => current.filter((row) => row.id !== item.id));
      if (experienceDraft?.id === item.id) setExperienceDraft(null);
      setMessage('Опыт удалён.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Не удалось удалить опыт.');
    } finally {
      setBusy('');
    }
  }

  if (loadError) {
    return (
      <AdminPanel title="Kitchen site content" icon="terminal">
        <p className="admin-soft-alert">{loadError}</p>
        <ActionButton icon="refresh" onClick={() => void load()}>Retry</ActionButton>
      </AdminPanel>
    );
  }

  return (
    <section className="kitchen-cms-sections">
      <div className="admin-section-heading">
        <div>
          <p>Kitchen backend CMS</p>
          <h3>Страница, работы и опыт</h3>
        </div>
        <LanguageTabs active={lang} onChange={setLang} />
      </div>

      {message ? <p className="admin-soft-alert">{message}</p> : null}

      {about ? (
        <AdminPanel title="About / hero" icon="home" meta="backend">
          <div className="kitchen-editor-grid">
            <div className="admin-form-grid">
              <label><span>Title {lang.toUpperCase()}</span><input value={localized(about, 'title', lang)} onChange={(event) => setAbout(patchLocalized(about, 'title', lang, event.target.value))} /><FieldError message={errors.aboutTitle} /></label>
              <label><span>Content {lang.toUpperCase()}</span><textarea value={localized(about, 'content', lang)} onChange={(event) => setAbout(patchLocalized(about, 'content', lang, event.target.value))} /></label>
              <label><span>Image URL</span><input value={about.image_url || ''} onChange={(event) => setAbout({ ...about, image_url: event.target.value })} /><FieldError message={errors.aboutImage} /></label>
              <div className="admin-panel-actions">
                <label className="admin-btn secondary"><input className="visually-hidden" type="file" accept="image/*" disabled={Boolean(busy)} onChange={(event) => void uploadAbout(event.target.files?.[0] ?? null)} /><AppIcon name="image" /><span>{busy === 'about-photo' ? 'Uploading' : 'Upload photo'}</span></label>
                <ActionButton tone="primary" icon="save" disabled={Boolean(busy)} onClick={() => void saveAbout()}>{busy === 'about' ? 'Saving' : 'Save About'}</ActionButton>
              </div>
            </div>
            <article className="publishing-preview">
              {about.image_url ? <img src={about.image_url} alt="" /> : null}
              <p>about</p>
              <h3>{localized(about, 'title', lang) || 'Untitled'}</h3>
              <span>{localized(about, 'content', lang) || 'No content'}</span>
            </article>
          </div>
        </AdminPanel>
      ) : null}

      <AdminPanel title="Избранные работы / gallery" icon="image" meta={`${gallery.length} items`}>
        <div className="kitchen-admin-list">
          {gallery.map((item) => (
            <article className={galleryDraft?.id === item.id ? 'active' : ''} key={item.id}>
              {item.image_url ? <img src={item.image_url} alt="" loading="lazy" /> : <span className="admin-row-thumb empty">-</span>}
              <button type="button" onClick={() => setGalleryDraft(item)}><strong>{localized(item, 'title', lang) || item.slug}</strong><small>{item.category_slug || item.status}</small></button>
              <button className="table-action danger" type="button" disabled={Boolean(busy)} onClick={() => void removeGallery(item)}>Delete</button>
            </article>
          ))}
        </div>
        <ActionButton icon="sparkles" onClick={() => setGalleryDraft(newGalleryItem(kitchenCategory))}>New work</ActionButton>
        {galleryDraft ? (
          <div className="kitchen-inline-editor">
            <div className="admin-form-grid">
              <label><span>Title {lang.toUpperCase()}</span><input value={localized(galleryDraft, 'title', lang)} onChange={(event) => setGalleryDraft(patchLocalized(galleryDraft, 'title', lang, event.target.value))} /></label>
              <label><span>Description {lang.toUpperCase()}</span><textarea value={localized(galleryDraft, 'description', lang)} onChange={(event) => setGalleryDraft(patchLocalized(galleryDraft, 'description', lang, event.target.value))} /></label>
              <label><span>Alt {lang.toUpperCase()}</span><input value={localized(galleryDraft, 'alt', lang)} onChange={(event) => setGalleryDraft(patchLocalized(galleryDraft, 'alt', lang, event.target.value))} /></label>
              <label><span>Image URL</span><input value={galleryDraft.image_url} onChange={(event) => setGalleryDraft({ ...galleryDraft, image_url: event.target.value })} /><FieldError message={errors.galleryImage} /></label>
              <label><span>Category</span><select value={galleryDraft.category_id || ''} onChange={(event) => {
                const category = galleryCategories.find((item) => item.id === event.target.value);
                setGalleryDraft({ ...galleryDraft, category_id: category?.id || null, category_slug: category?.slug || null });
              }}>{galleryCategories.map((category) => <option key={category.id} value={category.id}>{category.title_ru || category.slug}</option>)}</select></label>
              <label><span>Status</span><select value={galleryDraft.status} onChange={(event) => setGalleryDraft({ ...galleryDraft, status: event.target.value })}><option value="published">published</option><option value="draft">draft</option></select></label>
              <label><span>Order</span><input value={galleryDraft.order_index} onChange={(event) => setGalleryDraft({ ...galleryDraft, order_index: Number(event.target.value) || 0 })} /></label>
              <div className="admin-panel-actions">
                <label className="admin-btn secondary"><input className="visually-hidden" type="file" accept="image/*" disabled={Boolean(busy)} onChange={(event) => void uploadGallery(event.target.files?.[0] ?? null)} /><AppIcon name="image" /><span>{busy === 'gallery-photo' ? 'Uploading' : 'Upload photo'}</span></label>
                <ActionButton tone="primary" icon="save" disabled={Boolean(busy)} onClick={() => void saveGalleryDraft()}>{busy === 'gallery' ? 'Saving' : 'Save work'}</ActionButton>
              </div>
            </div>
            <article className="publishing-preview">{galleryDraft.image_url ? <img src={galleryDraft.image_url} alt="" /> : null}<p>{galleryDraft.status}</p><h3>{localized(galleryDraft, 'title', lang) || 'Untitled work'}</h3><span>{localized(galleryDraft, 'description', lang) || 'No description'}</span></article>
          </div>
        ) : null}
      </AdminPanel>

      <AdminPanel title="Моя работа / experience" icon="factory" meta={`${experience.length} items`}>
        <div className="kitchen-admin-list text-only">
          {experience.map((item) => (
            <article className={experienceDraft?.id === item.id ? 'active' : ''} key={item.id}>
              <button type="button" onClick={() => setExperienceDraft(item)}><strong>{item.restaurant}</strong><small>{item.position} · {item.country}</small></button>
              <button className="table-action danger" type="button" disabled={Boolean(busy)} onClick={() => void removeExperience(item)}>Delete</button>
            </article>
          ))}
        </div>
        <ActionButton icon="sparkles" onClick={() => setExperienceDraft(newExperienceItem())}>New experience</ActionButton>
        {experienceDraft ? (
          <div className="admin-form-grid kitchen-inline-editor single">
            <label><span>Restaurant</span><input value={experienceDraft.restaurant} onChange={(event) => setExperienceDraft({ ...experienceDraft, restaurant: event.target.value })} /></label>
            <label><span>Country</span><input value={experienceDraft.country} onChange={(event) => setExperienceDraft({ ...experienceDraft, country: event.target.value })} /></label>
            <label><span>Position</span><input value={experienceDraft.position} onChange={(event) => setExperienceDraft({ ...experienceDraft, position: event.target.value })} /></label>
            <label><span>Start year</span><input value={experienceDraft.start_year ?? ''} onChange={(event) => setExperienceDraft({ ...experienceDraft, start_year: numberOrNull(event.target.value) })} /></label>
            <label><span>End year</span><input value={experienceDraft.end_year ?? ''} onChange={(event) => setExperienceDraft({ ...experienceDraft, end_year: numberOrNull(event.target.value) })} /></label>
            <label><span>Order</span><input value={experienceDraft.order_index} onChange={(event) => setExperienceDraft({ ...experienceDraft, order_index: Number(event.target.value) || 0 })} /></label>
            <label><span>Description {lang.toUpperCase()}</span><textarea value={localized(experienceDraft, 'description', lang)} onChange={(event) => setExperienceDraft(patchLocalized(experienceDraft, 'description', lang, event.target.value))} /></label>
            <ActionButton tone="primary" icon="save" disabled={Boolean(busy)} onClick={() => void saveExperienceDraft()}>{busy === 'experience' ? 'Saving' : 'Save experience'}</ActionButton>
          </div>
        ) : null}
      </AdminPanel>

      <AdminPanel title="Expertise" icon="sparkles" meta={`${expertise.length} items`}>
        <div className="kitchen-admin-list text-only">
          {expertise.map((item) => (
            <article className={`${expertiseDraft?.id === item.id ? 'active ' : ''}with-actions`.trim()} key={item.id}>
              <button type="button" onClick={() => setExpertiseDraft(item)}><strong>{isImageUrl(item.icon) ? <img className="kitchen-expertise-thumb" src={item.icon} alt="" loading="lazy" /> : <span className="kitchen-expertise-emoji">{item.icon}</span>} {localized(item, 'title', lang)}</strong><small>#{item.order_index}</small></button>
              <button className="table-action" type="button" disabled={Boolean(busy)} onClick={() => setExpertiseDraft(item)}>Edit</button>
              <label className={`table-action ${busy === `expertise-photo-${item.id}` ? 'disabled' : ''}`.trim()}>
                <input className="visually-hidden" type="file" accept="image/*" disabled={Boolean(busy)} onChange={(event) => void uploadExpertiseIconForItem(event.target.files?.[0] ?? null, item)} />
                {busy === `expertise-photo-${item.id}` ? 'Photo...' : 'Photo'}
              </label>
              <button className="table-action danger" type="button" disabled={Boolean(busy)} onClick={() => void removeExpertise(item)}>Delete</button>
            </article>
          ))}
        </div>
        <ActionButton icon="sparkles" onClick={() => setExpertiseDraft(newExpertiseItem())}>New expertise</ActionButton>
        {expertiseDraft ? (
          <div className="admin-form-grid kitchen-inline-editor single">
            <div className="kitchen-expertise-photo-field">
              {isImageUrl(expertiseDraft.icon) ? <img src={expertiseDraft.icon} alt="" /> : <span>{expertiseDraft.icon || 'Photo'}</span>}
              <div>
                <label><span>Photo URL</span><input value={expertiseDraft.icon} onChange={(event) => setExpertiseDraft({ ...expertiseDraft, icon: event.target.value })} placeholder="https://... или emoji fallback" /></label>
                <label className="admin-btn secondary"><input className="visually-hidden" type="file" accept="image/*" disabled={Boolean(busy)} onChange={(event) => void uploadExpertiseIcon(event.target.files?.[0] ?? null)} /><AppIcon name="image" /><span>{busy === 'expertise-photo' ? 'Uploading' : 'Upload photo from PC'}</span></label>
              </div>
            </div>
            <label><span>Title {lang.toUpperCase()}</span><input value={localized(expertiseDraft, 'title', lang)} onChange={(event) => setExpertiseDraft(patchLocalized(expertiseDraft, 'title', lang, event.target.value))} /></label>
            <label><span>Order</span><input value={expertiseDraft.order_index} onChange={(event) => setExpertiseDraft({ ...expertiseDraft, order_index: Number(event.target.value) || 0 })} /></label>
            <ActionButton tone="primary" icon="save" disabled={Boolean(busy)} onClick={() => void saveExpertiseDraft()}>{busy === 'expertise' ? 'Saving' : 'Save expertise'}</ActionButton>
          </div>
        ) : null}
      </AdminPanel>
    </section>
  );
}
