import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';

type AdminPageHeaderProps = {
  title: string;
  eyebrow?: string;
  description?: string;
  icon?: AppIconName;
  meta?: ReactNode;
  actions?: ReactNode;
};

export function AdminPageHeader({ title, eyebrow, description, icon = 'dashboard', meta, actions }: AdminPageHeaderProps) {
  return (
    <header className="admin-page-header-v2">
      <div className="admin-page-header-icon"><AppIcon name={icon} /></div>
      <div className="admin-page-header-copy">
        {eyebrow ? <p>{eyebrow}</p> : null}
        <h2>{title}</h2>
        {description ? <span>{description}</span> : null}
      </div>
      {meta ? <div className="admin-page-header-meta">{meta}</div> : null}
      {actions ? <div className="admin-page-header-actions">{actions}</div> : null}
    </header>
  );
}
