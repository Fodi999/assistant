import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';
import { EmptyState } from './EmptyState';

type AdminStateProps = {
  loading?: boolean;
  error?: string | null;
  empty?: boolean;
  emptyTitle?: string;
  emptyDescription?: string;
  icon?: AppIconName;
  onRetry?: () => void;
  children: ReactNode;
};

export function AdminState({ loading = false, error, empty = false, emptyTitle = 'No data', emptyDescription, icon = 'folder', onRetry, children }: AdminStateProps) {
  if (loading) {
    return (
      <div className="admin-state-box">
        <span><AppIcon name="refresh" /></span>
        <strong>Loading</strong>
      </div>
    );
  }

  if (error) {
    return (
      <div className="admin-state-box error">
        <span><AppIcon name="terminal" /></span>
        <strong>Could not load data</strong>
        <p>{error}</p>
        {onRetry ? <button className="admin-btn secondary" type="button" onClick={onRetry}>Retry</button> : null}
      </div>
    );
  }

  if (empty) {
    return <EmptyState icon={icon} title={emptyTitle} description={emptyDescription} />;
  }

  return <>{children}</>;
}
