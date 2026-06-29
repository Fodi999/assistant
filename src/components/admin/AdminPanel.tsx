import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';

type AdminPanelProps = {
  title?: string;
  icon?: AppIconName;
  meta?: ReactNode;
  children: ReactNode;
  className?: string;
};

export function AdminPanel({ title, icon = 'folder', meta, children, className = '' }: AdminPanelProps) {
  return (
    <section className={`admin-panel-card admin-panel-v2 ${className}`.trim()}>
      {title || meta ? (
        <div className="admin-panel-title">
          {title ? <span><AppIcon name={icon} />{title}</span> : <span />}
          {meta ? <small>{meta}</small> : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}
