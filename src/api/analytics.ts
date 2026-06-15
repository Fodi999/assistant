import { apiFetch } from './client';

export interface AnalyticsPageRow {
  path: string;
  title: string;
  views: number;
  active_users: number;
  engagement_rate: number;
  conversions: number;
  total_revenue: number;
}

export interface AnalyticsEventRow {
  event_name: string;
  count: number;
  users: number;
  total_revenue: number;
}

export interface AnalyticsDailyRow {
  date: string;
  active_users: number;
  sessions: number;
  page_views: number;
  conversions: number;
  total_revenue: number;
}

export interface AnalyticsDimensionRow {
  name: string;
  active_users: number;
  sessions: number;
  page_views: number;
  conversions: number;
}

export interface AnalyticsOverview {
  configured: boolean;
  property_id?: string | null;
  date_range: string;
  active_users: number;
  sessions: number;
  page_views: number;
  conversions: number;
  total_revenue: number;
  engagement_rate: number;
  average_session_duration: number;
  events: AnalyticsEventRow[];
  daily: AnalyticsDailyRow[];
  top_pages: AnalyticsPageRow[];
  countries: AnalyticsDimensionRow[];
  cities: AnalyticsDimensionRow[];
  regions: AnalyticsDimensionRow[];
  languages: AnalyticsDimensionRow[];
  devices: AnalyticsDimensionRow[];
  traffic_sources: AnalyticsDimensionRow[];
}

export interface AnalyticsRealtimePage {
  path: string;
  active_users: number;
}

export interface AnalyticsRealtimeEvent {
  event_name: string;
  active_users: number;
  event_count: number;
}

export interface AnalyticsRealtime {
  configured: boolean;
  property_id?: string | null;
  active_users: number;
  pages: AnalyticsRealtimePage[];
  events: AnalyticsRealtimeEvent[];
}

export interface AnalyticsOAuthUrl {
  url: string;
  redirect_uri: string;
  scope: string;
}

export interface SearchConsoleSite {
  site_url: string;
  permission_level: string;
}

export interface SearchConsoleOverview {
  configured: boolean;
  site_url: string;
  date_range: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
}

export interface SearchConsoleRow {
  key: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
}

export interface SearchConsoleDailyRow {
  date: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
}

export interface SearchConsoleBundle {
  sites: SearchConsoleSite[];
  overview: SearchConsoleOverview | null;
  queries: SearchConsoleRow[];
  pages: SearchConsoleRow[];
  daily: SearchConsoleDailyRow[];
}

export function getAnalyticsOverview(days = 30): Promise<AnalyticsOverview> {
  return apiFetch<AnalyticsOverview>(`/api/admin/analytics/overview?days=${days}`);
}

export function getAnalyticsRealtime(): Promise<AnalyticsRealtime> {
  return apiFetch<AnalyticsRealtime>('/api/admin/analytics/realtime');
}

export function getAnalyticsOAuthUrl(): Promise<AnalyticsOAuthUrl> {
  return apiFetch<AnalyticsOAuthUrl>('/api/admin/analytics/oauth/url');
}

export async function getSearchConsoleBundle(days = 30): Promise<SearchConsoleBundle> {
  const sites = await apiFetch<SearchConsoleSite[]>('/api/admin/search-console/sites');
  const siteUrl = sites[0]?.site_url;
  const siteParam = siteUrl ? `&site_url=${encodeURIComponent(siteUrl)}` : '';
  const [overview, queries, pages, daily] = await Promise.all([
    apiFetch<SearchConsoleOverview>(`/api/admin/search-console/overview?days=${days}${siteParam}`).catch(() => null),
    apiFetch<SearchConsoleRow[]>(`/api/admin/search-console/queries?days=${days}&limit=10${siteParam}`).catch(() => []),
    apiFetch<SearchConsoleRow[]>(`/api/admin/search-console/pages?days=${days}&limit=10${siteParam}`).catch(() => []),
    apiFetch<SearchConsoleDailyRow[]>(`/api/admin/search-console/daily?days=${days}&limit=90${siteParam}`).catch(() => []),
  ]);

  return { sites, overview, queries, pages, daily };
}
