import { useEffect, useRef, useState } from 'react';
import * as echarts from 'echarts';
import { AppIcon, type AppIconName } from '../components/AppIcon';
import type { AppPage, ManagedSite } from '../components/Sidebar';
import { aiEditAlmabuildItem, saveAlmabuildContent, type AlmabuildContent, type Kit, type MaterialCategory, type Product, type Project } from '../api/almabuild';
import { findUsbKey, runAdminTool, type AdminToolOutput, type UsbKeyStatus } from '../api/localAdmin';
import { aiCreateArticleDraft, updateArticle } from '../api/cms';
import type { AdminCategory, AdminProduct, AdminStats, AdminUser, CmsArticle, ShopProduct } from '../types/admin';
import type { AnalyticsOverview, AnalyticsRealtime, SearchConsoleBundle } from '../api/analytics';

interface OperationsPageProps {
  page: AppPage;
  activeSite: ManagedSite;
  stats: AdminStats | null;
  users: AdminUser[];
  products: AdminProduct[];
  categories: AdminCategory[];
  articles: CmsArticle[];
  shopProducts: ShopProduct[];
  almabuildContent: AlmabuildContent | null;
  analytics: AnalyticsOverview | null;
  realtime: AnalyticsRealtime | null;
  searchConsole: SearchConsoleBundle | null;
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
}

type Tone = 'good' | 'warning' | 'danger' | 'info' | 'neutral';
type SiteDataset = {
  name: string;
  domain: string;
  cloudflare: string;
  github: string;
  backend: string;
  languages: string;
  ga4: boolean;
  searchConsole: boolean;
  categories: number;
  products: number;
  projects: number;
  seoPages: number;
  leadsToday: number;
  leadsMonth: number;
  activeObjects: number;
  deployLabel: string;
};

const TONE_LABELS: Record<Tone, string> = {
  good: 'норма',
  warning: 'внимание',
  danger: 'ошибка',
  info: 'инфо',
  neutral: 'нет данных'
};

const MODULE_META: Record<AppPage, { title: string; subtitle: string; icon: AppIconName }> = {
  overview: { title: 'Панель управления', subtitle: 'Отдельная панель выбранного сайта без смешивания данных.', icon: 'dashboard' },
  sites: { title: 'Настройки сайта', subtitle: 'Домен, GitHub, Cloudflare Pages, API бэкенда и интеграции выбранного сайта.', icon: 'globe' },
  leads: { title: 'CRM заявок', subtitle: 'Заявки и воронка только выбранного сайта.', icon: 'leads' },
  catalog: { title: 'Каталог', subtitle: 'Каталог и карточки, которые относятся только к текущему сайту.', icon: 'catalog' },
  materials: { title: 'Материалы', subtitle: 'Материалы, категории, наличие и публикация на выбранный сайт.', icon: 'materials' },
  suppliers: { title: 'Поставщики', subtitle: 'Поставщики и прайсы выбранного сайта.', icon: 'suppliers' },
  projects: { title: 'Проекты', subtitle: 'Проекты и кейсы, публикуемые на выбранный сайт.', icon: 'folder' },
  seo: { title: 'SEO-фабрика', subtitle: 'SEO-страницы, sitemap и индексация выбранного сайта.', icon: 'seo' },
  analytics: { title: 'Аналитика', subtitle: 'GA4 и Search Console только для выбранного сайта.', icon: 'analytics' },
  ai: { title: 'AI-студия', subtitle: 'Генерация контента в контексте выбранного сайта.', icon: 'bot' },
  usb: { title: 'USB Key', subtitle: 'Локальная станция тяжёлых задач с флешки, без нагрузки на Koyeb.', icon: 'hard-drive' },
  deployments: { title: 'Деплои', subtitle: 'Деплои Cloudflare Pages выбранного сайта.', icon: 'deploy' },
  settings: { title: 'Настройки', subtitle: 'Настройки выбранного сайта и его интеграций.', icon: 'settings' }
};

function dataset(props: OperationsPageProps): SiteDataset {
  if (props.activeSite === 'almabuild') {
    const content = props.almabuildContent;
    return {
      name: 'KAZAXBUD',
      domain: 'https://kazaxbud.pages.dev/',
      cloudflare: 'kazaxbud.pages.dev',
      github: 'kazaxbud',
      backend: 'Koyeb / /api/admin/almabuild/content',
      languages: 'RU / KK / EN',
      ga4: false,
      searchConsole: false,
      categories: content?.materialCategories.length ?? 0,
      products: content?.products.length ?? 0,
      projects: content?.projects.length ?? 0,
      seoPages: (content?.materialCategories.length ?? 0) * 3,
      leadsToday: 0,
      leadsMonth: 0,
      activeObjects: content?.projects.length ?? 0,
      deployLabel: 'kazaxbud.pages.dev продакшн'
    };
  }

  return {
    name: 'Dima Fomin',
    domain: 'https://dima-fomin.pl/',
    cloudflare: 'dima-fomin.pl',
    github: 'dima-fomin.pl',
    backend: 'API админки Koyeb',
    languages: 'PL / RU / EN / UK',
    ga4: true,
    searchConsole: true,
    categories: props.categories.length,
    products: props.products.length + props.shopProducts.length,
    projects: props.articles.length,
    seoPages: props.articles.length,
    leadsToday: 3,
    leadsMonth: 47,
    activeObjects: props.stats?.total_restaurants ?? 0,
    deployLabel: 'dima-fomin.pl продакшн'
  };
}

function metric(title: string, value: string | number, change: string, tone: Tone, icon: AppIconName) {
  return { title, value: String(value), change, tone, icon };
}

