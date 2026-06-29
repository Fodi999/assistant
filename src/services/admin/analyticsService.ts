import type { AnalyticsRow, SiteId } from '../../types/admin';
import { analyticsAdapter } from './adapters/analyticsAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const analyticsService = createAdminResourceService<AnalyticsRow, 'analytics'>('analytics', (row) => ({ ...row, resource: 'analytics' }), analyticsAdapter);

export const list = analyticsService.list;
export const getById = analyticsService.getById;
export const create = analyticsService.create;
export const update = analyticsService.update;
export const remove = analyticsService.remove;

export function listAnalyticsRows(siteId: SiteId): Promise<AnalyticsRow[]> {
  return analyticsService.list(siteId);
}
