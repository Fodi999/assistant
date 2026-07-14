import { FormEvent, useState } from 'react';
import { AppIcon } from '../../../components/AppIcon';
import { ActionButton } from '../../../components/admin/ActionButton';
import { isValidUrl } from '../../../components/admin/forms/formUtils';
import type { ChurchCalendarType, ChurchDayType, ChurchLanguage } from '../../../api/churchContent';
import { calendarTypeLabels, dayTypeLabels, dayTypes, languages } from './churchTypes';
import type { ChurchAiDraftPayload } from './churchHelpers';

export function ChurchAiDraftForm({ saving, onSubmit }: { saving: boolean; onSubmit: (payload: ChurchAiDraftPayload) => Promise<void> }) {
  const today = new Date().toISOString().slice(0, 10);
  const [form, setForm] = useState<ChurchAiDraftPayload>({
    topic: '',
    dateOldStyle: '',
    dateNewStyle: today,
    calendarType: 'both',
    dayType: 'saint',
    language: 'uk',
    rank: '0',
    imageUrl: '',
    generateImage: true
  });
  const [error, setError] = useState('');
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.topic.trim()) return setError('Напишите святого, праздник или церковный день для Gemini.');
    if (!form.dateOldStyle && !form.dateNewStyle) return setError('Укажите дату по новому или старому стилю.');
    if (form.imageUrl && !isValidUrl(form.imageUrl)) return setError('Фото должно быть http/https URL.');
    setError('');
    void onSubmit(form);
  }
  return (
    <form className="admin-form-grid church-ai-draft-form" onSubmit={submit}>
      <div className="church-ai-help">
        <AppIcon name="bot" />
        <div>
          <strong>Gemini заполнит всю цепочку</strong>
          <span>Будет создан черновик: день → икона → молитва → статья. После этого можно открыть каждый блок и поправить текст вручную.</span>
        </div>
      </div>
      <label className="admin-form-wide"><span>Что создать</span><textarea value={form.topic} onChange={(event) => setForm({ ...form, topic: event.target.value })} placeholder="Например: Святитель Николай Чудотворец, Рождество Христово, Казанская икона Божией Матери" /></label>
      <label><span>Дата новый стиль</span><input type="date" value={form.dateNewStyle} onChange={(event) => setForm({ ...form, dateNewStyle: event.target.value })} /></label>
      <label><span>Дата старый стиль</span><input type="date" value={form.dateOldStyle} onChange={(event) => setForm({ ...form, dateOldStyle: event.target.value })} /></label>
      <label><span>Тип дня</span><select value={form.dayType} onChange={(event) => setForm({ ...form, dayType: event.target.value as ChurchDayType })}>{dayTypes.map((type) => <option key={type} value={type}>{dayTypeLabels[type]}</option>)}</select></label>
      <label><span>Календарь</span><select value={form.calendarType} onChange={(event) => setForm({ ...form, calendarType: event.target.value as ChurchCalendarType })}><option value="both">{calendarTypeLabels.both}</option><option value="old_style">{calendarTypeLabels.old_style}</option><option value="new_style">{calendarTypeLabels.new_style}</option></select></label>
      <label><span>Язык</span><select value={form.language} onChange={(event) => setForm({ ...form, language: event.target.value as ChurchLanguage })}>{languages.map((lang) => <option key={lang} value={lang}>{lang.toUpperCase()}</option>)}</select></label>
      <label><span>Приоритет</span><input type="number" value={form.rank} onChange={(event) => setForm({ ...form, rank: event.target.value })} /></label>
      <label className="admin-form-wide"><span>Существующий URL фото, необязательно</span><input value={form.imageUrl} onChange={(event) => setForm({ ...form, imageUrl: event.target.value })} placeholder="https://..." /></label>
      <label className="admin-inline-check"><input type="checkbox" checked={form.generateImage} onChange={(event) => setForm({ ...form, generateImage: event.target.checked })} /><span>Сгенерировать фото для иконы, если URL не указан</span></label>
      {error ? <small className="admin-form-error">{error}</small> : null}
      <ActionButton tone="primary" icon="bot" type="submit" disabled={saving}>{saving ? 'Gemini создает...' : 'Заполнить с помощью Gemini'}</ActionButton>
    </form>
  );
}
