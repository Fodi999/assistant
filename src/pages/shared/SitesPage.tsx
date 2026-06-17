import { siteConfigs } from '../../lib/mockData';
import { AppIcon } from '../../components/AppIcon';
import { DataSourceBadge } from '../../components/DataSourceBadge';
import { apiStatusLabels, revalidateStatusLabels, siteNames } from '../../lib/labels';

export function SitesPage() {
  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="globe" /></div><div><p className="eyebrow">Сайты</p><h2>Сайты</h2><p>Сайты, домены, языки, статус API и ревалидации.</p></div><div className="ops-header-actions"><DataSourceBadge source="mock" label="Конфиг" /></div></div>
      <div className="site-card-grid">{siteConfigs.map((site) => <article key={site.key} className="site-card"><div><h3>{site.name}</h3><span className={`status-pill ${site.apiStatus === 'online' ? 'good' : 'warning'}`}><i />{apiStatusLabels[site.apiStatus]}</span></div><p>{site.domain}</p><dl><dt>Ключ</dt><dd>{siteNames[site.key]}</dd><dt>Языки</dt><dd>{site.languages.map((item) => item.toUpperCase()).join(' / ')}</dd><dt>Регион</dt><dd>{site.region}</dd><dt>Валюта</dt><dd>{site.defaultCurrency}</dd><dt>Ревалидация</dt><dd>{revalidateStatusLabels[site.revalidateStatus]}</dd></dl></article>)}</div>
    </section>
  );
}
