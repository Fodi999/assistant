import { FormEvent, useEffect, useMemo, useState } from 'react';
import QRCode from 'qrcode';
import { AppIcon } from '../../../components/AppIcon';
import { ActionButton } from '../../../components/admin/ActionButton';
import { isValidSlug, isValidUrl } from '../../../components/admin/forms/formUtils';
import { uploadCmsReference, uploadPrayerAudio as uploadPrayerAudioFile } from '../../../api/cms';
import type {
  ChurchArticle,
  ChurchArticlePayload,
  ChurchCalendarDay,
  ChurchCalendarDayPayload,
  ChurchCalendarType,
  ChurchContentStatus,
  ChurchDayType,
  ChurchGospel,
  ChurchGospelPayload,
  ChurchIcon,
  ChurchIconPayload,
  ChurchPrayer,
  ChurchPrayerPayload,
  ChurchPrayerType,
  ChurchSaint,
  ChurchSaintPayload
} from '../../../api/churchContent';
import type { CalendarDay as PublicCalendarDay } from '../../../api/iconsSite';
import {
  calendarTypeLabels,
  churchPublicUrl,
  cmsLanguages,
  dayTypeLabels,
  dayTypes,
  iconPhotoSlots,
  prayerTypeLabels,
  prayerTypes,
  statusLabels,
  statuses,
  type CalendarDateBasis,
  type CmsLanguage,
  type IconPhotoRole,
  type LanguageFieldStatus,
  type WorkflowStep
} from './churchTypes';
import {
  buildWorkflowSteps,
  compactText,
  contentLanguageStatus,
  imageForContent,
  isCalendarDayComplete,
  nextCalendarStyleDates,
  normalizeCalendarStyleDates,
  parseIconExtraPhotos,
  prayerQrPath,
  stripIconExtraPhotos,
  workflowStatusLabel
} from './churchHelpers';

async function dataUrlToFile(dataUrl: string, filename: string): Promise<File> {
  const response = await fetch(dataUrl);
  const blob = await response.blob();
  return new File([blob], filename, { type: blob.type });
}

export function LanguageTabs({ active, statuses: languageStatuses, onChange }: { active: CmsLanguage; statuses: LanguageFieldStatus; onChange: (language: CmsLanguage) => void }) {
  return (
    <div className="church-language-tabs" role="tablist" aria-label="Language content">
      {cmsLanguages.map((language) => (
        <button key={language.id} type="button" className={active === language.id ? 'active' : ''} onClick={() => onChange(language.id)}>
          <strong>{language.short}</strong>
          <span>{language.label}</span>
          <em className={languageStatuses[language.id]}>{languageStatuses[language.id]}</em>
        </button>
      ))}
    </div>
  );
}

export function CalendarWorkflowIntro({ step, description }: { step: WorkflowStep; description: string }) {
  return (
    <section className={`church-workflow-intro ${step.status}`}>
      <div>
        <span>{step.index}</span>
        <strong>{step.label}</strong>
        <small>{workflowStatusLabel(step.status)}</small>
      </div>
      <p>{description}</p>
    </section>
  );
}

export function WorkflowStepActions({ saving, canGoBack, canGoNext, saveFormId, saveNextDisabled, saveNextLabel = 'Сохранить и дальше', onBack, onNext, onSave, onSaveNext }: {
  saving: boolean;
  canGoBack: boolean;
  canGoNext: boolean;
  saveFormId?: string;
  saveNextDisabled?: boolean;
  saveNextLabel?: string;
  onBack?: () => void;
  onNext?: () => void;
  onSave?: () => void;
  onSaveNext?: () => void;
}) {
  return (
    <div className="church-workflow-actions">
      <ActionButton onClick={onBack} disabled={!canGoBack || saving}>Назад</ActionButton>
      <ActionButton type={saveFormId ? 'submit' : 'button'} form={saveFormId} onClick={onSave} disabled={saving || (!saveFormId && !onSave)}>Сохранить</ActionButton>
      <ActionButton
        type={saveFormId ? 'submit' : 'button'}
        form={saveFormId}
        tone="primary"
        icon="check"
        onClick={saveFormId ? onSaveNext : onNext}
        disabled={saving || Boolean(saveNextDisabled) || (!saveFormId && !canGoNext)}
      >
        {saveNextLabel}
      </ActionButton>
    </div>
  );
}

