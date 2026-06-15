import { useEffect, useMemo, useRef, useState } from 'react';
import { LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import * as echarts from 'echarts/core';
import type { EChartsOption } from 'echarts';
import { CanvasRenderer } from 'echarts/renderers';
import { BadgeDollarSign, CreditCard, Eye, FileText, Mail, Phone, Send, ShoppingCart } from 'lucide-react';
import { MetricCard } from '../components/MetricCard';
import type { AnalyticsOverview, AnalyticsRealtime, SearchConsoleBundle, SearchConsoleRow } from '../api/analytics';
import type { AdminStats, AdminUser } from '../types/admin';

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer]);

interface DashboardPageProps {
  stats: AdminStats | null;
  users: AdminUser[];
  catalogCount: number | null;
  analytics: AnalyticsOverview | null;
  realtime: AnalyticsRealtime | null;
  searchConsole: SearchConsoleBundle | null;
  analyticsPeriod: AnalyticsPeriod;
  onChangeAnalyticsPeriod: (days: AnalyticsPeriod) => void;
  realtimeLoading: boolean;
  onRefreshRealtime: () => void;
  loading: boolean;
}

const trackedEvents = [
  ['lead_form_submit', 'Заявка', 'Отправка формы', FileText, 'green'],
  ['telegram_click', 'Telegram', 'Клик по Telegram', Send, 'blue'],
  ['whatsapp_click', 'WhatsApp', 'Клик по WhatsApp', Phone, 'green'],
  ['phone_click', 'Телефон', 'Клик по номеру', Phone, 'purple'],
  ['email_click', 'Email', 'Клик по email', Mail, 'blue'],
  ['view_item', 'Просмотр', 'Просмотр товара', Eye, 'purple'],
  ['add_to_cart', 'Корзина', 'Добавили в корзину', ShoppingCart, 'orange'],
  ['begin_checkout', 'Оплата', 'Начали оплату', CreditCard, 'blue'],
  ['purchase', 'Покупка', 'Успешная оплата', BadgeDollarSign, 'orange'],
] as const;

type ChartMetric = 'active_users' | 'sessions' | 'page_views' | 'conversions' | 'total_revenue';
type AnalyticsPeriod = 7 | 28 | 90;
type SeoOpportunity = {
  title: string;
  detail: string;
  action: string;
  tone: 'blue' | 'orange' | 'green' | 'red';
  score: number;
};

const chartLabels: Record<ChartMetric, string> = {
  active_users: 'Users',
  sessions: 'Sessions',
  page_views: 'Views',
  conversions: 'Leads',
  total_revenue: 'Sales',
};

const periodLabels: Record<AnalyticsPeriod, string> = {
  7: '7 дней',
  28: '28 дней',
  90: '3 месяца',
};

const chartIcons: Record<ChartMetric, string> = {
  active_users: '♙',
  sessions: '◷',
  page_views: '◉',
  conversions: '◎',
  total_revenue: '▣',
};

function numberValue(value?: number | null): string {
  return typeof value === 'number' ? new Intl.NumberFormat('ru-RU').format(Math.round(value)) : '0';
}

function percentValue(value?: number | null): string {
  if (typeof value !== 'number') return '0%';
  const percent = value * 100;
  const digits = percent > 0 && percent < 1 ? 2 : 0;
  return `${new Intl.NumberFormat('ru-RU', { maximumFractionDigits: digits }).format(percent)}%`;
}

function moneyValue(value?: number | null): string {
  return typeof value === 'number' ? `${new Intl.NumberFormat('ru-RU').format(Math.round(value))} zł` : '0 zł';
}

function decimalValue(value?: number | null, digits = 1): string {
  return typeof value === 'number' ? new Intl.NumberFormat('ru-RU', { maximumFractionDigits: digits }).format(value) : '0';
}

function dateLabel(value: string): string {
  return value.length === 8 ? `${value.slice(6, 8)}.${value.slice(4, 6)}` : value;
}

function toDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}${month}${day}`;
}

function fillDailyRange(rows: AnalyticsOverview['daily'], days: AnalyticsPeriod): AnalyticsOverview['daily'] {
  const byDate = new Map(rows.map((row) => [row.date, row]));
  const today = new Date();
  return Array.from({ length: days }, (_, index) => {
    const date = new Date(today);
    date.setDate(today.getDate() - (days - 1 - index));
    const key = toDateKey(date);
    return byDate.get(key) || {
      date: key,
      active_users: 0,
      sessions: 0,
      page_views: 0,
      conversions: 0,
      total_revenue: 0,
    };
  });
}

function pageAdvice(path: string, conversions: number, engagementRate: number): string {
  if (path.includes('-avocado-k-c-b6-')) return '301 redirect на /avocado';
  if (conversions === 0 && path === '/pl') return 'добавить CTA';
  if (engagementRate === 0) return 'добавить события';
  return 'наблюдать';
}

function pathFromUrl(value: string): string {
  try {
    return new URL(value).pathname || '/';
  } catch {
    return value.startsWith('/') ? value : `/${value}`;
  }
}

function normalizeText(value: string): string {
  return value.toLowerCase().replace(/[^a-zа-яёąćęłńóśźż0-9]+/gi, ' ').trim();
}

function queryHasLandingPage(query: string, pages: SearchConsoleRow[]): boolean {
  const queryTerms = normalizeText(query).split(' ').filter((term) => term.length > 3);
  if (!queryTerms.length) return false;
  return pages.some((page) => {
    const path = normalizeText(pathFromUrl(page.key));
    return queryTerms.some((term) => path.includes(term));
  });
}

function buildSeoOpportunities(searchConsole: SearchConsoleBundle | null, analytics: AnalyticsOverview | null): SeoOpportunity[] {
  const opportunities: SeoOpportunity[] = [];
  const queries = searchConsole?.queries || [];
  const pages = searchConsole?.pages || [];

  queries.forEach((query) => {
    if (query.impressions > 50 && query.clicks === 0) {
      opportunities.push({
        title: 'Улучшить title/meta',
        detail: `"${query.key}" получает ${numberValue(query.impressions)} показов без кликов.`,
        action: 'Сделать title кликабельнее и добавить короткий ответ сверху страницы.',
        tone: 'orange',
        score: query.impressions + 80,
      });
    }

    if (query.position >= 8 && query.position <= 20) {
      opportunities.push({
        title: 'Усилить страницу',
        detail: `"${query.key}" уже близко: позиция ${decimalValue(query.position, 1)}.`,
        action: 'Добавить FAQ, внутренние ссылки, таблицу/калькулятор и точный H2 под запрос.',
        tone: 'blue',
        score: 60 + query.impressions,
      });
    }

    if (query.impressions >= 10 && !queryHasLandingPage(query.key, pages)) {
      opportunities.push({
        title: 'Создать landing page',
        detail: `"${query.key}" не имеет очевидной точной страницы.`,
        action: 'Сделать отдельную SEO-страницу под этот intent.',
        tone: 'green',
        score: 50 + query.impressions,
      });
    }
  });

  pages.forEach((page) => {
    const path = pathFromUrl(page.key);
    const ga4 = analytics?.top_pages?.find((item) => item.path === path);
    if ((ga4?.views || 0) > 0 && (ga4?.conversions || 0) === 0) {
      opportunities.push({
        title: 'Добавить CTA',
        detail: `${path} получает просмотры, но leads = 0.`,
        action: 'Добавить кнопку заявки, Telegram/WhatsApp или lead magnet рядом с первым экраном.',
        tone: 'red',
        score: 90 + (ga4?.views || 0),
      });
    }
  });

  return opportunities.sort((a, b) => b.score - a.score).slice(0, 8);
}

function DimensionList({ title, rows, metric = 'active_users' }: {
  title: string;
  rows?: { name: string; active_users: number; sessions: number; page_views: number; conversions: number }[];
  metric?: 'active_users' | 'sessions' | 'page_views' | 'conversions';
}) {
  return (
    <article className="geo-card">
      <h3>{title}</h3>
      <div className="geo-list">
        {rows?.length ? rows.slice(0, 6).map((row) => (
          <p key={`${title}-${row.name}`}>
            <span>{row.name}</span>
            <strong>{numberValue(row[metric])}</strong>
          </p>
        )) : <p className="page-muted">GA4 пока не вернул данные.</p>}
      </div>
    </article>
  );
}

function SearchConsoleList({ title, rows, type }: { title: string; rows?: SearchConsoleRow[]; type: 'query' | 'page' }) {
  return (
    <article className="search-console-list-card">
      <h3>{title}</h3>
      <div className="search-console-list">
        {rows?.length ? rows.slice(0, 7).map((row) => (
          <p key={`${type}-${row.key}`}>
            <span>{row.key}</span>
            <strong>{numberValue(row.clicks)} кликов</strong>
            <em>{numberValue(row.impressions)} показов</em>
          </p>
        )) : <p className="page-muted">Search Console пока не вернул данные.</p>}
      </div>
    </article>
  );
}

function AnalyticsLineChart({ rows, metric, period }: { rows: AnalyticsOverview['daily']; metric: ChartMetric; period: AnalyticsPeriod }) {
  const chartRef = useRef<HTMLDivElement | null>(null);
  const filledRows = useMemo(() => fillDailyRange(rows, period), [rows, period]);
  const labels = useMemo(() => filledRows.map((row) => dateLabel(row.date)), [filledRows]);
  const values = useMemo(() => filledRows.map((row) => Number(row[metric]) || 0), [filledRows, metric]);

  useEffect(() => {
    if (!chartRef.current) return;
    const chart = echarts.init(chartRef.current, undefined, { renderer: 'canvas' });
    const option: EChartsOption = {
      animationDuration: 420,
      color: ['#665dff'],
      grid: { top: 18, right: 22, bottom: 34, left: 42, containLabel: true },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(255,255,255,.97)',
        borderColor: 'rgba(226,232,240,.95)',
        borderWidth: 1,
        padding: [10, 12],
        textStyle: { color: '#111b2b', fontFamily: 'Inter, sans-serif', fontSize: 12, fontWeight: 700 },
        extraCssText: 'box-shadow:0 18px 32px rgba(15,23,42,.14);border-radius:14px;',
        axisPointer: {
          type: 'line',
          lineStyle: { color: 'rgba(102,93,255,.24)', width: 1, type: 'dashed' },
        },
        formatter: (params: unknown) => {
          const item = (Array.isArray(params) ? params[0] : params) as { axisValueLabel?: string; axisValue?: string; marker?: string; value?: number };
          const value = metric === 'total_revenue' ? moneyValue(Number(item.value || 0)) : numberValue(Number(item.value || 0));
          return `<div class="echarts-dashboard-tooltip"><strong>${item.axisValueLabel || item.axisValue || ''}</strong><span>${item.marker || ''}${chartLabels[metric]} <b>${value}</b></span></div>`;
        },
      },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: labels,
        axisTick: { show: false },
        axisLine: { lineStyle: { color: '#94a3b8' } },
        axisLabel: {
          color: '#6f7c91',
          fontFamily: 'Inter, sans-serif',
          fontSize: 11,
          hideOverlap: true,
          interval: period === 7 ? 0 : period === 28 ? 3 : 13,
        },
      },
      yAxis: {
        type: 'value',
        min: 0,
        splitNumber: 4,
        axisLabel: {
          color: '#6f7c91',
          fontFamily: 'Inter, sans-serif',
          fontSize: 11,
          formatter: (value: number) => metric === 'total_revenue' ? `${Math.round(value)} zł` : numberValue(value),
        },
        splitLine: { lineStyle: { color: 'rgba(148,163,184,.28)', type: 'dashed' } },
      },
      series: [{
        name: chartLabels[metric],
        type: 'line',
        data: values,
        smooth: true,
        showSymbol: true,
        symbol: 'circle',
        symbolSize: 7,
        lineStyle: { width: 3, color: '#665dff' },
        itemStyle: { color: '#fff', borderColor: '#665dff', borderWidth: 3 },
        emphasis: { focus: 'series', itemStyle: { color: '#665dff' } },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(102,93,255,.24)' },
            { offset: 1, color: 'rgba(102,93,255,.02)' },
          ]),
        },
      }],
    };
    chart.setOption(option);

    const observer = new ResizeObserver(() => chart.resize());
    observer.observe(chartRef.current);
    return () => {
      observer.disconnect();
      chart.dispose();
    };
  }, [labels, metric, period, values]);

  return (
    <div className="analytics-line-chart">
      <div ref={chartRef} className="analytics-echart" role="img" aria-label={`GA4 ${chartLabels[metric]} line chart`} />
    </div>
  );
}

export function DashboardPage({ stats, users, catalogCount, analytics, realtime, searchConsole, analyticsPeriod, onChangeAnalyticsPeriod, realtimeLoading, onRefreshRealtime, loading }: DashboardPageProps) {
  const [chartMetric, setChartMetric] = useState<ChartMetric>('active_users');
  const daily = analytics?.daily || [];
  const events = analytics?.events || [];
  const requiredEventNames = new Set(trackedEvents.map(([event]) => event));
  const trackedEventRows = events.filter((event) => requiredEventNames.has(event.event_name as typeof trackedEvents[number][0]));
  const searchOverview = searchConsole?.overview;
  const seoOpportunities = useMemo(() => buildSeoOpportunities(searchConsole, analytics), [searchConsole, analytics]);

  return (
    <section className="dashboard-page">
      <div className="metrics-grid">
        <MetricCard title="Admin users" value={loading ? '...' : String(stats?.total_users ?? 0)} note="Пользователи админки" icon="users" />
        <MetricCard title="Active users" value={loading ? '...' : numberValue(analytics?.active_users)} note={`GA4 · ${periodLabels[analyticsPeriod]}`} icon="trend" tone="green" />
        <MetricCard title="Sales" value={loading ? '...' : numberValue(analytics?.conversions)} note="GA4 conversions" icon="building" />
        <MetricCard title="Revenue" value={loading ? '...' : moneyValue(analytics?.total_revenue)} note="GA4 total revenue" icon="box" tone="orange" />
      </div>

      <div className="analytics-strip">
        <div>
          <span>GA4 Property</span>
          <strong>{analytics?.property_id || 'не подключен'}</strong>
        </div>
        <div>
          <span>Engagement rate</span>
          <strong>{percentValue(analytics?.engagement_rate)}</strong>
        </div>
        <div>
          <span>Avg. session</span>
          <strong>{numberValue(analytics?.average_session_duration)} сек</strong>
        </div>
        <div>
          <span>Catalog</span>
          <strong>{catalogCount ?? 0} продуктов</strong>
        </div>
      </div>

      <div className="dashboard-top-grid">
        <article className="page-card realtime-card">
          <div className="section-head">
            <div>
              <h2>Realtime сейчас</h2>
              <p className="page-muted">Живые данные GA4, автообновление каждые 30 секунд.</p>
            </div>
            <button className="realtime-refresh-button" type="button" onClick={onRefreshRealtime} disabled={realtimeLoading}>
              {realtimeLoading ? 'Обновляем...' : 'Обновить данные'}
            </button>
          </div>
          <div className="realtime-grid">
            <div className="realtime-now">
              <span>Active users right now</span>
              <strong>{realtimeLoading && !realtime ? '...' : numberValue(realtime?.active_users)}</strong>
              <small>{realtime?.property_id ? `GA4 ${realtime.property_id}` : 'Realtime API'}</small>
            </div>
            <div className="realtime-list">
              <h3>Текущие страницы</h3>
              {realtime?.pages?.length ? realtime.pages.slice(0, 4).map((page) => (
                <p key={page.path}>
                  <span>{page.path || '(not set)'}</span>
                  <strong>{numberValue(page.active_users)}</strong>
                </p>
              )) : <p className="page-muted">Сейчас GA4 не видит активных страниц.</p>}
            </div>
            <div className="realtime-list">
              <h3>Последние события</h3>
              {realtime?.events?.length ? realtime.events.slice(0, 4).map((event) => (
                <p key={event.event_name}>
                  <span>{event.event_name || '(not set)'}</span>
                  <strong>{numberValue(event.event_count)}</strong>
                </p>
              )) : <p className="page-muted">Свежих событий пока нет.</p>}
            </div>
          </div>
        </article>

        <article className="page-card geo-analytics-card">
          <div className="section-head">
            <div>
              <h2>География и аудитория</h2>
              <p className="page-muted">Разрешённые GA4 агрегаты: без IP и персональных данных.</p>
            </div>
            <b>{periodLabels[analyticsPeriod]}</b>
          </div>
          <div className="geo-grid">
            <DimensionList title="Страны" rows={analytics?.countries} />
            <DimensionList title="Города" rows={analytics?.cities} />
            <DimensionList title="Регионы" rows={analytics?.regions} />
            <DimensionList title="Языки" rows={analytics?.languages} />
            <DimensionList title="Устройства" rows={analytics?.devices} metric="sessions" />
            <DimensionList title="Источники" rows={analytics?.traffic_sources} metric="sessions" />
          </div>
        </article>
      </div>

      <div className="dashboard-main-grid">
      <article className="page-card activity-chart-card">
        <div className="section-head">
          <div>
            <h2>Google Analytics по дням</h2>
            <p className="page-muted">Линейный график по дням, как в Google Analytics.</p>
          </div>
          <div className="analytics-chart-controls">
            <div className="period-switcher">{([7, 28, 90] as AnalyticsPeriod[]).map((period) => <button key={period} className={analyticsPeriod === period ? 'active' : ''} type="button" onClick={() => onChangeAnalyticsPeriod(period)}>{periodLabels[period]}</button>)}</div>
            <div className="chart-switcher">{(Object.keys(chartLabels) as ChartMetric[]).map((metric) => <button key={metric} className={chartMetric === metric ? 'active' : ''} type="button" onClick={() => setChartMetric(metric)}>{chartLabels[metric]}</button>)}</div>
          </div>
        </div>
        <div className="analytics-chart-summary">
          <button type="button" onClick={() => setChartMetric('active_users')} className={chartMetric === 'active_users' ? 'active' : ''}><i>{chartIcons.active_users}</i><span>Users</span><strong>{numberValue(analytics?.active_users)}</strong></button>
          <button type="button" onClick={() => setChartMetric('sessions')} className={chartMetric === 'sessions' ? 'active' : ''}><i>{chartIcons.sessions}</i><span>Sessions</span><strong>{numberValue(analytics?.sessions)}</strong></button>
          <button type="button" onClick={() => setChartMetric('page_views')} className={chartMetric === 'page_views' ? 'active' : ''}><i>{chartIcons.page_views}</i><span>Views</span><strong>{numberValue(analytics?.page_views)}</strong></button>
          <button type="button" onClick={() => setChartMetric('conversions')} className={chartMetric === 'conversions' ? 'active' : ''}><i>{chartIcons.conversions}</i><span>Leads</span><strong>{numberValue(analytics?.conversions)}</strong></button>
          <button type="button" onClick={() => setChartMetric('total_revenue')} className={chartMetric === 'total_revenue' ? 'active' : ''}><i>{chartIcons.total_revenue}</i><span>Sales</span><strong>{moneyValue(analytics?.total_revenue)}</strong></button>
        </div>
        <AnalyticsLineChart rows={daily} metric={chartMetric} period={analyticsPeriod} />
      </article>
      <article className="page-card recent-actions"><div className="section-head"><h2>Top pages</h2><b>GA4</b></div>
        <div className="top-pages-list">
          {analytics?.top_pages?.length ? analytics.top_pages.slice(0, 6).map((page, index) => <div className="top-page-row" key={`${page.path}-${index}`}>
            <span>{index + 1}</span>
            <p><strong>{page.path}</strong><small>{page.title || 'Без title'}</small></p>
            <em>{numberValue(page.views)} views</em>
            <em>{numberValue(page.conversions)} leads</em>
            <b>{pageAdvice(page.path, page.conversions, page.engagement_rate)}</b>
          </div>) : <p className="page-muted">GA4 пока не вернул страницы.</p>}
        </div>
      </article>
      </div>

      <div className="dashboard-bottom-grid">
        <article className="page-card events-card">
          <div className="section-head">
            <div>
              <h2>События и конверсии</h2>
              <p className="page-muted">Ключевые действия сайта.</p>
            </div>
            <b>GA4</b>
          </div>
          <div className="event-grid">
            {trackedEvents.map(([event, label, description, Icon, tone]) => {
              const row = events.find((item) => item.event_name === event);
              const count = row?.count || 0;
              return (
              <div className={`event-pill event-pill--${tone}`} key={event}>
                <i aria-hidden="true"><Icon size={18} strokeWidth={2.1} /></i>
                <p>
                  <strong>{label}</strong>
                  <span>{description}</span>
                  <small>{event}</small>
                </p>
                <b>{numberValue(count)}</b>
                <em>{count > 0 ? 'активно' : 'нет данных'}</em>
              </div>
              );
            })}
          </div>
        </article>

        <article className="page-card search-console-card">
          <div className="section-head">
            <div>
              <h2>Search Console</h2>
              <p className="page-muted">Query → page → GA4.</p>
            </div>
            <b>{searchOverview?.site_url || searchConsole?.sites?.[0]?.site_url || 'GSC'}</b>
          </div>
          <div className="search-console-metrics">
            <div><span>Clicks</span><strong>{numberValue(searchOverview?.clicks)}</strong></div>
            <div><span>Impressions</span><strong>{numberValue(searchOverview?.impressions)}</strong></div>
            <div><span>CTR</span><strong>{percentValue(searchOverview?.ctr)}</strong></div>
            <div><span>Avg position</span><strong>{decimalValue(searchOverview?.position, 1)}</strong></div>
          </div>
          <div className="search-console-grid">
            <SearchConsoleList title="Top queries" rows={searchConsole?.queries} type="query" />
            <SearchConsoleList title="Top SEO pages" rows={searchConsole?.pages} type="page" />
            <article className="search-console-list-card search-console-funnel">
              <h3>Query → page → GA4</h3>
              <div className="search-console-list">
                {searchConsole?.pages?.length ? searchConsole.pages.slice(0, 5).map((page) => {
                  const path = new URL(page.key).pathname;
                  const ga4 = analytics?.top_pages?.find((item) => item.path === path);
                  return (
                    <p key={`funnel-${page.key}`}>
                      <span>{path}</span>
                      <strong>{numberValue(page.clicks)} → {numberValue(ga4?.views)}</strong>
                      <em>{numberValue(ga4?.conversions)} leads</em>
                    </p>
                  );
                }) : <p className="page-muted">Нужны данные Search Console pages.</p>}
              </div>
            </article>
          </div>
        </article>

        <article className="page-card seo-opportunities-card">
          <div className="section-head">
            <div>
              <h2>SEO задачи</h2>
              <p className="page-muted">Что улучшить дальше.</p>
            </div>
            <b>{seoOpportunities.length} задач</b>
          </div>
          <div className="seo-opportunity-list">
            {seoOpportunities.length ? seoOpportunities.slice(0, 5).map((item, index) => (
              <div className={`seo-opportunity seo-opportunity--${item.tone}`} key={`${item.title}-${index}`}>
                <span>{index + 1}</span>
                <p>
                  <strong>{item.title}</strong>
                  <small>{item.detail}</small>
                </p>
              </div>
            )) : <p className="page-muted">Пока нет явных SEO задач.</p>}
          </div>
        </article>
      </div>
    </section>
  );
}
