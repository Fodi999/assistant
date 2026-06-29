import { useEffect, useState } from 'react';
import { listCulinaryProducts, listRecipes, listReviews } from '../../api/culinary';
import { AppIcon } from '../../components/AppIcon';
import { DataSourceBadge, type DataSource } from '../../components/DataSourceBadge';
import { contentTypeLabels, publishStatusLabels } from '../../lib/labels';
import type { AffiliateProduct, ContentArticle } from '../../types/admin';

export function CulinaryPage() {
  const [products, setProducts] = useState<AffiliateProduct[]>([]);
  const [content, setContent] = useState<ContentArticle[]>([]);
  const [source, setSource] = useState<DataSource>('unavailable');
  const [sourceError, setSourceError] = useState<string | undefined>();

  useEffect(() => {
    void Promise.all([listCulinaryProducts(), listRecipes(), listReviews()])
      .then(([nextProducts, recipes, reviews]) => {
        setProducts(nextProducts);
        setContent([...recipes, ...reviews]);
        setSource('api');
        setSourceError(undefined);
      })
      .catch((error) => {
        setProducts([]);
        setContent([]);
        setSource('unavailable');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, []);

  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="shop" /></div><div><p className="eyebrow">Кулинарная партнерка</p><h2>Кулинария</h2><p>Кухонные товары, рецепты, обзоры, подборки, профессиональные инструменты и ресторанное оборудование.</p></div><div className="ops-header-actions"><DataSourceBadge source={source} label="Кулинария" /></div></div>
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул кулинарный раздел: {sourceError}. Демо-данные отключены.</p> : null}
      <div className="ops-grid two-one">
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="shop" />Кухонные товары</span></div><table className="ops-table"><tbody>{products.map((item) => <tr key={item.id}><td><strong>{item.title.ru}</strong><small>{item.category}</small></td><td>{item.network}</td><td>{item.price} {item.currency}</td><td>{publishStatusLabels[item.status]}</td></tr>)}</tbody></table>{products.length === 0 ? <p className="empty-state">Товаров из backend нет.</p> : null}</section>
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="cms" />Рецепты и обзоры</span></div><div className="mini-list static">{content.map((item) => <div key={item.id}><span><strong>{item.title.ru}</strong><small>{contentTypeLabels[item.type]} / связано: {item.affiliateProductIds.join(', ')}</small></span><span className="status-pill info"><i />{publishStatusLabels[item.status]}</span></div>)}</div>{content.length === 0 ? <p className="empty-state">Материалов из backend нет.</p> : null}</section>
      </div>
    </section>
  );
}
