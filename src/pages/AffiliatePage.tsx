import { useEffect, useMemo, useState } from 'react';
import { listAffiliateProductsWithSource } from '../api/affiliate';
import { AffiliateProductEditor } from '../components/AffiliateProductEditor';
import { AffiliateOfferCards } from '../components/AffiliateOfferCards';
import { DataSourceBadge, type DataSource } from '../components/DataSourceBadge';
import { LanguageChips } from '../components/LanguageChips';
import { affiliateProducts } from '../lib/mockData';
import type { AffiliateNetwork, AffiliateProduct, LanguageCode, PublishStatus, SiteKey } from '../types/admin';
import { AppIcon } from '../components/AppIcon';
import { publishStatusLabels, siteNames } from '../lib/labels';

export function AffiliatePage({ activeSite }: { activeSite: SiteKey }) {
  const [siteFilter, setSiteFilter] = useState<SiteKey | 'all'>(activeSite);
  const [network, setNetwork] = useState<AffiliateNetwork | 'all'>('all');
  const [status, setStatus] = useState<PublishStatus | 'all'>('all');
  const [language, setLanguage] = useState<LanguageCode | 'all'>('all');
  const [editorProduct, setEditorProduct] = useState<AffiliateProduct | null | undefined>(undefined);
  const [products, setProducts] = useState<AffiliateProduct[]>(affiliateProducts);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const rows = useMemo(() => products.filter((product) =>
    (siteFilter === 'all' || product.site === siteFilter)
    && (network === 'all' || product.network === network)
    && (status === 'all' || product.status === status)
    && (language === 'all' || product.languages.includes(language))
  ), [products, siteFilter, network, status, language]);

  useEffect(() => {
    void listAffiliateProductsWithSource(siteFilter === 'all' ? undefined : siteFilter).then((result) => {
      setProducts(result.data);
      setSource(result.source);
      setSourceError(result.error);
    });
  }, [siteFilter]);

  return (
    <section className="ops-page">
      <PageHead title="Партнерка" subtitle="Товары, сети, продавцы, партнерские ссылки, комиссии, cookie days, SEO и офферы." icon="shop" action={<><DataSourceBadge source={source} label="Товары" /><button className="btn btn-primary" type="button" onClick={() => setEditorProduct(null)}><AppIcon name="sparkles" />Создать партнерский товар</button></>} />
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул партнерские товары: {sourceError}. Показаны mock-данные.</p> : null}
      <div className="filter-bar">
        <select value={siteFilter} onChange={(event) => setSiteFilter(event.target.value as SiteKey | 'all')}><option value="all">Все сайты</option><option value="culinary">Кулинарный</option><option value="construction">Строительный</option></select>
        <select value={network} onChange={(event) => setNetwork(event.target.value as AffiliateNetwork | 'all')}><option value="all">Все сети</option><option value="amazon">Amazon</option><option value="allegro">Allegro</option><option value="ceneo">Ceneo</option><option value="awin">Awin</option><option value="custom">Своя сеть</option></select>
        <select value={language} onChange={(event) => setLanguage(event.target.value as LanguageCode | 'all')}><option value="all">Все языки</option><option value="ru">RU</option><option value="pl">PL</option><option value="en">EN</option><option value="kk">KK</option></select>
        <select value={status} onChange={(event) => setStatus(event.target.value as PublishStatus | 'all')}><option value="all">Все статусы</option><option value="draft">черновик</option><option value="active">активно</option><option value="published">опубликовано</option><option value="archived">архив</option></select>
      </div>
      <div className="ops-grid two-one">
        <section className="ops-panel">
          <table className="ops-table">
            <thead><tr><th>Товар</th><th>Сайт</th><th>Сеть</th><th>Цена</th><th>Языки</th><th>Статус</th><th /></tr></thead>
            <tbody>{rows.map((product) => <tr key={product.id}><td><strong>{product.title.ru}</strong><small>{product.slug}</small></td><td>{siteNames[product.site]}</td><td>{product.network}</td><td>{product.price?.toLocaleString('ru-RU')} {product.currency}</td><td><LanguageChips value={product.languages} /></td><td><span className="status-pill info"><i />{publishStatusLabels[product.status]}</span></td><td><button className="table-action" type="button" onClick={() => setEditorProduct(product)}>Открыть</button></td></tr>)}</tbody>
          </table>
        </section>
        <section className="ops-panel">
          <div className="panel-title"><span><AppIcon name="external" />Офферы сетей</span><small>Amazon / Allegro / Ceneo / Awin / своя сеть</small></div>
          <AffiliateOfferCards offers={rows.flatMap((item) => item.offers ?? [])} />
        </section>
      </div>
      {editorProduct !== undefined ? <AffiliateProductEditor site={activeSite} product={editorProduct ?? undefined} onClose={() => setEditorProduct(undefined)} /> : null}
    </section>
  );
}

function PageHead({ title, subtitle, icon, action }: { title: string; subtitle: string; icon: 'shop' | 'cms' | 'bot' | 'building' | 'leads' | 'suppliers' | 'analytics' | 'globe'; action?: React.ReactNode }) {
  return <div className="ops-header"><div className="ops-header-icon"><AppIcon name={icon} /></div><div><p className="eyebrow">Мультисайтовая админка</p><h2>{title}</h2><p>{subtitle}</p></div><div className="ops-header-actions">{action}</div></div>;
}
