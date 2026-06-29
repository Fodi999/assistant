import type { Order, SiteId } from '../../types/admin';
import { createAdminResourceService } from './resourceServiceFactory';
import { listMockResource } from './mockStore';

export const ordersService = createAdminResourceService<Order, 'orders'>('orders', (row) => ({ ...row, resource: 'orders' }));

export const list = ordersService.list;
export const getById = ordersService.getById;
export const create = ordersService.create;
export const update = ordersService.update;
export const remove = ordersService.remove;

export function listOrders(siteId: SiteId): Promise<Order[]> {
  return listMockResource('orders', siteId).then((rows) => rows.map((row) => ({ ...row, resource: 'orders' })));
}
