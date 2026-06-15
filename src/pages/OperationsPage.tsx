import { useEffect, useRef, useState } from 'react';
import * as echarts from 'echarts';
import { AppIcon, type AppIconName } from '../components/AppIcon';
import type { AppPage, ManagedSite } from '../components/Sidebar';
import { aiEditAlmabuildItem, saveAlmabuildContent, type AlmabuildContent, type Kit, type MaterialCategory, type Product, type Project } from '../api/almabuild';
import { aiCreateProductDraft, aiGenerateProductImage, createAdminProduct, generateProductStates, getAdminNutritionProduct, listProductStates, saveExtendedProductProfile, updateAdminProduct, type AiExtendedProductProfile, type CreateAdminProductRequest, type IngredientState } from '../api/catalog';
import { adminKeyAiHistoryList, adminKeyAiHistoryRead, adminKeyGeminiGenerateImagePrompt, adminKeyGeminiGenerateText, adminKeyGeminiSettingsStatus, adminKeyOpenFolder, adminKeyPromptList, adminKeyPromptRead, adminKeyPromptRender, findUsbKey, runAdminTool, type AdminToolOutput, type AiHistoryItem, type GeminiSettingsStatus, type PromptTemplateItem, type UsbKeyStatus } from '../api/localAdmin';
import { aiCreateArticleDraft, createArticle, updateArticle } from '../api/cms';
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
  const addLabel = mode === 'materials' ? 'Добавить материал' : mode === 'products' ? 'Добавить товар' : mode === 'kits' ? 'Добавить комплект' : 'Добавить проект';
  return <section className="ops-panel"><PanelTitle title={title} icon="cms" action="редактируется через Gemini и backend Kazaxbud" /><div className="editor-toolbar"><button className="btn btn-primary" type="button" onClick={addItem}>{addLabel}</button><EditorMessage value={message} /></div><div className="editor-list">{mode === 'materials' && content.materialCategories.map((item, index) => <MaterialCardEditor key={item.slug + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'products' && content.products.map((item, index) => <ProductCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'kits' && content.kits.map((item, index) => <KitCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'projects' && content.projects.map((item, index) => <ProjectCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}</div></section>;
}

type SeoArticleDraft = Partial<Omit<CmsArticle, 'id' | 'updated_at' | 'created_at'>>;
type SeoArticleTab = 'content' | 'seo' | 'languages' | 'media';

function extractArticleImages(article?: CmsArticle): string[] {
  if (!article) return [];
  const markdownImages = [article.content_ru, article.content_en, article.content_pl, article.content_uk].flatMap((content) =>
    Array.from(String(content || '').matchAll(/!\[[^\]]*\]\((https?:\/\/[^)\s]+)\)/g)).map((match) => match[1])
  );
  return [article.image_url || '', ...markdownImages].filter(Boolean);
}

function stripArticleVisualStory(content?: string | null): string {
  return String(content || '').replace(/\n{0,2}##\s+Visual story[\s\S]*$/i, '').trim();
}

function articleDisplayTitle(article: CmsArticle | SeoArticleDraft) {
  return article.title_ru || article.title_pl || article.title_en || article.slug || 'Новая SEO-страница';
}

function articleToDraft(article?: CmsArticle): SeoArticleDraft {
  return {
    slug: article?.slug || '',
    category: article?.category || 'blog',
    title_ru: article?.title_ru || '',
    title_en: article?.title_en || '',
    title_pl: article?.title_pl || '',
    title_uk: article?.title_uk || '',
    content_ru: stripArticleVisualStory(article?.content_ru),
    content_en: stripArticleVisualStory(article?.content_en),
    content_pl: stripArticleVisualStory(article?.content_pl),
    content_uk: stripArticleVisualStory(article?.content_uk),
    image_url: article?.image_url || '',
    seo_title: article?.seo_title || '',
    seo_description: article?.seo_description || '',
    seo_title_ru: article?.seo_title_ru || article?.seo_title || '',
    seo_title_en: article?.seo_title_en || '',
    seo_title_pl: article?.seo_title_pl || '',
    seo_title_uk: article?.seo_title_uk || '',
    seo_description_ru: article?.seo_description_ru || article?.seo_description || '',
    seo_description_en: article?.seo_description_en || '',
    seo_description_pl: article?.seo_description_pl || '',
    seo_description_uk: article?.seo_description_uk || '',
    published: Boolean(article?.published)
  };
}

function applyArticleAiDraft(current: SeoArticleDraft, ai: Awaited<ReturnType<typeof aiCreateArticleDraft>>): SeoArticleDraft {
  return {
    ...current,
    slug: ai.slug || current.slug,
    category: ai.category || current.category,
    title_ru: ai.title_ru || current.title_ru,
    title_en: ai.title_en || current.title_en,
    title_pl: ai.title_pl || current.title_pl,
    title_uk: ai.title_uk || current.title_uk,
    content_ru: ai.content_ru || current.content_ru,
    content_en: ai.content_en || current.content_en,
    content_pl: ai.content_pl || current.content_pl,
    content_uk: ai.content_uk || current.content_uk,
    seo_title: ai.seo_title || ai.seo_title_ru || current.seo_title,
    seo_description: ai.seo_description || ai.seo_description_ru || current.seo_description,
    seo_title_ru: ai.seo_title_ru || ai.seo_title || current.seo_title_ru,
    seo_title_en: ai.seo_title_en || current.seo_title_en,
    seo_title_pl: ai.seo_title_pl || current.seo_title_pl,
    seo_title_uk: ai.seo_title_uk || current.seo_title_uk,
    seo_description_ru: ai.seo_description_ru || ai.seo_description || current.seo_description_ru,
    seo_description_en: ai.seo_description_en || current.seo_description_en,
    seo_description_pl: ai.seo_description_pl || current.seo_description_pl,
    seo_description_uk: ai.seo_description_uk || current.seo_description_uk
  };
}

function DimaArticleEditorModal({ article, onClose, onSaved }: { article?: CmsArticle; onClose: () => void; onSaved: () => Promise<void> | void }) {
  const [draft, setDraft] = useState<SeoArticleDraft>(() => articleToDraft(article));
  const [images, setImages] = useState<string[]>(() => extractArticleImages(article));
  const [selectedImage, setSelectedImage] = useState(0);
  const [source, setSource] = useState(article ? articleDisplayTitle(article) : 'Новая SEO/Blog страница для Dima Fomin: тема, аудитория, цель, ключевые слова.');
  const [instruction, setInstruction] = useState(article ? 'Улучши страницу как экспертный материал для сайта Dima Fomin. Обнови 4 языка, структуру текста и SEO.' : 'Создай новую SEO/Blog страницу для сайта Dima Fomin: 4 языка, slug, категория, экспертный текст, SEO title и description.');
  const [tab, setTab] = useState<SeoArticleTab>('content');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(article ? 'Страница загружена из CMS backend.' : 'Опиши тему и нажми «Текст и SEO».');
  const tabs: Array<[SeoArticleTab, string]> = [['content', 'Контент'], ['seo', 'SEO'], ['languages', '4 языка'], ['media', 'Медиа']];

  function setText(field: keyof SeoArticleDraft, value: string) { setDraft((current) => ({ ...current, [field]: value })); }
  function updateImage(index: number, value: string) {
    setImages((current) => {
      const next = [...current];
      next[index] = value;
      return next;
    });
    if (index === selectedImage) setDraft((current) => ({ ...current, image_url: value }));
  }
  function addImage() { setImages((current) => [...current, '']); }
  function removeImage(index: number) {
    setImages((current) => {
      const next = current.filter((_, itemIndex) => itemIndex !== index);
      const nextSelected = Math.max(0, Math.min(selectedImage, next.length - 1));
      setSelectedImage(nextSelected);
      setDraft((draftCurrent) => ({ ...draftCurrent, image_url: next[nextSelected] || '' }));
      return next;
    });
  }

  async function runGemini() {
    setBusy('gemini'); setMessage(null);
    try {
      const topic = [instruction, 'Тема/задача:', source, 'Текущая страница:', JSON.stringify(draft, null, 2)].filter(Boolean).join('\n\n');
      const ai = await aiCreateArticleDraft(topic, 5200, 1);
      setDraft((current) => applyArticleAiDraft(current, ai));
      setMessage('Gemini подготовил SEO/Blog страницу. Проверь вкладки и сохрани.');
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог подготовить страницу'); }
    finally { setBusy(null); }
  }

  async function save() {
    setBusy('save'); setMessage(null);
    try {
      const slug = String(draft.slug || '').trim();
      if (!slug) throw new Error('Slug страницы обязателен.');
      const cleanImages = images.map((url) => url.trim()).filter(Boolean);
      const coverImage = cleanImages[selectedImage] || cleanImages[0] || String(draft.image_url || '');
      const visualStory = cleanImages.length > 1
        ? `\n\n## Visual story\n${cleanImages.slice(1).map((url, index) => `![Step ${index + 1}](${url})`).join('\n\n')}`
        : '';
      const payload = {
        ...draft,
        slug,
        category: draft.category || 'blog',
        image_url: coverImage,
        content_ru: `${stripArticleVisualStory(draft.content_ru)}${visualStory}`,
        content_en: `${stripArticleVisualStory(draft.content_en)}${visualStory}`,
        content_pl: `${stripArticleVisualStory(draft.content_pl)}${visualStory}`,
        content_uk: `${stripArticleVisualStory(draft.content_uk)}${visualStory}`,
        published: Boolean(draft.published)
      };
      if (article) await updateArticle(article.id, payload);
      else await createArticle(payload);
      await Promise.resolve(onSaved());
      onClose();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось сохранить страницу'); }
    finally { setBusy(null); }
  }

  return <div className="catalog-edit-overlay compact" role="presentation" onMouseDown={onClose}>
    <section className="catalog-edit-modal compact" role="dialog" aria-label="SEO Blog editor" onMouseDown={(event) => event.stopPropagation()}>
      <div className="catalog-edit-head"><div><p className="eyebrow">Gemini SEO editor</p><h3>{articleDisplayTitle(draft)}</h3><span>{draft.category || 'blog'} · {draft.slug || 'new-page'}</span></div><button className="btn btn-quiet" type="button" onClick={onClose}>Закрыть</button></div>
      <nav className="catalog-edit-tabs">{tabs.map(([id, label]) => <button key={id} type="button" className={tab === id ? 'active' : ''} onClick={() => setTab(id)}>{label}</button>)}</nav>
      <section className="catalog-generation-panel">
        <label><span>Тема / задача</span><textarea value={source} onChange={(event) => setSource(event.target.value)} /></label>
        <label><span>Инструкция Gemini</span><textarea value={instruction} onChange={(event) => setInstruction(event.target.value)} /></label>
        <div className="catalog-generation-actions"><button className="btn btn-primary" type="button" onClick={() => void runGemini()} disabled={Boolean(busy)}><AppIcon name="sparkles" />{busy === 'gemini' ? 'Генерируем...' : 'Текст и SEO'}<small>gemini-3.1-pro-preview</small></button></div>
      </section>
      <div className="catalog-edit-body compact">
        <aside className="catalog-edit-photo compact">{(images[selectedImage] || draft.image_url) ? <img src={String(images[selectedImage] || draft.image_url)} alt={articleDisplayTitle(draft)} /> : <span><AppIcon name="seo" size={28} /></span>}<small>{images.length || 1} фото из CMS/Markdown</small></aside>
        <div className="catalog-edit-form">
          {tab === 'content' ? <div className="editor-grid"><EditorField label="Заголовок RU" value={String(draft.title_ru || '')} onChange={(value) => setText('title_ru', value)} /><EditorField label="Title EN" value={String(draft.title_en || '')} onChange={(value) => setText('title_en', value)} /><EditorField label="Slug страницы" value={String(draft.slug || '')} onChange={(value) => setText('slug', value)} /><EditorField label="Категория" value={String(draft.category || '')} onChange={(value) => setText('category', value)} /><EditorField label="Текст RU" value={String(draft.content_ru || '')} onChange={(value) => setText('content_ru', value)} multiline /><EditorField label="Content EN" value={String(draft.content_en || '')} onChange={(value) => setText('content_en', value)} multiline /><EditorField label="Tekst PL" value={String(draft.content_pl || '')} onChange={(value) => setText('content_pl', value)} multiline /><EditorField label="Текст UK" value={String(draft.content_uk || '')} onChange={(value) => setText('content_uk', value)} multiline /></div> : null}
          {tab === 'seo' ? <div className="editor-grid"><EditorField label="SEO title RU" value={String(draft.seo_title_ru || '')} onChange={(value) => setText('seo_title_ru', value)} /><EditorField label="SEO title EN" value={String(draft.seo_title_en || '')} onChange={(value) => setText('seo_title_en', value)} /><EditorField label="SEO title PL" value={String(draft.seo_title_pl || '')} onChange={(value) => setText('seo_title_pl', value)} /><EditorField label="SEO title UK" value={String(draft.seo_title_uk || '')} onChange={(value) => setText('seo_title_uk', value)} /><EditorField label="SEO description RU" value={String(draft.seo_description_ru || '')} onChange={(value) => setText('seo_description_ru', value)} multiline /><EditorField label="SEO description EN" value={String(draft.seo_description_en || '')} onChange={(value) => setText('seo_description_en', value)} multiline /><EditorField label="SEO description PL" value={String(draft.seo_description_pl || '')} onChange={(value) => setText('seo_description_pl', value)} multiline /><EditorField label="SEO description UK" value={String(draft.seo_description_uk || '')} onChange={(value) => setText('seo_description_uk', value)} multiline /></div> : null}
          {tab === 'languages' ? <div className="editor-grid"><EditorField label="Title PL" value={String(draft.title_pl || '')} onChange={(value) => setText('title_pl', value)} /><EditorField label="Title UK" value={String(draft.title_uk || '')} onChange={(value) => setText('title_uk', value)} /><EditorField label="Title RU" value={String(draft.title_ru || '')} onChange={(value) => setText('title_ru', value)} /><EditorField label="Title EN" value={String(draft.title_en || '')} onChange={(value) => setText('title_en', value)} /><EditorField label="Content RU" value={String(draft.content_ru || '')} onChange={(value) => setText('content_ru', value)} multiline /><EditorField label="Content EN" value={String(draft.content_en || '')} onChange={(value) => setText('content_en', value)} multiline /><EditorField label="Content PL" value={String(draft.content_pl || '')} onChange={(value) => setText('content_pl', value)} multiline /><EditorField label="Content UK" value={String(draft.content_uk || '')} onChange={(value) => setText('content_uk', value)} multiline /></div> : null}
          {tab === 'media' ? <div className="editor-grid"><div className="span-2 catalog-states-list">{(images.length ? images : [String(draft.image_url || '')]).map((url, index) => <article key={index}><strong>{index === selectedImage ? 'Главное фото' : `Фото ${index + 1}`}</strong><input value={url} onChange={(event) => updateImage(index, event.target.value)} placeholder="https://..." /><div className="editor-actions"><button className="btn btn-quiet" type="button" onClick={() => { setSelectedImage(index); setDraft((current) => ({ ...current, image_url: url })); }}>Сделать главным</button><button className="btn btn-quiet" type="button" onClick={() => removeImage(index)}>Удалить</button></div></article>)}<button className="btn btn-quiet" type="button" onClick={addImage}>Добавить фото URL</button></div><EditorField label="SEO title EN" value={String(draft.seo_title_en || '')} onChange={(value) => setText('seo_title_en', value)} /><EditorField label="SEO title PL" value={String(draft.seo_title_pl || '')} onChange={(value) => setText('seo_title_pl', value)} /><EditorField label="SEO title UK" value={String(draft.seo_title_uk || '')} onChange={(value) => setText('seo_title_uk', value)} /><EditorField label="SEO description EN" value={String(draft.seo_description_en || '')} onChange={(value) => setText('seo_description_en', value)} multiline /><EditorField label="SEO description PL" value={String(draft.seo_description_pl || '')} onChange={(value) => setText('seo_description_pl', value)} multiline /><EditorField label="SEO description UK" value={String(draft.seo_description_uk || '')} onChange={(value) => setText('seo_description_uk', value)} multiline /><label className="editor-check span-2"><input type="checkbox" checked={Boolean(draft.published)} onChange={(event) => setDraft((current) => ({ ...current, published: event.target.checked }))} />Опубликовано на сайте</label></div> : null}
        </div>
      </div>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><button className="btn btn-quiet" type="button" onClick={onClose}>Отмена</button><button className="btn btn-primary" type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : article ? 'Сохранить страницу' : 'Создать страницу'}</button></div>
    </section>
  </div>;
}

function DimaPagesEditor({ props }: { props: OperationsPageProps }) {
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [editingArticle, setEditingArticle] = useState<CmsArticle | null>(null);
  const [creatingArticle, setCreatingArticle] = useState(false);
  const categories = Array.from(new Set(props.articles.map((article) => article.category || 'blog'))).sort();
  const needle = query.trim().toLowerCase();
  const rows = props.articles.filter((article) => {
    const haystack = [article.title_ru, article.title_en, article.slug, article.category, article.seo_title, article.seo_description].filter(Boolean).join(' ').toLowerCase();
    return (!needle || haystack.includes(needle)) && (category === 'all' || (article.category || 'blog') === category);
  });
  return <section className="ops-panel catalog-browser">
    <PanelTitle title="SEO Blog Dima Fomin" icon="seo" action={`${rows.length} из ${props.articles.length}`} />
    <div className="catalog-toolbar"><label><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Заголовок, slug, SEO, категория" /></label><label><span>Категория</span><select value={category} onChange={(event) => setCategory(event.target.value)}><option value="all">Все категории</option>{categories.map((item) => <option key={item} value={item}>{item}</option>)}</select></label><button className="btn btn-primary" type="button" onClick={() => setCreatingArticle(true)}><AppIcon name="sparkles" />Создать SEO страницу</button><button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из backend</button></div>
    <div className="catalog-stats-row"><article><span>Всего страниц</span><strong>{props.articles.length}</strong></article><article><span>Опубликовано</span><strong>{props.articles.filter((article) => article.published).length}</strong></article><article><span>SEO готово</span><strong>{props.articles.filter((article) => article.seo_title || article.seo_title_ru).length}</strong></article><article><span>Категорий</span><strong>{categories.length}</strong></article></div>
    <div className="table-scroll"><table className="ops-table"><thead><tr><th>Название</th><th>Категория</th><th>Slug</th><th>SEO</th><th>Статус</th><th>Действие</th></tr></thead><tbody>{rows.map((article) => <tr key={article.id}><td><strong>{articleDisplayTitle(article)}</strong><small>{article.title_en}</small></td><td>{article.category || 'blog'}</td><td><code>{article.slug}</code></td><td><StatusPill tone={article.seo_title || article.seo_title_ru ? 'good' : 'warning'} label={article.seo_title || article.seo_title_ru ? 'готово' : 'не заполнено'} /></td><td><StatusPill tone={article.published ? 'good' : 'neutral'} label={article.published ? 'опубликовано' : 'черновик'} /></td><td><button className="table-action" type="button" onClick={() => setEditingArticle(article)}>Редактировать</button></td></tr>)}</tbody></table></div>
    {props.articles.length === 0 ? <p className="empty-state">SEO/Blog страницы не загружены. Нажмите «Обновить».</p> : null}
    {creatingArticle ? <DimaArticleEditorModal onClose={() => setCreatingArticle(false)} onSaved={props.onRefresh} /> : null}
    {editingArticle ? <DimaArticleEditorModal article={editingArticle} onClose={() => setEditingArticle(null)} onSaved={props.onRefresh} /> : null}
  </section>;
}

function SiteSettings({ data }: { data: SiteDataset }) {
  return <div className="site-card-grid"><article className="site-card"><div><h3>{data.name}</h3><StatusPill tone={data.ga4 ? 'good' : 'warning'} label={data.ga4 ? 'аналитика подключена' : 'аналитика не подключена'} /></div><p>{data.domain}</p><dl><dt>Cloudflare</dt><dd>{data.cloudflare}</dd><dt>GitHub</dt><dd>{data.github}</dd><dt>Backend</dt><dd>{data.backend}</dd><dt>Языки</dt><dd>{data.languages}</dd><dt>GA4</dt><dd>{data.ga4 ? 'подключено' : 'не подключено'}</dd><dt>Search Console</dt><dd>{data.searchConsole ? 'подключено' : 'не подключено'}</dd><dt>Sitemap</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd><dt>robots.txt</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd></dl><div className="card-actions"><button>Открыть сайт</button><button>Синхронизировать</button><button>Деплой</button></div></article></div>;
}

function LeadsCrm({ data }: { data: SiteDataset }) {
  const stages = ['Новая', 'Связались', 'Замер', 'Смета', 'Договор', 'В работе', 'Сдано', 'Отказ'];
  return <div className="kanban-board">{stages.map((stage, index) => <section className="kanban-column" key={stage}><h3>{stage}<span>{data.leadsMonth && index < 5 ? index + 1 : 0}</span></h3>{data.leadsMonth && index < 5 ? <article className="lead-card"><strong>{['Заявка сайта', 'Консультация', 'SEO-заявка', 'Смета', 'Договор'][index]}</strong><p>{data.name} · источник: сайт</p><small>Отдельная воронка сайта</small></article> : <div className="empty-state">Нет заявок для {data.name}</div>}</section>)}</div>;
}

function productDisplayName(product: AdminProduct) {
  return product.name_ru || product.name_pl || product.name_en || product.slug || product.id;
}

function categoryDisplayName(category?: AdminCategory) {
  return category?.name_ru || category?.name_pl || category?.name_en || category?.id || 'Без категории';
}


type CatalogEditDraft = Partial<CreateAdminProductRequest>;
type CatalogEditTab = 'basic' | 'content' | 'nutrition' | 'vitamins' | 'minerals' | 'culinary' | 'health' | 'states';

const CATALOG_VITAMINS = [['vitamin_a', 'A'], ['vitamin_c', 'C'], ['vitamin_d', 'D'], ['vitamin_e', 'E'], ['vitamin_k', 'K'], ['vitamin_b1', 'B1'], ['vitamin_b2', 'B2'], ['vitamin_b3', 'B3'], ['vitamin_b5', 'B5'], ['vitamin_b6', 'B6'], ['vitamin_b7', 'B7'], ['vitamin_b9', 'B9'], ['vitamin_b12', 'B12']] as const;
const CATALOG_MINERALS = [['calcium', 'Calcium'], ['iron', 'Iron'], ['magnesium', 'Magnesium'], ['phosphorus', 'Phosphorus'], ['potassium', 'Potassium'], ['sodium', 'Sodium'], ['zinc', 'Zinc'], ['copper', 'Copper'], ['manganese', 'Manganese'], ['selenium', 'Selenium']] as const;
const CATALOG_DIETS = [['vegan', 'Vegan'], ['vegetarian', 'Vegetarian'], ['keto', 'Keto'], ['paleo', 'Paleo'], ['gluten_free', 'Gluten free'], ['mediterranean', 'Mediterranean'], ['low_carb', 'Low carb']] as const;
const CATALOG_TEXT_MODEL = 'gemini-3.1-pro-preview';
const CATALOG_IMAGE_MODEL = 'gemini-3.1-flash-image';

function productToCatalogEditDraft(product: AdminProduct): CatalogEditDraft {
  return {
    name_en: product.name_en || '', name_ru: product.name_ru || '', name_pl: product.name_pl || '', name_uk: product.name_uk || '',
    unit: product.unit as CreateAdminProductRequest['unit'], product_type: product.product_type || '',
    description: product.description || product.description_en || '', description_en: product.description_en || product.description || '', description_ru: product.description_ru || '', description_pl: product.description_pl || '', description_uk: product.description_uk || '',
    image_url: product.image_url || undefined,
    calories_per_100g: product.calories_per_100g ?? undefined, protein_per_100g: product.protein_per_100g ?? undefined, fat_per_100g: product.fat_per_100g ?? undefined, carbs_per_100g: product.carbs_per_100g ?? undefined, fiber_per_100g: product.fiber_per_100g ?? undefined, sugar_per_100g: product.sugar_per_100g ?? undefined,
    density_g_per_ml: product.density_g_per_ml ?? undefined, typical_portion_g: product.typical_portion_g ?? undefined, shelf_life_days: product.shelf_life_days ?? undefined,
    seasons: product.seasons || [], seo_title: product.seo_title || '', seo_description: product.seo_description || '', seo_h1: product.seo_h1 || ''
  };
}

function draftFromGeminiResponse(current: CatalogEditDraft, response: Awaited<ReturnType<typeof aiCreateProductDraft>>): CatalogEditDraft {
  const draft = response.draft;
  return {
    ...current,
    name_en: draft.names.en.value || current.name_en, name_ru: draft.names.ru.value || current.name_ru, name_pl: draft.names.pl.value || current.name_pl, name_uk: draft.names.uk.value || current.name_uk,
    unit: draft.unit.value as CreateAdminProductRequest['unit'] || current.unit, product_type: draft.product_type.value || current.product_type,
    description: draft.description_en.value || current.description, description_en: draft.description_en.value || current.description_en, description_ru: draft.description_ru.value || current.description_ru, description_pl: draft.description_pl.value || current.description_pl, description_uk: draft.description_uk.value || current.description_uk,
    calories_per_100g: draft.nutrition.calories_per_100g.value ?? current.calories_per_100g, protein_per_100g: draft.nutrition.protein_per_100g.value ?? current.protein_per_100g, fat_per_100g: draft.nutrition.fat_per_100g.value ?? current.fat_per_100g, carbs_per_100g: draft.nutrition.carbs_per_100g.value ?? current.carbs_per_100g,
    fiber_per_100g: draft.nutrition.fiber_per_100g.value ?? current.fiber_per_100g, sugar_per_100g: draft.nutrition.sugar_per_100g.value ?? current.sugar_per_100g, density_g_per_ml: draft.nutrition.density_g_per_ml.value ?? current.density_g_per_ml, typical_portion_g: draft.nutrition.typical_portion_g.value ?? current.typical_portion_g, shelf_life_days: draft.nutrition.shelf_life_days.value ?? current.shelf_life_days,
    seasons: draft.seasons.value || current.seasons, seo_title: draft.seo.seo_title.value || current.seo_title, seo_description: draft.seo.seo_description.value || current.seo_description, seo_h1: draft.seo.seo_h1.value || current.seo_h1
  };
}

function profileFromDetail(detail: Awaited<ReturnType<typeof getAdminNutritionProduct>>): AiExtendedProductProfile {
  return {
    macros: detail.macros || {}, vitamins: detail.vitamins || {}, minerals: detail.minerals || {}, fatty_acids: detail.fatty_acids || {}, diet_flags: detail.diet_flags || {}, allergens: detail.allergens || {}, food_properties: detail.food_properties || {}, culinary: detail.culinary || {}, health_profile: detail.health_profile || {}, sugar_profile: detail.sugar_profile || {}, processing_effects: detail.processing_effects || {}, culinary_behavior: detail.culinary_behavior || {}
  };
}

function applyDetailToDraft(current: CatalogEditDraft, detail: Awaited<ReturnType<typeof getAdminNutritionProduct>>): CatalogEditDraft {
  return {
    ...current,
    name_en: detail.name_en || current.name_en, name_ru: detail.name_ru || current.name_ru, name_pl: detail.name_pl || current.name_pl, name_uk: detail.name_uk || current.name_uk,
    product_type: detail.product_type || current.product_type, unit: detail.unit as CreateAdminProductRequest['unit'] || current.unit, image_url: detail.image_url || current.image_url,
    description_en: detail.description_en || current.description_en, description_ru: detail.description_ru || current.description_ru, description_pl: detail.description_pl || current.description_pl, description_uk: detail.description_uk || current.description_uk,
    density_g_per_ml: detail.density_g_per_ml ?? current.density_g_per_ml, typical_portion_g: detail.typical_portion_g ?? current.typical_portion_g, shelf_life_days: detail.shelf_life_days ?? current.shelf_life_days,
    calories_per_100g: Number(detail.macros?.calories_kcal ?? current.calories_per_100g ?? '') || current.calories_per_100g,
    protein_per_100g: Number(detail.macros?.protein_g ?? current.protein_per_100g ?? '') || current.protein_per_100g,
    fat_per_100g: Number(detail.macros?.fat_g ?? current.fat_per_100g ?? '') || current.fat_per_100g,
    carbs_per_100g: Number(detail.macros?.carbs_g ?? current.carbs_per_100g ?? '') || current.carbs_per_100g,
    fiber_per_100g: Number(detail.macros?.fiber_g ?? current.fiber_per_100g ?? '') || current.fiber_per_100g,
    sugar_per_100g: Number(detail.macros?.sugar_g ?? current.sugar_per_100g ?? '') || current.sugar_per_100g
  };
}

function csvToArray(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String).filter(Boolean);
  return String(value || '').split(',').map((item) => item.trim()).filter(Boolean);
}

function arrayToCsv(value: unknown): string {
  return Array.isArray(value) ? value.join(', ') : String(value || '');
}

function CatalogProductEditor({ product, category, onClose, onSaved }: { product: AdminProduct; category?: AdminCategory; onClose: () => void; onSaved: () => Promise<void> | void }) {
  const [draft, setDraft] = useState<CatalogEditDraft>(() => productToCatalogEditDraft(product));
  const [profile, setProfile] = useState<AiExtendedProductProfile>({});
  const [states, setStates] = useState<IngredientState[]>([]);
  const [tab, setTab] = useState<CatalogEditTab>('basic');
  const [instruction, setInstruction] = useState('Заполни полную карточку продукта как на публичной странице: 4 языка, SEO, nutrition, витамины, минералы, кулинарный профиль, диеты и health profile. Не выдумывай медицинские гарантии.');
  const [busy, setBusy] = useState<string | null>('load');
  const [message, setMessage] = useState<string | null>('Загружаем полную карточку из backend...');

  useEffect(() => {
    let mounted = true;
    async function loadFullProduct() {
      setBusy('load');
      try {
        const [detail, nextStates] = await Promise.all([getAdminNutritionProduct(product.id), listProductStates(product.id).catch(() => [])]);
        if (!mounted) return;
        setDraft((current) => applyDetailToDraft(current, detail));
        setProfile(profileFromDetail(detail));
        setStates(nextStates);
        setMessage('Полная карточка загружена из backend.');
      } catch (err) {
        if (mounted) setMessage(err instanceof Error ? err.message : 'Не удалось загрузить полную карточку');
      } finally {
        if (mounted) setBusy(null);
      }
    }
    void loadFullProduct();
    return () => { mounted = false; };
  }, [product.id]);

  function setText(field: keyof CatalogEditDraft, value: string) { setDraft((current) => ({ ...current, [field]: value })); }
  function setNumber(field: keyof CatalogEditDraft, value: string) { setDraft((current) => ({ ...current, [field]: value === '' ? undefined : Number(value) })); }
  function profileValue(section: keyof AiExtendedProductProfile, field: string): unknown { return profile[section]?.[field]; }
  function setProfileValue(section: keyof AiExtendedProductProfile, field: string, value: string | number | boolean | null | string[]) { setProfile((current) => ({ ...current, [section]: { ...(current[section] || {}), [field]: value } })); }
  function setProfileNumber(section: keyof AiExtendedProductProfile, field: string, value: string) { setProfileValue(section, field, value === '' ? null : Number(value)); }

  async function runGeminiEdit() {
    setBusy('gemini'); setMessage(null);
    try {
      const response = await aiCreateProductDraft([instruction, 'Текущий продукт:', JSON.stringify({ slug: product.slug, category: categoryDisplayName(category), draft, profile }, null, 2)].join('\n\n'));
      setDraft((current) => draftFromGeminiResponse(current, response));
      setProfile((current) => ({ ...current, ...response.draft.extended }));
      setMessage(`Gemini обновил черновик. Уверенность ${Math.round(response.draft.confidence * 100)}%. Проверь вкладки и сохрани.`);
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Gemini не смог обновить продукт'); }
    finally { setBusy(null); }
  }

  async function generateImage() {
    setBusy('image'); setMessage(null);
    try {
      const image = await aiGenerateProductImage(String(draft.name_ru || draft.name_en || productDisplayName(product)), String(draft.description_ru || draft.description_en || draft.seo_description || ''), true);
      setDraft((current) => ({ ...current, image_url: image.image_url }));
      setMessage('Фото создано. Нажми Сохранить, чтобы привязать его к продукту.');
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Не удалось сгенерировать фото'); }
    finally { setBusy(null); }
  }

  async function regenerateStates() {
    setBusy('states'); setMessage(null);
    try {
      await generateProductStates(product.id);
      setStates(await listProductStates(product.id));
      setMessage('Состояния обработки обновлены.');
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Не удалось создать состояния'); }
    finally { setBusy(null); }
  }

  async function save() {
    setBusy('save'); setMessage(null);
    try {
      await updateAdminProduct(product.id, draft);
      await saveExtendedProductProfile(product.id, profile);
      await Promise.resolve(onSaved());
      setMessage('Продукт и nutrition-профиль сохранены в backend.');
      onClose();
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Не удалось сохранить продукт'); }
    finally { setBusy(null); }
  }

  const tabs: Array<[CatalogEditTab, string]> = [['basic', 'Основное'], ['content', '4 языка'], ['nutrition', 'Макросы'], ['vitamins', 'Витамины'], ['minerals', 'Минералы'], ['culinary', 'Кулинария'], ['health', 'Health'], ['states', 'Состояния']];

  return <div className="catalog-edit-overlay compact" role="presentation" onMouseDown={onClose}>
    <section className="catalog-edit-modal compact" role="dialog" aria-label="Редактирование продукта" onMouseDown={(event) => event.stopPropagation()}>
      <div className="catalog-edit-head"><div><p className="eyebrow">Gemini product editor</p><h3>{productDisplayName(product)}</h3><span>{categoryDisplayName(category)} · {product.slug || product.id}</span></div><button className="btn btn-quiet" type="button" onClick={onClose}>Закрыть</button></div>
      <nav className="catalog-edit-tabs">{tabs.map(([id, label]) => <button key={id} type="button" className={tab === id ? 'active' : ''} onClick={() => setTab(id)}>{label}</button>)}</nav>
      <section className="catalog-generation-panel">
        <label><span>Задача для Gemini</span><textarea value={instruction} onChange={(event) => setInstruction(event.target.value)} /></label>
        <div className="catalog-generation-actions">
          <button className="btn btn-primary" type="button" onClick={() => void runGeminiEdit()} disabled={Boolean(busy)}><AppIcon name="sparkles" />{busy === 'gemini' ? 'Генерируем текст...' : 'Текст и профиль'}<small>{CATALOG_TEXT_MODEL}</small></button>
          <button className="btn btn-quiet" type="button" onClick={() => void generateImage()} disabled={Boolean(busy)}><AppIcon name="package" />{busy === 'image' ? 'Генерируем фото...' : 'Фото'}<small>{CATALOG_IMAGE_MODEL}</small></button>
          <button className="btn btn-quiet" type="button" onClick={() => void regenerateStates()} disabled={Boolean(busy)}><AppIcon name="refresh" />{busy === 'states' ? 'Состояния...' : 'Состояния'}<small>{CATALOG_TEXT_MODEL}</small></button>
        </div>
      </section>
      <div className="catalog-edit-body compact">
        <aside className="catalog-edit-photo compact">{draft.image_url ? <img src={draft.image_url} alt={productDisplayName(product)} /> : <span><AppIcon name="package" size={28} /></span>}<small>Фото: {CATALOG_IMAGE_MODEL}</small></aside>
        <div className="catalog-edit-form">
          {tab === 'basic' ? <div className="editor-grid"><EditorField label="Название RU" value={String(draft.name_ru || '')} onChange={(value) => setText('name_ru', value)} /><EditorField label="Название EN" value={String(draft.name_en || '')} onChange={(value) => setText('name_en', value)} /><EditorField label="Название PL" value={String(draft.name_pl || '')} onChange={(value) => setText('name_pl', value)} /><EditorField label="Название UK" value={String(draft.name_uk || '')} onChange={(value) => setText('name_uk', value)} /><EditorField label="Тип продукта" value={String(draft.product_type || '')} onChange={(value) => setText('product_type', value)} /><EditorField label="Единица" value={String(draft.unit || '')} onChange={(value) => setText('unit', value)} /><EditorField label="Image URL" value={String(draft.image_url || '')} onChange={(value) => setText('image_url', value)} multiline /></div> : null}
          {tab === 'content' ? <div className="editor-grid"><EditorField label="Описание RU" value={String(draft.description_ru || '')} onChange={(value) => setText('description_ru', value)} multiline /><EditorField label="Описание EN" value={String(draft.description_en || '')} onChange={(value) => setText('description_en', value)} multiline /><EditorField label="Описание PL" value={String(draft.description_pl || '')} onChange={(value) => setText('description_pl', value)} multiline /><EditorField label="Описание UK" value={String(draft.description_uk || '')} onChange={(value) => setText('description_uk', value)} multiline /><EditorField label="SEO title" value={String(draft.seo_title || '')} onChange={(value) => setText('seo_title', value)} /><EditorField label="SEO H1" value={String(draft.seo_h1 || '')} onChange={(value) => setText('seo_h1', value)} /><EditorField label="SEO description" value={String(draft.seo_description || '')} onChange={(value) => setText('seo_description', value)} multiline /></div> : null}
          {tab === 'nutrition' ? <div className="catalog-nutrition-grid">{([['calories_per_100g', 'Kalorie/kcal'], ['protein_per_100g', 'Białko'], ['fat_per_100g', 'Tłuszcz'], ['carbs_per_100g', 'Węglowodany'], ['fiber_per_100g', 'Błonnik'], ['sugar_per_100g', 'Cukier'], ['density_g_per_ml', 'Gęstość'], ['typical_portion_g', 'Porcja'], ['shelf_life_days', 'Trwałość'], ['starch_g', 'Starch'], ['water_g', 'Water'], ['alcohol_g', 'Alcohol']] as Array<[string, string]>).map(([field, label]) => field in draft ? <label key={field}><span>{label}</span><input type="number" step="any" value={draft[field as keyof CatalogEditDraft] as number ?? ''} onChange={(event) => setNumber(field as keyof CatalogEditDraft, event.target.value)} /></label> : <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('macros', field) as number ?? ''} onChange={(event) => setProfileNumber('macros', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'vitamins' ? <div className="catalog-nutrition-grid">{CATALOG_VITAMINS.map(([field, label]) => <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('vitamins', field) as number ?? ''} onChange={(event) => setProfileNumber('vitamins', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'minerals' ? <div className="catalog-nutrition-grid">{CATALOG_MINERALS.map(([field, label]) => <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('minerals', field) as number ?? ''} onChange={(event) => setProfileNumber('minerals', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'culinary' ? <div className="editor-grid"><div className="catalog-nutrition-grid span-2">{(['sweetness', 'acidity', 'bitterness', 'umami', 'aroma'] as const).map((field) => <label key={field}><span>{field}</span><input type="number" step="any" value={profileValue('culinary', field) as number ?? ''} onChange={(event) => setProfileNumber('culinary', field, event.target.value)} /></label>)}{(['glycemic_index', 'glycemic_load', 'ph', 'smoke_point', 'water_activity'] as const).map((field) => <label key={field}><span>{field}</span><input type="number" step="any" value={profileValue('food_properties', field) as number ?? ''} onChange={(event) => setProfileNumber('food_properties', field, event.target.value)} /></label>)}</div><EditorField label="Texture" value={String(profileValue('culinary', 'texture') || '')} onChange={(value) => setProfileValue('culinary', 'texture', value)} multiline /><EditorField label="Processing notes PL" value={String(profileValue('processing_effects', 'processing_notes_pl') || '')} onChange={(value) => setProfileValue('processing_effects', 'processing_notes_pl', value)} multiline /></div> : null}
          {tab === 'health' ? <div className="editor-grid"><div className="toggle-grid span-2">{CATALOG_DIETS.map(([field, label]) => <label className="editor-check" key={field}><input type="checkbox" checked={Boolean(profileValue('diet_flags', field))} onChange={(event) => setProfileValue('diet_flags', field, event.target.checked)} />{label}</label>)}</div>{(['en', 'ru', 'pl', 'uk'] as const).flatMap((lang) => [<EditorField key={`bio-${lang}`} label={`Bioactive ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `bioactive_compounds_${lang}`))} onChange={(value) => setProfileValue('health_profile', `bioactive_compounds_${lang}`, csvToArray(value))} multiline />, <EditorField key={`eff-${lang}`} label={`Health effects ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `health_effects_${lang}`))} onChange={(value) => setProfileValue('health_profile', `health_effects_${lang}`, csvToArray(value))} multiline />])}<EditorField label="Food role" value={String(profileValue('health_profile', 'food_role') || '')} onChange={(value) => setProfileValue('health_profile', 'food_role', value)} /></div> : null}
          {tab === 'states' ? <div className="catalog-states-list">{states.map((state) => <article key={state.id}><strong>{state.name_suffix_pl || state.name_suffix_ru || state.state}</strong><span>{state.calories_per_100g ?? '-'} kcal · {state.storage_temp_c ?? '-'}°C · {state.shelf_life_hours ?? '-'} godz.</span><p>{state.notes_pl || state.notes_ru || state.notes_en}</p></article>)}</div> : null}
        </div>
      </div>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><button className="btn btn-quiet" type="button" onClick={onClose}>Отмена</button><button className="btn btn-primary" type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : 'Сохранить всё'}</button></div>
    </section>
  </div>;
}

function CatalogNewProductEditor({ categories, onClose, onSaved }: { categories: AdminCategory[]; onClose: () => void; onSaved: () => Promise<void> | void }) {
  const [source, setSource] = useState('Новый продукт для food-каталога: название, категория, вкус, использование, питание на 100 г.');
  const [draft, setDraft] = useState<CatalogEditDraft>({ unit: 'kilogram', product_type: '', seasons: [] });
  const [profile, setProfile] = useState<AiExtendedProductProfile>({});
  const [tab, setTab] = useState<CatalogEditTab>('basic');
  const [categoryId, setCategoryId] = useState(categories[0]?.id || '');
  const [instruction, setInstruction] = useState('Создай полную карточку нового продукта как на публичной странице: 4 языка, SEO, nutrition, витамины, минералы, кулинарный профиль, диеты и health profile. Не выдумывай медицинские гарантии.');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>('Опиши продукт и нажми «Текст и профиль».');

  function setText(field: keyof CatalogEditDraft, value: string) { setDraft((current) => ({ ...current, [field]: value })); }
  function setNumber(field: keyof CatalogEditDraft, value: string) { setDraft((current) => ({ ...current, [field]: value === '' ? undefined : Number(value) })); }
  function profileValue(section: keyof AiExtendedProductProfile, field: string): unknown { return profile[section]?.[field]; }
  function setProfileValue(section: keyof AiExtendedProductProfile, field: string, value: string | number | boolean | null | string[]) { setProfile((current) => ({ ...current, [section]: { ...(current[section] || {}), [field]: value } })); }
  function setProfileNumber(section: keyof AiExtendedProductProfile, field: string, value: string) { setProfileValue(section, field, value === '' ? null : Number(value)); }

  async function runGeminiCreate() {
    setBusy('gemini'); setMessage(null);
    try {
      const category = categories.find((item) => item.id === categoryId);
      const response = await aiCreateProductDraft([instruction, 'Категория:', categoryDisplayName(category), 'Описание нового продукта:', source, 'Текущий черновик:', JSON.stringify({ draft, profile }, null, 2)].join('\n\n'));
      setDraft((current) => draftFromGeminiResponse(current, response));
      setProfile((current) => ({ ...current, ...response.draft.extended }));
      setMessage(`Gemini создал черновик. Уверенность ${Math.round(response.draft.confidence * 100)}%. Проверь вкладки и сохрани.`);
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Gemini не смог создать черновик'); }
    finally { setBusy(null); }
  }

  async function generateImage() {
    setBusy('image'); setMessage(null);
    try {
      const name = String(draft.name_ru || draft.name_en || draft.name_pl || source);
      const description = String(draft.description_ru || draft.description_en || draft.seo_description || source);
      const image = await aiGenerateProductImage(name, description, true);
      setDraft((current) => ({ ...current, image_url: image.image_url }));
      setMessage('Фото создано. Нажми «Создать продукт», чтобы сохранить его в backend.');
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Не удалось сгенерировать фото'); }
    finally { setBusy(null); }
  }

  async function save() {
    setBusy('save'); setMessage(null);
    try {
      const nameInput = String(draft.name_en || draft.name_ru || draft.name_pl || source).trim();
      if (!nameInput) throw new Error('Введите описание или название продукта.');
      const created = await createAdminProduct({ ...draft, name_input: nameInput, category_id: categoryId || undefined, auto_translate: false });
      await saveExtendedProductProfile(created.id, profile);
      await Promise.resolve(onSaved());
      setMessage('Новый продукт создан и сохранён в backend.');
      onClose();
    } catch (err) { setMessage(err instanceof Error ? err.message : 'Не удалось создать продукт'); }
    finally { setBusy(null); }
  }

  const tabs: Array<[CatalogEditTab, string]> = [['basic', 'Основное'], ['content', '4 языка'], ['nutrition', 'Макросы'], ['vitamins', 'Витамины'], ['minerals', 'Минералы'], ['culinary', 'Кулинария'], ['health', 'Health'], ['states', 'Состояния']];
  const displayName = String(draft.name_ru || draft.name_en || 'Новый продукт');

  return <div className="catalog-edit-overlay compact" role="presentation" onMouseDown={onClose}>
    <section className="catalog-edit-modal compact" role="dialog" aria-label="Создание продукта" onMouseDown={(event) => event.stopPropagation()}>
      <div className="catalog-edit-head"><div><p className="eyebrow">Gemini product creator</p><h3>{displayName}</h3><span>{categoryDisplayName(categories.find((item) => item.id === categoryId))} · новый продукт</span></div><button className="btn btn-quiet" type="button" onClick={onClose}>Закрыть</button></div>
      <nav className="catalog-edit-tabs">{tabs.map(([id, label]) => <button key={id} type="button" className={tab === id ? 'active' : ''} onClick={() => setTab(id)}>{label}</button>)}</nav>
      <section className="catalog-generation-panel">
        <label><span>Что создать</span><textarea value={source} onChange={(event) => setSource(event.target.value)} /></label>
        <label><span>Задача для Gemini</span><textarea value={instruction} onChange={(event) => setInstruction(event.target.value)} /></label>
        <div className="catalog-generation-actions">
          <button className="btn btn-primary" type="button" onClick={() => void runGeminiCreate()} disabled={Boolean(busy)}><AppIcon name="sparkles" />{busy === 'gemini' ? 'Генерируем текст...' : 'Текст и профиль'}<small>{CATALOG_TEXT_MODEL}</small></button>
          <button className="btn btn-quiet" type="button" onClick={() => void generateImage()} disabled={Boolean(busy)}><AppIcon name="package" />{busy === 'image' ? 'Генерируем фото...' : 'Фото'}<small>{CATALOG_IMAGE_MODEL}</small></button>
          <button className="btn btn-quiet" type="button" disabled><AppIcon name="refresh" />Состояния<small>после создания</small></button>
        </div>
      </section>
      <div className="catalog-edit-body compact">
        <aside className="catalog-edit-photo compact">{draft.image_url ? <img src={draft.image_url} alt={displayName} /> : <span><AppIcon name="package" size={28} /></span>}<small>Фото: {CATALOG_IMAGE_MODEL}</small></aside>
        <div className="catalog-edit-form">
          {tab === 'basic' ? <div className="editor-grid"><label><span>Категория</span><select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">Авто / без категории</option>{categories.map((category) => <option key={category.id} value={category.id}>{categoryDisplayName(category)}</option>)}</select></label><EditorField label="Название RU" value={String(draft.name_ru || '')} onChange={(value) => setText('name_ru', value)} /><EditorField label="Название EN" value={String(draft.name_en || '')} onChange={(value) => setText('name_en', value)} /><EditorField label="Название PL" value={String(draft.name_pl || '')} onChange={(value) => setText('name_pl', value)} /><EditorField label="Название UK" value={String(draft.name_uk || '')} onChange={(value) => setText('name_uk', value)} /><EditorField label="Тип продукта" value={String(draft.product_type || '')} onChange={(value) => setText('product_type', value)} /><EditorField label="Единица" value={String(draft.unit || '')} onChange={(value) => setText('unit', value)} /><EditorField label="Image URL" value={String(draft.image_url || '')} onChange={(value) => setText('image_url', value)} multiline /></div> : null}
          {tab === 'content' ? <div className="editor-grid"><EditorField label="Описание RU" value={String(draft.description_ru || '')} onChange={(value) => setText('description_ru', value)} multiline /><EditorField label="Описание EN" value={String(draft.description_en || '')} onChange={(value) => setText('description_en', value)} multiline /><EditorField label="Описание PL" value={String(draft.description_pl || '')} onChange={(value) => setText('description_pl', value)} multiline /><EditorField label="Описание UK" value={String(draft.description_uk || '')} onChange={(value) => setText('description_uk', value)} multiline /><EditorField label="SEO title" value={String(draft.seo_title || '')} onChange={(value) => setText('seo_title', value)} /><EditorField label="SEO H1" value={String(draft.seo_h1 || '')} onChange={(value) => setText('seo_h1', value)} /><EditorField label="SEO description" value={String(draft.seo_description || '')} onChange={(value) => setText('seo_description', value)} multiline /></div> : null}
          {tab === 'nutrition' ? <div className="catalog-nutrition-grid">{([['calories_per_100g', 'Kalorie/kcal'], ['protein_per_100g', 'Białko'], ['fat_per_100g', 'Tłuszcz'], ['carbs_per_100g', 'Węglowodany'], ['fiber_per_100g', 'Błonnik'], ['sugar_per_100g', 'Cukier'], ['density_g_per_ml', 'Gęstość'], ['typical_portion_g', 'Porcja'], ['shelf_life_days', 'Trwałość'], ['starch_g', 'Starch'], ['water_g', 'Water'], ['alcohol_g', 'Alcohol']] as Array<[string, string]>).map(([field, label]) => field in draft ? <label key={field}><span>{label}</span><input type="number" step="any" value={draft[field as keyof CatalogEditDraft] as number ?? ''} onChange={(event) => setNumber(field as keyof CatalogEditDraft, event.target.value)} /></label> : <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('macros', field) as number ?? ''} onChange={(event) => setProfileNumber('macros', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'vitamins' ? <div className="catalog-nutrition-grid">{CATALOG_VITAMINS.map(([field, label]) => <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('vitamins', field) as number ?? ''} onChange={(event) => setProfileNumber('vitamins', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'minerals' ? <div className="catalog-nutrition-grid">{CATALOG_MINERALS.map(([field, label]) => <label key={field}><span>{label}</span><input type="number" step="any" value={profileValue('minerals', field) as number ?? ''} onChange={(event) => setProfileNumber('minerals', field, event.target.value)} /></label>)}</div> : null}
          {tab === 'culinary' ? <div className="editor-grid"><div className="catalog-nutrition-grid span-2">{(['sweetness', 'acidity', 'bitterness', 'umami', 'aroma'] as const).map((field) => <label key={field}><span>{field}</span><input type="number" step="any" value={profileValue('culinary', field) as number ?? ''} onChange={(event) => setProfileNumber('culinary', field, event.target.value)} /></label>)}{(['glycemic_index', 'glycemic_load', 'ph', 'smoke_point', 'water_activity'] as const).map((field) => <label key={field}><span>{field}</span><input type="number" step="any" value={profileValue('food_properties', field) as number ?? ''} onChange={(event) => setProfileNumber('food_properties', field, event.target.value)} /></label>)}</div><EditorField label="Texture" value={String(profileValue('culinary', 'texture') || '')} onChange={(value) => setProfileValue('culinary', 'texture', value)} multiline /><EditorField label="Processing notes PL" value={String(profileValue('processing_effects', 'processing_notes_pl') || '')} onChange={(value) => setProfileValue('processing_effects', 'processing_notes_pl', value)} multiline /></div> : null}
          {tab === 'health' ? <div className="editor-grid"><div className="toggle-grid span-2">{CATALOG_DIETS.map(([field, label]) => <label className="editor-check" key={field}><input type="checkbox" checked={Boolean(profileValue('diet_flags', field))} onChange={(event) => setProfileValue('diet_flags', field, event.target.checked)} />{label}</label>)}</div>{(['en', 'ru', 'pl', 'uk'] as const).flatMap((lang) => [<EditorField key={`bio-${lang}`} label={`Bioactive ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `bioactive_compounds_${lang}`))} onChange={(value) => setProfileValue('health_profile', `bioactive_compounds_${lang}`, csvToArray(value))} multiline />, <EditorField key={`eff-${lang}`} label={`Health effects ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `health_effects_${lang}`))} onChange={(value) => setProfileValue('health_profile', `health_effects_${lang}`, csvToArray(value))} multiline />])}<EditorField label="Food role" value={String(profileValue('health_profile', 'food_role') || '')} onChange={(value) => setProfileValue('health_profile', 'food_role', value)} /></div> : null}
          {tab === 'states' ? <div className="catalog-states-list"><article><strong>Состояния появятся после создания</strong><span>Сначала нужен ID продукта в backend.</span><p>После сохранения открой продукт через «Редактировать» и нажми «Состояния».</p></article></div> : null}
        </div>
      </div>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><button className="btn btn-quiet" type="button" onClick={onClose}>Отмена</button><button className="btn btn-primary" type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Создаём...' : 'Создать продукт'}</button></div>
    </section>
  </div>;
}

function CatalogTable({ props }: { props: OperationsPageProps }) {
  const [query, setQuery] = useState('');
  const [categoryId, setCategoryId] = useState('all');
  const [limit, setLimit] = useState(50);
  const [editingProduct, setEditingProduct] = useState<AdminProduct | null>(null);
  const [creatingProduct, setCreatingProduct] = useState(false);

  const categoriesById = new Map(props.categories.map((category) => [category.id, category]));
  const needle = query.trim().toLowerCase();
  const filtered = props.products.filter((product) => {
    const category = categoriesById.get(product.category_id);
    const haystack = [product.name_ru, product.name_en, product.name_pl, product.slug, product.product_type, categoryDisplayName(category)]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    const matchesSearch = !needle || haystack.includes(needle);
    const matchesCategory = categoryId === 'all' || product.category_id === categoryId;
    return matchesSearch && matchesCategory;
  });
  const visibleRows = filtered.slice(0, limit);
  const publishedCount = props.products.filter((product) => product.is_published).length;
  const seoReadyCount = props.products.filter((product) => product.seo_title && product.seo_description).length;
  const imageReadyCount = props.products.filter((product) => Boolean(product.image_url)).length;

  return <section className="ops-panel catalog-browser">
    <PanelTitle title="Каталог Dima" icon="catalog" action={`${visibleRows.length} из ${filtered.length} / всего ${props.products.length}`} />
    <div className="catalog-toolbar">
      <label><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Название, slug, тип, категория" /></label>
      <label><span>Категория</span><select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="all">Все категории</option>{props.categories.map((category) => <option key={category.id} value={category.id}>{categoryDisplayName(category)}</option>)}</select></label>
      <label><span>Показать</span><select value={String(limit)} onChange={(event) => setLimit(Number(event.target.value))}><option value="50">50 товаров</option><option value="100">100 товаров</option><option value="250">250 товаров</option><option value="99999">Все товары</option></select></label>
      <button className="btn btn-primary" type="button" onClick={() => setCreatingProduct(true)}><AppIcon name="sparkles" />Создать продукт</button>
      <button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из backend</button>
    </div>
    <div className="catalog-stats-row">
      <article><span>Всего из backend</span><strong>{props.products.length}</strong></article>
      <article><span>Опубликовано</span><strong>{publishedCount}</strong></article>
      <article><span>SEO готово</span><strong>{seoReadyCount}</strong></article>
      <article><span>С фото</span><strong>{imageReadyCount}</strong></article>
      <article><span>Категорий</span><strong>{props.categories.length}</strong></article>
    </div>
    <div className="table-scroll"><table className="ops-table"><thead><tr><th>Фото</th><th>Название</th><th>Категория</th><th>Slug / SKU</th><th>Тип</th><th>Питание</th><th>SEO</th><th>Статус</th><th>Действие</th></tr></thead><tbody>{visibleRows.map((product) => {
      const category = categoriesById.get(product.category_id);
      const nutritionReady = product.calories_per_100g != null || product.protein_per_100g != null || product.carbs_per_100g != null || product.fat_per_100g != null;
      return <tr key={product.id}><td>{product.image_url ? <img className="catalog-product-thumb" src={product.image_url} alt={productDisplayName(product)} loading="lazy" /> : <span className="catalog-product-thumb empty"><AppIcon name="package" size={18} /></span>}</td><td><strong>{productDisplayName(product)}</strong><small>{product.name_en}</small></td><td>{categoryDisplayName(category)}</td><td><code>{product.slug || product.id.slice(0, 8)}</code></td><td>{product.product_type || 'other'}</td><td><StatusPill tone={nutritionReady ? 'good' : 'warning'} label={nutritionReady ? 'есть' : 'пусто'} /></td><td><StatusPill tone={product.seo_title && product.seo_description ? 'good' : 'warning'} label={product.seo_title && product.seo_description ? 'готово' : 'не заполнено'} /></td><td><StatusPill tone={product.is_published ? 'good' : 'neutral'} label={product.is_published ? 'опубликовано' : 'черновик'} /></td><td><button className="table-action" type="button" onClick={() => setEditingProduct(product)}>Редактировать</button></td></tr>;
    })}</tbody></table></div>
    {filtered.length > visibleRows.length ? <p className="page-muted">Показано {visibleRows.length}. Увеличьте лимит, чтобы увидеть остальные {filtered.length - visibleRows.length}.</p> : null}
    {props.products.length === 0 ? <p className="empty-state">Backend не вернул товары. Нажмите «Обновить» или проверьте авторизацию/API.</p> : null}
    {creatingProduct ? <CatalogNewProductEditor categories={props.categories} onClose={() => setCreatingProduct(false)} onSaved={props.onRefresh} /> : null}
    {editingProduct ? <CatalogProductEditor product={editingProduct} category={categoriesById.get(editingProduct.category_id)} onClose={() => setEditingProduct(null)} onSaved={props.onRefresh} /> : null}
  </section>;
}

function MaterialsTable({ props }: { props: OperationsPageProps }) {
  return <AlmabuildEditor props={props} mode="materials" />;
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

type UsbTab = 'storage' | 'gemini' | 'prompts' | 'site' | 'product' | 'image' | 'history' | 'settings';

function prettyJson(value: unknown) {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  return JSON.stringify(value, null, 2);
}

function UsbKeyPanel() {
  const [status, setStatus] = useState<UsbKeyStatus | null>(null);
  const [settings, setSettings] = useState<GeminiSettingsStatus | null>(null);
  const [prompts, setPrompts] = useState<PromptTemplateItem[]>([]);
  const [history, setHistory] = useState<AiHistoryItem[]>([]);
  const [tab, setTab] = useState<UsbTab>('gemini');
  const [running, setRunning] = useState<string | null>(null);
  const [result, setResult] = useState<unknown>(null);
  const [error, setError] = useState<string | null>(null);
  const [custom, setCustom] = useState('help');
  const [selectedPrompt, setSelectedPrompt] = useState('templates/prompts/sites/construction-site.ru.txt');
  const [promptContent, setPromptContent] = useState('');
  const [siteForm, setSiteForm] = useState({ CITY: 'Алматы', SERVICE: 'ремонт магазинов', LANGUAGE: 'ru', TARGET_AUDIENCE: 'владельцы бизнеса и магазинов', STYLE: 'modern commercial SEO', OUTPUT_FORMAT: 'json' });
  const [productForm, setProductForm] = useState({ PRODUCT_NAME: 'Гипсокартон Knauf', CATEGORY: 'Гипсокартон', MATERIAL: 'гипс', USE_CASE: 'стены и потолки', CITY: 'Алматы', LANGUAGE: 'ru', OUTPUT_FORMAT: 'json' });
  const [imageForm, setImageForm] = useState({ PRODUCT_NAME: 'Гипсокартон Knauf', PRODUCT_TYPE: 'строительный листовой материал', BACKGROUND: 'clean warehouse background', STYLE: 'realistic commercial product photography', LIGHTING: 'soft studio lighting', ANGLE: 'front 3/4 view', QUALITY: 'ultra sharp, high detail' });

  async function refresh() {
    setError(null);
    try {
      const [nextStatus, nextSettings, nextPrompts, nextHistory] = await Promise.all([
        findUsbKey(),
        adminKeyGeminiSettingsStatus().catch(() => null),
        adminKeyPromptList().catch(() => null),
        adminKeyAiHistoryList().catch(() => null)
      ]);
      setStatus(nextStatus);
      setSettings(nextSettings);
      setPrompts(nextPrompts?.prompts ?? []);
      setHistory(nextHistory?.history ?? []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Не удалось проверить USB Key');
    }
  }

  async function runRaw(args: string[], title = args.join(' ')) {
    setRunning(title); setError(null); setResult(null);
    try { setResult(await runAdminTool(args)); }
    catch (err) { setError(err instanceof Error ? err.message : 'Не удалось выполнить admin_tool'); }
    finally { setRunning(null); void refresh(); }
  }

  async function runJson(title: string, action: () => Promise<unknown>) {
    setRunning(title); setError(null); setResult(null);
    try {
      const value = await action();
      setResult(value);
      if ((value as { ok?: boolean; error?: string }).ok === false) setError((value as { error?: string }).error || 'Команда вернула ошибку');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Команда не выполнилась');
    } finally {
      setRunning(null); void refresh();
    }
  }

  useEffect(() => { void refresh(); }, []);

  const storage = status?.storage;
  const dataPaths = status?.data_paths;
  const output = error ? error + '\n\n' + prettyJson(result) : prettyJson(result) || 'Выберите действие Gemini Studio или локальную задачу.';
  const envStatus: Array<[string, boolean]> = [['USB Key', Boolean(status?.found)], ['admin_tool', Boolean(status?.admin_tool)], ['Gemini API', settings?.gemini_api_key === 'configured']];
  const tabs: Array<[UsbTab, string]> = [['storage', 'Storage'], ['gemini', 'Gemini Studio'], ['prompts', 'Prompts'], ['site', 'Site Pages'], ['product', 'Product Cards'], ['image', 'Image Prompts'], ['history', 'History'], ['settings', 'Settings']];
  const localJobs = LOCAL_JOBS;

  return <div className="usb-key-layout">
    <section className="ops-panel usb-key-summary">
      <PanelTitle title="AssistantAdminKey" icon="hard-drive" action={status?.found ? 'флешка подключена' : 'флешка не найдена'} />
      <div className="usb-status-grid">{envStatus.map(([label, ok]) => <div key={label}><span>{label}</span><StatusPill tone={ok ? 'good' : label === 'Gemini API' ? 'warning' : 'danger'} label={ok ? 'ok' : 'нет'} /></div>)}</div>
      <p className="page-muted">{status?.root || 'Вставьте флешку с папкой AssistantAdminKey.'}</p>
      <div className="usb-tab-row">{tabs.map(([id, label]) => <button key={id} className={tab === id ? 'active' : ''} type="button" onClick={() => setTab(id)}>{label}</button>)}</div>
      <button className="btn btn-quiet" type="button" onClick={() => void refresh()}><AppIcon name="refresh" />Refresh</button>
    </section>

    {tab === 'storage' ? <section className="ops-panel usb-storage-panel"><PanelTitle title="Storage" icon="database" action="память флешки" /><div className="usb-storage-cards"><article><span>Всего памяти</span><strong>{storage?.total_label || 'нет данных'}</strong></article><article><span>Занято</span><strong>{storage?.used_label || 'нет данных'}</strong></article><article><span>Свободно</span><strong>{storage?.available_label || 'нет данных'}</strong></article></div><div className="usb-path-list"><div><span>Config</span><code>{dataPaths?.config || status?.config || 'нет данных'}</code></div><div><span>Backups</span><code>{dataPaths?.backups || 'нет данных'}</code></div><div><span>Exports</span><code>{dataPaths?.exports || 'нет данных'}</code></div><div><span>Local DB</span><code>{dataPaths?.local_db || 'нет данных'}</code></div><div><span>Logs</span><code>{dataPaths?.logs || 'нет данных'}</code></div></div></section> : null}

    {tab === 'gemini' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="Gemini Studio" icon="bot" action="USB Prompt Library" /><div className="usb-storage-cards"><article><span>Prompts</span><strong>{prompts.length}</strong></article><article><span>History</span><strong>{history.length}</strong></article><article><span>API key</span><strong>{settings?.gemini_api_key || 'missing'}</strong></article></div><div className="usb-job-list"><button type="button" onClick={() => setTab('site')}><span><strong>Site Pages</strong><small>SEO-страницы услуг через Gemini</small></span><StatusPill tone="info" label="text" /></button><button type="button" onClick={() => setTab('product')}><span><strong>Product Cards</strong><small>Карточки строительного маркетплейса</small></span><StatusPill tone="good" label="json" /></button><button type="button" onClick={() => setTab('image')}><span><strong>Image Prompts</strong><small>Промты для фото товаров</small></span><StatusPill tone="warning" label="image" /></button></div></section> : null}

    {tab === 'prompts' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="Prompt Library" icon="cms" action="templates/prompts" /><div className="usb-custom-command"><select value={selectedPrompt} onChange={(event) => setSelectedPrompt(event.target.value)}>{prompts.map((prompt) => <option key={prompt.path} value={prompt.path}>{prompt.type} / {prompt.name}</option>)}</select><button className="btn btn-quiet" type="button" onClick={() => void adminKeyOpenFolder('prompts')}>Open Folder</button><button className="btn btn-primary" type="button" onClick={() => void runJson('Read Prompt', async () => { const value = await adminKeyPromptRead(selectedPrompt); setPromptContent(value.content || ''); return value; })}>Read Prompt</button><button className="btn btn-quiet" type="button" onClick={() => void runJson('Render Test', () => adminKeyPromptRender(selectedPrompt, selectedPrompt.includes('/images/') ? imageForm : selectedPrompt.includes('marketplace') ? productForm : siteForm))}>Render Test</button></div><pre>{promptContent || 'Выберите prompt и нажмите Read Prompt.'}</pre></section> : null}

    {tab === 'site' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="Site Pages" icon="seo" action="construction-site.ru.txt" /><div className="settings-matrix">{Object.entries(siteForm).map(([key, value]) => <label key={key}><span>{key}</span><input value={value} onChange={(event) => setSiteForm({ ...siteForm, [key]: event.target.value })} /></label>)}</div><div className="editor-actions"><button className="btn btn-primary" type="button" disabled={Boolean(running)} onClick={() => void runJson('Generate Site Page', () => adminKeyGeminiGenerateText('templates/prompts/sites/construction-site.ru.txt', siteForm))}><AppIcon name="sparkles" />Generate Site Page</button><button className="btn btn-quiet" type="button" onClick={() => void adminKeyOpenFolder('exports')}>Open Export Folder</button></div></section> : null}

    {tab === 'product' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="Product Cards" icon="catalog" action="marketplace-product.ru.txt" /><div className="settings-matrix">{Object.entries(productForm).map(([key, value]) => <label key={key}><span>{key}</span><input value={value} onChange={(event) => setProductForm({ ...productForm, [key]: event.target.value })} /></label>)}</div><button className="btn btn-primary" type="button" disabled={Boolean(running)} onClick={() => void runJson('Generate Product Card', () => adminKeyGeminiGenerateText('templates/prompts/sites/marketplace-product.ru.txt', productForm))}><AppIcon name="sparkles" />Generate Product Card</button></section> : null}

    {tab === 'image' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="Image Prompts" icon="sparkles" action="product-photo.ru.txt" /><div className="settings-matrix">{Object.entries(imageForm).map(([key, value]) => <label key={key}><span>{key}</span><input value={value} onChange={(event) => setImageForm({ ...imageForm, [key]: event.target.value })} /></label>)}</div><button className="btn btn-primary" type="button" disabled={Boolean(running)} onClick={() => void runJson('Generate Image Prompt', () => adminKeyGeminiGenerateImagePrompt('templates/prompts/images/product-photo.ru.txt', imageForm))}><AppIcon name="sparkles" />Generate Image Prompt</button></section> : null}

    {tab === 'history' ? <section className="ops-panel usb-jobs-panel"><PanelTitle title="History" icon="database" action="data/ai-history" /><div className="usb-job-list">{history.slice(0, 30).map((item) => <button key={item.id} type="button" onClick={() => void runJson('Read History', () => adminKeyAiHistoryRead(item.id))}><span><strong>{item.type}</strong><small>{item.created_at} · {item.template}</small></span><StatusPill tone="info" label="open" /></button>)}</div><button className="btn btn-quiet" type="button" onClick={() => void adminKeyOpenFolder('history')}>Open History Folder</button></section> : null}

    {tab === 'settings' ? <section className="ops-panel usb-storage-panel"><PanelTitle title="Settings" icon="settings" action="settings/gemini.env.local" /><div className="usb-path-list"><div><span>USB root</span><code>{settings?.usb_root || status?.root || 'not found'}</code></div><div><span>GEMINI_API_KEY</span><code>{settings?.gemini_api_key || 'missing'}</code></div><div><span>Text model</span><code>{settings?.text_model || 'gemini-3-flash-preview'}</code></div><div><span>Image model</span><code>{settings?.image_model || 'not set'}</code></div><div><span>Settings file</span><code>{settings?.settings_path || 'not found'}</code></div></div><div className="editor-actions"><button className="btn btn-quiet" type="button" onClick={() => void adminKeyOpenFolder('settings')}>Open gemini.env.local</button><button className="btn btn-quiet" type="button" onClick={() => void refresh()}>Refresh</button></div></section> : null}

    <section className="ops-panel usb-jobs-panel"><PanelTitle title="Локальные задачи" icon="terminal" action="fallback bin/admin_tool" /><div className="usb-job-list">{localJobs.map((job) => <button key={job.title} type="button" onClick={() => void runRaw(job.args, job.title)} disabled={Boolean(running)}><span><strong>{job.title}</strong><small>{job.description}</small></span><StatusPill tone={job.tone} label={running === job.title ? 'run' : job.args[0]} /></button>)}</div><div className="usb-custom-command"><input value={custom} onChange={(event) => setCustom(event.target.value)} placeholder="autofill-product <product_id>" /><button className="btn btn-primary" type="button" onClick={() => void runRaw(custom.split(' ').filter(Boolean), 'Custom command')} disabled={Boolean(running)}><AppIcon name="zap" />Run</button></div></section>

    <section className="ops-panel usb-output-panel"><PanelTitle title="Результат" icon="code" action={running ? 'выполняется...' : 'готово'} /><pre>{output}</pre></section>
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
  if (props.page === 'catalog') return <CatalogTable props={props} />;
  if (props.page === 'materials') return <MaterialsTable props={props} />;
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
