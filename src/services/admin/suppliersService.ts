import type { AdminResourceRow, SiteId } from '../../types/admin';
import { suppliersAdapter } from './adapters/suppliersAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const suppliersService = createAdminResourceService<AdminResourceRow, 'suppliers'>('suppliers', undefined, suppliersAdapter);

export const list = suppliersService.list;
export const getById = suppliersService.getById;
export const create = suppliersService.create;
export const update = suppliersService.update;
export const remove = suppliersService.remove;

export function listSuppliers(siteId: SiteId): Promise<AdminResourceRow[]> {
  return suppliersService.list(siteId);
}
