import type { ReactNode } from 'react';
import { SearchInput } from './SearchInput';

type AdminToolbarProps = {
  searchValue?: string;
  searchPlaceholder?: string;
  onSearchChange?: (value: string) => void;
  filters?: ReactNode;
  actions?: ReactNode;
};

export function AdminToolbar({ searchValue, searchPlaceholder, onSearchChange, filters, actions }: AdminToolbarProps) {
  return (
    <div className="admin-toolbar">
      {onSearchChange ? (
        <SearchInput value={searchValue ?? ''} placeholder={searchPlaceholder ?? 'Search'} onChange={onSearchChange} />
      ) : null}
      {filters ? <div className="admin-toolbar-filters">{filters}</div> : null}
      {actions ? <div className="admin-toolbar-actions">{actions}</div> : null}
    </div>
  );
}
