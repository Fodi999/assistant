import { useEffect, useState } from 'react';
import { listBundles, listMaterials } from '../../api/construction';
import { listLeadsWithSource } from '../../api/leads';
import { listSuppliersWithSource } from '../../api/suppliers';
import { BundleEditor } from '../../components/BundleEditor';
import { ConstructionCalculatorEditor } from '../../components/ConstructionCalculatorEditor';
import { DataSourceBadge, type DataSource } from '../../components/DataSourceBadge';
import { LeadStatusBadge } from '../../components/LeadStatusBadge';
import { constructionBundles, constructionMaterials, leads, suppliers } from '../../lib/mockData';
import { AppIcon } from '../../components/AppIcon';
import { supplierTypeLabels } from '../../lib/labels';
import type { ConstructionBundle, ConstructionMaterial, Lead, Supplier } from '../../types/admin';

type Tab = 'materials' | 'calculator' | 'bundles' | 'suppliers' | 'leads';

export function ConstructionPage() {
  const [tab, setTab] = useState<Tab>('materials');
  const [materials, setMaterials] = useState<ConstructionMaterial[]>(constructionMaterials);
  const [bundles, setBundles] = useState<ConstructionBundle[]>(constructionBundles);
  const [supplierRows, setSupplierRows] = useState<Supplier[]>(suppliers);
  const [siteLeads, setSiteLeads] = useState<Lead[]>(leads.filter((lead) => lead.sourceSite === 'construction'));
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();

  useEffect(() => {
    void Promise.all([
      listMaterials(),
      listBundles(),
      listSuppliersWithSource(),
      listLeadsWithSource('construction')
    ])
      .then(([nextMaterials, nextBundles, supplierResult, leadResult]) => {
        setMaterials(nextMaterials);
        setBundles(nextBundles);
        setSupplierRows(supplierResult.data);
        setSiteLeads(leadResult.data);
        setSource(supplierResult.source === 'api' && leadResult.source === 'api' ? 'api' : 'mock');
        setSourceError(supplierResult.error || leadResult.error);
      })
      .catch((error) => {
        setSource('mock');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, []);

  return (
    <section className="ops-page">
      <Head source={source} />
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул часть строительных данных: {sourceError}. Показаны доступные данные.</p> : null}
      <div className="tab-row">{(['materials', 'calculator', 'bundles', 'suppliers', 'leads'] as Tab[]).map((item) => <button key={item} className={tab === item ? 'active' : ''} type="button" onClick={() => setTab(item)}>{TAB_LABELS[item]}</button>)}</div>
      {tab === 'materials' ? <section className="ops-panel"><table className="ops-table"><thead><tr><th>Материал</th><th>Город</th><th>Цена материала</th><th>Работа</th><th>Маржа</th></tr></thead><tbody>{materials.map((item) => <tr key={item.id}><td><strong>{item.title.ru}</strong><small>{item.category}</small></td><td>{item.city}</td><td>{item.materialPrice?.toLocaleString('ru-RU')} {item.currency}/{item.unit}</td><td>{item.workPrice?.toLocaleString('ru-RU')} {item.currency}</td><td>{item.marginPercent}%</td></tr>)}</tbody></table></section> : null}
      {tab === 'calculator' ? <ConstructionCalculatorEditor /> : null}
      {tab === 'bundles' ? <BundleEditor bundles={bundles} /> : null}
      {tab === 'suppliers' ? <section className="ops-panel"><table className="ops-table"><tbody>{supplierRows.filter((item) => item.city === 'Алматы').map((supplier) => <tr key={supplier.id}><td><strong>{supplier.name}</strong><small>{supplier.categories.join(', ')}</small></td><td>{supplierTypeLabels[supplier.type]}</td><td>{supplier.contact}</td><td>{supplier.commissionTerms}</td></tr>)}</tbody></table></section> : null}
      {tab === 'leads' ? <section className="ops-panel"><table className="ops-table"><tbody>{siteLeads.map((lead) => <tr key={lead.id}><td><strong>{lead.clientName}</strong><small>{lead.message}</small></td><td>{lead.city}</td><td>{lead.potentialValue?.toLocaleString('ru-RU')} {lead.currency}</td><td><LeadStatusBadge status={lead.status} /></td></tr>)}</tbody></table></section> : null}
    </section>
  );
}

function Head({ source }: { source: DataSource }) {
  return <div className="ops-header"><div className="ops-header-icon"><AppIcon name="building" /></div><div><p className="eyebrow">Строительство / Алматы</p><h2>Стройка</h2><p>Материалы, калькулятор ремонта, комплекты, поставщики, заявки и расчет маржи.</p></div><div className="ops-header-actions"><DataSourceBadge source={source} label="Стройка" /></div></div>;
}

const TAB_LABELS: Record<Tab, string> = {
  materials: 'Материалы',
  calculator: 'Калькулятор',
  bundles: 'Комплекты',
  suppliers: 'Поставщики',
  leads: 'Заявки'
};
