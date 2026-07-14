import { AppIcon } from '../../../components/AppIcon';
import type { ChurchArticle, ChurchContentStatus, ChurchIcon, ChurchPrayer } from '../../../api/churchContent';
import type { CalendarSlot, CalendarViewMode } from './churchTypes';
import { calendarViewModes } from './churchTypes';
import { contentCounts, isCalendarDayComplete } from './churchHelpers';

export type DayFillStatus = 'done' | 'partial' | 'empty';

const fillStatusLabels: Record<DayFillStatus, string> = {
  done: 'Готово',
  partial: 'Частично',
  empty: 'Не заполнено'
};

export function dayFillStatus(slot: CalendarSlot, icons: ChurchIcon[], prayers: ChurchPrayer[], articles: ChurchArticle[]): DayFillStatus {
  if (!slot.day) return 'empty';
  const counts = contentCounts(slot.day.id, icons, prayers, articles);
  if (isCalendarDayComplete(slot.day) && counts.icons > 0 && counts.prayers > 0 && counts.articles > 0) return 'done';
  return 'partial';
}

export function ChurchCalendarPanel({ mode, periodLabel, slots, icons, prayers, articles, onModeChange, onShift, onToday, onSelectSlot }: {
  mode: CalendarViewMode;
  periodLabel: string;
  slots: CalendarSlot[];
  icons: ChurchIcon[];
  prayers: ChurchPrayer[];
  articles: ChurchArticle[];
  onModeChange: (mode: CalendarViewMode) => void;
  onShift: (direction: -1 | 1) => void;
  onToday: () => void;
  onSelectSlot: (slot: CalendarSlot) => void;
}) {
  return (
    <div className="church-calendar-navigator">
      <div className="church-calendar-toolbar">
        <div className="church-calendar-segments" role="tablist" aria-label="Режим календаря">
          {calendarViewModes.map((item) => (
            <button key={item.value} type="button" className={mode === item.value ? 'active' : ''} onClick={() => onModeChange(item.value)}>
              {item.label}
            </button>
          ))}
        </div>
        <div className="church-calendar-period">
          <button type="button" aria-label="Предыдущий период" onClick={() => onShift(-1)}><AppIcon name="chevron-left" /></button>
          <strong>{periodLabel}</strong>
          <button type="button" aria-label="Следующий период" onClick={() => onShift(1)}><span aria-hidden="true">›</span></button>
          <button type="button" onClick={onToday}>Сегодня</button>
        </div>
      </div>

      <div className="church-calendar-legend">
        <span className="done"><i />Готово</span>
        <span className="partial"><i />Частично</span>
        <span className="empty"><i />Не заполнено</span>
      </div>

      {mode === 'list' ? (
        <div className="church-calendar-list">
          {slots.map((slot) => (
            <ChurchCalendarListRow key={slot.key} slot={slot} status={dayFillStatus(slot, icons, prayers, articles)} onSelect={() => onSelectSlot(slot)} />
          ))}
        </div>
      ) : (
        <div className={'church-calendar-month-cells mode-' + mode}>
          {slots.map((slot) => (
            <ChurchCalendarCell key={slot.key} slot={slot} status={dayFillStatus(slot, icons, prayers, articles)} onSelect={() => onSelectSlot(slot)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ChurchCalendarCell({ slot, status, onSelect }: { slot: CalendarSlot; status: DayFillStatus; onSelect: () => void }) {
  return (
    <button
      type="button"
      className={`church-calendar-cell ${status}${slot.isSelected ? ' selected' : ''}${slot.isToday ? ' today' : ''}`}
      onClick={onSelect}
    >
      <span className="church-calendar-cell__day">{slot.label}</span>
      {slot.day ? <strong className="church-calendar-cell__title">{slot.day.title}</strong> : null}
      <em className="church-calendar-cell__status">{fillStatusLabels[status]}</em>
    </button>
  );
}

function ChurchCalendarListRow({ slot, status, onSelect }: { slot: CalendarSlot; status: DayFillStatus; onSelect: () => void }) {
  return (
    <button type="button" className={`church-calendar-list-row ${status}${slot.isSelected ? ' selected' : ''}${slot.isToday ? ' today' : ''}`} onClick={onSelect}>
      <span className="church-calendar-list-row__date">{slot.date}<small>{slot.weekday}</small></span>
      <span className="church-calendar-list-row__title">{slot.day ? slot.day.title : 'День не заполнен'}</span>
      <span className={'admin-status-chip ' + statusChipTone(slot.day?.status, status)}><i />{fillStatusLabels[status]}</span>
    </button>
  );
}

function statusChipTone(dayStatus: ChurchContentStatus | undefined, status: DayFillStatus) {
  if (status === 'empty') return 'draft';
  if (dayStatus === 'published') return 'published';
  if (dayStatus === 'archived') return 'offline';
  return 'draft';
}
