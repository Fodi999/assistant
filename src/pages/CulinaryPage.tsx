import { affiliateProducts, contentArticles } from '../lib/mockData';
import { AppIcon } from '../components/AppIcon';
import { DataSourceBadge } from '../components/DataSourceBadge';
import { contentTypeLabels, publishStatusLabels } from '../lib/labels';

export function CulinaryPage() {
  const products = affiliateProducts.filter((item) => item.site === 'culinary');
  const content = contentArticles.filter((item) => item.site === 'culinary');
  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="shop" /></div><div><p className="eyebrow">Кулинарная партнерка</p><h2>Кулинария</h2><p>Кухонные товары, рецепты, обзоры, подборки, профессиональные инструменты и ресторанное оборудование.</p></div><div className="ops-header-actions"><DataSourceBadge source="mock" label="Кулинария" /></div></div>
      <div className="ops-grid two-one">
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="shop" />Кухонные товары</span></div><table className="ops-table"><tbody>{products.map((item) => <tr key={item.id}><td><strong>{item.title.ru}</strong><small>{item.category}</small></td><td>{item.network}</td><td>{item.price} {item.currency}</td><td>{publishStatusLabels[item.status]}</td></tr>)}</tbody></table></section>
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="cms" />Рецепты и обзоры</span></div><div className="mini-list static">{content.map((item) => <div key={item.id}><span><strong>{item.title.ru}</strong><small>{contentTypeLabels[item.type]} / связано: {item.affiliateProductIds.join(', ')}</small></span><span className="status-pill info"><i />{publishStatusLabels[item.status]}</span></div>)}</div></section>
      </div>
    </section>
  );
}
