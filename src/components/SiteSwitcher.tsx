import { siteConfigs } from '../lib/mockData';
import type { SiteKey } from '../types/admin';

interface SiteSwitcherProps {
  activeSite: SiteKey;
  onSiteChange: (site: SiteKey) => void;
}

export function SiteSwitcher({ activeSite, onSiteChange }: SiteSwitcherProps) {
  return (
    <div className="site-switcher-inline" role="group" aria-label="Активный сайт">
      {siteConfigs.map((site) => (
        <button key={site.key} className={site.key === activeSite ? 'active' : ''} type="button" onClick={() => onSiteChange(site.key)}>
          <span>{site.key === 'culinary' ? 'CU' : 'CO'}</span>
          <strong>{site.name}</strong>
        </button>
      ))}
    </div>
  );
}
