import type { ActiveSiteId } from '../../lib/useActiveSite';

export type AdminResourceKey =
  | 'catalog'
  | 'cms'
  | 'shop'
  | 'orders'
  | 'suppliers'
  | 'leads'
  | 'users'
  | 'analytics'
  | 'settings';

export type AdminResourceRow = {
  id: string;
  title: string;
  type: string;
  status: 'active' | 'published' | 'draft' | 'archived' | 'new' | 'warning' | 'neutral';
  owner: string;
  updated: string;
  metric: string;
};

const bySite = <T,>(church: T, construction: T, kitchen: T): Record<ActiveSiteId, T> => ({
  church,
  construction,
  kitchen
});

export const adminMockData: Record<AdminResourceKey, Record<ActiveSiteId, AdminResourceRow[]>> = {
  cms: bySite(
    [
      { id: 'cms-ch-1', title: 'Православный календарь', type: 'Page', status: 'published', owner: 'Content', updated: 'Сегодня', metric: 'RU' },
      { id: 'cms-ch-2', title: 'Молитвы дня', type: 'Collection', status: 'draft', owner: 'Editor', updated: 'Вчера', metric: '12 items' }
    ],
    [
      { id: 'cms-co-1', title: 'Услуги ремонта', type: 'Landing', status: 'published', owner: 'Sales', updated: 'Сегодня', metric: 'RU' },
      { id: 'cms-co-2', title: 'Проекты Алматы', type: 'Cases', status: 'active', owner: 'Manager', updated: 'Пн', metric: '8 cases' }
    ],
    [
      { id: 'cms-ki-1', title: 'Рецепты недели', type: 'Article', status: 'published', owner: 'Editor', updated: 'Сегодня', metric: 'PL' },
      { id: 'cms-ki-2', title: 'Гайды по ингредиентам', type: 'Guide', status: 'draft', owner: 'SEO', updated: 'Вчера', metric: '24 pages' }
    ]
  ),
  catalog: bySite(
    [
      { id: 'cat-ch-1', title: 'Иконы', type: 'Category', status: 'active', owner: 'Catalog', updated: 'Сегодня', metric: '128 items' },
      { id: 'cat-ch-2', title: 'QR-страницы', type: 'Set', status: 'warning', owner: 'Ops', updated: 'Вчера', metric: 'needs sync' }
    ],
    [
      { id: 'cat-co-1', title: 'Материалы', type: 'Category', status: 'active', owner: 'Procurement', updated: 'Сегодня', metric: '64 items' },
      { id: 'cat-co-2', title: 'Комплекты смет', type: 'Bundle', status: 'draft', owner: 'Estimator', updated: 'Ср', metric: '9 bundles' }
    ],
    [
      { id: 'cat-ki-1', title: 'Ингредиенты', type: 'Category', status: 'active', owner: 'Kitchen', updated: 'Сегодня', metric: '310 items' },
      { id: 'cat-ki-2', title: 'Товары кухни', type: 'Products', status: 'published', owner: 'Shop', updated: 'Пн', metric: '42 items' }
    ]
  ),
  shop: bySite(
    [
      { id: 'shop-ch-1', title: 'Печатная икона', type: 'Product', status: 'draft', owner: 'Shop', updated: 'Вчера', metric: 'EUR' }
    ],
    [
      { id: 'shop-co-1', title: 'Партнерские материалы', type: 'Affiliate', status: 'active', owner: 'Suppliers', updated: 'Сегодня', metric: 'KZT' },
      { id: 'shop-co-2', title: 'Инструменты', type: 'Category', status: 'draft', owner: 'Shop', updated: 'Пн', metric: '18 offers' }
    ],
    [
      { id: 'shop-ki-1', title: 'Кухонные товары', type: 'Affiliate', status: 'active', owner: 'Shop', updated: 'Сегодня', metric: 'PLN' },
      { id: 'shop-ki-2', title: 'Подборки Allegro', type: 'Collection', status: 'published', owner: 'Affiliate', updated: 'Вчера', metric: '31 offers' }
    ]
  ),
  orders: bySite(
    [
      { id: 'ord-ch-1', title: 'Заказ молитвенного набора', type: 'Request', status: 'new', owner: 'Admin', updated: 'Сегодня', metric: '1 item' }
    ],
    [
      { id: 'ord-co-1', title: 'Заявка на материалы', type: 'Order', status: 'new', owner: 'Sales', updated: 'Сегодня', metric: 'KZT 420k' },
      { id: 'ord-co-2', title: 'Смета для ремонта', type: 'Estimate', status: 'active', owner: 'Manager', updated: 'Вчера', metric: 'in work' }
    ],
    [
      { id: 'ord-ki-1', title: 'Партнерский заказ', type: 'Affiliate', status: 'active', owner: 'Shop', updated: 'Сегодня', metric: 'PLN 180' }
    ]
  ),
  suppliers: bySite(
    [
      { id: 'sup-ch-1', title: 'Мастерская икон', type: 'Partner', status: 'active', owner: 'Admin', updated: 'Пн', metric: 'EUR' }
    ],
    [
      { id: 'sup-co-1', title: 'Алматы строймаркет', type: 'Local', status: 'active', owner: 'Procurement', updated: 'Сегодня', metric: 'KZT' },
      { id: 'sup-co-2', title: 'Поставщик плитки', type: 'Local', status: 'draft', owner: 'Procurement', updated: 'Вчера', metric: 'margin 12%' }
    ],
    [
      { id: 'sup-ki-1', title: 'Allegro Kitchen', type: 'Marketplace', status: 'active', owner: 'Affiliate', updated: 'Сегодня', metric: 'PLN' }
    ]
  ),
  leads: bySite(
    [
      { id: 'lead-ch-1', title: 'Вопрос по иконе', type: 'Contact', status: 'new', owner: 'Admin', updated: 'Сегодня', metric: 'RU' }
    ],
    [
      { id: 'lead-co-1', title: 'Ремонт квартиры', type: 'Lead', status: 'new', owner: 'Sales', updated: 'Сегодня', metric: 'Алматы' },
      { id: 'lead-co-2', title: 'Расчёт материалов', type: 'Lead', status: 'active', owner: 'Estimator', updated: 'Сегодня', metric: 'urgent' }
    ],
    [
      { id: 'lead-ki-1', title: 'Запрос рецепта', type: 'Contact', status: 'new', owner: 'Editor', updated: 'Сегодня', metric: 'PL' }
    ]
  ),
  users: bySite(
    [
      { id: 'user-ch-1', title: 'Church editor', type: 'Editor', status: 'active', owner: 'Admin', updated: 'Пн', metric: 'RU' }
    ],
    [
      { id: 'user-co-1', title: 'Construction manager', type: 'Manager', status: 'active', owner: 'Admin', updated: 'Сегодня', metric: 'RU' },
      { id: 'user-co-2', title: 'Estimator', type: 'Editor', status: 'active', owner: 'Admin', updated: 'Вчера', metric: 'KZ' }
    ],
    [
      { id: 'user-ki-1', title: 'Kitchen editor', type: 'Editor', status: 'active', owner: 'Admin', updated: 'Сегодня', metric: 'PL' }
    ]
  ),
  analytics: bySite(
    [
      { id: 'ana-ch-1', title: 'Календарь', type: 'Traffic', status: 'active', owner: 'Analytics', updated: 'Сегодня', metric: '1.8k views' }
    ],
    [
      { id: 'ana-co-1', title: 'Заявки', type: 'Conversion', status: 'active', owner: 'Analytics', updated: 'Сегодня', metric: '6 leads' },
      { id: 'ana-co-2', title: 'Каталог', type: 'Traffic', status: 'warning', owner: 'SEO', updated: 'Вчера', metric: 'CTR 2.4%' }
    ],
    [
      { id: 'ana-ki-1', title: 'Рецепты', type: 'Traffic', status: 'active', owner: 'Analytics', updated: 'Сегодня', metric: '4.2k views' }
    ]
  ),
  settings: bySite(
    [
      { id: 'set-ch-1', title: 'Языки сайта', type: 'Config', status: 'active', owner: 'Admin', updated: 'Сегодня', metric: 'RU/EN/PL' }
    ],
    [
      { id: 'set-co-1', title: 'Валюта и регион', type: 'Config', status: 'active', owner: 'Admin', updated: 'Сегодня', metric: 'KZT' }
    ],
    [
      { id: 'set-ki-1', title: 'Affiliate настройки', type: 'Config', status: 'active', owner: 'Admin', updated: 'Сегодня', metric: 'PLN' }
    ]
  )
};
