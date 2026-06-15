import { useEffect, useMemo, useRef, useState } from 'react';
import * as echarts from 'echarts';
import {
  getAnalyticsOAuthUrl,
  getAnalyticsOverview,
  getAnalyticsRealtime,
  getSearchConsoleBundle,
  type AnalyticsOverview,
  type AnalyticsRealtime,
  type SearchConsoleBundle
} from '../api/analytics';
import { listAffiliateProductsWithSource } from '../api/affiliate';
import { dashboardMetrics } from '../lib/mockData';
import type { AffiliateProduct, SiteKey } from '../types/admin';
import { AppIcon } from '../components/AppIcon';
import { DataSourceBadge, type DataSource } from '../components/DataSourceBadge';
import { siteNames } from '../lib/labels';
import { cn } from '../lib/utils';

type ChartMode = 'traffic' | 'revenue' | 'search';
type Period = 7 | 28 | 90;

const MODE_LABELS: Record<ChartMode, string> = {
  traffic: 'GA4 трафик',
  revenue: 'Доход и конверсии',
  search: 'Search Console'
};

const PERIOD_LABELS: Record<Period, string> = {
  7: '7 дней',
  28: '28 дней',
  90: '90 дней'
};

function numberValue(value?: number | null): string {
  return typeof value === 'number' ? Math.round(value).toLocaleString('ru-RU') : '0';
}

function percentValue(value?: number | null): string {
  if (typeof value !== 'number') return '0%';
  return `${(value * 100).toLocaleString('ru-RU', { maximumFractionDigits: value > 0 && value < 0.01 ? 2 : 1 })}%`;
}

function decimalValue(value?: number | null, digits = 1): string {
  return typeof value === 'number' ? value.toLocaleString('ru-RU', { maximumFractionDigits: digits }) : '0';
}

function moneyValue(value?: number | null, currency = 'PLN'): string {
  return `${numberValue(value)} ${currency}`;
}

function durationValue(seconds?: number | null): string {
  if (typeof seconds !== 'number' || !Number.isFinite(seconds)) return '0 сек';
  if (seconds < 60) return `${Math.round(seconds)} сек`;
  const minutes = Math.floor(seconds / 60);
  const rest = Math.round(seconds % 60);
  return `${minutes} мин ${rest} сек`;
}

function dateLabel(value: string): string {
  return value.length === 8 ? `${value.slice(6, 8)}.${value.slice(4, 6)}` : value;
}

function localTitle(product: AffiliateProduct): string {
  return product.title.ru || product.title.pl || product.title.en || Object.values(product.title)[0] || product.slug;
}

function siteMatchesPath(activeSite: SiteKey, path: string): boolean {
  if (activeSite === 'construction') {
    return /almabuild|kazaxbud|stroi|remont|material|construction|build/i.test(path);
  }
  return !/almabuild|kazaxbud|stroi|remont|material|construction|build/i.test(path);
}

function buildMockSeries(seed: number, points = 14) {
  return Array.from({ length: points }, (_, index) => {
    const wave = Math.sin((index + 1) / 2) * 0.18 + 1;
    const trend = 1 + index / (points * 5);
    return Math.max(1, Math.round(seed * wave * trend));
  });
}