export function WorkflowDataAudit({ items }: { items: Array<[string, string]> }) {
  return (
    <dl className="church-workflow-audit">
      {items.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

export function ContentRow({ title, meta, body, image, onEdit, onDelete }: { title: string; meta: string; body?: string; image?: string; onEdit?: () => void; onDelete: () => void }) {
  return (
    <article className="church-content-row">
      {image ? <img src={image} alt={title} loading="lazy" /> : <span><AppIcon name="image" /></span>}
      <div>
        <strong>{title}</strong>
        <small>{meta}</small>
        {body ? <p>{body}</p> : null}
      </div>
      <div className="admin-table-actions">
        {onEdit ? <button className="table-action" type="button" onClick={onEdit}>Edit</button> : null}
        <button className="table-action danger" type="button" onClick={onDelete}>Delete</button>
      </div>
    </article>
  );
}

export function OverviewTab({ day, icons, prayers, articles, onEdit }: { day: ChurchCalendarDay; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; onEdit: () => void }) {
  return (
    <div className="church-overview">
      <div>
        <span>{formatOverviewDate(day)}</span>
        <h3>{day.title}</h3>
        <p>{day.description || 'Опишите, кто или какой праздник связан с этим днем. Это главный узел для икон, молитв и статьи.'}</p>
        <ActionButton icon="sliders" onClick={onEdit}>Редактировать святого / праздник</ActionButton>
      </div>
      <dl>
        <dt>Тип</dt><dd>{day.dayType}</dd>
        <dt>Календарь</dt><dd>{day.calendarType}</dd>
        <dt>Приоритет</dt><dd>{day.rank}</dd>
        <dt>Статус</dt><dd>{day.status}</dd>
        <dt>Иконы</dt><dd>{icons.length}</dd>
        <dt>Молитвы</dt><dd>{prayers.length}</dd>
        <dt>Статьи</dt><dd>{articles.length}</dd>
      </dl>
    </div>
  );
}

function formatOverviewDate(day: ChurchCalendarDay) {
  return [day.dateNewStyle, day.dateOldStyle ? `${day.dateOldStyle} old` : ''].filter(Boolean).join(' / ') || 'Дата не задана';
}

export function SaintWorkflowStep({ saints, icons, allIcons, saving, onSaveSaint, onDeleteItem }: { saints: ChurchSaint[]; icons: ChurchIcon[]; allIcons: ChurchIcon[]; saving: boolean; onSaveSaint: (payload: ChurchSaintPayload, id?: string) => Promise<void>; onDeleteItem: (type: 'saint', id: string, title: string) => Promise<void> }) {
  const selectedSaint = saints[0] ?? null;
  const [editingSaintId, setEditingSaintId] = useState(selectedSaint?.id ?? '');
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>(selectedSaint?.language ?? 'uk');
  const [form, setForm] = useState({
    iconId: selectedSaint?.iconId ?? icons[0]?.id ?? '',
    name: selectedSaint?.name ?? '',
    slug: selectedSaint?.slug ?? '',
    shortDescription: selectedSaint?.shortDescription ?? '',
    biography: selectedSaint?.biography ?? '',
    feastDay: selectedSaint?.feastDay ?? '',
    imageUrl: selectedSaint?.imageUrl ?? '',
    language: selectedSaint?.language ?? 'uk',
    status: selectedSaint?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  const [uploadingImage, setUploadingImage] = useState(false);

  useEffect(() => {
    setEditingSaintId(selectedSaint?.id ?? '');
    setActiveLanguage(selectedSaint?.language ?? 'uk');
    setForm({
      iconId: selectedSaint?.iconId ?? icons[0]?.id ?? '',
      name: selectedSaint?.name ?? '',
      slug: selectedSaint?.slug ?? '',
      shortDescription: selectedSaint?.shortDescription ?? '',
      biography: selectedSaint?.biography ?? '',
      feastDay: selectedSaint?.feastDay ?? '',
      imageUrl: selectedSaint?.imageUrl ?? '',
      language: selectedSaint?.language ?? 'uk',
      status: selectedSaint?.status ?? 'draft'
    });
    setError('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedSaint?.id, icons[0]?.id]);

  const siblingSaints = useMemo(() => (form.slug ? saints.filter((saint) => saint.slug === form.slug) : []), [saints, form.slug]);
  const languageStatuses = useMemo(
    () => contentLanguageStatus(siblingSaints, (saint) => Boolean(saint.name.trim() && saint.biography.trim()) && saint.status === 'published'),
    [siblingSaints]
  );

  function switchLanguage(language: CmsLanguage) {
    const sibling = siblingSaints.find((saint) => saint.language === language);
    setActiveLanguage(language);
    setEditingSaintId(sibling?.id ?? '');
    setForm((current) => ({
      ...current,
      language,
      name: sibling?.name ?? '',
      shortDescription: sibling?.shortDescription ?? '',
      biography: sibling?.biography ?? '',
      feastDay: sibling?.feastDay ?? '',
      imageUrl: sibling?.imageUrl ?? '',
      status: sibling?.status ?? 'draft'
    }));
    setError('');
  }

  function startNewSaint() {
    setEditingSaintId('');
    setActiveLanguage('uk');
    setForm({
      iconId: icons[0]?.id ?? '',
      name: '',
      slug: '',
      shortDescription: '',
      biography: '',
      feastDay: '',
      imageUrl: '',
      language: 'uk',
      status: 'draft'
    });
    setError('');
  }

  function editSaint(saint: ChurchSaint) {
    setEditingSaintId(saint.id);
    setActiveLanguage(saint.language);
    setForm({
      iconId: saint.iconId ?? icons[0]?.id ?? '',
      name: saint.name,
      slug: saint.slug,
      shortDescription: saint.shortDescription,
      biography: saint.biography,
      feastDay: saint.feastDay,
      imageUrl: saint.imageUrl,
      language: saint.language,
      status: saint.status
    });
    setError('');
  }

  async function uploadSaintImage(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setError('Выберите файл изображения.');
      return;
    }
    if (file.size > 10 * 1024 * 1024) {
      setError('Фото должно быть меньше 10 MB.');
      return;
    }
    setUploadingImage(true);
    setError('');
    try {
      const imageUrl = await uploadCmsReference(file);
      setForm((current) => ({ ...current, imageUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото.');
    } finally {
      setUploadingImage(false);
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.name.trim()) return setError('Имя святого обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.biography.trim()) return setError('Житие обязательно.');
    if (form.imageUrl && !isValidUrl(form.imageUrl)) return setError('Фото должно быть корректным http/https URL.');
    setError('');
    void onSaveSaint({
      ...form,
      iconId: form.iconId || undefined,
      imageUrl: form.imageUrl || undefined
    } as ChurchSaintPayload, editingSaintId || undefined);
  }

  const previewImage = form.imageUrl || imageForContent(form.iconId, icons, allIcons);

  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(null, saints, [], [], [])[1]} description="Заполните отдельную страницу святого: имя, день памяти, краткое описание, житие и изображение на каждом языке." />
      <form className="church-article-workspace-form" onSubmit={submit}>
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={switchLanguage} />
        <div className="church-article-main-fields">
          <label><span>Связанная икона</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">Без иконы</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
          <label><span>Имя святого ({activeLanguage.toUpperCase()})</span><input value={form.name} onChange={(event) => setForm({ ...form, name: event.target.value })} placeholder="Святитель Николай Чудотворец" /></label>
          <label><span>Slug страницы (общий для всех языков)</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="svyatitel-nikolay" /></label>
          <label><span>День памяти ({activeLanguage.toUpperCase()})</span><input value={form.feastDay} onChange={(event) => setForm({ ...form, feastDay: event.target.value })} placeholder="19 декабря / 6 декабря" /></label>
          <label><span>Статус ({activeLanguage.toUpperCase()})</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
          <label className="admin-form-wide"><span>Краткое описание ({activeLanguage.toUpperCase()})</span><textarea value={form.shortDescription} onChange={(event) => setForm({ ...form, shortDescription: event.target.value })} placeholder="Короткий лид для каталога и SEO." /></label>
          <label className="admin-form-wide article-body"><span>Житие ({activeLanguage.toUpperCase()})</span><textarea value={form.biography} onChange={(event) => setForm({ ...form, biography: event.target.value })} placeholder="Полное житие святого, история почитания и духовный смысл." /></label>
        </div>

        <div className="church-article-preview-grid">
          <article>
            <div className="church-article-preview-image">{previewImage ? <img src={previewImage} alt={form.name || 'Фото святого'} /> : <span><AppIcon name="sparkles" /></span>}</div>
            <div>
              <strong>{form.name || 'Новый святой'}</strong>
              <small>{form.language.toUpperCase()} · {statusLabels[form.status]} · {form.slug ? `/saints/${form.slug}` : 'URL не заполнен'}</small>
              <p>{form.shortDescription || form.biography.replace(/\s+/g, ' ').trim().slice(0, 180) || 'Описание появится после заполнения текста.'}</p>
              <label className="church-upload-button compact"><input type="file" accept="image/*" onChange={(event) => void uploadSaintImage(event.target.files?.[0] ?? null)} disabled={saving || uploadingImage} /><span>{uploadingImage ? 'Загрузка фото...' : 'Загрузить фото'}</span></label>
              <input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://.../saint.jpg" />
            </div>
          </article>
          <dl>
            <div><dt>URL святого</dt><dd>{form.slug ? `/saints/${form.slug}` : 'не заполнен'}</dd></div>
            <div><dt>Житие</dt><dd>{form.biography ? 'заполнено' : 'пусто'}</dd></div>
            <div><dt>День памяти</dt><dd>{form.feastDay || 'не указан'}</dd></div>
            <div><dt>Изображение</dt><dd>{previewImage ? 'заполнено' : 'нет'}</dd></div>
          </dl>
        </div>

        {error ? <small className="admin-form-error">{error}</small> : null}
        <div className="church-article-form-actions">
          <ActionButton type="button" onClick={startNewSaint} disabled={saving}>Новый святой</ActionButton>
          <ActionButton icon="save" tone="primary" type="submit" disabled={saving || uploadingImage}>{saving ? 'Сохраняем' : editingSaintId ? `Сохранить (${activeLanguage.toUpperCase()})` : `Создать (${activeLanguage.toUpperCase()})`}</ActionButton>
        </div>
      </form>

      <WorkflowDataAudit
        items={[
          ['Святой', form.name || selectedSaint?.name || 'не создан'],
          ['URL святого', form.slug ? `/saints/${form.slug}` : 'не заполнен'],
          ['Язык', form.language.toUpperCase()],
          ['День памяти', form.feastDay || 'не указан'],
          ['Житие', form.biography ? 'заполнено' : 'пусто'],
          ['Икона', form.iconId ? 'связана' : 'не связана']
        ]}
      />

      {saints.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Сохраненные святые</strong><ActionButton icon="sparkles" onClick={startNewSaint}>Добавить святого</ActionButton></div>
          <div className="church-content-list">
            {saints.map((saint) => (
              <ContentRow key={saint.id} title={saint.name} meta={`${saint.language} · ${statusLabels[saint.status]} · /saints/${saint.slug}`} body={saint.shortDescription || saint.biography} image={saint.imageUrl || imageForContent(saint.iconId, icons, allIcons)} onEdit={() => editSaint(saint)} onDelete={() => void onDeleteItem('saint', saint.id, saint.name)} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function IconWorkflowStep({ icons, allIcons, saving, onSaveIcon, onDeleteItem }: { icons: ChurchIcon[]; allIcons: ChurchIcon[]; saving: boolean; onSaveIcon: (payload: ChurchIconPayload, id?: string) => Promise<void>; onDeleteItem: (type: 'icon' | 'prayer' | 'article', id: string, title: string) => Promise<void> }) {
  const selectedIcon = icons[0] ?? null;
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>(selectedIcon?.language ?? 'uk');
  const [form, setForm] = useState({
    title: selectedIcon?.title ?? '',
    slug: selectedIcon?.slug ?? '',
    saintName: selectedIcon?.saintName ?? '',
    feastName: selectedIcon?.feastName ?? '',
    description: stripIconExtraPhotos(selectedIcon?.description ?? ''),
    language: selectedIcon?.language ?? 'uk',
    status: selectedIcon?.status ?? 'draft'
  });
  const [photos, setPhotos] = useState<Record<IconPhotoRole, string>>({
    cover: selectedIcon?.imageUrl ?? '',
    ...parseIconExtraPhotos(selectedIcon?.description ?? '')
  });
  const [uploadingRole, setUploadingRole] = useState<IconPhotoRole | null>(null);
  const [editingIconId, setEditingIconId] = useState(selectedIcon?.id ?? '');
  const [error, setError] = useState('');

  useEffect(() => {
    setActiveLanguage(selectedIcon?.language ?? 'uk');
    setForm({
      title: selectedIcon?.title ?? '',
      slug: selectedIcon?.slug ?? '',
      saintName: selectedIcon?.saintName ?? '',
      feastName: selectedIcon?.feastName ?? '',
      description: stripIconExtraPhotos(selectedIcon?.description ?? ''),
      language: selectedIcon?.language ?? 'uk',
      status: selectedIcon?.status ?? 'draft'
    });
    setPhotos({ cover: selectedIcon?.imageUrl ?? '', ...parseIconExtraPhotos(selectedIcon?.description ?? '') });
    setEditingIconId(selectedIcon?.id ?? '');
    setError('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIcon?.id]);

  const siblingIcons = useMemo(() => (form.slug ? icons.filter((icon) => icon.slug === form.slug) : []), [icons, form.slug]);
  const languageStatuses = useMemo(
    () => contentLanguageStatus(siblingIcons, (icon) => Boolean(icon.imageUrl.trim()) && icon.status === 'published'),
    [siblingIcons]
  );

  function switchLanguage(language: CmsLanguage) {
    const sibling = siblingIcons.find((icon) => icon.language === language);
    setActiveLanguage(language);
    setEditingIconId(sibling?.id ?? '');
    setForm((current) => ({
      ...current,
      language,
      title: sibling?.title ?? '',
      saintName: sibling?.saintName ?? '',
      feastName: sibling?.feastName ?? '',
      description: stripIconExtraPhotos(sibling?.description ?? ''),
      status: sibling?.status ?? 'draft'
    }));
    setPhotos({ cover: sibling?.imageUrl ?? '', ...parseIconExtraPhotos(sibling?.description ?? '') });
    setError('');
  }

  async function uploadIconPhoto(role: IconPhotoRole, file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setError('Выберите файл изображения.');
      return;
    }
    if (file.size > 10 * 1024 * 1024) {
      setError('Фото должно быть меньше 10 MB.');
      return;
    }
    setUploadingRole(role);
    setError('');
    try {
      const imageUrl = await uploadCmsReference(file);
      setPhotos((current) => ({ ...current, [role]: imageUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото.');
    } finally {
      setUploadingRole(null);
    }
  }

  function updatePhoto(role: IconPhotoRole, value: string) {
    setPhotos((current) => ({ ...current, [role]: value }));
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const mainImage = photos.cover || photos.icon;
    if (!form.title.trim()) return setError('Название иконы обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!mainImage || !isValidUrl(mainImage)) return setError('Загрузите обложку или вставьте корректный URL основного фото.');
    const extraPhotos = iconPhotoSlots
      .filter((slot) => slot.role !== 'cover' && photos[slot.role])
      .map((slot) => `${slot.title}: ${photos[slot.role]}`)
      .join('\n');
    setError('');
    void onSaveIcon({
      ...form,
      imageUrl: mainImage,
      description: [stripIconExtraPhotos(form.description).trim(), extraPhotos ? `Дополнительные фото:\n${extraPhotos}` : ''].filter(Boolean).join('\n\n')
    } as ChurchIconPayload, editingIconId || undefined);
  }

  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(null, [], icons, [], [])[2]} description="Заполните икону прямо здесь: данные, основную обложку, фото самой иконы и два дополнительных ракурса." />
      <form className="church-icon-workspace-form" onSubmit={submit}>
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={switchLanguage} />
        <div className="church-icon-photo-grid">
          {iconPhotoSlots.map((slot) => (
            <article key={slot.role} className={photos[slot.role] ? 'filled' : ''}>
              <div className="church-icon-photo-card__media">
                {photos[slot.role] ? <img src={photos[slot.role]} alt={slot.title} loading="lazy" /> : <span><AppIcon name="image" /></span>}
              </div>
              <div>
                <small>{slot.index}</small>
                <strong>{slot.title}</strong>
                <p>{slot.description}</p>
              </div>
              <label><span>URL фото</span><input value={photos[slot.role]} onChange={(event) => updatePhoto(slot.role, event.target.value)} placeholder="https://..." /></label>
              <label className="church-upload-button compact">
                <input type="file" accept="image/*" onChange={(event) => void uploadIconPhoto(slot.role, event.target.files?.[0] ?? null)} disabled={saving || Boolean(uploadingRole)} />
                <span>{uploadingRole === slot.role ? 'Загрузка...' : 'Загрузить фото'}</span>
              </label>
            </article>
          ))}
        </div>

        <div className="church-icon-main-fields">
          <label><span>Название ({activeLanguage.toUpperCase()})</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Икона святителя Николая" /></label>
          <label><span>Slug (общий для всех языков)</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="nikolay-chudotvorets" /></label>
          <label><span>Святой ({activeLanguage.toUpperCase()})</span><input value={form.saintName} onChange={(event) => setForm({ ...form, saintName: event.target.value })} /></label>
          <label><span>Праздник ({activeLanguage.toUpperCase()})</span><input value={form.feastName} onChange={(event) => setForm({ ...form, feastName: event.target.value })} /></label>
          <label><span>Статус ({activeLanguage.toUpperCase()})</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
          <label className="admin-form-wide"><span>Описание ({activeLanguage.toUpperCase()})</span><textarea value={form.description} onChange={(event) => setForm({ ...form, description: event.target.value })} placeholder="Описание иконы для страницы и SEO-связки" /></label>
        </div>

        {error ? <small className="admin-form-error">{error}</small> : null}
        <div className="church-icon-form-actions">
          <ActionButton icon="save" tone="primary" type="submit" disabled={saving || Boolean(uploadingRole)}>{saving ? 'Сохраняем' : selectedIcon ? 'Сохранить икону' : 'Создать икону'}</ActionButton>
        </div>
      </form>

      <WorkflowDataAudit
        items={[
          ['Фото на сайте', photos.cover || selectedIcon?.imageUrl ? 'заполнено' : 'не загружено'],
          ['Название', form.title || 'не заполнено'],
          ['URL иконы', form.slug ? `/icons/${form.slug}` : 'не заполнен'],
          ['Святой / праздник', form.saintName || form.feastName || 'не заполнено'],
          ['Язык', form.language.toUpperCase()],
          ['Доступно в библиотеке', `${allIcons.length} икон`]
        ]}
      />

      {icons.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Сохраненные иконы</strong></div>
          <div className="church-content-list">
            {icons.map((icon) => (
              <ContentRow key={icon.id} title={icon.title} meta={`${icon.language} · ${icon.status} · /icons/${icon.slug}`} image={icon.imageUrl} onEdit={() => { setEditingIconId(icon.id); setActiveLanguage(icon.language); setForm({ title: icon.title, slug: icon.slug, saintName: icon.saintName, feastName: icon.feastName, description: stripIconExtraPhotos(icon.description), language: icon.language, status: icon.status }); setPhotos({ cover: icon.imageUrl, ...parseIconExtraPhotos(icon.description) }); }} onDelete={() => void onDeleteItem('icon', icon.id, icon.title)} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function PrayerWorkflowStep({ prayers, icons, allIcons, saving, onSavePrayer, onDeleteItem }: { prayers: ChurchPrayer[]; icons: ChurchIcon[]; allIcons: ChurchIcon[]; saving: boolean; onSavePrayer: (payload: ChurchPrayerPayload, id?: string) => Promise<void>; onDeleteItem: (type: 'icon' | 'prayer' | 'article', id: string, title: string) => Promise<void> }) {
  const selectedPrayer = prayers[0] ?? null;
  const [editingPrayerId, setEditingPrayerId] = useState(selectedPrayer?.id ?? '');
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>(selectedPrayer?.language ?? 'uk');
  const [form, setForm] = useState({
    iconId: selectedPrayer?.iconId ?? icons[0]?.id ?? '',
    prayerType: selectedPrayer?.prayerType ?? 'prayer',
    title: selectedPrayer?.title ?? '',
    slug: selectedPrayer?.slug ?? '',
    language: selectedPrayer?.language ?? 'uk',
    status: selectedPrayer?.status ?? 'draft',
    text: selectedPrayer?.text ?? '',
    audioUrl: selectedPrayer?.audioUrl ?? '',
    imageUrl: selectedPrayer?.imageUrl ?? '',
    source: selectedPrayer?.source ?? '',
    sourceUrl: selectedPrayer?.sourceUrl ?? '',
    note: selectedPrayer?.note ?? '',
    qrCodeUrl: selectedPrayer?.qrCodeUrl ?? ''
  });
  const [error, setError] = useState('');
  const [uploadingAudio, setUploadingAudio] = useState(false);
  const [uploadingImage, setUploadingImage] = useState(false);
  const [uploadingQr, setUploadingQr] = useState(false);
  const [qrPreview, setQrPreview] = useState('');

  useEffect(() => {
    setEditingPrayerId(selectedPrayer?.id ?? '');
    setActiveLanguage(selectedPrayer?.language ?? 'uk');
    setForm({
      iconId: selectedPrayer?.iconId ?? icons[0]?.id ?? '',
      prayerType: selectedPrayer?.prayerType ?? 'prayer',
      title: selectedPrayer?.title ?? '',
      slug: selectedPrayer?.slug ?? '',
      language: selectedPrayer?.language ?? 'uk',
      status: selectedPrayer?.status ?? 'draft',
      text: selectedPrayer?.text ?? '',
      audioUrl: selectedPrayer?.audioUrl ?? '',
      imageUrl: selectedPrayer?.imageUrl ?? '',
      source: selectedPrayer?.source ?? '',
      sourceUrl: selectedPrayer?.sourceUrl ?? '',
      note: selectedPrayer?.note ?? '',
      qrCodeUrl: selectedPrayer?.qrCodeUrl ?? prayerQrPath(selectedPrayer?.slug ?? '')
    });
    setError('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedPrayer?.id, icons[0]?.id]);

  useEffect(() => {
    const path = prayerQrPath(form.slug);
    if (!path) {
      setQrPreview('');
      return;
    }
    const target = `${churchPublicUrl}${path}`;
    let cancelled = false;
    void QRCode.toDataURL(target, { margin: 1, width: 220, color: { dark: '#111827', light: '#ffffff' } })
      .then((dataUrl) => { if (!cancelled) setQrPreview(dataUrl); })
      .catch(() => { if (!cancelled) setQrPreview(''); });
    return () => { cancelled = true; };
  }, [form.slug]);

  const siblingPrayers = useMemo(() => (form.slug ? prayers.filter((prayer) => prayer.slug === form.slug) : []), [prayers, form.slug]);
  const languageStatuses = useMemo(
    () => contentLanguageStatus(siblingPrayers, (prayer) => Boolean(prayer.text.trim()) && prayer.status === 'published'),
    [siblingPrayers]
  );

  function switchLanguage(language: CmsLanguage) {
    const sibling = siblingPrayers.find((prayer) => prayer.language === language);
    setActiveLanguage(language);
    setEditingPrayerId(sibling?.id ?? '');
    setForm((current) => ({
      ...current,
      language,
      title: sibling?.title ?? '',
      text: sibling?.text ?? '',
      status: sibling?.status ?? 'draft',
      audioUrl: sibling?.audioUrl ?? '',
      imageUrl: sibling?.imageUrl ?? '',
      source: sibling?.source ?? '',
      sourceUrl: sibling?.sourceUrl ?? '',
      note: sibling?.note ?? '',
      qrCodeUrl: sibling?.qrCodeUrl ?? ''
    }));
    setError('');
  }

  function startNewPrayer() {
    setEditingPrayerId('');
    setActiveLanguage('uk');
    setForm({
      iconId: icons[0]?.id ?? '',
      prayerType: 'prayer',
      title: '',
      slug: '',
      language: 'uk',
      status: 'draft',
      text: '',
      audioUrl: '',
      imageUrl: '',
      source: '',
      sourceUrl: '',
      note: '',
      qrCodeUrl: ''
    });
    setError('');
  }

  function editPrayer(prayer: ChurchPrayer) {
    setEditingPrayerId(prayer.id);
    setActiveLanguage(prayer.language);
    setForm({
      iconId: prayer.iconId ?? icons[0]?.id ?? '',
      prayerType: prayer.prayerType,
      title: prayer.title,
      slug: prayer.slug,
      language: prayer.language,
      status: prayer.status,
      text: prayer.text,
      audioUrl: prayer.audioUrl ?? '',
      imageUrl: prayer.imageUrl ?? '',
      source: prayer.source ?? '',
      sourceUrl: prayer.sourceUrl ?? '',
      note: prayer.note ?? '',
      qrCodeUrl: prayer.qrCodeUrl ?? ''
    });
    setError('');
  }

  async function uploadPrayerAudio(file: File | null) {
    if (!file) return;
    const isMp3 = file.type === 'audio/mpeg' || file.name.toLowerCase().endsWith('.mp3');
    if (!isMp3) {
      setError('Загрузите аудио в формате MP3.');
      return;
    }
    if (file.size > 50 * 1024 * 1024) {
      setError('MP3-файл должен быть меньше 50 MB.');
      return;
    }
    setUploadingAudio(true);
    setError('');
    try {
      const audioUrl = await uploadPrayerAudioFile(file);
      setForm((current) => ({ ...current, audioUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить MP3.');
    } finally {
      setUploadingAudio(false);
    }
  }

  async function uploadPrayerImage(file: File | null) {
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      setError('Выберите файл изображения.');
      return;
    }
    if (file.size > 10 * 1024 * 1024) {
      setError('Фото должно быть меньше 10 MB.');
      return;
    }
    setUploadingImage(true);
    setError('');
    try {
      const imageUrl = await uploadCmsReference(file);
      setForm((current) => ({ ...current, imageUrl }));
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Не удалось загрузить фото.');
    } finally {
      setUploadingImage(false);
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название молитвы обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.text.trim()) return setError('Текст молитвы обязателен.');
    if (form.audioUrl && !isValidUrl(form.audioUrl)) return setError('MP3 должен быть корректным http/https URL.');
    setError('');

    let qrCodeUrl = form.qrCodeUrl;
    if (qrPreview) {
      setUploadingQr(true);
      try {
        const qrFile = await dataUrlToFile(qrPreview, `qr-${form.slug}.png`);
        qrCodeUrl = await uploadCmsReference(qrFile);
      } catch (uploadError) {
        setUploadingQr(false);
        setError(uploadError instanceof Error ? uploadError.message : 'Не удалось сохранить QR-код.');
        return;
      }
      setUploadingQr(false);
    }

    void onSavePrayer({
      ...form,
      iconId: form.iconId || undefined,
      audioUrl: form.audioUrl || undefined,
      imageUrl: form.imageUrl || undefined,
      qrCodeUrl: qrCodeUrl || undefined
    } as ChurchPrayerPayload, editingPrayerId || undefined);
  }

  const selectedIconImage = imageForContent(form.iconId, icons, allIcons);
  const qrTarget = form.slug ? `${churchPublicUrl}${prayerQrPath(form.slug)}` : '';

  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(null, [], [], prayers, [])[3]} description="Заполните молитву прямо в основной панели: тип, текст, фото, MP3-аудио и QR-ссылка для публичной страницы — на трёх языках." />
      <form className="church-prayer-workspace-form" onSubmit={submit}>
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={switchLanguage} />
        <div className="church-prayer-main-fields">
          <label><span>Икона для молитвы</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">Без иконы</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
          <label><span>Тип молитвы</span><select value={form.prayerType} onChange={(event) => setForm({ ...form, prayerType: event.target.value as ChurchPrayerType })}>{prayerTypes.map((type) => <option key={type} value={type}>{prayerTypeLabels[type]}</option>)}</select></label>
          <label><span>Название ({activeLanguage.toUpperCase()})</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Тропарь Рождества" /></label>
          <label><span>Slug страницы (общий для всех языков)</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="tropar-rozhdestva" /></label>
          <label><span>Статус ({activeLanguage.toUpperCase()})</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
          <label className="admin-form-wide"><span>Текст молитвы ({activeLanguage.toUpperCase()})</span><textarea value={form.text} onChange={(event) => setForm({ ...form, text: event.target.value })} placeholder="Введите текст молитвы, тропаря или кондака" /></label>
          <label><span>Источник ({activeLanguage.toUpperCase()})</span><input value={form.source} onChange={(event) => setForm({ ...form, source: event.target.value })} placeholder="Православный молитвослов" /></label>
          <label><span>Ссылка на первоисточник ({activeLanguage.toUpperCase()})</span><input value={form.sourceUrl} onChange={(event) => setForm({ ...form, sourceUrl: event.target.value })} placeholder="https://..." /></label>
          <label className="admin-form-wide"><span>Благословение / примечание ({activeLanguage.toUpperCase()})</span><textarea value={form.note} onChange={(event) => setForm({ ...form, note: event.target.value })} placeholder="Например: текст приведён по современному изданию с благословения..." /></label>
        </div>

        <div className="church-prayer-media-grid">
          <article>
            <div className="church-prayer-media-preview">{form.imageUrl ? <img src={form.imageUrl} alt={form.title || 'Фото молитвы'} /> : selectedIconImage ? <img src={selectedIconImage} alt={form.title || 'Икона молитвы'} /> : <span><AppIcon name="image" /></span>}</div>
            <div>
              <strong>Фото молитвенного листа ({activeLanguage.toUpperCase()})</strong>
              <p>Своё фото для этого языка. Если не загрузить — на сайте покажется фото связанной иконы.</p>
              <label className="church-upload-button compact"><input type="file" accept="image/*" onChange={(event) => void uploadPrayerImage(event.target.files?.[0] ?? null)} disabled={saving || uploadingImage} /><span>{uploadingImage ? 'Загрузка фото...' : 'Загрузить фото'}</span></label>
              <input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://.../prayer.jpg" />
            </div>
          </article>
          <article>
            <div className="church-prayer-media-preview audio"><AppIcon name="sparkles" /></div>
            <div>
              <strong>MP3-аудио молитвы ({activeLanguage.toUpperCase()})</strong>
              <p>Своя запись для этого языка.</p>
              <label className="church-upload-button compact"><input type="file" accept="audio/mpeg,.mp3" onChange={(event) => void uploadPrayerAudio(event.target.files?.[0] ?? null)} disabled={saving || uploadingAudio} /><span>{uploadingAudio ? 'Загрузка MP3...' : 'Загрузить MP3'}</span></label>
              <input value={form.audioUrl} onChange={(event) => setForm({ ...form, audioUrl: event.target.value })} placeholder="https://.../prayer.mp3" />
              {form.audioUrl ? <audio controls src={form.audioUrl}>Ваш браузер не поддерживает аудио.</audio> : null}
            </div>
          </article>
          <article>
            <div className="church-prayer-media-preview qr">{qrPreview ? <img src={qrPreview} alt="QR молитвы" /> : <AppIcon name="qr" />}</div>
            <div>
              <strong>QR для молитвы</strong>
              <p>QR формируется автоматически по ссылке на публичную страницу и загружается как изображение при сохранении.</p>
              <small>{qrTarget || 'Введите slug, чтобы получить QR-ссылку'}</small>
              {form.qrCodeUrl ? <a href={form.qrCodeUrl} target="_blank" rel="noreferrer">Текущий загруженный QR</a> : null}
            </div>
          </article>
        </div>

        {error ? <small className="admin-form-error">{error}</small> : null}
        <div className="church-prayer-form-actions">
          <ActionButton type="button" onClick={startNewPrayer} disabled={saving}>Новая молитва</ActionButton>
          <ActionButton icon="save" tone="primary" type="submit" disabled={saving || uploadingAudio || uploadingImage || uploadingQr}>{saving || uploadingQr ? 'Сохраняем' : editingPrayerId ? `Сохранить (${activeLanguage.toUpperCase()})` : `Создать (${activeLanguage.toUpperCase()})`}</ActionButton>
        </div>
      </form>

      <WorkflowDataAudit
        items={[
          ['Связанная молитва', form.title || selectedPrayer?.title || 'не создана'],
          ['Тип', prayerTypeLabels[form.prayerType]],
          ['URL молитвы', form.slug ? `/church/prayers/${form.slug}` : 'не заполнен'],
          ['Икона молитвы', form.iconId ? 'связана' : icons[0] ? 'можно выбрать первую икону дня' : 'не связана'],
          ['Фото листа', form.imageUrl ? 'загружено' : 'не загружено'],
          ['MP3-аудио', form.audioUrl ? 'загружено' : 'не загружено'],
          ['QR', qrTarget ? 'готов' : 'не создан'],
          ['Язык', form.language.toUpperCase()],
          ['Текст', form.text ? 'заполнен' : 'пусто'],
          ['Источник', form.source || 'не указан'],
          ['Примечание', form.note ? 'заполнено' : 'нет']
        ]}
      />

      {prayers.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Связанные молитвы</strong><ActionButton icon="sparkles" onClick={startNewPrayer}>Добавить молитву</ActionButton></div>
          <div className="church-content-list">
            {prayers.map((prayer) => (
              <ContentRow key={prayer.id} title={prayer.title} meta={`${prayer.language} · ${prayerTypeLabels[prayer.prayerType]} · ${statusLabels[prayer.status]} · /prayers/${prayer.slug}${prayer.imageUrl ? ' · Фото' : ''}${prayer.audioUrl ? ' · MP3' : ''}${prayer.qrCodeUrl ? ' · QR' : ''}`} body={prayer.text} image={prayer.imageUrl || imageForContent(prayer.iconId, icons, allIcons)} onEdit={() => editPrayer(prayer)} onDelete={() => void onDeleteItem('prayer', prayer.id, prayer.title)} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function ArticleWorkflowStep({ articles, icons, allIcons, saving, onSaveArticle, onDeleteItem }: { articles: ChurchArticle[]; icons: ChurchIcon[]; allIcons: ChurchIcon[]; saving: boolean; onSaveArticle: (payload: ChurchArticlePayload, id?: string) => Promise<void>; onDeleteItem: (type: 'icon' | 'prayer' | 'article', id: string, title: string) => Promise<void> }) {
  const selectedArticle = articles[0] ?? null;
  const [editingArticleId, setEditingArticleId] = useState(selectedArticle?.id ?? '');
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>(selectedArticle?.language ?? 'uk');
  const [form, setForm] = useState({
    iconId: selectedArticle?.iconId ?? icons[0]?.id ?? '',
    title: selectedArticle?.title ?? '',
    slug: selectedArticle?.slug ?? '',
    language: selectedArticle?.language ?? 'uk',
    seoTitle: selectedArticle?.seoTitle ?? '',
    seoDescription: selectedArticle?.seoDescription ?? '',
    status: selectedArticle?.status ?? 'draft',
    content: selectedArticle?.content ?? ''
  });
  const [error, setError] = useState('');

  useEffect(() => {
    setEditingArticleId(selectedArticle?.id ?? '');
    setActiveLanguage(selectedArticle?.language ?? 'uk');
    setForm({
      iconId: selectedArticle?.iconId ?? icons[0]?.id ?? '',
      title: selectedArticle?.title ?? '',
      slug: selectedArticle?.slug ?? '',
      language: selectedArticle?.language ?? 'uk',
      seoTitle: selectedArticle?.seoTitle ?? '',
      seoDescription: selectedArticle?.seoDescription ?? '',
      status: selectedArticle?.status ?? 'draft',
      content: selectedArticle?.content ?? ''
    });
    setError('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedArticle?.id, icons[0]?.id]);

  const siblingArticles = useMemo(() => (form.slug ? articles.filter((article) => article.slug === form.slug) : []), [articles, form.slug]);
  const languageStatuses = useMemo(
    () => contentLanguageStatus(siblingArticles, (article) => Boolean(article.seoTitle.trim() && article.seoDescription.trim()) && article.status === 'published'),
    [siblingArticles]
  );

  function switchLanguage(language: CmsLanguage) {
    const sibling = siblingArticles.find((article) => article.language === language);
    setActiveLanguage(language);
    setEditingArticleId(sibling?.id ?? '');
    setForm((current) => ({
      ...current,
      language,
      title: sibling?.title ?? '',
      seoTitle: sibling?.seoTitle ?? '',
      seoDescription: sibling?.seoDescription ?? '',
      status: sibling?.status ?? 'draft',
      content: sibling?.content ?? ''
    }));
    setError('');
  }

  function startNewArticle() {
    setEditingArticleId('');
    setActiveLanguage('uk');
    setForm({
      iconId: icons[0]?.id ?? '',
      title: '',
      slug: '',
      language: 'uk',
      seoTitle: '',
      seoDescription: '',
      status: 'draft',
      content: ''
    });
    setError('');
  }

  function editArticle(article: ChurchArticle) {
    setEditingArticleId(article.id);
    setActiveLanguage(article.language);
    setForm({
      iconId: article.iconId ?? icons[0]?.id ?? '',
      title: article.title,
      slug: article.slug,
      language: article.language,
      seoTitle: article.seoTitle,
      seoDescription: article.seoDescription,
      status: article.status,
      content: article.content
    });
    setError('');
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название статьи обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.content.trim()) return setError('Текст статьи обязателен.');
    setError('');
    void onSaveArticle({
      ...form,
      iconId: form.iconId || undefined,
      seoTitle: form.seoTitle || form.title,
      seoDescription: form.seoDescription || form.content.replace(/\s+/g, ' ').trim().slice(0, 180)
    } as ChurchArticlePayload, editingArticleId || undefined);
  }

  const articleImage = imageForContent(form.iconId, icons, allIcons);
  const previewUrl = form.slug ? `/church/articles/${form.slug}` : 'не заполнен';

  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(null, [], [], [], articles)[4]} description="Заполните статью прямо в основной панели: заголовок, slug, SEO и полный текст." />
      <form className="church-article-workspace-form" onSubmit={submit}>
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={switchLanguage} />
        <div className="church-article-main-fields">
          <label><span>Икона для статьи</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">Без иконы</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
          <label><span>Название ({activeLanguage.toUpperCase()})</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="История, смысл и духовное значение" /></label>
          <label><span>Slug статьи (общий для всех языков)</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="den-svyatyh-apostolov-petra-i-pavla" /></label>
          <label><span>Статус ({activeLanguage.toUpperCase()})</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
          <label><span>SEO title ({activeLanguage.toUpperCase()})</span><input value={form.seoTitle} onChange={(event) => setForm({ ...form, seoTitle: event.target.value })} placeholder="Если пусто, возьмем название статьи" /></label>
          <label className="admin-form-wide"><span>SEO description</span><textarea value={form.seoDescription} onChange={(event) => setForm({ ...form, seoDescription: event.target.value })} placeholder="Краткое описание для поисковиков и preview" /></label>
          <label className="admin-form-wide article-body"><span>Текст статьи</span><textarea value={form.content} onChange={(event) => setForm({ ...form, content: event.target.value })} placeholder="Полный текст статьи, жития или SEO-материала" /></label>
        </div>

        <div className="church-article-preview-grid">
          <article>
            <div className="church-article-preview-image">{articleImage ? <img src={articleImage} alt={form.title || 'Изображение статьи'} /> : <span><AppIcon name="cms" /></span>}</div>
            <div>
              <strong>{form.title || 'Новая статья'}</strong>
              <small>{form.language.toUpperCase()} · {statusLabels[form.status]} · {previewUrl}</small>
              <p>{form.seoDescription || form.content.replace(/\s+/g, ' ').trim().slice(0, 180) || 'Описание появится после заполнения SEO или текста статьи.'}</p>
            </div>
          </article>
          <dl>
            <div><dt>URL статьи</dt><dd>{previewUrl}</dd></div>
            <div><dt>SEO title</dt><dd>{form.seoTitle ? 'заполнен' : 'будет из названия'}</dd></div>
            <div><dt>SEO description</dt><dd>{form.seoDescription ? 'заполнен' : 'будет из текста'}</dd></div>
            <div><dt>Изображение</dt><dd>{articleImage ? 'берется из иконы' : 'нет'}</dd></div>
          </dl>
        </div>

        {error ? <small className="admin-form-error">{error}</small> : null}
        <div className="church-article-form-actions">
          <ActionButton type="button" onClick={startNewArticle} disabled={saving}>Новая статья</ActionButton>
          <ActionButton icon="save" tone="primary" type="submit" disabled={saving}>{saving ? 'Сохраняем' : editingArticleId ? 'Сохранить статью' : 'Создать статью'}</ActionButton>
        </div>
      </form>

      <WorkflowDataAudit
        items={[
          ['Связанная статья', form.title || selectedArticle?.title || 'не создана'],
          ['URL статьи', previewUrl],
          ['SEO title', form.seoTitle ? 'заполнен' : 'пусто'],
          ['SEO description', form.seoDescription ? 'заполнен' : 'пусто'],
          ['Язык', form.language.toUpperCase()],
          ['Изображение', articleImage ? 'берется из иконы' : 'нет']
        ]}
      />

      {articles.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Связанные статьи</strong><ActionButton icon="sparkles" onClick={startNewArticle}>Добавить статью</ActionButton></div>
          <div className="church-content-list">
            {articles.map((article) => (
              <ContentRow key={article.id} title={article.title} meta={`${article.language} · ${statusLabels[article.status]} · /church/articles/${article.slug}`} body={article.seoDescription || article.content} image={imageForContent(article.iconId, icons, allIcons)} onEdit={() => editArticle(article)} onDelete={() => void onDeleteItem('article', article.id, article.title)} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function GospelWorkflowStep({ gospel, icons, allIcons, saving, onSaveGospel, onDeleteItem }: { gospel: ChurchGospel[]; icons: ChurchIcon[]; allIcons: ChurchIcon[]; saving: boolean; onSaveGospel: (payload: ChurchGospelPayload, id?: string) => Promise<void>; onDeleteItem: (type: 'icon' | 'prayer' | 'article' | 'gospel', id: string, title: string) => Promise<void> }) {
  const selectedGospel = gospel[0] ?? null;
  const [editingGospelId, setEditingGospelId] = useState(selectedGospel?.id ?? '');
  const [activeLanguage, setActiveLanguage] = useState<CmsLanguage>(selectedGospel?.language ?? 'uk');
  const [form, setForm] = useState({
    iconId: selectedGospel?.iconId ?? icons[0]?.id ?? '',
    title: selectedGospel?.title ?? '',
    slug: selectedGospel?.slug ?? '',
    language: selectedGospel?.language ?? 'uk',
    reference: selectedGospel?.reference ?? '',
    text: selectedGospel?.text ?? '',
    explanation: selectedGospel?.explanation ?? '',
    status: selectedGospel?.status ?? 'draft'
  });
  const [error, setError] = useState('');

  useEffect(() => {
    setEditingGospelId(selectedGospel?.id ?? '');
    setActiveLanguage(selectedGospel?.language ?? 'uk');
    setForm({
      iconId: selectedGospel?.iconId ?? icons[0]?.id ?? '',
      title: selectedGospel?.title ?? '',
      slug: selectedGospel?.slug ?? '',
      language: selectedGospel?.language ?? 'uk',
      reference: selectedGospel?.reference ?? '',
      text: selectedGospel?.text ?? '',
      explanation: selectedGospel?.explanation ?? '',
      status: selectedGospel?.status ?? 'draft'
    });
    setError('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedGospel?.id, icons[0]?.id]);

  const siblingGospel = useMemo(() => (form.slug ? gospel.filter((item) => item.slug === form.slug) : []), [gospel, form.slug]);
  const languageStatuses = useMemo(
    () => contentLanguageStatus(siblingGospel, (item) => Boolean(item.text.trim()) && item.status === 'published'),
    [siblingGospel]
  );

  function switchLanguage(language: CmsLanguage) {
    const sibling = siblingGospel.find((item) => item.language === language);
    setActiveLanguage(language);
    setEditingGospelId(sibling?.id ?? '');
    setForm((current) => ({
      ...current,
      language,
      title: sibling?.title ?? '',
      reference: sibling?.reference ?? '',
      text: sibling?.text ?? '',
      explanation: sibling?.explanation ?? '',
      status: sibling?.status ?? 'draft'
    }));
    setError('');
  }

  function startNewGospel() {
    setEditingGospelId('');
    setActiveLanguage('uk');
    setForm({
      iconId: icons[0]?.id ?? '',
      title: '',
      slug: '',
      language: 'uk',
      reference: '',
      text: '',
      explanation: '',
      status: 'draft'
    });
    setError('');
  }

  function editGospel(item: ChurchGospel) {
    setEditingGospelId(item.id);
    setActiveLanguage(item.language);
    setForm({
      iconId: item.iconId ?? icons[0]?.id ?? '',
      title: item.title,
      slug: item.slug,
      language: item.language,
      reference: item.reference,
      text: item.text,
      explanation: item.explanation,
      status: item.status
    });
    setError('');
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название чтения обязательно.');
    if (!isValidSlug(form.slug)) return setError('Slug должен быть в формате latin-slug.');
    if (!form.text.trim()) return setError('Текст Евангелия обязателен.');
    setError('');
    void onSaveGospel({ ...form, iconId: form.iconId || undefined } as ChurchGospelPayload, editingGospelId || undefined);
  }

  const gospelImage = imageForContent(form.iconId, icons, allIcons);
  const previewUrl = form.slug ? `/church/gospel/${form.slug}` : 'не заполнен';

  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(null, [], [], [], [], gospel)[5]} description="Необязательный шаг: добавьте евангельское чтение дня — ссылку, текст и простое объяснение, на трёх языках." />
      <form className="church-article-workspace-form" onSubmit={submit}>
        <LanguageTabs active={activeLanguage} statuses={languageStatuses} onChange={switchLanguage} />
        <div className="church-article-main-fields">
          <label><span>Икона (необязательно)</span><select value={form.iconId} onChange={(event) => setForm({ ...form, iconId: event.target.value })}><option value="">Без иконы</option>{icons.map((icon) => <option key={icon.id} value={icon.id}>{icon.title}</option>)}</select></label>
          <label><span>Название ({activeLanguage.toUpperCase()})</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="Неделя 32-я по Пятидесятнице" /></label>
          <label><span>Slug (общий для всех языков)</span><input value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="nedelya-32-po-pyatidesyatnitse" /></label>
          <label><span>Ссылка на чтение ({activeLanguage.toUpperCase()})</span><input value={form.reference} onChange={(event) => setForm({ ...form, reference: event.target.value })} placeholder="Мф. 5:14-16" /></label>
          <label><span>Статус ({activeLanguage.toUpperCase()})</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
          <label className="admin-form-wide"><span>Текст чтения ({activeLanguage.toUpperCase()})</span><textarea value={form.text} onChange={(event) => setForm({ ...form, text: event.target.value })} placeholder="Текст евангельского чтения" /></label>
          <label className="admin-form-wide"><span>Простое объяснение ({activeLanguage.toUpperCase()})</span><textarea value={form.explanation} onChange={(event) => setForm({ ...form, explanation: event.target.value })} placeholder="Спокойное объяснение простыми словами" /></label>
        </div>

        <div className="church-article-preview-grid">
          <article>
            <div className="church-article-preview-image">{gospelImage ? <img src={gospelImage} alt={form.title || 'Изображение чтения'} /> : <span><AppIcon name="globe" /></span>}</div>
            <div>
              <strong>{form.title || 'Новое чтение'}</strong>
              <small>{form.language.toUpperCase()} · {statusLabels[form.status]} · {previewUrl}</small>
              <p>{form.explanation || form.text.replace(/\s+/g, ' ').trim().slice(0, 180) || 'Описание появится после заполнения текста или объяснения.'}</p>
            </div>
          </article>
          <dl>
            <div><dt>URL чтения</dt><dd>{previewUrl}</dd></div>
            <div><dt>Ссылка</dt><dd>{form.reference || 'не заполнена'}</dd></div>
            <div><dt>Объяснение</dt><dd>{form.explanation ? 'заполнено' : 'нет'}</dd></div>
          </dl>
        </div>

        {error ? <small className="admin-form-error">{error}</small> : null}
        <div className="church-article-form-actions">
          <ActionButton type="button" onClick={startNewGospel} disabled={saving}>Новое чтение</ActionButton>
          <ActionButton icon="save" tone="primary" type="submit" disabled={saving}>{saving ? 'Сохраняем' : editingGospelId ? 'Сохранить чтение' : 'Создать чтение'}</ActionButton>
        </div>
      </form>

      <WorkflowDataAudit
        items={[
          ['Связанное чтение', form.title || selectedGospel?.title || 'не создано'],
          ['URL чтения', previewUrl],
          ['Ссылка', form.reference || 'не заполнена'],
          ['Язык', form.language.toUpperCase()],
          ['Текст', form.text ? 'заполнен' : 'пусто']
        ]}
      />

      {gospel.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Связанные чтения</strong><ActionButton icon="sparkles" onClick={startNewGospel}>Добавить чтение</ActionButton></div>
          <div className="church-content-list">
            {gospel.map((item) => (
              <ContentRow key={item.id} title={item.title} meta={`${item.language} · ${statusLabels[item.status]} · /church/gospel/${item.slug}`} body={item.explanation || item.text} image={imageForContent(item.iconId, icons, allIcons)} onEdit={() => editGospel(item)} onDelete={() => void onDeleteItem('gospel', item.id, item.title)} />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function SeoPublishWorkflowStep({ day, saints, icons, prayers, articles, saving, onPublish, onDraft, onPreview, onOpenPath }: { day: ChurchCalendarDay; saints: ChurchSaint[]; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; saving: boolean; onPublish: () => void; onDraft: () => void; onPreview: () => void; onOpenPath: (path: string) => void }) {
  const checks = [
    { label: 'День заполнен', ok: isCalendarDayComplete(day) },
    { label: 'Святой создан', ok: saints.length > 0 },
    { label: 'Икона связана', ok: icons.length > 0 },
    { label: 'Молитва связана', ok: prayers.length > 0 },
    { label: 'Статья связана', ok: articles.length > 0 },
    { label: 'SEO заполнено', ok: articles.some((article) => Boolean(article.slug.trim() && article.seoTitle.trim() && article.seoDescription.trim())) }
  ];
  const article = articles[0];
  return (
    <section className="church-workflow-panel">
      <CalendarWorkflowIntro step={buildWorkflowSteps(day, saints, icons, prayers, articles)[6]} description="Публикация сверяется по данным, которые реально уходят на сайт: день, святой, икона, молитва, статья и SEO статьи." />
      <WorkflowDataAudit
        items={[
          ['Страница дня', day.dateNewStyle ? `/church/calendar/${day.dateNewStyle}` : 'дата не задана'],
          ['Статья canonical', article?.slug ? `/church/articles/${article.slug}` : 'нет статьи'],
          ['Meta title', article?.seoTitle || 'не заполнен'],
          ['Meta description', article?.seoDescription || 'не заполнен'],
          ['Изображение', icons[0]?.imageUrl ? 'есть' : 'не загружено'],
          ['Статус дня', day.status]
        ]}
      />
      <div className="church-workflow-checklist">
        {[
          ...checks,
          { label: 'Slug статьи заполнен', ok: Boolean(article?.slug.trim()) },
          { label: 'Главное фото есть', ok: Boolean(icons[0]?.imageUrl) },
          { label: 'Preview URL есть', ok: Boolean(day.dateNewStyle || day.dateOldStyle) }
        ].map((check) => <span key={check.label} className={check.ok ? 'ok' : 'warn'}><i />{check.label}</span>)}
      </div>
      <PublishTab day={day} saints={saints} icons={icons} prayers={prayers} articles={articles} saving={saving} onPublish={onPublish} onDraft={onDraft} onPreview={onPreview} onOpenPath={onOpenPath} />
    </section>
  );
}

function PublishTab({ day, saints, icons, prayers, articles, saving, onPublish, onDraft, onPreview, onOpenPath }: { day: ChurchCalendarDay; saints: ChurchSaint[]; icons: ChurchIcon[]; prayers: ChurchPrayer[]; articles: ChurchArticle[]; saving: boolean; onPublish: () => void; onDraft: () => void; onPreview: () => void; onOpenPath: (path: string) => void }) {
  const draftItems = [
    ...saints.filter((item) => item.status === 'draft').map((item) => item.name),
    ...icons.filter((item) => item.status === 'draft').map((item) => item.title),
    ...prayers.filter((item) => item.status === 'draft').map((item) => item.title),
    ...articles.filter((item) => item.status === 'draft').map((item) => item.title)
  ];
  const checks: Array<{ label: string; ok: boolean; tone?: 'warn' }> = [
    { label: 'Название дня', ok: Boolean(day.title.trim()) },
    { label: 'Новый или старый стиль даты', ok: Boolean(day.dateNewStyle || day.dateOldStyle) },
    { label: 'Хотя бы один святой', ok: saints.length > 0 },
    { label: 'Хотя бы одна икона', ok: icons.length > 0 },
    { label: 'Молитва или тропарь', ok: prayers.length > 0 },
    { label: 'Статья / описание', ok: articles.length > 0 || Boolean(day.description.trim()) },
    { label: 'Slug иконы заполнен', ok: icons.every((item) => isValidSlug(item.slug)) },
    { label: 'Slug святого заполнен', ok: saints.every((item) => isValidSlug(item.slug)) },
    { label: 'Slug молитвы заполнен', ok: prayers.every((item) => isValidSlug(item.slug)) },
    { label: 'Slug статьи заполнен', ok: articles.every((item) => isValidSlug(item.slug)) },
    { label: 'Фото иконы есть', ok: icons.length > 0 && icons.every((item) => Boolean(item.imageUrl.trim()) && isValidUrl(item.imageUrl)) },
    { label: 'SEO title заполнен', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoTitle.trim())) },
    { label: 'SEO description заполнен', ok: articles.length > 0 && articles.every((item) => Boolean(item.seoDescription.trim())) },
    { label: draftItems.length ? `Черновики без причины: ${draftItems.slice(0, 2).join(', ')}` : 'Нет необъяснённых черновиков', ok: draftItems.length === 0, tone: 'warn' }
  ];
  const publicDate = day.dateNewStyle || day.dateOldStyle || '';
  const publicLinks = [
    publicDate ? { label: 'День в календаре', path: `/church/calendar/${publicDate}` } : null,
    saints[0]?.slug ? { label: 'Страница святого', path: `/saints/${saints[0].slug}` } : null,
    icons[0]?.slug ? { label: 'Страница иконы', path: `/icons/${icons[0].slug}` } : null,
    prayers[0]?.slug ? { label: 'Страница молитвы', path: `/church/prayers/${prayers[0].slug}` } : null,
    articles[0]?.slug ? { label: 'Страница статьи', path: `/church/articles/${articles[0].slug}` } : null
  ].filter(Boolean) as Array<{ label: string; path: string }>;
  return (
    <div className="church-publish-panel">
      <div className="church-public-links">
        <div>
          <strong>Публичные ссылки</strong>
          <span>Публикация делает доступными эти прямые страницы.</span>
        </div>
        <div>
          {publicLinks.map((link) => (
            <button key={link.path} type="button" onClick={() => onOpenPath(link.path)} disabled={saving || day.status !== 'published'}>
              <AppIcon name="external" />
              <span>{link.label}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="church-generated-preview">
        <strong>Предпросмотр созданного контента</strong>
        <div>
          {icons[0] ? (
            <article>
              {icons[0].imageUrl ? <img src={icons[0].imageUrl} alt={icons[0].title} loading="lazy" /> : <span><AppIcon name="image" /></span>}
              <div>
                <b>{icons[0].title}</b>
                <small>{icons[0].imageUrl ? 'Фото иконы подключено' : 'Фото не добавлено. Откройте шаг «Икона» и загрузите с ПК или вставьте URL.'}</small>
                <p>{icons[0].description || 'Описание иконы пока пустое.'}</p>
              </div>
            </article>
          ) : <p>Икона ещё не создана.</p>}
          {prayers[0] ? (
            <article>
              <span><AppIcon name="sparkles" /></span>
              <div>
                <b>{prayers[0].title}</b>
                <small>{prayers[0].prayerType} · {prayers[0].language.toUpperCase()}</small>
                <p>{prayers[0].text || 'Текст молитвы пока пустой.'}</p>
              </div>
            </article>
          ) : <p>Молитва ещё не создана.</p>}
          {articles[0] ? (
            <article>
              <span><AppIcon name="cms" /></span>
              <div>
                <b>{articles[0].title}</b>
                <small>{articles[0].seoTitle || 'SEO title не заполнен'}</small>
                <p>{articles[0].seoDescription || articles[0].content || 'Текст статьи пока пустой.'}</p>
              </div>
            </article>
          ) : <p>Статья ещё не создана.</p>}
        </div>
      </div>
      <div className="church-publish-checks">
        {checks.map((check) => <span key={check.label} className={check.ok ? 'ok' : check.tone === 'warn' ? 'warn' : ''}><i />{check.label}</span>)}
      </div>
      <div className="church-publish-actions">
        <ActionButton icon="qr" onClick={onPreview} disabled={saving}>Предпросмотр</ActionButton>
        <ActionButton onClick={onDraft} disabled={saving}>В черновик</ActionButton>
        <ActionButton tone="primary" icon="save" onClick={onPublish} disabled={saving}>Опубликовать день</ActionButton>
      </div>
    </div>
  );
}

export function CalendarSiteModelPreview({ item, status, onEdit, onOpen }: { item: PublicCalendarDay; status: ChurchContentStatus; onEdit: () => void; onOpen: () => void }) {
  const links = [
    item.iconSlug ? { label: 'Икона', value: `/icons/${item.iconSlug}` } : null,
    item.prayerSlug ? { label: 'Молитва', value: `/church/prayers/${item.prayerSlug}` } : null,
    item.gospelSlug ? { label: 'Евангелие', value: `/gospel/${item.gospelSlug}` } : null,
    item.detailHref ? { label: 'День', value: item.detailHref } : null
  ].filter(Boolean) as Array<{ label: string; value: string }>;

  return (
    <section className={'church-site-calendar-model day-kind-' + item.kind}>
      <div className="church-site-calendar-model__date">
        <strong>{item.day}</strong>
        <span>{item.gregorianDate || 'дата не задана'}</span>
        {item.julianDate ? <small>{item.julianDate}</small> : null}
      </div>
      {item.imageUrl && !item.textOnly ? (
        <img src={item.imageUrl} alt={item.label} loading="lazy" />
      ) : (
        <div className="church-site-calendar-model__empty"><AppIcon name="calendar" /></div>
      )}
      <div className="church-site-calendar-model__copy">
        <p>Модель карточки сайта</p>
        <h3>{item.label || 'Без названия'}</h3>
        <span>{item.note || item.kind}</span>
        {item.description ? <em>{compactText(item.description, 220)}</em> : null}
        <div className="church-site-calendar-model__badges">
          <b>{item.kind}</b>
          {item.feast ? <b>праздник</b> : null}
          {item.current ? <b>сегодня</b> : null}
          <b>{status}</b>
        </div>
        {links.length ? (
          <div className="church-site-calendar-model__links">
            {links.map((link) => <code key={link.value}>{link.label}: {link.value}</code>)}
          </div>
        ) : null}
        <div className="church-site-calendar-model__actions">
          <ActionButton icon="sliders" onClick={onEdit}>Редактировать дату</ActionButton>
          <ActionButton icon="globe" onClick={onOpen} disabled={!item.gregorianDate && !item.julianDate}>Открыть на сайте</ActionButton>
        </div>
      </div>
    </section>
  );
}

export function CalendarDayForm({ item, saving, onSubmit, variant = 'drawer', onCancel, formId, hideActions = false }: {
  item?: ChurchCalendarDay;
  saving: boolean;
  onSubmit: (payload: ChurchCalendarDayPayload, id?: string) => Promise<void>;
  variant?: 'drawer' | 'inline' | 'workspace';
  onCancel?: () => void;
  formId?: string;
  hideActions?: boolean;
}) {
  const initialDateBasis: CalendarDateBasis = item?.calendarType === 'old_style' && item?.dateOldStyle && !item?.dateNewStyle ? 'old' : 'new';
  const initialDates = normalizeCalendarStyleDates(item?.dateNewStyle ?? '', item?.dateOldStyle ?? '', initialDateBasis);
  const [dateBasis, setDateBasis] = useState<CalendarDateBasis>(initialDateBasis);
  const [form, setForm] = useState({
    dateOldStyle: initialDates.oldStyle,
    dateNewStyle: initialDates.newStyle,
    title: item?.title ?? '',
    description: item?.description ?? '',
    calendarType: item?.calendarType ?? 'both',
    dayType: item?.dayType ?? 'saint',
    rank: String(item?.rank ?? 0),
    status: item?.status ?? 'draft'
  });
  const [error, setError] = useState('');
  function updateDateBasis(nextBasis: CalendarDateBasis) {
    setDateBasis(nextBasis);
    setForm((current) => ({
      ...current,
      ...normalizeCalendarStyleDates(current.dateNewStyle, current.dateOldStyle, nextBasis)
    }));
  }
  function updatePrimaryDate(value: string) {
    const dates = nextCalendarStyleDates(dateBasis, value);
    setForm((current) => ({ ...current, dateNewStyle: dates.newStyle, dateOldStyle: dates.oldStyle }));
  }
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.title.trim()) return setError('Название дня обязательно.');
    if (!form.dateOldStyle && !form.dateNewStyle) return setError('Выберите дату. Второй стиль будет рассчитан автоматически.');
    if (!form.description.trim()) return setError('Описание дня обязательно для публичной страницы.');
    setError('');
    const itemId = item?.id?.startsWith('new-calendar-day-') ? undefined : item?.id;
    void onSubmit({
      dateOldStyle: form.dateOldStyle,
      dateNewStyle: form.dateNewStyle,
      title: form.title,
      description: form.description,
      calendarType: form.calendarType,
      dayType: form.dayType,
      status: form.status,
      rank: Number(form.rank) || 0
    } as ChurchCalendarDayPayload, itemId);
  }
  const primaryDate = dateBasis === 'new' ? form.dateNewStyle : form.dateOldStyle;
  const secondaryDate = dateBasis === 'new' ? form.dateOldStyle : form.dateNewStyle;
  return (
    <form id={formId} className={'admin-form-grid calendar-day-form ' + variant} onSubmit={submit}>
      <fieldset className="church-cms-fieldset">
        <legend>Данные дня, которые видны на сайте</legend>
        <div className="calendar-date-style-control">
          <span>Дата календаря</span>
          <div className="calendar-date-style-control__switch" role="tablist" aria-label="Какую дату вводим">
            <button type="button" className={dateBasis === 'new' ? 'active' : ''} onClick={() => updateDateBasis('new')}>Новый стиль</button>
            <button type="button" className={dateBasis === 'old' ? 'active' : ''} onClick={() => updateDateBasis('old')}>Старый стиль</button>
          </div>
          <input type="date" value={primaryDate} onChange={(event) => updatePrimaryDate(event.target.value)} />
          <small>{dateBasis === 'new' ? 'Старый стиль считается автоматически' : 'Новый стиль считается автоматически'}: {secondaryDate || '-'}</small>
        </div>
        <label><span>Новый стиль</span><input type="date" value={form.dateNewStyle} readOnly /></label>
        <label><span>Старый стиль</span><input type="date" value={form.dateOldStyle} readOnly /></label>
        <label><span>Название</span><input value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} placeholder="День святых первоверховных апостолов Петра и Павла" /></label>
        <label><span>Тип дня</span><select value={form.dayType} onChange={(event) => setForm({ ...form, dayType: event.target.value as ChurchDayType })}>{dayTypes.map((type) => <option key={type} value={type}>{dayTypeLabels[type]}</option>)}</select></label>
        <label><span>Стиль календаря</span><select value={form.calendarType} onChange={(event) => setForm({ ...form, calendarType: event.target.value as ChurchCalendarType })}><option value="both">{calendarTypeLabels.both}</option><option value="old_style">{calendarTypeLabels.old_style}</option><option value="new_style">{calendarTypeLabels.new_style}</option></select></label>
        <label><span>Приоритет</span><input type="number" value={form.rank} onChange={(event) => setForm({ ...form, rank: event.target.value })} /></label>
        <label><span>Статус</span><select value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as ChurchContentStatus })}>{statuses.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}</select></label>
        <label className="admin-form-wide"><span>Описание</span><textarea value={form.description} onChange={(event) => setForm({ ...form, description: event.target.value })} placeholder="Короткое описание дня, праздника или святого для hero-блока страницы" /></label>
      </fieldset>
      {error ? <small className="admin-form-error">{error}</small> : null}
      {!hideActions ? (
        <div className="calendar-day-form-actions">
          {onCancel ? <ActionButton type="button" onClick={onCancel} disabled={saving}>Отмена</ActionButton> : null}
          <ActionButton tone="primary" icon="save" type="submit" disabled={saving}>{saving ? 'Сохраняем' : 'Сохранить'}</ActionButton>
        </div>
      ) : null}
    </form>
  );
}
