import type { AdminResourceRow, SiteId } from '../../types/admin';
import { shopAdapter } from './adapters/shopAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const shopService = createAdminResourceService<AdminResourceRow, 'shop'>('shop', undefined, shopAdapter);

export const list = shopService.list;
export const getById = shopService.getById;
export const create = shopService.create;
export const update = shopService.update;
export const remove = shopService.remove;

export function listShopProducts(siteId: SiteId): Promise<AdminResourceRow[]> {
  return shopService.list(siteId);
}
