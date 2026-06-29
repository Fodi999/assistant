import type { AnalyticsRow, SiteId } from '../../../types/admin';
import { baseRow } from './shared';

type BackendAnalyticsOverview = {
  configured?: boolean;
  property_id?: string | null;
  date_range?: string;
  active_users?: number;
  sessions?: number;
  page_views?: number;
  conversions?: number;
  total_revenue?: number;
  engagement_rate?: number;
};

export const analyticsAdapter = {
  normalizeList(response: BackendAnalyticsOverview): BackendAnalyticsOverview[] {
    return [response];
  },

  fromBackend(overview: BackendAnalyticsOverview, siteId: SiteId): AnalyticsRow {
    const status = overview.configured === false ? 'warning' : 'active';
    return {
      ...baseRow({
        id: `analytics-${siteId}`,
        title: 'GA4 overview',
        type: overview.property_id || 'Analytics',
        status,
        owner: overview.date_range || '30 days',
        updated: 'backend',
        metric: `${Math.round(overview.active_users ?? 0)} users / ${Math.round(overview.page_views ?? 0)} views`
      }, siteId),
      resource: 'analytics' as const
    };
  }
};
