import { FormEvent, useEffect, useState } from 'react';
import { AppIcon } from '../../../components/AppIcon';
import { ActionButton } from '../../../components/admin/ActionButton';
import { AdminPanel } from '../../../components/admin/AdminPanel';
import { StatusBadge } from '../../../components/admin/StatusBadge';
import { useAdminToast } from '../../../components/admin/useAdminToast';
import { uploadCmsReference } from '../../../api/cms';
import {
  churchContentApi,
  emptyChurchInfoTranslation,
  type ChurchContentStatus,
  type ChurchInfo,
  type ChurchInfoPayload,
  type ChurchInfoTranslation,
  type ChurchLanguage
} from '../../../api/churchContent';
import { cmsLanguages, statusLabels, statuses } from './churchTypes';
import { languageStatusFromRecord } from './churchHelpers';
import { LanguageTabs } from './ChurchWorkflowSteps';

export function ChurchInfoSection() {
  const toast = useAdminToast();
  const [info, setInfo] = useState<ChurchInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    churchContentApi.getInfo({ site: 'church' })
      .then((next) => { if (!cancelled) setInfo(next); })
      .catch(() => { if (!cancelled) setInfo(null); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  async function saveInfo(payload: ChurchInfoPayload) {
    setSaving(true);
    try {
      const next = await churchContentApi.updateInfo(payload, { site: 'church' });
      setInfo(next);
      toast.success('Информация о храме сохранена.');
      setEditing(false);
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось сохранить информацию о храме.');
    } finally {
      setSaving(false);
    }
  }

  async function publishInfo(status: ChurchContentStatus) {
    if (!info) return;
    setSaving(true);
    try {
      const next = await churchContentApi.updateInfo({
        address: info.address,
        mapsUrl: info.mapsUrl,
        phoneOrSite: info.phoneOrSite,
        priestPhone: info.priestPhone,
        imageUrl: info.imageUrl,
        galleryImages: info.galleryImages,
        translations: info.translations,
        status
      }, { site: 'church' });
      setInfo(next);
      toast.success(status === 'published' ? 'Информация о храме опубликована.' : 'Информация о храме снята с публикации.');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Не удалось изменить статус.');
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <AdminPanel title="Информация о храме" icon="cms">
        <p className="admin-table-empty">Загружаем информацию о храме...</p>
      </AdminPanel>
    );
  }

  if (editing) {
    return (
      <AdminPanel title="Информация о храме" icon="cms" meta={<ActionButton onClick={() => setEditing(false)} disabled={saving}>Отмена</ActionButton>}>
        <ChurchInfoForm item={info} saving={saving} onSubmit={saveInfo} />
      </AdminPanel>
    );
  }

  return <ChurchInfoSummaryPanel info={info} saving={saving} onEdit={() => setEditing(true)} onPublish={publishInfo} />;
}

function ChurchInfoSummaryPanel({ info, saving, onEdit, onPublish }: { info: ChurchInfo | null; saving: boolean; onEdit: () => void; onPublish: (status: ChurchContentStatus) => Promise<void> }) {
  const uk = info?.translations.uk;
  const filledLanguages = cmsLanguages.filter((language) => {
    const translation = info?.translations[language.id];
    return Boolean(translation?.title.trim());
  });
  const hasCoreFacts = Boolean(info?.address.trim() || info?.priestPhone.trim());
  const isPublished = info?.status === 'published';
  return (
    <AdminPanel
      title="Информация о храме"
      icon="cms"
      meta={isPublished ? <StatusBadge status="online" label="published" /> : <StatusBadge status="offline" label={info ? 'draft' : 'not filled'} />}
    >
      {hasCoreFacts || filledLanguages.length ? (
        <div className="church-info-summary">
          <div>
            <strong>{uk?.title?.trim() || info?.address || 'Храм ещё не заполнен'}</strong>
            {info?.address ? <span>{info.address}</span> : null}
            {uk?.priest ? <span>{uk.priest}{info?.priestPhone ? ` · ${info.priestPhone}` : ''}</span> : null}
          </div>
          <div className="church-info-summary-languages">
            {cmsLanguages.map((language) => (
              <em key={language.id} className={filledLanguages.some((item) => item.id === language.id) ? 'completed' : 'empty'}>{language.short}</em>
            ))}
          </div>
        </div>
      ) : (
        <p className="admin-table-empty">Адрес, настоятель и контакты храма ещё не заполнены. Заполните их один раз — они появятся на публичной странице /churches на всех языках.</p>
      )}
      <div className="admin-header-actions">
        <ActionButton icon="sliders" onClick={onEdit}>{hasCoreFacts || filledLanguages.length ? 'Редактировать' : 'Заполнить'}</ActionButton>
        {hasCoreFacts && uk?.title ? (
          isPublished ? (
            <ActionButton icon="save" onClick={() => void onPublish('draft')} disabled={saving}>Снять с публикации</ActionButton>
          ) : (
            <ActionButton icon="save" tone="primary" onClick={() => void onPublish('published')} disabled={saving}>Опубликовать</ActionButton>
          )
        ) : null}
      </div>
    </AdminPanel>
  );
}

function ChurchInfoForm({ item, saving, onSubmit }: { item: ChurchInfo | null; saving: boolean; onSubmit: (payload: ChurchInfoPayload) => Promise<void> }) {
  const toast = useAdminToast();
  const [form, setForm] = useState({
    address: item?.address ?? '',
    mapsUrl: item?.mapsUrl ?? '',
    phoneOrSite: item?.phoneOrSite ?? '',
    priestPhone: item?.priestPhone ?? '',
    imageUrl: item?.imageUrl ?? '',
    status: item?.status ?? 'draft'
  });
  const [galleryImages, setGalleryImages] = useState<string[]>(item?.galleryImages ?? []);
  const [translations, setTranslations] = useState<Record<ChurchLanguage, ChurchInfoTranslation>>({
    uk: { ...emptyChurchInfoTranslation, ...item?.translations.uk },
    ru: { ...emptyChurchInfoTranslation, ...item?.translations.ru },
    en: { ...emptyChurchInfoTranslation, ...item?.translations.en }
  });
  const [activeLanguage, setActiveLanguage] = useState<ChurchLanguage>('uk');
  const [error, setError] = useState('');
  const [uploadingCover, setUploadingCover] = useState(false);
  const [uploadingGallery, setUploadingGallery] = useState(false);

  function setField(language: ChurchLanguage, field: keyof ChurchInfoTranslation, value: string) {
    setTranslations((current) => ({ ...current, [language]: { ...current[language], [field]: value } }));
  }

  const languageStatuses = languageStatusFromRecord(translations, (value) => [value.title, value.description, value.schedule, value.dedication, value.shrines, value.priest]);
  const current = translations[activeLanguage];

  async function uploadCoverPhoto(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) return setError('Выберите файл изображения.');
    if (file.size > 10 * 1024 * 1024) return setError('Фото должно быть меньше 10 MB.');
    setUploadingCover(true);
    setError('');
    try {
      const imageUrl = await uploadCmsReference(file);
      setForm((current) => ({ ...current, imageUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото.');
    } finally {
      setUploadingCover(false);
    }
  }

  async function uploadGalleryPhotos(files: FileList | null) {
    if (!files || !files.length) return;
    const selected = Array.from(files).filter((file) => file.type.startsWith('image/'));
    if (!selected.length) return setError('Выберите файлы изображений.');
    if (selected.some((file) => file.size > 10 * 1024 * 1024)) return setError('Каждое фото должно быть меньше 10 MB.');
    setUploadingGallery(true);
    setError('');
    try {
      const uploaded: string[] = [];
      for (const file of selected) uploaded.push(await uploadCmsReference(file));
      setGalleryImages((current) => [...current, ...uploaded]);
      toast.success(`Добавлено фото в галерею: ${uploaded.length}.`);
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото галереи.');
    } finally {
      setUploadingGallery(false);
    }
  }

  function removeGalleryImage(index: number) {
    setGalleryImages((current) => current.filter((_, itemIndex) => itemIndex !== index));
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.address.trim()) return setError('Укажите адрес храма.');
    if (!translations.uk.title.trim()) return setError('Название храма на украинском обязательно.');
    setError('');
    void onSubmit({ ...form, galleryImages, translations } as ChurchInfoPayload);
  }

  return (
    <form className="admin-form-grid" onSubmit={submit}>
      <label className="admin-form-wide"><span>Адрес</span><input value={form.address} onChange={(event) => setForm({ ...form, address: event.target.value })} placeholder="вул. Романа Шухевича, 68" /></label>
      <label><span>Настоятель (текущий язык: {activeLanguage.toUpperCase()})</span><input value={current.priest} onChange={(event) => setField(activeLanguage, 'priest', event.target.value)} placeholder="Отец Сергій" /></label>
      <label><span>Телефон настоятеля</span><input value={form.priestPhone} onChange={(event) => setForm({ ...form, priestPhone: event.target.value })} placeholder="(067) 631-91-10" /></label>
      <label><span>Телефон / сайт храма</span><input value={form.phoneOrSite} onChange={(event) => setForm({ ...form, phoneOrSite: event.target.value })} /></label>
      <label><span>Google Maps ссылка</span><input value={form.mapsUrl} onChange={(event) => setForm({ ...form, mapsUrl: event.target.value })} placeholder="https://maps.google.com/..." /></label>
      <label><span>Статус</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>

      <div className="admin-form-wide church-image-editor">
        {form.imageUrl ? <img src={form.imageUrl} alt="Фото храма" loading="lazy" /> : <span><AppIcon name="image" /><small>Главное фото ещё не добавлено</small></span>}
        <div>
          <strong>Главное фото храма</strong>
          <p>Это фото показывается в шапке публичной страницы храма. Загрузите файл с компьютера или вставьте готовый URL.</p>
          <label className="church-upload-button">
            <input type="file" accept="image/*" onChange={(event) => void uploadCoverPhoto(event.target.files?.[0] ?? null)} disabled={saving || uploadingCover} />
            <span>{uploadingCover ? 'Загрузка...' : 'Загрузить фото с ПК'}</span>
          </label>
          <input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://..." />
        </div>
      </div>

      <div className="admin-form-wide">
        <strong>Галерея фото храма</strong>
        <p className="section-note">Дополнительные фото храма для страницы /churches: интерьер, святыни, богослужения.</p>
        <div className="church-icon-photo-grid">
          {galleryImages.map((url, index) => (
            <article key={`${url}-${index}`} className="filled">
              <div className="church-icon-photo-card__media"><img src={url} alt={`Фото храма ${index + 1}`} loading="lazy" /></div>
              <ActionButton icon="sliders" tone="danger" type="button" onClick={() => removeGalleryImage(index)}>Удалить</ActionButton>
            </article>
          ))}
          <label className="church-upload-button compact">
            <input type="file" accept="image/*" multiple onChange={(event) => { void uploadGalleryPhotos(event.target.files); event.currentTarget.value = ''; }} disabled={saving || uploadingGallery} />
            <span>{uploadingGallery ? 'Загрузка...' : 'Добавить фото'}</span>
          </label>
        </div>
      </div>

      <div className="admin-form-wide">
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={setActiveLanguage} />
      </div>
      <label className="admin-form-wide"><span>Название храма ({activeLanguage.toUpperCase()})</span><input value={current.title} onChange={(event) => setField(activeLanguage, 'title', event.target.value)} placeholder="Свято-Микільський храм" /></label>
      <label className="admin-form-wide"><span>Кому посвящён</span><input value={current.dedication} onChange={(event) => setField(activeLanguage, 'dedication', event.target.value)} /></label>
      <label className="admin-form-wide"><span>Расписание богослужений</span><textarea value={current.schedule} onChange={(event) => setField(activeLanguage, 'schedule', event.target.value)} /></label>
      <label className="admin-form-wide"><span>Святыни / иконы / мощи</span><textarea value={current.shrines} onChange={(event) => setField(activeLanguage, 'shrines', event.target.value)} /></label>
      <label className="admin-form-wide"><span>Краткое описание</span><textarea value={current.description} onChange={(event) => setField(activeLanguage, 'description', event.target.value)} /></label>

      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="save" type="submit" disabled={saving || uploadingCover || uploadingGallery}>{saving ? 'Сохраняем' : 'Сохранить информацию о храме'}</ActionButton>
    </form>
  );
}
