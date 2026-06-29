import { AppIcon } from '../AppIcon';
import { adminPageLabels, type AppPage } from './AdminSidebar';
import { useActiveSite } from './ActiveSiteContext';

type AdminHeaderProps = {
  activePage: AppPage;
  connectionState: 'online' | 'limited' | 'offline';
  onLogout: () => void;
  onToggleSidebar: () => void;
};

const statusLabels: Record<string, string> = {
  active: 'active',
  published: 'published',
  draft: 'draft',
  archived: 'archived',
  online: 'online',
  limited: 'limited',
  offline: 'offline'
};

export function AdminHeader({ activePage, connectionState, onLogout, onToggleSidebar }: AdminHeaderProps) {
  const { activeSite } = useActiveSite();

  return (
    <header className="admin-header">
      <button className="admin-icon-button admin-mobile-menu" type="button" aria-label="Toggle menu" onClick={onToggleSidebar}>
        <AppIcon name="menu" />
      </button>

      <div className="admin-header-context">
        <p>{adminPageLabels[activePage]}</p>
        <h1>{activeSite.name}</h1>
      </div>

      <div className="admin-header-meta" aria-label="Active site metadata">
        <span>{activeSite.id}</span>
        <span>{activeSite.language}</span>
        <span className={'admin-status-chip ' + activeSite.status}><i />{statusLabels[activeSite.status]}</span>
        <span className={'admin-status-chip ' + connectionState}><i />API {statusLabels[connectionState]}</span>
      </div>

      <div className="admin-header-actions">
        <button className="admin-profile-button" type="button" onClick={onLogout}>
          <span>DA</span>
          <strong>Admin<small>Logout</small></strong>
        </button>
      </div>
    </header>
  );
}
