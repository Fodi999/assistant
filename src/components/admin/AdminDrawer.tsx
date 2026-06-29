import type { ReactNode } from 'react';
import { AppIcon } from '../AppIcon';

type AdminDrawerProps = {
  open: boolean;
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
};

export function AdminDrawer({ open, title, description, children, footer, onClose }: AdminDrawerProps) {
  if (!open) return null;

  return (
    <div className="admin-drawer-overlay" role="presentation" onMouseDown={onClose}>
      <aside className="admin-drawer" role="dialog" aria-modal="true" aria-label={title} onMouseDown={(event) => event.stopPropagation()}>
        <header className="admin-drawer-header">
          <div>
            <h2>{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
          <button className="admin-icon-button" type="button" aria-label="Close drawer" onClick={onClose}>
            <AppIcon name="chevron-left" />
          </button>
        </header>
        <div className="admin-drawer-body">{children}</div>
        {footer ? <footer className="admin-drawer-footer">{footer}</footer> : null}
      </aside>
    </div>
  );
}
