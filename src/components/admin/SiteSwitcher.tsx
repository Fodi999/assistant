import type { ActiveSiteId } from './ActiveSiteContext';
import { useActiveSite } from './ActiveSiteContext';

type SiteSwitcherProps = {
  collapsed?: boolean;
  onSiteChange: (siteId: ActiveSiteId) => void;
};

export function SiteSwitcher({ collapsed = false, onSiteChange }: SiteSwitcherProps) {
  const { activeSiteId, sites } = useActiveSite();

  return (
    <div className={'admin-site-switcher' + (collapsed ? ' collapsed' : '')} aria-label="Active site">
      <p className="admin-sidebar-label">Site</p>
      {sites.map((site) => {
        const active = site.id === activeSiteId;
        return (
          <button
            key={site.id}
            className={'admin-site-option' + (active ? ' active' : '')}
            data-site-accent={site.accent}
            type="button"
            title={site.name}
            aria-current={active ? 'true' : undefined}
            onClick={() => onSiteChange(site.id)}
          >
            <span className="admin-site-mark">{site.shortName}</span>
            <span className="admin-site-copy">
              <strong>{site.id}</strong>
              <small>{site.language} / {site.apiStatus}</small>
            </span>
            <i className={'admin-site-dot ' + site.apiStatus} />
          </button>
        );
      })}
    </div>
  );
}
