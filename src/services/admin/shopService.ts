import type { AdminResourceRow, ResourceStatus, SiteId } from '../../types/admin';
import type { UpdateShopProductDto } from '../../types/adminApi';
import { isApiMode } from '../../config/adminConfig';
import { adminApiClient } from './adminApiClient';
import { adminApiRoutes, adminShopProductStatusRoute } from './adminApiRoutes';
import { shopAdapter } from './adapters/shopAdapter';
import { createAdminResourceService } from './resourceServiceFactory';
import { updateMockResource } from './mockStore';

export const shopService = createAdminResourceService<AdminResourceRow, 'shop'>('shop', undefined, shopAdapter);

export const list = shopService.list;
export const getById = shopService.getById;
export const create = shopService.create;
export const update = shopService.update;
export const remove = shopService.remove;

export function listShopProducts(siteId: SiteId): Promise<AdminResourceRow[]> {
  return shopService.list(siteId);
}

export async function updateShopProduct(id: string, payload: UpdateShopProductDto): Promise<AdminResourceRow> {
  if (isApiMode) {
    const siteId = payload.siteId || 'kitchen';
    const path = adminApiRoutes.shop.update(id, siteId);
    if (!path) throw new Error('Endpoint не найден');
    const response = await adminApiClient.put<unknown>(
      path,
      shopAdapter.toUpdate(payload)
    );
    return shopAdapter.fromBackend(response as never, siteId);
  }

  return updateMockResource('shop', id, payload);
}

export async function updateShopProductStatus(id: string, siteId: SiteId, status: ResourceStatus): Promise<AdminResourceRow> {
  if (isApiMode) {
    const response = await adminApiClient.put<unknown>(
      adminShopProductStatusRoute(id, siteId),
      { status }
    );
    return shopAdapter.fromBackend(response as never, siteId);
  }

  return updateMockResource('shop', id, { siteId, status });
}
