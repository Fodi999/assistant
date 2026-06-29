import type { AdminResourceRow, SiteId, SiteSettings } from '../../types/admin';
import { getMockSiteSettings, listMockResource } from './mockStore';
import { createAdminResourceService } from './resourceServiceFactory';

export const settingsService = createAdminResourceService<AdminResourceRow, 'settings'>('settings');

export const list = settingsService.list;
export const getById = settingsService.getById;
export const create = settingsService.create;
export const update = settingsService.update;
export const remove = settingsService.remove;

export function listSettingsRows(siteId: SiteId): Promise<AdminResourceRow[]> {
  return listMockResource('settings', siteId);
}

export function getSiteSettings(siteId: SiteId): Promise<SiteSettings> {
  return getMockSiteSettings(siteId);
}
