import type { AdminResourceRow, SiteId } from '../../types/admin';
import { leadsAdapter } from './adapters/leadsAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const leadsService = createAdminResourceService<AdminResourceRow, 'leads'>('leads', undefined, leadsAdapter);

export const list = leadsService.list;
export const getById = leadsService.getById;
export const create = leadsService.create;
export const update = leadsService.update;
export const remove = leadsService.remove;

export function listLeads(siteId: SiteId): Promise<AdminResourceRow[]> {
  return leadsService.list(siteId);
}
