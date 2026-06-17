import { useEffect, useMemo, useState } from 'react';
import { getSiteDashboardMetrics } from '../../api/admin';
import { dashboardMetrics, siteConfigs } from '../../lib/mockData';
import type { SiteDashboardMetrics, SiteKey } from '../../types/admin';
import { AppIcon } from '../../components/AppIcon';
import { DataSourceBadge, type DataSource } from '../../components/DataSourceBadge';
import { LeadStatusBadge } from '../../components/LeadStatusBadge';
import { priorityLabels, publishStatusLabels, seoStatusLabels } from '../../lib/labels';

export function DashboardPage({ activeSite }: { activeSite: SiteKey }) {
  const mockMetrics = dashboardMetrics.find((item) => item.site === activeSite) ?? dashboardMetrics[0];
  const site = siteConfigs.find((item) => item.key === activeSite) ?? siteConfigs[0];
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const [apiMetrics, setApiMetrics] = useState<SiteDashboardMetrics | null>(null);
  const metrics = useMemo(() => {
    if (!apiMetrics) return mockMetrics;
    return { ...mockMetrics, ...apiMetrics };
  }, [apiMetrics, mockMetrics]);

  useEffect(() => {
    void getSiteDashboardMetrics(activeSite)
      .then((nextMetrics) => {
        setApiMetrics(nextMetrics);
        setSource('api');
        setSourceError(undefined);
      })
      .catch((error) => {
        setApiMetrics(null);
        setSource('mock');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, [activeSite]);
  const kpis = [
    ['Посетители', metrics.visitors.toLocaleString('ru-RU'), 'трафик', 'analytics'],
    ['Партнерские клики', metrics.affiliateClicks.toLocaleString('ru-RU'), 'переходы', 'external'],
    ['Заявки', String(metrics.leads), 'CRM', 'leads'],
    ['Прогноз дохода', `${metrics.revenueEstimate.toLocaleString('ru-RU')} ${metrics.currency}`, 'партнерка + заявки', 'trend'],
    ['Опубликованные страницы', String(metrics.publishedPages), 'индекс контента', 'cms'],
    ['AI-черновики', String(metrics.aiDrafts), 'к проверке', 'bot']
  ] as const;

  return (
    <section className="ops-page">
      <div className="ops-header">
        <div className="ops-header-icon"><AppIcon name="dashboard" /></div>
        <div>
          <p className="eyebrow">{site.name} / {site.domain}</p>
          <h2>Панель управления</h2>
          <p>Трафик, партнерские переходы, заявки, доход, публикации и SEO-статус выбранного сайта.</p>
        </div>
        <div className="ops-header-actions"><DataSourceBadge source={source} label="Сводка" /><span className={`status-pill ${metrics.seoStatus === 'good' ? 'good' : 'warning'}`}><i />SEO: {seoStatusLabels[metrics.seoStatus]}</span></div>
      </div>
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул сводку: {sourceError}. Показаны mock-данные.</p> : null}

      <div className="kpi-grid six">
        {kpis.map(([title, value, hint, icon]) => (
          <article key={title} className="kpi-card">
            <div className="kpi-card-head"><span><AppIcon name={icon} /></span><small>{hint}</small></div>
            <p>{title}</p>
            <strong>{value}</strong>
            <div className="sparkline"><i /><i /><i /><i /><i /><i /></div>
          </article>
        ))}
      </div>

      <div className="ops-grid two-two">
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="cms" />Лучшие страницы</span></div>
          <table className="ops-table"><tbody>{metrics.topPages.map((page) => <tr key={page.path}><td><strong>{page.title}</strong><small>{page.path}</small></td><td>{page.visitors.toLocaleString('ru-RU')}</td><td>{page.ctr}% CTR</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="shop" />Лучшие партнерские товары</span></div>
          <table className="ops-table"><tbody>{metrics.topProducts.map((product) => <tr key={product.productId}><td><strong>{product.title}</strong></td><td>{product.clicks} кликов</td><td>{product.revenue.toLocaleString('ru-RU')} {metrics.currency}</td></tr>)}</tbody></table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="leads" />Последние заявки</span></div>
          <div className="mini-list static">{metrics.recentLeads.map((lead) => <div key={lead.id}><span><strong>{lead.clientName}</strong><small>{lead.category} / {lead.city}</small></span><LeadStatusBadge status={lead.status} /></div>)}</div>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="seo" />SEO задачи</span></div>
          <div className="mini-list static">{metrics.seoTasks.map((task) => <div key={task.title}><span><strong>{task.title}</strong><small>{priorityLabels[task.priority]}</small></span><span className="status-pill warning"><i />{publishStatusLabels[task.status]}</span></div>)}</div>
        </section>
      </div>
    </section>
  );
}