function TrafficChart({ connected = true }: { connected?: boolean }) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!ref.current || !connected) return;
    const chart = echarts.init(ref.current, 'dark');
    chart.setOption({
      backgroundColor: 'transparent',
      grid: { top: 18, right: 12, bottom: 24, left: 34 },
      tooltip: { trigger: 'axis', backgroundColor: '#18181b', borderColor: '#27272a', textStyle: { color: '#fafafa' } },
      xAxis: { type: 'category', boundaryGap: false, data: ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'], axisLine: { lineStyle: { color: '#27272a' } }, axisLabel: { color: '#a1a1aa' } },
      yAxis: { type: 'value', splitLine: { lineStyle: { color: '#27272a' } }, axisLabel: { color: '#a1a1aa' } },
      series: [
        { name: 'Пользователи GA4', type: 'line', smooth: true, symbol: 'none', lineStyle: { width: 3, color: '#ff6a2a' }, areaStyle: { color: 'rgba(255,106,42,.13)' }, data: [42, 57, 48, 76, 92, 84, 118] },
        { name: 'Клики GSC', type: 'line', smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#3b82f6' }, data: [18, 22, 31, 28, 44, 39, 53] }
      ]
    });
    const resize = () => chart.resize();
    window.addEventListener('resize', resize);
    return () => { window.removeEventListener('resize', resize); chart.dispose(); };
  }, [connected]);

  if (!connected) return <div className="ops-chart empty-analytics"><AppIcon name="analytics" size={28} /><strong>Аналитика не подключена</strong><span>Для этого сайта еще не подключены GA4 и Search Console.</span></div>;
  return <div className="ops-chart" ref={ref} />;
}

function StatusPill({ tone, label }: { tone: Tone; label: string }) {
  return <span className={'status-pill ' + tone}><i />{label}</span>;
}

function PanelTitle({ title, icon, action }: { title: string; icon: AppIconName; action?: string }) {
  return <div className="panel-title"><span><AppIcon name={icon} />{title}</span>{action ? <small>{action}</small> : null}</div>;
}

function PageHeader({ props, data }: { props: OperationsPageProps; data: SiteDataset }) {
  const meta = MODULE_META[props.page];
  return (
    <section className="ops-header">
      <div className="ops-header-icon"><AppIcon name={meta.icon} size={22} /></div>
      <div>
        <p className="eyebrow">{data.name} / {data.domain}</p>
        <h2>{meta.title}</h2>
        <p>{meta.subtitle}</p>
      </div>
      <div className="ops-header-actions">
        <button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить</button>
        <button className="btn btn-primary" type="button"><AppIcon name="sparkles" />Генерировать</button>
      </div>
    </section>
  );
}

function KpiCard({ title, value, change, tone, icon }: ReturnType<typeof metric>) {
  return <article className="kpi-card"><div className="kpi-card-head"><span><AppIcon name={icon} /></span><StatusPill tone={tone} label={TONE_LABELS[tone]} /></div><p>{title}</p><strong>{value}</strong><small>{change}</small><div className="sparkline"><i /><i /><i /><i /><i /><i /></div></article>;
}

function Overview({ props, data }: { props: OperationsPageProps; data: SiteDataset }) {
  const ga4Users = data.ga4 ? (props.analytics?.active_users ?? props.realtime?.active_users ?? 0) : 0;
  const gscClicks = data.searchConsole ? (props.searchConsole?.overview?.clicks ?? 0) : 0;
  const cards = [
    metric('Заявки сегодня', data.leadsToday, data.ga4 ? '+18% к вчерашнему дню' : 'CRM для сайта пока не подключена', data.leadsToday ? 'good' : 'neutral', 'leads'),
    metric('Заявки за месяц', data.leadsMonth, data.ga4 ? '+11% за месяц' : 'отдельная воронка пустая', data.leadsMonth ? 'good' : 'neutral', 'trend'),
    metric('Активные объекты', data.activeObjects, 'только текущий сайт', data.activeObjects ? 'warning' : 'neutral', 'folder'),
    metric('SEO-страницы', data.seoPages, 'только этот сайт', 'info', 'seo'),
    metric('Товары в каталоге', data.products, 'каталог этого сайта', 'good', 'catalog'),
    metric('Материалы / категории', data.categories, 'контент этого сайта', 'info', 'materials'),
    metric('Пользователи GA4', ga4Users, data.ga4 ? 'подключено для dima-fomin.pl' : 'не подключено для kazaxbud', data.ga4 ? 'info' : 'warning', 'analytics'),
    metric('Клики GSC', gscClicks, data.searchConsole ? 'Search Console подключен' : 'не подключено для kazaxbud', data.searchConsole ? 'info' : 'warning', 'activity'),
    metric('Последний деплой', '12м', data.deployLabel, 'good', 'deploy'),
    metric('Ошибки сайта', 0, 'критических ошибок нет', 'good', 'shield')
  ];

  const seoItems = props.activeSite === 'almabuild'
    ? ['/ru/stroymaterialy-almaty', '/ru/gipsokarton-almaty', '/kk/almaty-qurylys-materialdary']
    : ['/pl', '/ru', '/blog'];

  return <><div className="kpi-grid">{cards.map((card) => <KpiCard key={card.title} {...card} />)}</div><div className="ops-grid two-one"><section className="ops-panel wide"><PanelTitle title="Трафик и аналитика" icon="analytics" action={data.ga4 ? 'GA4 + GSC подключены' : 'GA4 / GSC не подключены'} /><TrafficChart connected={data.ga4 || data.searchConsole} /></section><section className="ops-panel"><PanelTitle title="Очередь сайта" icon="terminal" action={data.name} /><QueueList data={data} /></section></div><div className="ops-grid three"><MiniList title="Последние заявки" icon="leads" items={data.leadsToday ? ['Новая заявка с сайта', 'Форма консультации', 'Email-заявка'] : ['CRM сайта пустая', 'Подключить формы заявок', 'Настроить источники']} /><MiniList title="SEO задачи" icon="seo" items={seoItems} /><MiniList title="Последние деплои" icon="deploy" items={[data.deployLabel, 'проверка sitemap', 'проверка robots.txt']} /></div></>;
}

