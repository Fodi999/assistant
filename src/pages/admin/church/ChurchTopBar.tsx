import { AppIcon } from '../../../components/AppIcon';
import { ActionButton } from '../../../components/admin/ActionButton';
import { StatusBadge } from '../../../components/admin/StatusBadge';

export function ChurchTopBar({
  searchQuery,
  onSearchChange,
  filledDays,
  totalDays,
  aiCreating,
  onCreateDay,
  onGeminiFill,
  importAvailable,
  onToggleImport
}: {
  searchQuery: string;
  onSearchChange: (value: string) => void;
  filledDays: number;
  totalDays: number;
  aiCreating: boolean;
  onCreateDay: () => void;
  onGeminiFill: () => void;
  importAvailable: boolean;
  onToggleImport: () => void;
}) {
  return (
    <div className="church-top-bar">
      <label className="church-top-bar__search">
        <AppIcon name="search" />
        <input
          value={searchQuery}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder="Поиск по контенту, датам, святым..."
        />
      </label>

      <div className="church-top-bar__status">
        <StatusBadge status="online" label={`${filledDays} / ${totalDays} дней заполнено`} />
      </div>

      <div className="church-top-bar__actions">
        {importAvailable ? <ActionButton icon="save" onClick={onToggleImport}>Импорт</ActionButton> : null}
        <ActionButton icon="bot" onClick={onGeminiFill} disabled={aiCreating}>{aiCreating ? 'Gemini...' : 'Gemini fill'}</ActionButton>
        <ActionButton icon="sparkles" tone="primary" onClick={onCreateDay}>Создать день</ActionButton>
      </div>
    </div>
  );
}
