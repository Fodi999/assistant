import { useEffect, useState } from 'react';
import {
  createGalleryItem,
  deleteGalleryItem,
  getAboutPage,
  listGallery,
  updateAboutPage,
  updateGalleryItem,
  uploadAboutPhoto,
  uploadGalleryPhoto,
  type AboutPageContent,
  type GalleryItem
} from '../api/cms';
import { revalidateSite } from '../api/revalidate';

type Lang = 'ru' | 'en' | 'pl' | 'uk';

const LANGS: Array<{ code: Lang; label: string }> = [
  { code: 'ru', label: 'RU' },
  { code: 'en', label: 'EN' },
  { code: 'pl', label: 'PL' },
  { code: 'uk', label: 'UK' }
];

const EMPTY: AboutPageContent = {
  id: '',
  title_en: '',
  title_pl: '',
  title_ru: '',
  title_uk: '',
  content_en: '',
  content_pl: '',
  content_ru: '',
  content_uk: '',
  image_url: null,
  updated_at: ''
};

function formatUpdatedAt(value: string | number[]): string {
  if (Array.isArray(value)) {
    const [year, ordinal, hour = 0, minute = 0] = value;
    const date = new Date(year, 0, 1, hour, minute);
    date.setDate(Number(ordinal));
    return date.toLocaleString('ru-RU');
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? 'только что' : date.toLocaleString('ru-RU');
}

export function AboutPage() {
  const [about, setAbout] = useState<AboutPageContent>(EMPTY);
  const [lang, setLang] = useState<Lang>('ru');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [works, setWorks] = useState<GalleryItem[]>([]);
  const [workBusy, setWorkBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void Promise.all([getAboutPage(), listGallery()])
      .then(([aboutPage, gallery]) => {
        setAbout(aboutPage);
        setWorks(gallery);
      })
      .catch((cause) => setError(cause instanceof Error ? cause.message : 'Не удалось загрузить страницу'))
      .finally(() => setLoading(false));
  }, []);

  function updateField(field: 'title' | 'content', value: string) {
    setAbout((current) => ({ ...current, [`${field}_${lang}`]: value }));
  }

  async function uploadPhoto(file?: File) {
    if (!file) return;
    setUploading(true); setError(null); setMessage(null);
    try {
      const imageUrl = await uploadAboutPhoto(file);
      setAbout((current) => ({ ...current, image_url: imageUrl }));
      setMessage('Фото загружено. Нажмите «Сохранить страницу», чтобы опубликовать его.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось загрузить фото');
    } finally {
      setUploading(false);
    }
  }

  async function save() {
    setSaving(true); setError(null); setMessage(null);
    try {
      const updated = await updateAboutPage(about);
      setAbout(updated);
      await revalidateSite({ type: 'about' });
      setMessage('Страница «Обо мне» сохранена и доступна публичному сайту.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось сохранить страницу');
    } finally {
      setSaving(false);
    }
  }

  function updateWork(id: string, field: string, value: string | number) {
    setWorks((current) => current.map((work) => work.id === id ? { ...work, [field]: value } : work));
  }

  async function addWork(file?: File) {
    if (!file) return;
    setWorkBusy('new'); setError(null); setMessage(null);
    try {
      const imageUrl = await uploadGalleryPhoto(file);
      const created = await createGalleryItem({
        image_url: imageUrl,
        status: 'published',
        order_index: works.length
      });
      setWorks((current) => [...current, created]);
      await revalidateSite({ type: 'gallery' });
      setMessage('Работа добавлена. Теперь заполните заголовок и описание.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось добавить работу');
    } finally {
      setWorkBusy(null);
    }
  }

  async function replaceWorkPhoto(work: GalleryItem, file?: File) {
    if (!file) return;
    setWorkBusy(work.id); setError(null);
    try {
      const imageUrl = await uploadGalleryPhoto(file);
      const updated = await updateGalleryItem(work.id, { image_url: imageUrl });
      setWorks((current) => current.map((item) => item.id === work.id ? updated : item));
      await revalidateSite({ type: 'gallery' });
      setMessage('Фото работы заменено.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось заменить фото');
    } finally {
      setWorkBusy(null);
    }
  }

  async function saveWork(work: GalleryItem) {
    setWorkBusy(work.id); setError(null); setMessage(null);
    try {
      const updated = await updateGalleryItem(work.id, work);
      setWorks((current) => current.map((item) => item.id === work.id ? updated : item));
      await revalidateSite({ type: 'gallery' });
      setMessage('Работа сохранена и обновлена на сайте.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось сохранить работу');
    } finally {
      setWorkBusy(null);
    }
  }

  async function removeWork(work: GalleryItem) {
    if (!window.confirm('Удалить эту работу из галереи?')) return;
    setWorkBusy(work.id); setError(null);
    try {
      await deleteGalleryItem(work.id);
      setWorks((current) => current.filter((item) => item.id !== work.id));
      await revalidateSite({ type: 'gallery' });
      setMessage('Работа удалена.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Не удалось удалить работу');
    } finally {
      setWorkBusy(null);
    }
  }

  const title = about[`title_${lang}`];
  const content = about[`content_${lang}`];

  return (
    <section className="about-admin-page">
      <header className="about-admin-hero">
        <div>
          <p className="about-admin-kicker">PERSONAL CMS</p>
          <h1>Страница «Обо мне»</h1>
          <p>Управляйте фотографией и текстами на четырёх языках. Изменения появятся на сайте после сохранения.</p>
        </div>
        <button className="btn btn-primary" onClick={() => void save()} disabled={saving || loading}>{saving ? 'Сохраняем…' : 'Сохранить страницу'}</button>
      </header>

      {error && <p className="about-admin-alert about-admin-alert--error">{error}</p>}
      {message && <p className="about-admin-alert">{message}</p>}

      <div className="about-admin-grid">
        <article className="page-card about-photo-card">
          <div className="section-head"><div><h2>Портрет</h2><p className="page-muted">Главная фотография для публичной страницы</p></div></div>
          <div className="about-photo-preview">
            {about.image_url ? <img src={about.image_url} alt="About page portrait" /> : <span>Фото ещё не загружено</span>}
          </div>
          <label className="about-photo-upload">
            <input type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => void uploadPhoto(event.target.files?.[0])} disabled={uploading} />
            <strong>{uploading ? 'Загружаем в R2…' : 'Загрузить фото с компьютера'}</strong>
            <span>JPG, PNG или WebP. Лучше вертикальный портрет высокого разрешения.</span>
          </label>
        </article>

        <article className="page-card about-copy-card">
          <div className="about-copy-head">
            <div><h2>Текст страницы</h2><p className="page-muted">Каждый язык редактируется отдельно</p></div>
            <div className="about-lang-tabs">
              {LANGS.map((item) => <button className={lang === item.code ? 'active' : ''} key={item.code} onClick={() => setLang(item.code)}>{item.label}</button>)}
            </div>
          </div>
          <label className="field">
            <span className="field-label">Заголовок · {lang.toUpperCase()}</span>
            <textarea className="about-title-input" value={title} onChange={(event) => updateField('title', event.target.value)} placeholder="Главный заголовок страницы" />
          </label>
          <label className="field">
            <span className="field-label">Основной текст · {lang.toUpperCase()}</span>
            <textarea className="about-content-input" value={content} onChange={(event) => updateField('content', event.target.value)} placeholder="Расскажите о себе, опыте и подходе к работе" />
          </label>
          <div className="about-copy-meta"><span>{content.length} символов</span><span>{about.updated_at ? `Обновлено ${formatUpdatedAt(about.updated_at)}` : 'Ещё не сохранено'}</span></div>
        </article>
      </div>

      <section className="page-card about-works">
        <div className="about-works-head">
          <div>
            <p className="about-admin-kicker">WORK GALLERY</p>
            <h2>Фото моей работы</h2>
            <p className="page-muted">Добавляйте проекты, блюда и процессы. Заголовок и описание сохраняются на четырёх языках.</p>
          </div>
          <label className="btn btn-primary about-work-add">
            <input type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => void addWork(event.target.files?.[0])} disabled={workBusy === 'new'} />
            {workBusy === 'new' ? 'Загружаем…' : '+ Добавить работу'}
          </label>
        </div>

        {works.length === 0 ? (
          <label className="about-works-empty">
            <input type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => void addWork(event.target.files?.[0])} />
            <strong>+ Добавить первое фото работы</strong>
            <span>После загрузки появятся поля заголовка и описания.</span>
          </label>
        ) : (
          <div className="about-works-grid">
            {works.map((work) => (
              <article className="about-work-card" key={work.id}>
                <div className="about-work-image">
                  <img src={work.image_url} alt={work[`alt_${lang}`] || work[`title_${lang}`] || 'Фото работы'} />
                  <label className="about-work-replace">
                    <input type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => void replaceWorkPhoto(work, event.target.files?.[0])} />
                    Заменить фото
                  </label>
                </div>
                <div className="about-work-fields">
                  <div className="about-work-toolbar">
                    <label>
                      <span>Порядок</span>
                      <input type="number" min="0" value={work.order_index} onChange={(event) => updateWork(work.id, 'order_index', Number(event.target.value))} />
                    </label>
                    <select value={work.status} onChange={(event) => updateWork(work.id, 'status', event.target.value)}>
                      <option value="published">Опубликована</option>
                      <option value="draft">Скрыта</option>
                    </select>
                  </div>
                  <label className="field">
                    <span className="field-label">Название · {lang.toUpperCase()}</span>
                    <input value={work[`title_${lang}`]} onChange={(event) => updateWork(work.id, `title_${lang}`, event.target.value)} placeholder="Например: Авторский сет для ресторана" />
                  </label>
                  <label className="field">
                    <span className="field-label">Описание · {lang.toUpperCase()}</span>
                    <textarea value={work[`description_${lang}`]} onChange={(event) => updateWork(work.id, `description_${lang}`, event.target.value)} placeholder="Расскажите о задаче, процессе и результате" />
                  </label>
                  <div className="about-work-actions">
                    <button className="btn" onClick={() => void removeWork(work)} disabled={workBusy === work.id}>Удалить</button>
                    <button className="btn btn-primary" onClick={() => void saveWork(work)} disabled={workBusy === work.id}>{workBusy === work.id ? 'Сохраняем…' : 'Сохранить работу'}</button>
                  </div>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </section>
  );
}
