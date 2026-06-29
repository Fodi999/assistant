import { createContext, useContext } from 'react';
import { siteConfigs } from '../../lib/siteConfig';
import type { SiteConfig, SiteKey } from '../../types/admin';

export type ActiveSiteId = 'church' | 'construction' | 'kitchen';

export type ActiveSiteOption = {
  id: ActiveSiteId;
  legacyKey: SiteKey;
  name: string;
  shortName: string;
  domain: string;
  language: string;
  status: SiteConfig['status'];
  apiStatus: SiteConfig['apiStatus'];
  region: string;
  accent: 'gold' | 'blue' | 'violet';
};

export const ACTIVE_SITE_STORAGE_KEY = 'admin_active_site_id';

const legacyByActiveSite: Record<ActiveSiteId, SiteKey> = {
  church: 'icons',
  construction: 'construction',
  kitchen: 'culinary'
};

const activeByLegacySite: Record<SiteKey, ActiveSiteId> = {
  icons: 'church',
  construction: 'construction',
  culinary: 'kitchen'
};

const shortNames: Record<ActiveSiteId, string> = {
  church: 'CH',
  construction: 'CO',
  kitchen: 'KI'
};

const displayNames: Record<ActiveSiteId, string> = {
  church: 'Church',
  construction: 'Construction',
  kitchen: 'Kitchen'
};

const accents: Record<ActiveSiteId, ActiveSiteOption['accent']> = {
  church: 'gold',
  construction: 'blue',
  kitchen: 'violet'
};

export const activeSiteIds: ActiveSiteId[] = ['church', 'construction', 'kitchen'];

export function activeSiteIdToLegacyKey(siteId: ActiveSiteId): SiteKey {
  return legacyByActiveSite[siteId];
}

export function legacyKeyToActiveSiteId(site: SiteKey): ActiveSiteId {
  return activeByLegacySite[site];
}

export function normalizeActiveSiteId(siteId: string | null | undefined): ActiveSiteId {
  return activeSiteIds.includes(siteId as ActiveSiteId) ? (siteId as ActiveSiteId) : 'construction';
}

export function getActiveSiteOptions(): ActiveSiteOption[] {
  return activeSiteIds.map((id) => {
    const legacyKey = activeSiteIdToLegacyKey(id);
    const config = siteConfigs.find((site) => site.key === legacyKey) ?? siteConfigs[0];
    return {
      id,
      legacyKey,
      name: displayNames[id],
      shortName: shortNames[id],
      domain: config.domain,
      language: config.primaryLanguage.toUpperCase(),
      status: config.status,
      apiStatus: config.apiStatus,
      region: config.region,
      accent: accents[id]
    };
  });
}

export type ActiveSiteContextValue = {
  activeSiteId: ActiveSiteId;
  activeSite: ActiveSiteOption;
  activeLegacySite: SiteKey;
  sites: ActiveSiteOption[];
  setActiveSiteId: (siteId: ActiveSiteId) => void;
};

const fallbackSites = getActiveSiteOptions();
const fallbackSite = fallbackSites.find((site) => site.id === 'construction') ?? fallbackSites[0];

const ActiveSiteContext = createContext<ActiveSiteContextValue>({
  activeSiteId: fallbackSite.id,
  activeSite: fallbackSite,
  activeLegacySite: fallbackSite.legacyKey,
  sites: fallbackSites,
  setActiveSiteId: () => undefined
});

export const ActiveSiteProvider = ActiveSiteContext.Provider;

export function useActiveSite() {
  return useContext(ActiveSiteContext);
}
