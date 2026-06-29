import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';

type EmptyStateProps = {
  title: string;
  description?: string;
  icon?: AppIconName;
  action?: ReactNode;
};

export function EmptyState({ title, description, icon = 'folder', action }: EmptyStateProps) {
  return (
    <div className="admin-empty-state">
      <span><AppIcon name={icon} /></span>
      <strong>{title}</strong>
      {description ? <p>{description}</p> : null}
      {action ? <div>{action}</div> : null}
    </div>
  );
}
