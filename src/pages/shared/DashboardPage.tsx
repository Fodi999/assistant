import { useEffect, useMemo, useState } from 'react';
import { getSiteDashboardMetrics } from '../../api/admin';
import { AppIcon, type AppIconName } from '../../components/AppIcon';
import { adminPageLabels, type AppPage } from '../../components/admin/AdminSidebar';
import { DataTable } from '../../components/admin/DataTable';
import { EmptyState } from '../../components/admin/EmptyState';
import { StatCard } from '../../components/admin/StatCard';
import { StatusBadge } from '../../components/admin/StatusBadge';
import { useActiveSite } from '../../components/admin/ActiveSiteContext';
import type { SiteDashboardMetrics, SiteKey } from '../../types/admin';

type DashboardPageProps = {
  activeSite: SiteKey;
  onNavigate?: (page: AppPage) => void;
};

type ActivityRow = {
  id: string;
  title: string;
  area: string;
  time: string;
  status: 'active' | 'draft' | 'warning' | 'new';
};

const mockActivities: ActivityRow[] = [
  { id: 'a1', title: 'Обновлён черновик CMS-страницы', area: 'CMS', time: '10:20', status: 'draft' },
  { id: 'a2', title: 'Новая заявка ожидает обработки', area: 'Leads', time: '09:45', status: 'new' },
  { id: 'a3', title: 'Проверка переводов поставлена в очередь', area: 'Translations', time: 'Вчера', status: 'warning' },
  { id: 'a4', title: 'Медиа-файлы готовы к публикации', area: 'Media', time: 'Вчера', status: 'active' }
];

const quickActions: Array<{ label: string; icon: AppIconName; page: AppPage }> = [
  { label: 'Страница', icon: 'cms', page: 'cms' },
  { label: 'Событие', icon: 'calendar', page: 'calendar' },
  { label: 'Заявка', icon: 'leads', page: 'leads' },
  { label: 'Медиа', icon: 'image', page: 'media' },
  { label: 'Переводы', icon: 'globe', page: 'translations' }
];

export function DashboardPage({ activeSite, onNavigate }: DashboardPageProps) {
  const { activeSite: activeSiteMeta, sites } = useActiveSite();
  const [apiMetrics, setApiMetrics] = useState<SiteDashboardMetrics | null>(null);
  const [apiError, setApiError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void getSiteDashboardMetrics(activeSite)
      .then((metrics) => {
        if (!alive) return;
        setApiMetrics(metrics);
        setApiError(null);
      })
      .catch((error) => {
        if (!alive) return;
        setApiMetrics(null);
        setApiError(error instanceof Error ? error.message : 'API недоступен');
      });
    return () => {
      alive = false;
    };
  }, [activeSite]);

  const kpis = useMemo(() => {
    const cmsPages = apiMetrics?.publishedPages ?? (activeSiteMeta.id === 'church' ? 48 : activeSiteMeta.id === 'construction' ? 32 : 76);
    const leadsToday = apiMetrics?.leads ?? (activeSiteMeta.id === 'construction' ? 6 : activeSiteMeta.id === 'kitchen' ? 3 : 2);
    return [
      { title: 'Сайты', value: sites.length, hint: 'active workspace', icon: 'globe' as const },
      { title: 'Заявки', value: leadsToday, hint: 'today', icon: 'leads' as const },
      { title: 'CMS', value: cmsPages, hint: 'pages', icon: 'cms' as const },
      { title: 'API', value: apiError ? 'Mock' : 'Live', hint: apiError ? 'fallback data' : 'connected', icon: 'cloud' as const, tone: apiError ? 'warning' as const : 'good' as const }
    ];
  }, [activeSiteMeta.id, apiError, apiMetrics, sites.length]);

  return (
    <section className="admin-dashboard-page">
      <div className="admin-page-hero">
        <div>
          <p>Dashboard</p>
          <h2>Overview</h2>
          <span>{activeSiteMeta.id} / {activeSiteMeta.domain} / {activeSiteMeta.language}</span>
        </div>
        <StatusBadge status={activeSiteMeta.status} />
      </div>

      <div className="admin-stat-grid">
        {kpis.map((kpi) => <StatCard key={kpi.title} {...kpi} />)}
      </div>

      <div className="admin-dashboard-grid">
        <section className="admin-panel-card">
          <div className="admin-panel-title">
            <span><AppIcon name="zap" />Быстрые действия</span>
          </div>
          <div className="admin-quick-grid">
            {quickActions.map((action) => (
              <button key={action.label} className="admin-quick-action" type="button" onClick={() => onNavigate?.(action.page)}>
                <AppIcon name={action.icon} />
                <span>{action.label}</span>
                <small>{adminPageLabels[action.page]}</small>
              </button>
            ))}
          </div>
        </section>

        <section className="admin-panel-card">
          <div className="admin-panel-title">
            <span><AppIcon name="activity" />Последние действия</span>
            <small>{apiMetrics ? 'live' : 'mock'}</small>
          </div>
          <DataTable
            rows={mockActivities}
            getRowKey={(row) => row.id}
            empty={<EmptyState title="Действий пока нет" description="TODO: подключить журнал событий backend." />}
            columns={[
              { key: 'title', header: 'Action', render: (row) => <strong>{row.title}</strong> },
              { key: 'area', header: 'Area', render: (row) => row.area },
              { key: 'time', header: 'Time', render: (row) => row.time },
              { key: 'status', header: 'Status', render: (row) => <StatusBadge status={row.status} /> }
            ]}
          />
        </section>
      </div>
    </section>
  );
}
