import { useMemo, type ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';
import { AdminHeader } from './AdminHeader';
import { AdminSidebar, type AppPage } from './AdminSidebar';
import {
  ActiveSiteProvider,
  activeSiteIdToLegacyKey,
  getActiveSiteOptions,
  type ActiveSiteId
} from './ActiveSiteContext';

type AdminLayoutProps = {
  activePage: AppPage;
  activeSiteId: ActiveSiteId;
  collapsed: boolean;
  connectionState: 'online' | 'limited' | 'offline';
  children: ReactNode;
  onNavigate: (page: AppPage) => void;
  onSiteChange: (siteId: ActiveSiteId) => void;
  onToggleSidebar: () => void;
  onLogout: () => void;
};

type AdminPageHeaderProps = {
  eyebrow?: string;
  title: string;
  subtitle?: string;
  icon: AppIconName;
  actions?: ReactNode;
};

type AdminPanelProps = {
  title?: string;
  icon?: AppIconName;
  meta?: ReactNode;
  children: ReactNode;
  className?: string;
};

type AdminModalProps = {
  eyebrow?: string;
  title: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  ariaLabel?: string;
  onClose: () => void;
};

type AdminFieldProps = {
  label: string;
  children: ReactNode;
  help?: ReactNode;
  className?: string;
};

export function AdminLayout({
  activePage,
  activeSiteId,
  collapsed,
  connectionState,
  children,
  onNavigate,
  onSiteChange,
  onToggleSidebar,
  onLogout
}: AdminLayoutProps) {
  const sites = useMemo(() => getActiveSiteOptions(), []);
  const activeSite = sites.find((site) => site.id === activeSiteId) ?? sites[0];
  const contextValue = useMemo(() => ({
    activeSiteId: activeSite.id,
    activeSite,
    activeLegacySite: activeSiteIdToLegacyKey(activeSite.id),
    sites,
    setActiveSiteId: onSiteChange
  }), [activeSite, onSiteChange, sites]);

  return (
    <ActiveSiteProvider value={contextValue}>
      <div className={'admin-crm-shell' + (collapsed ? ' sidebar-collapsed' : '')} data-site-accent={activeSite.accent}>
        <AdminSidebar
          activePage={activePage}
          collapsed={collapsed}
          onNavigate={onNavigate}
          onSiteChange={onSiteChange}
          onToggleCollapse={onToggleSidebar}
        />
        <div className="admin-main-shell">
          <AdminHeader
            activePage={activePage}
            connectionState={connectionState}
            onLogout={onLogout}
            onToggleSidebar={onToggleSidebar}
          />
          <main className="admin-page-content">{children}</main>
        </div>
      </div>
    </ActiveSiteProvider>
  );
}

export function AdminPageHeader({ eyebrow = 'Админ-панель', title, subtitle, icon, actions }: AdminPageHeaderProps) {
  return (
    <div className="ops-header admin-page-header">
      <div className="ops-header-icon"><AppIcon name={icon} /></div>
      <div>
        <p className="eyebrow">{eyebrow}</p>
        <h2>{title}</h2>
        {subtitle ? <p>{subtitle}</p> : null}
      </div>
      {actions ? <div className="ops-header-actions">{actions}</div> : null}
    </div>
  );
}

export function AdminPanel({ title, icon = 'cms', meta, children, className = '' }: AdminPanelProps) {
  return (
    <section className={'ops-panel admin-panel ' + className}>
      {title || meta ? (
        <div className="panel-title">
          {title ? <span><AppIcon name={icon} />{title}</span> : <span />}
          {meta ? <small>{meta}</small> : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}

export function AdminModal({ eyebrow = 'Редактирование', title, actions, children, className = '', ariaLabel, onClose }: AdminModalProps) {
  return (
    <div className="modal-overlay" role="presentation" onMouseDown={onClose}>
      <section className={'editor-modal admin-modal ' + className} role="dialog" aria-modal="true" aria-label={ariaLabel || title} onMouseDown={(event) => event.stopPropagation()}>
        <div className="editor-modal-head admin-modal-head">
          <div>
            <p className="eyebrow">{eyebrow}</p>
            <h2>{title}</h2>
          </div>
          {actions ? <div className="editor-actions">{actions}</div> : null}
        </div>
        {children}
      </section>
    </div>
  );
}

export function AdminField({ label, children, help, className = '' }: AdminFieldProps) {
  return (
    <label className={'editor-field admin-field ' + className}>
      <span>{label}</span>
      {children}
      {help ? <small>{help}</small> : null}
    </label>
  );
}

export function AdminActionBar({ children, align = 'end' }: { children: ReactNode; align?: 'start' | 'end' | 'between' }) {
  return <div className={'editor-actions admin-action-bar ' + align}>{children}</div>;
}
