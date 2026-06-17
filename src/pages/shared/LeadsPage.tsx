import { useEffect, useMemo, useState } from 'react';
import { listLeadsWithSource } from '../../api/leads';
import { DataSourceBadge, type DataSource } from '../../components/DataSourceBadge';
import { LeadStatusBadge } from '../../components/LeadStatusBadge';
import { leads } from '../../lib/mockData';
import type { LeadStatus, SiteKey } from '../../types/admin';
import { AppIcon } from '../../components/AppIcon';
import { siteNames } from '../../lib/labels';

export function LeadsPage({ activeSite }: { activeSite: SiteKey }) {
  const [site, setSite] = useState<SiteKey | 'all'>(activeSite);
  const [status, setStatus] = useState<LeadStatus | 'all'>('all');
  const [leadRows, setLeadRows] = useState(leads);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const rows = useMemo(() => leadRows.filter((lead) => (site === 'all' || lead.sourceSite === site) && (status === 'all' || lead.status === status)), [leadRows, site, status]);

  useEffect(() => {
    void listLeadsWithSource(site === 'all' ? undefined : site).then((result) => {
      setLeadRows(result.data);
      setSource(result.source);
      setSourceError(result.error);
    });
  }, [site]);

  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="leads" /></div><div><p className="eyebrow">CRM</p><h2>Заявки</h2><p>Заявки, источник, категория, город, сообщение, статус и сумма потенциальной сделки.</p></div><div className="ops-header-actions"><DataSourceBadge source={source} label="Заявки" /></div></div>
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул заявки: {sourceError}. Показаны mock-данные.</p> : null}
      <div className="filter-bar"><select value={site} onChange={(event) => setSite(event.target.value as SiteKey | 'all')}><option value="all">Все сайты</option><option value="culinary">Кулинарный</option><option value="construction">Строительный</option></select><select value={status} onChange={(event) => setStatus(event.target.value as LeadStatus | 'all')}><option value="all">Все статусы</option><option value="new">новая</option><option value="contacted">связались</option><option value="quoted">смета</option><option value="won">выиграно</option><option value="lost">потеряно</option></select></div>
      <section className="ops-panel"><table className="ops-table"><thead><tr><th>Клиент</th><th>Источник</th><th>Город</th><th>Сумма</th><th>Статус</th></tr></thead><tbody>{rows.map((lead) => <tr key={lead.id}><td><strong>{lead.clientName}</strong><small>{lead.contact} / {lead.message}</small></td><td>{siteNames[lead.sourceSite]} / {lead.category}</td><td>{lead.city}</td><td>{lead.potentialValue?.toLocaleString('ru-RU')} {lead.currency}</td><td><LeadStatusBadge status={lead.status} /></td></tr>)}</tbody></table></section>
    </section>
  );
}