function QueueList({ data }: { data: SiteDataset }) {
  const rows: Array<[string, string, Tone]> = [
    ['Область данных', data.name, 'info'],
    ['Аналитика', data.ga4 ? 'подключено' : 'не подключено', data.ga4 ? 'good' : 'warning'],
    ['Search Console', data.searchConsole ? 'подключено' : 'не подключено', data.searchConsole ? 'good' : 'warning'],
    ['Деплой Cloudflare', 'успешно', 'good']
  ];
  return <div className="ops-list">{rows.map(([name, value, tone]) => <div key={name}><span>{name}</span><StatusPill tone={tone} label={value} /></div>)}</div>;
}

function MiniList({ title, icon, items }: { title: string; icon: AppIconName; items: string[] }) {
  return <section className="ops-panel"><PanelTitle title={title} icon={icon} /><div className="mini-list">{items.map((item) => <button key={item} type="button"><span>{item}</span><AppIcon name="external" /></button>)}</div></section>;
}



function EditorField({ label, value, onChange, multiline = false }: { label: string; value: string; onChange: (value: string) => void; multiline?: boolean }) {
  return <label className="editor-field"><span>{label}</span>{multiline ? <textarea value={value} onChange={(event) => onChange(event.target.value)} /> : <input value={value} onChange={(event) => onChange(event.target.value)} />}</label>;
}

function EditorMessage({ value }: { value: string | null }) {
  return value ? <p className="editor-message">{value}</p> : null;
}

function GeminiBar({ instruction, busy, onChange, onRun }: { instruction: string; busy: boolean; onChange: (value: string) => void; onRun: () => void }) {
  return <div className="gemini-bar"><label><span>Задача для Gemini</span><textarea value={instruction} onChange={(event) => onChange(event.target.value)} placeholder="Например: сделай текст сильнее для коммерческих клиентов, сохрани slug и визуальный класс" /></label><button className="btn btn-quiet" type="button" onClick={onRun} disabled={busy}>{busy ? 'Gemini редактирует...' : 'Улучшить Gemini'}</button></div>;
}

function splitLines(value: string): string[] {
  return value.split('\n').map((item) => item.trim()).filter(Boolean);
}

async function saveAlmabuildDraft(nextContent: AlmabuildContent, onRefresh: () => void, setMessage: (value: string | null) => void) {
  setMessage('Сохраняем изменения...');
  await saveAlmabuildContent(nextContent);
  setMessage('Изменения сохранены и доступны сайту Kazaxbud');
  await onRefresh();
}

