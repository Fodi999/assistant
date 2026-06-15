import { useEffect, useMemo, useState } from 'react';
import { listArticles } from '../api/cms';
import { DataSourceBadge, type DataSource } from '../components/DataSourceBadge';
import { contentArticles, siteLabel } from '../lib/mockData';
import type { CmsArticle, ContentArticle, ContentType, SiteKey } from '../types/admin';
import { AppIcon } from '../components/AppIcon';
import { contentTypeLabels, publishStatusLabels } from '../lib/labels';

function articleFromApi(article: CmsArticle): ContentArticle {
  return {
    id: article.id,
    site: 'culinary',
    type: 'article',
    title: {
      ru: article.title_ru || article.title_en || article.slug,
      pl: article.title_pl || article.title_en || article.slug,
      en: article.title_en || article.title_ru || article.slug,
      kk: article.title_ru || article.title_en || article.slug
    },
    slug: article.slug,
    excerpt: {
      ru: article.seo_description_ru || article.seo_description || '',
      pl: article.seo_description_pl || article.seo_description || '',
      en: article.seo_description_en || article.seo_description || '',
      kk: article.seo_description_ru || article.seo_description || ''
    },
    status: article.published ? 'published' : 'draft',
    languages: ['ru', 'pl', 'en'],
    affiliateProductIds: [],
    seoTitle: {
      ru: article.seo_title_ru || article.seo_title || '',
      pl: article.seo_title_pl || article.seo_title || '',
      en: article.seo_title_en || article.seo_title || '',
      kk: article.seo_title_ru || article.seo_title || ''
    },
    seoDescription: {
      ru: article.seo_description_ru || article.seo_description || '',
      pl: article.seo_description_pl || article.seo_description || '',
      en: article.seo_description_en || article.seo_description || '',
      kk: article.seo_description_ru || article.seo_description || ''
    },
    publishedAt: typeof article.published_at === 'string' ? article.published_at : undefined
  };
}

export function ContentPage({ activeSite }: { activeSite: SiteKey }) {
  const [site, setSite] = useState<SiteKey | 'all'>(activeSite);
  const [type, setType] = useState<ContentType | 'all'>('all');
  const [items, setItems] = useState<ContentArticle[]>(contentArticles);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();
  const rows = useMemo(() => items.filter((item) => (site === 'all' || item.site === site) && (type === 'all' || item.type === type)), [items, site, type]);

  useEffect(() => {
    void listArticles()
      .then((articles) => {
        setItems(articles.map(articleFromApi));
        setSource('api');
        setSourceError(undefined);
      })
      .catch((error) => {
        setItems(contentArticles);
        setSource('mock');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, []);

  return (
    <section className="ops-page">
      <Header title="Контент" subtitle="Статьи, обзоры, сравнения, подборки и связка с партнерскими товарами." icon="cms" source={source} />
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул контент: {sourceError}. Показаны mock-данные.</p> : null}
      <div className="filter-bar"><select value={site} onChange={(event) => setSite(event.target.value as SiteKey | 'all')}><option value="all">Все сайты</option><option value="culinary">Кулинарный</option><option value="construction">Строительный</option></select><select value={type} onChange={(event) => setType(event.target.value as ContentType | 'all')}><option value="all">Все типы</option><option value="article">статья</option><option value="review">обзор</option><option value="comparison">сравнение</option><option value="roundup">подборка</option><option value="recipe">рецепт</option></select></div>
      <section className="ops-panel"><table className="ops-table"><thead><tr><th>Материал</th><th>Сайт</th><th>Тип</th><th>Партнерские товары</th><th>Статус</th></tr></thead><tbody>{rows.map((item) => <tr key={item.id}><td><strong>{item.title.ru}</strong><small>{item.slug}</small></td><td>{siteLabel(item.site)}</td><td>{contentTypeLabels[item.type]}</td><td>{item.affiliateProductIds.join(', ') || 'нет'}</td><td><span className="status-pill info"><i />{publishStatusLabels[item.status]}</span></td></tr>)}</tbody></table></section>
    </section>
  );
}

function Header({ title, subtitle, icon, source }: { title: string; subtitle: string; icon: 'cms'; source: DataSource }) {
  return <div className="ops-header"><div className="ops-header-icon"><AppIcon name={icon} /></div><div><p className="eyebrow">Публикации</p><h2>{title}</h2><p>{subtitle}</p></div><div className="ops-header-actions"><DataSourceBadge source={source} label="Контент" /></div></div>;
}
