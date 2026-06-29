import type { CMSPageItem, SiteId } from '../../types/admin';
import { cmsAdapter } from './adapters/cmsAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const cmsService = createAdminResourceService<CMSPageItem, 'cms'>('cms', (row) => ({ ...row, resource: 'cms' }), cmsAdapter);

export const list = cmsService.list;
export const getById = cmsService.getById;
export const create = cmsService.create;
export const update = cmsService.update;
export const remove = cmsService.remove;

export function listCMSPages(siteId: SiteId): Promise<CMSPageItem[]> {
  return cmsService.list(siteId);
}