function MaterialCardEditor({ item, index, content, onRefresh }: { item: MaterialCategory; index: number; content: AlmabuildContent; onRefresh: () => void }) {
  const [draft, setDraft] = useState(item);
  const [message, setMessage] = useState<string | null>(null);
  const [aiInstruction, setAiInstruction] = useState('Улучши текст карточки для сайта строительных материалов. Сделай описание конкретнее и продающим, пункты короткими.');
  const [aiBusy, setAiBusy] = useState(false);
  async function runGemini() {
    setAiBusy(true); setMessage(null);
    try { setDraft(await aiEditAlmabuildItem<MaterialCategory>('material', aiInstruction, draft)); setMessage('Gemini обновил карточку. Проверьте и нажмите «Сохранить карточку».'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить карточку'); }
    finally { setAiBusy(false); }
  }
  async function save() { await saveAlmabuildDraft({ ...content, materialCategories: content.materialCategories.map((row, rowIndex) => rowIndex === index ? draft : row) }, onRefresh, setMessage); }
  async function remove() { await saveAlmabuildDraft({ ...content, materialCategories: content.materialCategories.filter((_, rowIndex) => rowIndex !== index) }, onRefresh, setMessage); }
  return <article className="editor-card"><div className="editor-card-head"><strong>Карточка материала #{index + 1}</strong><StatusPill tone="good" label="блок Материалы" /></div><div className="editor-grid"><EditorField label="Название карточки" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} /><EditorField label="URL slug" value={draft.slug} onChange={(slug) => setDraft({ ...draft, slug })} /><EditorField label="Индекс" value={draft.index} onChange={(nextIndex) => setDraft({ ...draft, index: nextIndex })} /><EditorField label="Визуальный класс" value={draft.photo} onChange={(photo) => setDraft({ ...draft, photo })} /><EditorField label="Описание" value={draft.text} onChange={(nextText) => setDraft({ ...draft, text: nextText })} multiline /><EditorField label="Пункты внутри карточки, каждый с новой строки" value={draft.bullets.join('\n')} onChange={(bullets) => setDraft({ ...draft, bullets: splitLines(bullets) })} multiline /></div><GeminiBar instruction={aiInstruction} busy={aiBusy} onChange={setAiInstruction} onRun={() => void runGemini()} /><div className="editor-actions"><button className="btn btn-primary" type="button" onClick={save}>Сохранить карточку</button><button className="btn btn-quiet" type="button" onClick={remove}>Удалить</button></div><EditorMessage value={message} /></article>;
}

function ProductCardEditor({ item, index, content, onRefresh }: { item: Product; index: number; content: AlmabuildContent; onRefresh: () => void }) {
  const [draft, setDraft] = useState(item);
  const [message, setMessage] = useState<string | null>(null);
  const [aiInstruction, setAiInstruction] = useState('Улучши товарную карточку: название понятно, характеристики короткие, стиль B2B для строительного каталога.');
  const [aiBusy, setAiBusy] = useState(false);
  async function runGemini() {
    setAiBusy(true); setMessage(null);
    try { setDraft(await aiEditAlmabuildItem<Product>('product', aiInstruction, draft)); setMessage('Gemini обновил товар. Проверьте и сохраните.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить товар'); }
    finally { setAiBusy(false); }
  }
  async function save() { await saveAlmabuildDraft({ ...content, products: content.products.map((row, rowIndex) => rowIndex === index ? draft : row) }, onRefresh, setMessage); }
  async function remove() { await saveAlmabuildDraft({ ...content, products: content.products.filter((_, rowIndex) => rowIndex !== index) }, onRefresh, setMessage); }
  return <article className="editor-card"><div className="editor-card-head"><strong>Товарная карточка #{index + 1}</strong><StatusPill tone="good" label="каталог" /></div><div className="editor-grid"><EditorField label="Название товара" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} /><EditorField label="Метка категории" value={draft.category} onChange={(category) => setDraft({ ...draft, category })} /><EditorField label="Slug категории" value={draft.categorySlug} onChange={(categorySlug) => setDraft({ ...draft, categorySlug })} /><EditorField label="Визуальный класс" value={draft.photo} onChange={(photo) => setDraft({ ...draft, photo })} /><EditorField label="Характеристики" value={draft.spec} onChange={(spec) => setDraft({ ...draft, spec })} multiline /></div><GeminiBar instruction={aiInstruction} busy={aiBusy} onChange={setAiInstruction} onRun={() => void runGemini()} /><div className="editor-actions"><button className="btn btn-primary" type="button" onClick={save}>Сохранить товар</button><button className="btn btn-quiet" type="button" onClick={remove}>Удалить</button></div><EditorMessage value={message} /></article>;
}

function KitCardEditor({ item, index, content, onRefresh }: { item: Kit; index: number; content: AlmabuildContent; onRefresh: () => void }) {
  const [draft, setDraft] = useState(item);
  const [message, setMessage] = useState<string | null>(null);
  const [aiInstruction, setAiInstruction] = useState('Улучши комплект материалов: сделай описание понятным для закупщика, состав коротким и практичным.');
  const [aiBusy, setAiBusy] = useState(false);
  async function runGemini() {
    setAiBusy(true); setMessage(null);
    try { setDraft(await aiEditAlmabuildItem<Kit>('kit', aiInstruction, draft)); setMessage('Gemini обновил комплект. Проверьте и сохраните.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить комплект'); }
    finally { setAiBusy(false); }
  }
  async function save() { await saveAlmabuildDraft({ ...content, kits: content.kits.map((row, rowIndex) => rowIndex === index ? draft : row) }, onRefresh, setMessage); }
  async function remove() { await saveAlmabuildDraft({ ...content, kits: content.kits.filter((_, rowIndex) => rowIndex !== index) }, onRefresh, setMessage); }
  return <article className="editor-card"><div className="editor-card-head"><strong>Комплект #{index + 1}</strong><StatusPill tone="info" label="готовый набор" /></div><div className="editor-grid"><EditorField label="Название комплекта" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} /><EditorField label="Описание" value={draft.text} onChange={(nextText) => setDraft({ ...draft, text: nextText })} multiline /><EditorField label="Состав, каждый пункт с новой строки" value={draft.items.join('\n')} onChange={(items) => setDraft({ ...draft, items: splitLines(items) })} multiline /></div><GeminiBar instruction={aiInstruction} busy={aiBusy} onChange={setAiInstruction} onRun={() => void runGemini()} /><div className="editor-actions"><button className="btn btn-primary" type="button" onClick={save}>Сохранить комплект</button><button className="btn btn-quiet" type="button" onClick={remove}>Удалить</button></div><EditorMessage value={message} /></article>;
}

function ProjectCardEditor({ item, index, content, onRefresh }: { item: Project; index: number; content: AlmabuildContent; onRefresh: () => void }) {
  const [draft, setDraft] = useState(item);
  const [message, setMessage] = useState<string | null>(null);
  const [aiInstruction, setAiInstruction] = useState('Улучши проектную карточку: название оставить, meta сделать убедительным и кратким для портфолио.');
  const [aiBusy, setAiBusy] = useState(false);
  async function runGemini() {
    setAiBusy(true); setMessage(null);
    try { setDraft(await aiEditAlmabuildItem<Project>('project', aiInstruction, draft)); setMessage('Gemini обновил проект. Проверьте и сохраните.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить проект'); }
    finally { setAiBusy(false); }
  }
  async function save() { await saveAlmabuildDraft({ ...content, projects: content.projects.map((row, rowIndex) => rowIndex === index ? draft : row) }, onRefresh, setMessage); }
  async function remove() { await saveAlmabuildDraft({ ...content, projects: content.projects.filter((_, rowIndex) => rowIndex !== index) }, onRefresh, setMessage); }
  return <article className="editor-card"><div className="editor-card-head"><strong>Проект #{index + 1}</strong><StatusPill tone="info" label="кейс на главной" /></div><div className="editor-grid"><EditorField label="Название проекта" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} /><EditorField label="Метаданные" value={draft.meta} onChange={(meta) => setDraft({ ...draft, meta })} /><EditorField label="Визуальный класс" value={draft.photo} onChange={(photo) => setDraft({ ...draft, photo })} /></div><GeminiBar instruction={aiInstruction} busy={aiBusy} onChange={setAiInstruction} onRun={() => void runGemini()} /><div className="editor-actions"><button className="btn btn-primary" type="button" onClick={save}>Сохранить проект</button><button className="btn btn-quiet" type="button" onClick={remove}>Удалить</button></div><EditorMessage value={message} /></article>;
}

function AlmabuildEditor({ props, mode }: { props: OperationsPageProps; mode: 'materials' | 'products' | 'kits' | 'projects' }) {
  const [message, setMessage] = useState<string | null>(null);
  const content = props.almabuildContent;
  if (!content) return <section className="ops-panel"><PanelTitle title="Редактор Kazaxbud" icon="cms" action="контент не загружен" /><p className="empty-state">Нажмите «Обновить», чтобы загрузить контент сайта.</p></section>;
  async function addItem() {
    const current = props.almabuildContent;
    if (!current) return;
    if (mode === 'materials') await saveAlmabuildDraft({ ...current, materialCategories: [...current.materialCategories, { index: '[0:0]', slug: 'novaya-kategoriya', title: 'Новая категория', text: 'Описание категории', bullets: ['Пункт'], photo: 'material-new' }] }, props.onRefresh, setMessage);
    if (mode === 'products') await saveAlmabuildDraft({ ...current, products: [...current.products, { title: 'Новый товар', category: 'Категория', categorySlug: 'novaya-kategoriya', spec: 'Характеристики', photo: 'photo-building' }] }, props.onRefresh, setMessage);
    if (mode === 'kits') await saveAlmabuildDraft({ ...current, kits: [...current.kits, { title: 'Новый комплект', text: 'Описание комплекта', items: ['Позиция'] }] }, props.onRefresh, setMessage);
    if (mode === 'projects') await saveAlmabuildDraft({ ...current, projects: [...current.projects, { title: 'Новый проект', meta: 'Тип · площадь · срок', photo: 'photo-building' }] }, props.onRefresh, setMessage);
  }
  const title = mode === 'materials' ? 'Карточки материалов Kazaxbud' : mode === 'products' ? 'Товарные карточки Kazaxbud' : mode === 'kits' ? 'Комплекты Kazaxbud' : 'Проектные карточки Kazaxbud';
  return <section className="ops-panel"><PanelTitle title={title} icon="cms" action="редактируется через Gemini и backend Kazaxbud" /><div className="editor-toolbar"><button className="btn btn-primary" type="button" onClick={addItem}>Добавить</button><EditorMessage value={message} /></div><div className="editor-list">{mode === 'materials' && content.materialCategories.map((item, index) => <MaterialCardEditor key={item.slug + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'products' && content.products.map((item, index) => <ProductCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'kits' && content.kits.map((item, index) => <KitCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'projects' && content.projects.map((item, index) => <ProjectCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}</div></section>;
}

function DimaArticleEditor({ article, onRefresh }: { article: CmsArticle; onRefresh: () => void }) {
  const [draft, setDraft] = useState({ title_ru: article.title_ru || '', title_en: article.title_en || '', content_ru: article.content_ru || '', slug: article.slug || '', category: article.category || '', seo_title_ru: article.seo_title_ru || article.seo_title || '', seo_description_ru: article.seo_description_ru || article.seo_description || '', published: Boolean(article.published) });
  const [aiInstruction, setAiInstruction] = useState('Улучши страницу как экспертный материал для сайта Dima Fomin. Обнови заголовок, текст и SEO на русском.');
  const [aiBusy, setAiBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  async function runGemini() {
    setAiBusy(true); setMessage(null);
    try {
      const topic = [draft.title_ru || draft.title_en || article.slug, aiInstruction, draft.content_ru].filter(Boolean).join('\n\n');
      const ai = await aiCreateArticleDraft(topic, 3500, 1);
      setDraft({ ...draft, title_ru: ai.title_ru || draft.title_ru, title_en: ai.title_en || draft.title_en, content_ru: ai.content_ru || draft.content_ru, slug: ai.slug || draft.slug, category: ai.category || draft.category, seo_title_ru: ai.seo_title_ru || ai.seo_title || draft.seo_title_ru, seo_description_ru: ai.seo_description_ru || ai.seo_description || draft.seo_description_ru });
      setMessage('Gemini подготовил обновление. Проверьте и нажмите «Сохранить страницу».');
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить страницу'); }
    finally { setAiBusy(false); }
  }
  async function save() { setMessage('Сохраняем страницу...'); await updateArticle(article.id, draft); setMessage('Страница Dima Fomin сохранена'); await onRefresh(); }
  return <article className="editor-card"><div className="editor-card-head"><strong>{draft.title_ru || draft.title_en || article.slug}</strong><StatusPill tone={draft.published ? 'good' : 'neutral'} label={draft.published ? 'опубликовано' : 'черновик'} /></div><div className="editor-grid"><EditorField label="Заголовок RU" value={draft.title_ru} onChange={(title_ru) => setDraft({ ...draft, title_ru })} /><EditorField label="Заголовок EN" value={draft.title_en} onChange={(title_en) => setDraft({ ...draft, title_en })} /><EditorField label="Slug страницы" value={draft.slug} onChange={(slug) => setDraft({ ...draft, slug })} /><EditorField label="Категория" value={draft.category} onChange={(category) => setDraft({ ...draft, category })} /><EditorField label="Текст RU" value={draft.content_ru} onChange={(content_ru) => setDraft({ ...draft, content_ru })} multiline /><EditorField label="SEO title RU" value={draft.seo_title_ru} onChange={(seo_title_ru) => setDraft({ ...draft, seo_title_ru })} /><EditorField label="SEO description RU" value={draft.seo_description_ru} onChange={(seo_description_ru) => setDraft({ ...draft, seo_description_ru })} multiline /></div><GeminiBar instruction={aiInstruction} busy={aiBusy} onChange={setAiInstruction} onRun={() => void runGemini()} /><label className="editor-check"><input type="checkbox" checked={draft.published} onChange={(event) => setDraft({ ...draft, published: event.target.checked })} />Опубликовано на сайте</label><div className="editor-actions"><button className="btn btn-primary" type="button" onClick={save}>Сохранить страницу</button></div><EditorMessage value={message} /></article>;
}

function DimaPagesEditor({ props }: { props: OperationsPageProps }) {
  const rows = props.articles.slice(0, 12);
  return <section className="ops-panel"><PanelTitle title="Страницы и карточки Dima Fomin" icon="cms" action="редактируется через Gemini и CMS backend" />{rows.length === 0 ? <p className="empty-state">Страницы не загружены. Нажмите «Обновить».</p> : <div className="editor-list">{rows.map((article) => <DimaArticleEditor key={article.id} article={article} onRefresh={props.onRefresh} />)}</div>}</section>;
}

function SiteSettings({ data }: { data: SiteDataset }) {
  return <div className="site-card-grid"><article className="site-card"><div><h3>{data.name}</h3><StatusPill tone={data.ga4 ? 'good' : 'warning'} label={data.ga4 ? 'аналитика подключена' : 'аналитика не подключена'} /></div><p>{data.domain}</p><dl><dt>Cloudflare</dt><dd>{data.cloudflare}</dd><dt>GitHub</dt><dd>{data.github}</dd><dt>Backend</dt><dd>{data.backend}</dd><dt>Языки</dt><dd>{data.languages}</dd><dt>GA4</dt><dd>{data.ga4 ? 'подключено' : 'не подключено'}</dd><dt>Search Console</dt><dd>{data.searchConsole ? 'подключено' : 'не подключено'}</dd><dt>Sitemap</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd><dt>robots.txt</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd></dl><div className="card-actions"><button>Открыть сайт</button><button>Синхронизировать</button><button>Деплой</button></div></article></div>;
}

function LeadsCrm({ data }: { data: SiteDataset }) {
  const stages = ['Новая', 'Связались', 'Замер', 'Смета', 'Договор', 'В работе', 'Сдано', 'Отказ'];
  return <div className="kanban-board">{stages.map((stage, index) => <section className="kanban-column" key={stage}><h3>{stage}<span>{data.leadsMonth && index < 5 ? index + 1 : 0}</span></h3>{data.leadsMonth && index < 5 ? <article className="lead-card"><strong>{['Заявка сайта', 'Консультация', 'SEO-заявка', 'Смета', 'Договор'][index]}</strong><p>{data.name} · источник: сайт</p><small>Отдельная воронка сайта</small></article> : <div className="empty-state">Нет заявок для {data.name}</div>}</section>)}</div>;
}

function CatalogTable({ props }: { props: OperationsPageProps }) {
  if (props.activeSite === 'almabuild') {
    const rows = props.almabuildContent?.products ?? [];
    return <section className="ops-panel"><PanelTitle title="Товары Kazaxbud" icon="catalog" action="из /almabuild/content" /><table className="ops-table"><thead><tr><th>Название</th><th>Категория</th><th>Slug</th><th>Характеристики</th><th>Статус</th></tr></thead><tbody>{rows.map((product) => <tr key={product.title}><td>{product.title}</td><td>{product.category}</td><td>{product.categorySlug}</td><td>{product.spec}</td><td><StatusPill tone="good" label="опубликовано" /></td></tr>)}</tbody></table></section>;
  }

  const rows = props.products.slice(0, 8);
  return <section className="ops-panel"><PanelTitle title="Каталог Dima" icon="catalog" action="из бэкенда Dima" /><table className="ops-table"><thead><tr><th>Название</th><th>Категория</th><th>SKU</th><th>SEO</th><th>Статус</th></tr></thead><tbody>{rows.map((product) => <tr key={product.id}><td>{product.name_ru || product.name_en}</td><td>{product.category_id}</td><td>{product.slug || product.id.slice(0, 8)}</td><td><StatusPill tone={product.seo_title ? 'good' : 'warning'} label={product.seo_title ? 'готово' : 'не заполнено'} /></td><td><StatusPill tone={product.is_published ? 'good' : 'neutral'} label={product.is_published ? 'опубликовано' : 'черновик'} /></td></tr>)}</tbody></table></section>;
}

function MaterialsTable({ props }: { props: OperationsPageProps }) {
  if (props.activeSite === 'almabuild') {
    const rows = props.almabuildContent?.materialCategories ?? [];
    return <section className="ops-panel"><PanelTitle title="Категории материалов Kazaxbud" icon="materials" action="блок сайта: материалы" /><table className="ops-table"><thead><tr><th>Название</th><th>Slug</th><th>Индекс</th><th>Пункты</th><th>Статус</th></tr></thead><tbody>{rows.map((category) => <tr key={category.slug}><td>{category.title}</td><td>{category.slug}</td><td>{category.index}</td><td>{category.bullets.length}</td><td><StatusPill tone="good" label="на сайте" /></td></tr>)}</tbody></table></section>;
  }
  return <CatalogTable props={props} />;
}

function GenericCards({ data, icon, items }: { data: SiteDataset; icon: AppIconName; items: string[] }) {
  return <div className="ops-grid three">{items.map((item) => <article className="ops-panel module-card" key={item}><PanelTitle title={item} icon={icon} /><p>{data.name}</p><div className="card-actions"><button>Редактировать</button><button>Открыть</button><button>Опубликовать</button></div></article>)}</div>;
}

function SeoFactory({ data, activeSite }: { data: SiteDataset; activeSite: ManagedSite }) {
  const rows = activeSite === 'almabuild'
    ? ['/ru/stroymaterialy-almaty', '/ru/gipsokarton-almaty', '/kk/almaty-qurylys-materialdary', '/en/building-materials-almaty']
    : ['/pl', '/ru', '/en', '/blog'];
  return <section className="ops-panel"><PanelTitle title="Очередь SEO-страниц" icon="seo" action={data.name} /><table className="ops-table"><thead><tr><th>Slug</th><th>Статус</th><th>Клики</th><th>Показы</th><th>AI-оценка</th><th>Действие</th></tr></thead><tbody>{rows.map((row, index) => <tr key={row}><td>{row}</td><td><StatusPill tone={data.searchConsole ? 'good' : 'warning'} label={data.searchConsole ? 'в индексе' : 'не подключено'} /></td><td>{data.searchConsole ? 42 + index * 18 : 0}</td><td>{data.searchConsole ? 900 + index * 210 : 0}</td><td>{86 - index * 4}</td><td><button className="table-action">Обновить</button></td></tr>)}</tbody></table></section>;
}

function AiStudio({ data }: { data: SiteDataset }) {
  const templates = ['Описание товара', 'SEO-лендинг', 'Страница категории материалов', 'Кейс проекта', 'Перевод RU/KK/EN', 'Резюме сметы'];
  return <div className="ai-studio"><aside>{templates.map((item, index) => <button className={index === 1 ? 'active' : ''} key={item}>{item}</button>)}</aside><section className="ops-panel"><PanelTitle title="Рабочее поле промта" icon="bot" /><textarea placeholder="Опиши задачу для генерации..." defaultValue={'Сгенерируй SEO-лендинг для ' + data.name + ' на домене ' + data.domain} /><button className="btn btn-primary"><AppIcon name="sparkles" />Генерировать</button></section><section className="ops-panel preview-panel"><PanelTitle title="Предпросмотр" icon="cms" /><p>Результат будет привязан только к сайту {data.name}.</p></section></div>;
}


const LOCAL_JOBS: Array<{ title: string; description: string; args: string[]; tone: Tone }> = [
  { title: 'Admin tool help', description: 'Проверка локального бинарника на флешке.', args: ['help'], tone: 'info' },
  { title: 'Catalog state audit', description: 'Покрытие processing states по каталогу.', args: ['state-audit'], tone: 'info' },
  { title: 'Data quality report', description: 'Полный отчёт качества продуктов.', args: ['data-quality'], tone: 'warning' },
  { title: 'Generate all missing states', description: 'Массовая генерация недостающих состояний.', args: ['generate-states-all'], tone: 'warning' },
  { title: 'Catalog AI audit', description: 'AI-аудит каталога через локальный ключ.', args: ['catalog-audit'], tone: 'warning' },
  { title: 'Run intent-pages scheduler', description: 'Локальный запуск pSEO scheduler.', args: ['run-intent-scheduler'], tone: 'good' }
];

function formatAdminOutput(result: AdminToolOutput | null, error: string | null) {
  if (error) return error;
  if (!result) return 'Выберите локальную задачу слева. Команды выполняются с флешки через bin/admin_tool.';
  return [`$ ${result.command}`, `key: ${result.key_root}`, `exit=${result.status}`, '', result.stdout, result.stderr ? `\n[stderr]\n${result.stderr}` : ''].join('\n');
}

function UsbKeyPanel() {
  const [status, setStatus] = useState<UsbKeyStatus | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [result, setResult] = useState<AdminToolOutput | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [custom, setCustom] = useState('help');

  async function refresh() {
    setError(null);
    try { setStatus(await findUsbKey()); }
    catch (err) { setError(err instanceof Error ? err.message : 'Не удалось проверить USB Key'); }
  }

  async function run(args: string[], title = args.join(' ')) {
    setRunning(title);
    setError(null);
    setResult(null);
    try { setResult(await runAdminTool(args)); }
    catch (err) { setError(err instanceof Error ? err.message : 'Не удалось выполнить admin_tool'); }
    finally { setRunning(null); void refresh(); }
  }

  useEffect(() => { void refresh(); }, []);

  const output = formatAdminOutput(result, error);
  const envStatus: Array<[string, boolean]> = [
    ['USB Key', Boolean(status?.found)],
    ['admin_tool', Boolean(status?.admin_tool)],
    ['config', Boolean(status?.config)]
  ];
  const storage = status?.storage;
  const dataPaths = status?.data_paths;

  return <div className="usb-key-layout">
    <section className="ops-panel usb-key-summary">
      <PanelTitle title="AssistantAdminKey" icon="hard-drive" action={status?.found ? 'флешка подключена' : 'флешка не найдена'} />
      <div className="usb-status-grid">{envStatus.map(([label, ok]) => <div key={label}><span>{label}</span><StatusPill tone={ok ? 'good' : 'danger'} label={ok ? 'ok' : 'нет'} /></div>)}</div>
      <p className="page-muted">{status?.root || 'Вставьте флешку с папкой AssistantAdminKey. Koyeb остаётся лёгким, тяжёлые задачи запускаются локально.'}</p>
      <button className="btn btn-quiet" type="button" onClick={() => void refresh()}><AppIcon name="refresh" />Проверить ключ</button>
    </section>

    <section className="ops-panel usb-storage-panel">
      <PanelTitle title="Storage" icon="database" action="память флешки" />
      <div className="usb-storage-cards">
        <article><span>Всего памяти</span><strong>{storage?.total_label || 'нет данных'}</strong></article>
        <article><span>Занято</span><strong>{storage?.used_label || 'нет данных'}</strong></article>
        <article><span>Свободно</span><strong>{storage?.available_label || 'нет данных'}</strong></article>
      </div>
      <div className="usb-path-list">
        <div><span>Config</span><code>{dataPaths?.config || status?.config || 'нет данных'}</code></div>
        <div><span>Backups</span><code>{dataPaths?.backups || 'нет данных'}</code></div>
        <div><span>Exports</span><code>{dataPaths?.exports || 'нет данных'}</code></div>
        <div><span>Local DB</span><code>{dataPaths?.local_db || 'нет данных'}</code></div>
        <div><span>Logs</span><code>{dataPaths?.logs || 'нет данных'}</code></div>
      </div>
    </section>

    <section className="ops-panel usb-jobs-panel">
      <PanelTitle title="Локальные задачи" icon="terminal" action="bin/admin_tool" />
      <div className="usb-job-list">{LOCAL_JOBS.map((job) => <button key={job.title} type="button" onClick={() => void run(job.args, job.title)} disabled={Boolean(running)}><span><strong>{job.title}</strong><small>{job.description}</small></span><StatusPill tone={job.tone} label={running === job.title ? 'run' : job.args[0]} /></button>)}</div>
      <div className="usb-custom-command"><input value={custom} onChange={(event) => setCustom(event.target.value)} placeholder="autofill-product <product_id>" /><button className="btn btn-primary" type="button" onClick={() => void run(custom.split(' ').filter(Boolean), 'Custom command')} disabled={Boolean(running)}><AppIcon name="zap" />Run</button></div>
    </section>

    <section className="ops-panel usb-output-panel">
      <PanelTitle title="Результат" icon="code" action={running ? 'выполняется...' : 'готово'} />
      <pre>{output}</pre>
    </section>
  </div>;
}

function Deployments({ data }: { data: SiteDataset }) {
  return <section className="ops-panel"><PanelTitle title="Деплои Cloudflare Pages" icon="deploy" action={data.cloudflare} /><div className="deploy-list"><div><span><strong>{data.deployLabel}</strong><small>ветка main · последний коммит · продакшн</small></span><StatusPill tone="good" label="успешно" /><button>Логи</button></div></div></section>;
}

function SettingsMatrix({ data }: { data: SiteDataset }) {
  const rows = [['URL бэкенда Koyeb', data.backend], ['Проект Cloudflare Pages', data.cloudflare], ['Репозиторий GitHub', data.github], ['Свойство GA4', data.ga4 ? 'подключено' : 'не подключено'], ['Сайт Search Console', data.searchConsole ? 'подключено' : 'не подключено'], ['Языки', data.languages], ['Telegram / WhatsApp', 'не настроено']];
  return <section className="ops-panel"><PanelTitle title="Настройки сайта" icon="settings" /><div className="settings-matrix">{rows.map(([row, value]) => <label key={row}><span>{row}</span><input defaultValue={value} /></label>)}</div></section>;
}

function ModuleContent({ props, data }: { props: OperationsPageProps; data: SiteDataset }) {
  if (props.page === 'overview') return <Overview props={props} data={data} />;
  if (props.page === 'sites') return <SiteSettings data={data} />;
  if (props.page === 'leads') return <LeadsCrm data={data} />;
  if (props.page === 'catalog') return props.activeSite === 'almabuild' ? <AlmabuildEditor props={props} mode="products" /> : <CatalogTable props={props} />;
  if (props.page === 'materials') return props.activeSite === 'almabuild' ? <AlmabuildEditor props={props} mode="materials" /> : <MaterialsTable props={props} />;
  if (props.page === 'suppliers') return <GenericCards data={data} icon="suppliers" items={props.activeSite === 'almabuild' ? ['Партнер Knauf Алматы', 'Электрика Trade KZ', 'Плитка Market Almaty'] : ['Консалтинг-партнер', 'Хостинг-провайдер', 'Контент-подрядчик']} />;
  if (props.page === 'projects') return props.activeSite === 'almabuild' ? <AlmabuildEditor props={props} mode="projects" /> : <DimaPagesEditor props={props} />;
  if (props.page === 'seo') return props.activeSite === 'almabuild' ? <AlmabuildEditor props={props} mode="kits" /> : <DimaPagesEditor props={props} />;
  if (props.page === 'analytics') return <section className="ops-panel"><PanelTitle title="Аналитика" icon="analytics" action={data.ga4 ? 'GA4 + Search Console' : 'не подключено'} /><TrafficChart connected={data.ga4 || data.searchConsole} /></section>;
  if (props.page === 'ai') return <AiStudio data={data} />;
  if (props.page === 'usb') return <UsbKeyPanel />;
  if (props.page === 'deployments') return <Deployments data={data} />;
  return <SettingsMatrix data={data} />;
}

export function OperationsPage(props: OperationsPageProps) {
  const data = dataset(props);
  return <section className="ops-page" key={props.activeSite + '-' + props.page}><PageHeader props={props} data={data} />{props.error ? <p className="ops-alert"><AppIcon name="shield" />{props.error}</p> : null}<ModuleContent props={props} data={data} /></section>;
}