function AnalyticsChart({
  activeSite,
  mode,
  analytics,
  searchConsole
}: {
  activeSite: SiteKey;
  mode: ChartMode;
  analytics: AnalyticsOverview | null;
  searchConsole: SearchConsoleBundle | null;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  const mock = dashboardMetrics.find((item) => item.site === activeSite) ?? dashboardMetrics[0];
  const daily = analytics?.daily?.slice(-30) ?? [];
  const searchDaily = searchConsole?.daily?.slice(-30) ?? [];
  const labels = useMemo(() => {
    const rows = mode === 'search' ? searchDaily : daily;
    return rows.length ? rows.map((row) => dateLabel(row.date)) : Array.from({ length: 14 }, (_, index) => `${index + 1}`.padStart(2, '0'));
  }, [daily, mode, searchDaily]);
  const pageViews = useMemo(() => daily.length ? daily.map((row) => row.page_views) : buildMockSeries(Math.max(12, Math.round(mock.affiliateClicks / 18))), [daily, mock.affiliateClicks]);
  const activeUsers = useMemo(() => daily.length ? daily.map((row) => row.active_users) : buildMockSeries(Math.max(8, Math.round(mock.visitors / 240))), [daily, mock.visitors]);
  const conversions = useMemo(() => daily.length ? daily.map((row) => row.conversions) : buildMockSeries(Math.max(2, mock.leads + 2)).map((value) => Math.max(0, Math.round(value / 8))), [daily, mock.leads]);
  const revenue = useMemo(() => daily.length ? daily.map((row) => Math.round(row.total_revenue)) : buildMockSeries(Math.max(80, Math.round(mock.revenueEstimate / 28))), [daily, mock.revenueEstimate]);
  const clicks = useMemo(() => searchDaily.length ? searchDaily.map((row) => row.clicks) : buildMockSeries(Math.max(8, Math.round(mock.affiliateClicks / 25))), [mock.affiliateClicks, searchDaily]);
  const impressions = useMemo(() => searchDaily.length ? searchDaily.map((row) => row.impressions) : buildMockSeries(Math.max(80, Math.round(mock.visitors / 18))), [mock.visitors, searchDaily]);

  useEffect(() => {
    if (!ref.current) return;
    const chart = echarts.init(ref.current, 'dark');
    const axis = {
      axisLine: { lineStyle: { color: '#303036' } },
      axisLabel: { color: '#777973', fontFamily: 'monospace' },
      splitLine: { lineStyle: { color: 'rgba(231,229,220,.08)' } }
    };
    const base = {
      backgroundColor: 'transparent',
      grid: { top: 34, right: 18, bottom: 28, left: 48 },
      legend: { top: 0, right: 0, textStyle: { color: '#aaa9a1', fontFamily: 'monospace' } },
      tooltip: { trigger: 'axis', backgroundColor: '#101013', borderColor: '#08080b', textStyle: { color: '#e7e5dc', fontFamily: 'monospace' } },
      xAxis: { type: 'category', data: labels, ...axis },
      yAxis: { type: 'value', ...axis }
    };

    chart.setOption({
      ...base,
      series: mode === 'traffic'
        ? [
            { name: 'Просмотры', type: 'bar', barWidth: 16, itemStyle: { color: '#607195' }, data: pageViews },
            { name: 'Пользователи', type: 'line', smooth: true, symbol: 'circle', symbolSize: 6, lineStyle: { width: 3, color: '#b9b8af' }, itemStyle: { color: '#b9b8af' }, data: activeUsers }
          ]
        : mode === 'revenue'
          ? [
              { name: 'Доход', type: 'bar', barWidth: 16, itemStyle: { color: '#b9b8af' }, data: revenue },
              { name: 'Конверсии', type: 'line', smooth: true, symbol: 'circle', symbolSize: 6, lineStyle: { width: 3, color: '#607195' }, data: conversions }
            ]
          : [
              { name: 'Клики', type: 'bar', barWidth: 16, itemStyle: { color: '#607195' }, data: clicks },
              { name: 'Показы', type: 'line', smooth: true, symbol: 'none', lineStyle: { width: 3, color: '#b9b8af' }, data: impressions }
            ]
    });

    const resize = () => chart.resize();
    window.addEventListener('resize', resize);
    return () => {
      window.removeEventListener('resize', resize);
      chart.dispose();
    };
  }, [activeSite, activeUsers, clicks, conversions, impressions, labels, mode, pageViews, revenue]);

  return <div className="analytics-chart" ref={ref} />;
}

export function AnalyticsPage({ activeSite }: { activeSite: SiteKey }) {
  const [mode, setMode] = useState<ChartMode>('traffic');
  const [period, setPeriod] = useState<Period>(28);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const [analytics, setAnalytics] = useState<AnalyticsOverview | null>(null);
  const [realtime, setRealtime] = useState<AnalyticsRealtime | null>(null);
  const [searchConsole, setSearchConsole] = useState<SearchConsoleBundle | null>(null);
  const [affiliateProducts, setAffiliateProducts] = useState<AffiliateProduct[]>([]);
  const [oauthUrl, setOauthUrl] = useState<string | null>(null);
  const mock = dashboardMetrics.find((item) => item.site === activeSite) ?? dashboardMetrics[0];
  const filteredPages = useMemo(() => {
    const pages = analytics?.top_pages ?? [];
    const scoped = pages.filter((page) => siteMatchesPath(activeSite, page.path));
    return scoped.length ? scoped : pages;
  }, [activeSite, analytics?.top_pages]);
  const topSearchPages = searchConsole?.pages ?? [];
  const searchQueries = searchConsole?.queries ?? [];
  const rows = [
    ['GA4 пользователи', numberValue(analytics?.active_users ?? mock.visitors)],
    ['GA4 сессии', numberValue(analytics?.sessions)],
    ['GA4 просмотры', numberValue(analytics?.page_views ?? mock.affiliateClicks)],
    ['Engagement', percentValue(analytics?.engagement_rate)],
    ['Средняя сессия', durationValue(analytics?.average_session_duration)],
    ['Конверсии', numberValue(analytics?.conversions ?? mock.leads)],
    ['Доход', moneyValue(analytics?.total_revenue ?? mock.revenueEstimate, mock.currency)],
    ['GSC клики', numberValue(searchConsole?.overview?.clicks)],
    ['GSC показы', numberValue(searchConsole?.overview?.impressions)],
    ['GSC CTR', percentValue(searchConsole?.overview?.ctr)],
    ['GSC позиция', decimalValue(searchConsole?.overview?.position)]
  ];

  useEffect(() => {
    setOauthUrl(null);
    void Promise.allSettled([
      getAnalyticsOverview(period),
      getAnalyticsRealtime(),
      getSearchConsoleBundle(period)
    ]).then(([overviewResult, realtimeResult, searchResult]) => {
      if (overviewResult.status === 'fulfilled') {
        setAnalytics(overviewResult.value);
      } else {
        setAnalytics(null);
      }

      if (realtimeResult.status === 'fulfilled') {
        setRealtime(realtimeResult.value);
      } else {
        setRealtime(null);
      }

      if (searchResult.status === 'fulfilled') {
        setSearchConsole(searchResult.value);
      } else {
        setSearchConsole(null);
      }

      const errors = [overviewResult, realtimeResult, searchResult]
        .filter((result): result is PromiseRejectedResult => result.status === 'rejected')
        .map((result) => result.reason instanceof Error ? result.reason.message : 'API недоступен');

      if (overviewResult.status === 'fulfilled' || realtimeResult.status === 'fulfilled' || searchResult.status === 'fulfilled') {
        setSource('api');
        setSourceError(errors[0]);
      } else {
        setSource('mock');
        setSourceError(errors[0] ?? 'Google Analytics API недоступен');
      }
    });
  }, [period]);

  useEffect(() => {
    void listAffiliateProductsWithSource(activeSite).then((result) => setAffiliateProducts(result.data));
  }, [activeSite]);

  useEffect(() => {
    if (!sourceError || !/GOOGLE|GA4|OAuth|refresh_token|configured/i.test(sourceError)) return;
    void getAnalyticsOAuthUrl()
      .then((result) => setOauthUrl(result.url))
      .catch(() => setOauthUrl(null));
  }, [sourceError]);

  return (
    <section className="ops-page">
      <div className="ops-header">
        <div className="ops-header-icon"><AppIcon name="analytics" /></div>
        <div>
          <p className="eyebrow">Google Analytics / {siteNames[activeSite]}</p>
          <h2>Аналитика</h2>
          <p>GA4, realtime, события, доход, топ-страницы, география и Search Console по выбранному периоду.</p>
        </div>
        <div className="ops-header-actions">
          <DataSourceBadge source={source} label="Google" />
          <div className="analytics-mode-switcher">
            {([7, 28, 90] as Period[]).map((item) => (
              <button key={item} className={cn('analytics-mode-button', period === item && 'active')} type="button" onClick={() => setPeriod(item)}>
                {PERIOD_LABELS[item]}
              </button>
            ))}
          </div>
        </div>
      </div>
      {sourceError ? (
        <p className="ops-alert">
          <AppIcon name="terminal" />
          Google API вернул ошибку: {sourceError}. {oauthUrl ? <a href={oauthUrl} target="_blank" rel="noreferrer">Подключить OAuth</a> : 'Проверь GA4_PROPERTY_ID, GOOGLE_REFRESH_TOKEN и SEARCH_CONSOLE_SITE_URL.'}
        </p>
      ) : null}

      <div className="kpi-grid">
        {rows.map(([label, value]) => (
          <article key={label} className="kpi-card">
            <div className="kpi-card-head"><span><AppIcon name="trend" /></span><small>{PERIOD_LABELS[period]}</small></div>
            <p>{label}</p>
            <strong>{value}</strong>
            <small>{analytics?.property_id || searchConsole?.overview?.site_url || siteNames[activeSite]}</small>
          </article>
        ))}
      </div>

      <section className="ops-panel analytics-panel">
        <div className="panel-title">
          <span><AppIcon name="analytics" />График эффективности</span>
          <div className="analytics-mode-switcher">
            {(Object.keys(MODE_LABELS) as ChartMode[]).map((item) => (
              <button key={item} className={cn('analytics-mode-button', mode === item && 'active')} type="button" onClick={() => setMode(item)}>
                {MODE_LABELS[item]}
              </button>
            ))}
          </div>
        </div>
        <AnalyticsChart activeSite={activeSite} mode={mode} analytics={analytics} searchConsole={searchConsole} />
      </section>

      <div className="ops-grid two-two">
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="globe" />Realtime</span><small>{numberValue(realtime?.active_users)} active users</small></div>
          <table className="ops-table"><tbody>{(realtime?.pages ?? []).slice(0, 6).map((page) => <tr key={page.path}><td><strong>{page.path || '/'}</strong></td><td>{numberValue(page.active_users)}</td></tr>)}</tbody></table>
          {realtime?.events?.length ? (
            <div className="analytics-subtable">
              <div className="panel-title compact"><span><AppIcon name="external" />Realtime events</span><small>active / count</small></div>
              <table className="ops-table"><tbody>{realtime.events.slice(0, 5).map((event) => <tr key={event.event_name}><td><strong>{event.event_name}</strong></td><td>{numberValue(event.active_users)}</td><td>{numberValue(event.event_count)}</td></tr>)}</tbody></table>
            </div>
          ) : null}
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="cms" />GA4 топ-страницы</span><small>views / users / engagement</small></div>
          <table className="ops-table"><tbody>{filteredPages.slice(0, 10).map((page) => <tr key={`${page.path}-${page.title}`}><td><strong>{page.title || page.path}</strong><small>{page.path}</small></td><td>{numberValue(page.views)}</td><td>{numberValue(page.active_users)}</td><td>{percentValue(page.engagement_rate)}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="seo" />Search Console страницы</span><small>clicks / impressions</small></div>
          <table className="ops-table"><tbody>{topSearchPages.slice(0, 10).map((page) => <tr key={page.key}><td><strong>{page.key}</strong><small>pos {decimalValue(page.position)}</small></td><td>{numberValue(page.clicks)}</td><td>{numberValue(page.impressions)}</td><td>{percentValue(page.ctr)}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="seo" />Search Console запросы</span><small>clicks / impressions / pos</small></div>
          <table className="ops-table"><tbody>{searchQueries.slice(0, 10).map((query) => <tr key={query.key}><td><strong>{query.key}</strong><small>CTR {percentValue(query.ctr)}</small></td><td>{numberValue(query.clicks)}</td><td>{numberValue(query.impressions)}</td><td>{decimalValue(query.position)}</td></tr>)}</tbody></table>
        </section>
      </div>

      <div className="ops-grid two-two">
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="external" />События GA4</span><small>eventCount / users / revenue</small></div>
          <table className="ops-table"><tbody>{(analytics?.events ?? []).slice(0, 10).map((event) => <tr key={event.event_name}><td><strong>{event.event_name}</strong></td><td>{numberValue(event.count)}</td><td>{numberValue(event.users)}</td><td>{moneyValue(event.total_revenue, mock.currency)}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="globe" />Источники трафика</span><small>users / sessions / views</small></div>
          <table className="ops-table"><tbody>{(analytics?.traffic_sources ?? []).slice(0, 10).map((row) => <tr key={row.name}><td><strong>{row.name || '(not set)'}</strong></td><td>{numberValue(row.active_users)}</td><td>{numberValue(row.sessions)}</td><td>{numberValue(row.page_views)}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="globe" />Города и регионы</span><small>users / sessions</small></div>
          <table className="ops-table"><tbody>{[...(analytics?.cities ?? []).slice(0, 5), ...(analytics?.regions ?? []).slice(0, 5)].map((row, index) => <tr key={`${row.name}-${index}`}><td><strong>{row.name || '(not set)'}</strong></td><td>{numberValue(row.active_users)}</td><td>{numberValue(row.sessions)}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="globe" />Языки и устройства</span><small>users / sessions / views</small></div>
          <table className="ops-table"><tbody>{[...(analytics?.languages ?? []).slice(0, 5), ...(analytics?.devices ?? []).slice(0, 5)].map((row) => <tr key={row.name}><td><strong>{row.name || '(not set)'}</strong></td><td>{numberValue(row.active_users)}</td><td>{numberValue(row.sessions)}</td><td>{numberValue(row.page_views)}</td></tr>)}</tbody></table>
        </section>
      </div>

      <div className="ops-grid two-one">
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="shop" />Партнерские товары</span><small>backend products</small></div>
          <table className="ops-table"><tbody>{affiliateProducts.slice(0, 10).map((product) => <tr key={product.id}><td><strong>{localTitle(product)}</strong><small>{product.merchant} / {product.network}</small></td><td>{product.status}</td><td>{product.commissionPercent ? `${decimalValue(product.commissionPercent)}%` : '-'}</td><td>{product.price ? moneyValue(product.price, product.currency) : '-'}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="globe" />География GA4</span><small>countries / devices</small></div>
          <table className="ops-table"><tbody>{[...(analytics?.countries ?? []).slice(0, 4), ...(analytics?.devices ?? []).slice(0, 4)].map((row) => <tr key={row.name}><td><strong>{row.name || '(not set)'}</strong></td><td>{numberValue(row.active_users)} users</td><td>{numberValue(row.sessions)} sessions</td></tr>)}</tbody></table>
        </section>
      </div>
    </section>
  );
}
