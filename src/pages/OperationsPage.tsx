import { type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import * as echarts from 'echarts';
import { AppIcon, type AppIconName } from '../components/AppIcon';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Input } from '../components/ui/input';
import { ScrollArea } from '../components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs';
import { Textarea } from '../components/ui/textarea';
import type { AppPage, ManagedSite } from '../components/Sidebar';
import { aiEditAlmabuildItem, generateAlmabuildMaterialsFromPhoto, saveAlmabuildContent, type AlmabuildContent, type AlmabuildLead, type Kit, type MaterialCategory, type Product, type Project } from '../api/almabuild';
import { aiCreateProductDraft, aiGenerateProductImage, createAdminProduct, generateProductStates, getAdminNutritionProduct, listProductStates, saveExtendedProductProfile, updateAdminProduct, type AiExtendedProductProfile, type CreateAdminProductRequest, type IngredientState } from '../api/catalog';
import { adminKeyAiHistoryList, adminKeyAiHistoryRead, adminKeyGeminiGenerateImagePrompt, adminKeyGeminiGenerateText, adminKeyGeminiSettingsStatus, adminKeyOpenFolder, adminKeyPromptList, adminKeyPromptRead, adminKeyPromptRender, findUsbKey, runAdminTool, type AdminToolOutput, type AiHistoryItem, type GeminiSettingsStatus, type PromptTemplateItem, type UsbKeyStatus } from '../api/localAdmin';
import { aiCreateArticleDraft, aiGenerateArticleImage, createArticle, updateArticle } from '../api/cms';
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
  almabuildLeads: AlmabuildLead[];
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

function isToday(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return false;
  const today = new Date();
  return date.getFullYear() === today.getFullYear()
    && date.getMonth() === today.getMonth()
    && date.getDate() === today.getDate();
}

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
      backend: 'Koyeb /api/admin/almabuild + /public/almabuild',
      languages: 'RU / KK / EN',
      ga4: false,
      searchConsole: false,
      categories: content?.materialCategories.length ?? 0,
      products: content?.products.length ?? 0,
      projects: content?.projects.length ?? 0,
      seoPages: content?.kits.length ?? 0,
      leadsToday: props.almabuildLeads.filter((lead) => isToday(lead.createdAt)).length,
      leadsMonth: props.almabuildLeads.length,
      activeObjects: content?.projects.length ?? 0,
      deployLabel: 'нет API деплоев'
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
    leadsToday: 0,
    leadsMonth: 0,
    activeObjects: props.stats?.total_restaurants ?? 0,
    deployLabel: 'нет API деплоев'
  };
}

function metric(title: string, value: string | number, change: string, tone: Tone, icon: AppIconName) {
  return { title, value: String(value), change, tone, icon };
}

function TrafficChart({ analytics, searchConsole }: { analytics: AnalyticsOverview | null; searchConsole: SearchConsoleBundle | null }) {
  const ref = useRef<HTMLDivElement | null>(null);
  const analyticsDaily = useMemo(() => analytics?.daily ?? [], [analytics]);
  const searchDaily = useMemo(() => searchConsole?.daily ?? [], [searchConsole]);
  const connected = Boolean(analytics?.configured && analyticsDaily.length) || Boolean(searchConsole?.overview?.configured && searchDaily.length);
  useEffect(() => {
    if (!ref.current || !connected) return;
    const chart = echarts.init(ref.current, 'dark');
    const labels = (analyticsDaily.length ? analyticsDaily : searchDaily).slice(-14).map((row) => row.date);
    const usersByDate = new Map(analyticsDaily.map((row) => [row.date, row.active_users]));
    const clicksByDate = new Map(searchDaily.map((row) => [row.date, row.clicks]));
    chart.setOption({
      backgroundColor: 'transparent',
      grid: { top: 18, right: 12, bottom: 24, left: 34 },
      tooltip: { trigger: 'axis', backgroundColor: '#18181b', borderColor: '#27272a', textStyle: { color: '#fafafa' } },
      xAxis: { type: 'category', boundaryGap: false, data: labels, axisLine: { lineStyle: { color: '#27272a' } }, axisLabel: { color: '#a1a1aa' } },
      yAxis: { type: 'value', splitLine: { lineStyle: { color: '#27272a' } }, axisLabel: { color: '#a1a1aa' } },
      series: [
        { name: 'Пользователи GA4', type: 'line', smooth: true, symbol: 'none', lineStyle: { width: 3, color: '#ff6a2a' }, areaStyle: { color: 'rgba(255,106,42,.13)' }, data: labels.map((date) => usersByDate.get(date) ?? 0) },
        { name: 'Клики GSC', type: 'line', smooth: true, symbol: 'none', lineStyle: { width: 2, color: '#3b82f6' }, data: labels.map((date) => clicksByDate.get(date) ?? 0) }
      ]
    });
    const resize = () => chart.resize();
    window.addEventListener('resize', resize);
    return () => { window.removeEventListener('resize', resize); chart.dispose(); };
  }, [connected, analyticsDaily, searchDaily]);

  if (!connected) return <div className="ops-chart empty-analytics"><AppIcon name="analytics" size={28} /><strong>Нет live-графика</strong><span>Backend не вернул daily данные GA4/Search Console для выбранного сайта.</span></div>;
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
    metric('Активные объекты', data.activeObjects, 'получено из backend/admin данных', data.activeObjects ? 'info' : 'neutral', 'folder'),
    metric('SEO-страницы', data.seoPages, 'получено из CMS/Kazaxbud content', data.seoPages ? 'info' : 'neutral', 'seo'),
    metric('Товары / карточки', data.products, 'получено из API выбранного сайта', data.products ? 'good' : 'neutral', 'catalog'),
    metric('Материалы / категории', data.categories, 'получено из API выбранного сайта', data.categories ? 'info' : 'neutral', 'materials'),
    metric('Пользователи GA4', ga4Users, props.analytics?.configured ? 'получено из Analytics API' : 'нет данных Analytics API', props.analytics?.configured ? 'info' : 'warning', 'analytics'),
    metric('Клики GSC', gscClicks, props.searchConsole?.overview?.configured ? 'получено из Search Console API' : 'нет данных Search Console API', props.searchConsole?.overview?.configured ? 'info' : 'warning', 'activity')
  ];

  return <><div className="kpi-grid">{cards.map((card) => <KpiCard key={card.title} {...card} />)}</div><div className="ops-grid two-one"><section className="ops-panel wide"><PanelTitle title="Трафик и аналитика" icon="analytics" action={props.analytics?.configured || props.searchConsole?.overview?.configured ? 'данные из API' : 'нет API-данных'} /><TrafficChart analytics={props.analytics} searchConsole={props.searchConsole} /></section><section className="ops-panel"><PanelTitle title="Подключения backend" icon="terminal" action={data.name} /><QueueList data={data} /></section></div></>;
}

function QueueList({ data }: { data: SiteDataset }) {
  const rows: Array<[string, string, Tone]> = [
    ['Область данных', data.name, 'info'],
    ['Аналитика', data.ga4 ? 'подключено' : 'не подключено', data.ga4 ? 'good' : 'warning'],
    ['Search Console', data.searchConsole ? 'подключено' : 'не подключено', data.searchConsole ? 'good' : 'warning'],
    ['Деплой Cloudflare', 'нет API', 'warning']
  ];
  return <div className="ops-list">{rows.map(([name, value, tone]) => <div key={name}><span>{name}</span><StatusPill tone={tone} label={value} /></div>)}</div>;
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

function clampMaterialCount(value: number) {
  return Math.min(12, Math.max(1, value));
}

function withUniqueMaterialSlugs(existing: MaterialCategory[], incoming: MaterialCategory[]) {
  const used = new Set(existing.map((item) => item.slug).filter(Boolean));
  return incoming.map((item, itemIndex) => {
    const base = (item.slug || `material-${existing.length + itemIndex + 1}`).trim() || `material-${existing.length + itemIndex + 1}`;
    let slug = base;
    let suffix = 2;
    while (used.has(slug)) {
      slug = `${base}-${suffix}`;
      suffix += 1;
    }
    used.add(slug);
    return { ...item, slug, index: item.index || `[0:${existing.length + itemIndex + 1}]` };
  });
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

function KazaxbudMaterialEditorModal({ item, index, content, onClose, onRefresh, visionPanel }: { item?: MaterialCategory; index?: number; content: AlmabuildContent; onClose: () => void; onRefresh: () => void; visionPanel?: ReactNode }) {
  const [draft, setDraft] = useState<MaterialCategory>(() => item || {
    index: '',
    slug: '',
    title: '',
    text: '',
    bullets: [],
    photo: ''
  });
  const [instruction, setInstruction] = useState('Улучши карточку материала для сайта строительных материалов. Сделай описание конкретным, продающим и полезным для закупщика. Пункты короткие.');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(item ? 'Материал загружен из Kazaxbud content.' : 'Заполни материал и сохрани в backend.');
  const editing = typeof index === 'number';

  async function runGemini() {
    setBusy('gemini'); setMessage(null);
    try {
      setDraft(await aiEditAlmabuildItem<MaterialCategory>('material', instruction, draft));
      setMessage('Gemini обновил карточку. Проверь поля и сохрани.');
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить карточку'); }
    finally { setBusy(null); }
  }

  async function save() {
    setBusy('save'); setMessage(null);
    try {
      const nextRows = editing
        ? content.materialCategories.map((row, rowIndex) => rowIndex === index ? draft : row)
        : [...content.materialCategories, draft];
      await saveAlmabuildContent({ ...content, materialCategories: nextRows });
      await onRefresh();
      onClose();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось сохранить материал'); }
    finally { setBusy(null); }
  }

  async function remove() {
    if (!editing) return;
    setBusy('remove'); setMessage(null);
    try {
      await saveAlmabuildContent({ ...content, materialCategories: content.materialCategories.filter((_, rowIndex) => rowIndex !== index) });
      await onRefresh();
      onClose();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось удалить материал'); }
    finally { setBusy(null); }
  }

  return <div className="catalog-edit-overlay compact" role="presentation" onMouseDown={onClose}>
    <section className="catalog-edit-modal compact" role="dialog" aria-label="Редактирование материала Kazaxbud" onMouseDown={(event) => event.stopPropagation()}>
      <div className="catalog-edit-head"><div><p className="eyebrow">Kazaxbud material editor</p><h3>{draft.title || 'Новый материал'}</h3><span>{draft.slug || 'new-material'} · {draft.index || '[0:0]'}</span></div><Button variant="secondary" type="button" onClick={onClose}>Закрыть</Button></div>
      <div className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)]">
        <section className="grid gap-3 border-b border-zinc-800 p-4 xl:grid-cols-[minmax(0,1fr)_220px]">
          <SeoEditorField label="Задача для Gemini" value={instruction} onChange={setInstruction} multiline textareaClassName="min-h-16 max-h-28" />
          <Button className="grid min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start self-end bg-orange-500 text-black hover:bg-orange-400" type="button" onClick={() => void runGemini()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="sparkles" /></span>{busy === 'gemini' ? 'Улучшаем...' : 'Улучшить Gemini'}<small className="text-xs opacity-70">backend Kazaxbud</small></Button>
        </section>
        <div className={`grid min-h-0 gap-4 overflow-auto p-4 ${visionPanel && !editing ? 'lg:grid-cols-[220px_minmax(0,1fr)] 2xl:grid-cols-[220px_minmax(0,1fr)_minmax(360px,460px)]' : 'lg:grid-cols-[240px_minmax(0,1fr)]'}`}>
          <aside className="grid content-start gap-3">
            <Card className="bg-zinc-950">
              <CardContent className="grid aspect-square place-items-center p-4 text-center">
                <div className="grid gap-3 justify-items-center text-zinc-400">{draft.imageUrl ? <img className="h-full max-h-44 w-full rounded object-cover" src={draft.imageUrl} alt={draft.title || 'Материал'} /> : <AppIcon name="materials" size={36} />}<strong className="text-zinc-100">{draft.photo || 'visual class'}</strong><small>{draft.slug || 'slug'}</small></div>
              </CardContent>
            </Card>
            <Card className="bg-zinc-950"><CardContent className="grid gap-2 p-3 text-xs text-zinc-400"><strong className="text-sm text-zinc-100">{editing ? 'Редактирование' : 'Новый материал'}</strong><span>{draft.bullets.length} пунктов внутри карточки</span></CardContent></Card>
          </aside>
          <Card className="bg-zinc-950/60">
            <CardHeader><CardTitle>Карточка материала</CardTitle></CardHeader>
            <CardContent className="grid gap-4 xl:grid-cols-2">
              <SeoEditorField label="Название карточки" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} />
              <SeoEditorField label="URL slug" value={draft.slug} onChange={(slug) => setDraft({ ...draft, slug })} />
              <SeoEditorField label="Индекс" value={draft.index} onChange={(nextIndex) => setDraft({ ...draft, index: nextIndex })} />
              <SeoEditorField label="Визуальный класс" value={draft.photo} onChange={(photo) => setDraft({ ...draft, photo })} />
              <SeoEditorField label="Cloudflare R2 imageUrl" value={draft.imageUrl || ''} onChange={(imageUrl) => setDraft({ ...draft, imageUrl })} />
              <SeoEditorField label="Описание" value={draft.text} onChange={(text) => setDraft({ ...draft, text })} multiline textareaClassName="min-h-[190px]" />
              <SeoEditorField label="Пункты внутри карточки, каждый с новой строки" value={draft.bullets.join('\n')} onChange={(bullets) => setDraft({ ...draft, bullets: splitLines(bullets) })} multiline textareaClassName="min-h-[190px]" />
            </CardContent>
          </Card>
          {!editing ? visionPanel : null}
        </div>
      </div>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><Button variant="secondary" type="button" onClick={onClose}>Отмена</Button>{editing ? <Button variant="destructive" type="button" onClick={() => void remove()} disabled={Boolean(busy)}>Удалить</Button> : null}<Button type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : editing ? 'Сохранить материал' : 'Создать материал'}</Button></div>
    </section>
  </div>;
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

function KazaxbudSeoKitEditorModal({ item, index, content, onClose, onRefresh }: { item?: Kit; index?: number; content: AlmabuildContent; onClose: () => void; onRefresh: () => void }) {
  const [draft, setDraft] = useState<Kit>(() => item || {
    title: '',
    text: '',
    items: []
  });
  const [instruction, setInstruction] = useState('Улучши SEO-комплект материалов для Kazaxbud: название понятное, описание конкретное для закупщика, состав короткий и практичный.');
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(item ? 'SEO-комплект загружен из Kazaxbud content.' : 'Создай SEO-комплект и сохрани в backend.');
  const editing = typeof index === 'number';

  async function runGemini() {
    setBusy('gemini'); setMessage(null);
    try {
      setDraft(await aiEditAlmabuildItem<Kit>('kit', instruction, draft));
      setMessage('Gemini обновил SEO-комплект. Проверь поля и сохрани.');
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Gemini не смог обновить SEO-комплект'); }
    finally { setBusy(null); }
  }

  async function save() {
    setBusy('save'); setMessage(null);
    try {
      const nextRows = editing
        ? content.kits.map((row, rowIndex) => rowIndex === index ? draft : row)
        : [...content.kits, draft];
      await saveAlmabuildContent({ ...content, kits: nextRows });
      await onRefresh();
      onClose();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось сохранить SEO-комплект'); }
    finally { setBusy(null); }
  }

  async function remove() {
    if (!editing) return;
    setBusy('remove'); setMessage(null);
    try {
      await saveAlmabuildContent({ ...content, kits: content.kits.filter((_, rowIndex) => rowIndex !== index) });
      await onRefresh();
      onClose();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось удалить SEO-комплект'); }
    finally { setBusy(null); }
  }

  return <div className="catalog-edit-overlay compact" role="presentation" onMouseDown={onClose}>
    <section className="catalog-edit-modal compact" role="dialog" aria-label="Редактирование SEO Kazaxbud" onMouseDown={(event) => event.stopPropagation()}>
      <div className="catalog-edit-head"><div><p className="eyebrow">Kazaxbud SEO editor</p><h3>{draft.title || 'Новый SEO-комплект'}</h3><span>{draft.items.length} позиций в составе</span></div><Button variant="secondary" type="button" onClick={onClose}>Закрыть</Button></div>
      <div className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)]">
        <section className="grid gap-3 border-b border-zinc-800 p-4 xl:grid-cols-[minmax(0,1fr)_220px]">
          <SeoEditorField label="Задача для Gemini" value={instruction} onChange={setInstruction} multiline textareaClassName="min-h-16 max-h-28" />
          <Button className="grid min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start self-end bg-orange-500 text-black hover:bg-orange-400" type="button" onClick={() => void runGemini()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="sparkles" /></span>{busy === 'gemini' ? 'Улучшаем...' : 'Улучшить Gemini'}<small className="text-xs opacity-70">SEO Kazaxbud</small></Button>
        </section>
        <div className="grid min-h-0 gap-4 overflow-auto p-4 lg:grid-cols-[240px_minmax(0,1fr)]">
          <aside className="grid content-start gap-3">
            <Card className="bg-zinc-950"><CardContent className="grid aspect-square place-items-center p-4 text-center"><div className="grid gap-3 justify-items-center text-zinc-400"><AppIcon name="seo" size={36} /><strong className="text-zinc-100">SEO блок</strong><small>{draft.items.length} позиций</small></div></CardContent></Card>
            <Card className="bg-zinc-950"><CardContent className="grid gap-2 p-3 text-xs text-zinc-400"><strong className="text-sm text-zinc-100">{editing ? 'Редактирование' : 'Новый SEO-комплект'}</strong><span>{draft.text ? 'Описание заполнено' : 'Описание пустое'}</span></CardContent></Card>
          </aside>
          <Card className="bg-zinc-950/60">
            <CardHeader><CardTitle>SEO-комплект</CardTitle></CardHeader>
            <CardContent className="grid gap-4 xl:grid-cols-2">
              <SeoEditorField label="Название комплекта" value={draft.title} onChange={(title) => setDraft({ ...draft, title })} />
              <SeoEditorField label="Описание" value={draft.text} onChange={(text) => setDraft({ ...draft, text })} multiline textareaClassName="min-h-[220px]" />
              <SeoEditorField label="Состав, каждый пункт с новой строки" value={draft.items.join('\n')} onChange={(items) => setDraft({ ...draft, items: splitLines(items) })} multiline className="xl:col-span-2" textareaClassName="min-h-[220px]" />
            </CardContent>
          </Card>
        </div>
      </div>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><Button variant="secondary" type="button" onClick={onClose}>Отмена</Button>{editing ? <Button variant="destructive" type="button" onClick={() => void remove()} disabled={Boolean(busy)}>Удалить</Button> : null}<Button type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : editing ? 'Сохранить SEO' : 'Создать SEO'}</Button></div>
    </section>
  </div>;
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
    if (mode === 'materials') await saveAlmabuildDraft({ ...current, materialCategories: [...current.materialCategories, { index: '', slug: '', title: '', text: '', bullets: [], photo: '' }] }, props.onRefresh, setMessage);
    if (mode === 'products') await saveAlmabuildDraft({ ...current, products: [...current.products, { title: '', category: '', categorySlug: '', spec: '', photo: '' }] }, props.onRefresh, setMessage);
    if (mode === 'kits') await saveAlmabuildDraft({ ...current, kits: [...current.kits, { title: '', text: '', items: [] }] }, props.onRefresh, setMessage);
    if (mode === 'projects') await saveAlmabuildDraft({ ...current, projects: [...current.projects, { title: '', meta: '', photo: '' }] }, props.onRefresh, setMessage);
  }
  const title = mode === 'materials' ? 'Карточки материалов Kazaxbud' : mode === 'products' ? 'Товарные карточки Kazaxbud' : mode === 'kits' ? 'Комплекты Kazaxbud' : 'Проектные карточки Kazaxbud';
  const addLabel = mode === 'materials' ? 'Добавить материал' : mode === 'products' ? 'Добавить товар' : mode === 'kits' ? 'Добавить комплект' : 'Добавить проект';
  return <section className="ops-panel"><PanelTitle title={title} icon="cms" action="редактируется через Gemini и backend Kazaxbud" /><div className="editor-toolbar"><button className="btn btn-primary" type="button" onClick={addItem}>{addLabel}</button><EditorMessage value={message} /></div><div className="editor-list">{mode === 'materials' && content.materialCategories.map((item, index) => <MaterialCardEditor key={item.slug + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'products' && content.products.map((item, index) => <ProductCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'kits' && content.kits.map((item, index) => <KitCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}{mode === 'projects' && content.projects.map((item, index) => <ProjectCardEditor key={item.title + index} item={item} index={index} content={content} onRefresh={props.onRefresh} />)}</div></section>;
}

type SeoArticleDraft = Partial<Omit<CmsArticle, 'id' | 'updated_at' | 'created_at'>>;
type SeoArticleTab = 'content' | 'seo' | 'languages' | 'media';
type SeoArticleLang = 'ru' | 'en' | 'pl' | 'uk';

function extractArticleImages(article?: CmsArticle): string[] {
  if (!article) return [];
  const storySource = article.content_ru || article.content_en || article.content_pl || article.content_uk || '';
  const markdownImages = Array.from(String(storySource || '').matchAll(/!\[[^\]]*\]\((https?:\/\/[^)\s]+)\)/g)).map((match) => match[1]);
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

function SeoEditorField({ label, value, onChange, multiline = false, className = '', textareaClassName = '' }: { label: string; value: string; onChange: (value: string) => void; multiline?: boolean; className?: string; textareaClassName?: string }) {
  return <label className={`grid min-w-0 gap-2 text-sm font-black text-zinc-400 ${className}`}>
    <span>{label}</span>
    {multiline
      ? <Textarea className={`min-h-40 resize-y font-semibold leading-relaxed ${textareaClassName}`} value={value} onChange={(event) => onChange(event.target.value)} />
      : <Input className="font-semibold" value={value} onChange={(event) => onChange(event.target.value)} />}
  </label>;
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
  const [language, setLanguage] = useState<SeoArticleLang>('ru');
  const [busy, setBusy] = useState<string | null>(null);
  const [fullscreenImage, setFullscreenImage] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(article ? 'Страница загружена из CMS backend.' : 'Опиши тему и нажми «Текст и SEO».');
  const tabs: Array<[SeoArticleTab, string]> = [['content', 'Контент'], ['seo', 'SEO'], ['languages', '4 языка'], ['media', 'Медиа']];
  const langTabs: Array<[SeoArticleLang, string]> = [['ru', 'RU'], ['en', 'EN'], ['pl', 'PL'], ['uk', 'UK']];
  const titleField = `title_${language}` as keyof SeoArticleDraft;
  const contentField = `content_${language}` as keyof SeoArticleDraft;
  const seoTitleField = `seo_title_${language}` as keyof SeoArticleDraft;
  const seoDescriptionField = `seo_description_${language}` as keyof SeoArticleDraft;

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

  async function generateArticleImage(index: number) {
    setBusy(`image-${index}`); setMessage(null);
    try {
      const title = String(draft.title_en || draft.title_ru || articleDisplayTitle(draft));
      const prompt = [articleDisplayTitle(draft), draft.category, `editorial food article image ${index + 1}`, source].filter(Boolean).join(', ');
      const result = await aiGenerateArticleImage(title, prompt, index, false, [], 'flash', 'editorial', { photoScenarios: [] });
      setImages((current) => {
        const next = [...current];
        next[index] = result.image_url;
        return next;
      });
      setSelectedImage(index);
      setDraft((current) => ({ ...current, image_url: index === 0 ? result.image_url : current.image_url }));
      setMessage(index === 0 ? 'Главное фото перегенерировано.' : `Фото ${index + 1} перегенерировано.`);
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Не удалось сгенерировать фото'); }
    finally { setBusy(null); }
  }

  async function addGeneratedImage() {
    const index = images.length;
    setImages((current) => [...current, '']);
    await generateArticleImage(index);
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
      <div className="catalog-edit-head"><div><p className="eyebrow">Gemini SEO editor</p><h3>{articleDisplayTitle(draft)}</h3><span>{draft.category || 'blog'} · {draft.slug || 'new-page'}</span></div><Button variant="secondary" type="button" onClick={onClose}>Закрыть</Button></div>
      <Tabs value={tab} onValueChange={(value) => setTab(value as SeoArticleTab)} className="grid min-h-0 flex-1 grid-rows-[auto_auto_minmax(0,1fr)]">
        <div className="border-b border-zinc-800 px-4 py-3">
          <TabsList>{tabs.map(([id, label]) => <TabsTrigger key={id} value={id}>{label}</TabsTrigger>)}</TabsList>
        </div>
        <section className="grid gap-3 border-b border-zinc-800 p-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,.8fr)_200px]">
          <SeoEditorField label="Тема / задача" value={source} onChange={setSource} multiline textareaClassName="min-h-16 max-h-28" />
          <SeoEditorField label="Инструкция Gemini" value={instruction} onChange={setInstruction} multiline textareaClassName="min-h-16 max-h-28" />
          <Button className="grid h-full min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start self-end bg-orange-500 text-black hover:bg-orange-400" type="button" onClick={() => void runGemini()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="sparkles" /></span>{busy === 'gemini' ? 'Генерируем...' : 'Текст и SEO'}<small className="text-xs opacity-70">gemini-3.1-pro-preview</small></Button>
        </section>
        <div className="grid min-h-0 gap-4 overflow-auto p-4 lg:grid-cols-[240px_minmax(0,1fr)]">
          <aside className="grid content-start gap-3">
            <Card className="overflow-hidden bg-zinc-950">
              {(images[selectedImage] || draft.image_url) ? <img className="aspect-square w-full object-cover" src={String(images[selectedImage] || draft.image_url)} alt={articleDisplayTitle(draft)} /> : <div className="grid aspect-square place-items-center text-zinc-500"><AppIcon name="seo" size={28} /></div>}
              <CardContent className="p-3 text-xs font-bold text-zinc-500">{images.length || 1} фото из CMS/Markdown</CardContent>
            </Card>
            <Card className="bg-zinc-950">
              <CardContent className="grid gap-2 p-3 text-xs text-zinc-400">
                <strong className="text-sm text-zinc-100">{draft.published ? 'Опубликовано' : 'Черновик'}</strong>
                <span>{String(draft.slug || 'new-page')}</span>
              </CardContent>
            </Card>
          </aside>
          <div className="min-w-0">
            <TabsContent value="content" className="m-0">
              <Card className="bg-zinc-950/60">
                <CardHeader className="flex-row items-center justify-between gap-3"><CardTitle>Контент страницы</CardTitle><Tabs value={language} onValueChange={(value) => setLanguage(value as SeoArticleLang)}><TabsList>{langTabs.map(([id, label]) => <TabsTrigger key={id} value={id}>{label}</TabsTrigger>)}</TabsList></Tabs></CardHeader>
                <CardContent className="grid gap-4 xl:grid-cols-2">
                  <SeoEditorField label={`Title ${language.toUpperCase()}`} value={String(draft[titleField] || '')} onChange={(value) => setText(titleField, value)} />
                  <SeoEditorField label="Slug страницы" value={String(draft.slug || '')} onChange={(value) => setText('slug', value)} />
                  <SeoEditorField label="Категория" value={String(draft.category || '')} onChange={(value) => setText('category', value)} />
                  <SeoEditorField label={`Content ${language.toUpperCase()}`} value={String(draft[contentField] || '')} onChange={(value) => setText(contentField, value)} multiline className="xl:col-span-2" textareaClassName="min-h-[280px]" />
                </CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="languages" className="m-0">
              <Card className="bg-zinc-950/60">
                <CardHeader className="flex-row items-center justify-between gap-3"><CardTitle>4 языка</CardTitle><Tabs value={language} onValueChange={(value) => setLanguage(value as SeoArticleLang)}><TabsList>{langTabs.map(([id, label]) => <TabsTrigger key={id} value={id}>{label}</TabsTrigger>)}</TabsList></Tabs></CardHeader>
                <CardContent className="grid gap-4 xl:grid-cols-2">
                  <SeoEditorField label={`Title ${language.toUpperCase()}`} value={String(draft[titleField] || '')} onChange={(value) => setText(titleField, value)} />
                  <SeoEditorField label={`Content ${language.toUpperCase()}`} value={String(draft[contentField] || '')} onChange={(value) => setText(contentField, value)} multiline className="xl:col-span-2" textareaClassName="min-h-[340px]" />
                </CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="seo" className="m-0">
              <Card className="bg-zinc-950/60">
                <CardHeader className="flex-row items-center justify-between gap-3"><CardTitle>SEO</CardTitle><Tabs value={language} onValueChange={(value) => setLanguage(value as SeoArticleLang)}><TabsList>{langTabs.map(([id, label]) => <TabsTrigger key={id} value={id}>{label}</TabsTrigger>)}</TabsList></Tabs></CardHeader>
                <CardContent className="grid gap-4 xl:grid-cols-2">
                  <SeoEditorField label={`SEO title ${language.toUpperCase()}`} value={String(draft[seoTitleField] || '')} onChange={(value) => setText(seoTitleField, value)} />
                  <SeoEditorField label="Fallback SEO title" value={String(draft.seo_title || '')} onChange={(value) => setText('seo_title', value)} />
                  <SeoEditorField label={`SEO description ${language.toUpperCase()}`} value={String(draft[seoDescriptionField] || '')} onChange={(value) => setText(seoDescriptionField, value)} multiline textareaClassName="min-h-[220px]" />
                  <SeoEditorField label="Fallback SEO description" value={String(draft.seo_description || '')} onChange={(value) => setText('seo_description', value)} multiline textareaClassName="min-h-[220px]" />
                </CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="media" className="m-0">
              <div className="grid gap-3 xl:grid-cols-[minmax(0,1fr)_360px]">
            <Card className="bg-zinc-950/60">
              <CardHeader><CardTitle>Изображения статьи</CardTitle></CardHeader>
              <CardContent>
                <ScrollArea className="max-h-[58vh] pr-2">
                  <div className="grid gap-3 lg:grid-cols-2">
                    {(images.length ? images : [String(draft.image_url || '')]).map((url, index) => <Card key={index} className={index === selectedImage ? 'border-orange-500/70 bg-zinc-950' : 'bg-zinc-950'}>
                      <CardContent className="grid gap-3 p-3 sm:grid-cols-[220px_minmax(0,1fr)]">
                        <button className="relative block aspect-video overflow-hidden rounded-md border border-zinc-800 bg-zinc-950" type="button" onClick={() => url && setFullscreenImage(url)}>
                          {url ? <img className="absolute inset-0 block h-full w-full object-cover object-center" src={url} alt={`Фото ${index + 1}`} /> : <span className="grid h-full place-items-center text-zinc-500"><AppIcon name="seo" /></span>}
                        </button>
                        <div className="grid min-w-0 gap-2">
                          <strong>{index === 0 ? 'Обложка' : `Фото ${index + 1}`}</strong>
                          <Input value={url} onChange={(event) => updateImage(index, event.target.value)} placeholder="https://..." />
                          <div className="flex flex-wrap gap-2">
                            <Button size="sm" variant="secondary" type="button" onClick={() => { setSelectedImage(index); setDraft((current) => ({ ...current, image_url: url })); }}>Главное</Button>
                            <Button size="sm" variant="secondary" type="button" onClick={() => void generateArticleImage(index)} disabled={Boolean(busy)}>{busy === `image-${index}` ? 'Генерируем...' : 'Gemini фото'}</Button>
                            <Button size="sm" variant="secondary" type="button" onClick={() => url && setFullscreenImage(url)} disabled={!url}>Открыть</Button>
                            <Button size="sm" variant="destructive" type="button" onClick={() => removeImage(index)}>Удалить</Button>
                          </div>
                        </div>
                      </CardContent>
                    </Card>)}
                    <Button className="h-20 border border-dashed border-orange-500/70 bg-orange-500/10 text-orange-300 hover:bg-orange-500/20" type="button" onClick={() => void addGeneratedImage()} disabled={Boolean(busy)}><AppIcon name="sparkles" />+ Фото Gemini <small className="text-xs opacity-70">gemini-3.1-flash-image</small></Button>
                    <Button className="h-20 border-dashed" variant="outline" type="button" onClick={addImage}>+ URL</Button>
                  </div>
                </ScrollArea>
              </CardContent>
            </Card>
            <Card className="bg-zinc-950/60">
              <CardHeader><CardTitle>Параметры медиа</CardTitle></CardHeader>
              <CardContent className="grid gap-4">
                <Card className="bg-zinc-950"><CardContent className="flex items-center gap-3 p-3"><AppIcon name="sparkles" /><span><strong>Текст и SEO</strong><small className="block text-orange-400">gemini-3.1-pro-preview</small></span></CardContent></Card>
                <Card className="bg-zinc-950"><CardContent className="flex items-center gap-3 p-3"><AppIcon name="package" /><span><strong>Фото</strong><small className="block text-orange-400">gemini-3.1-flash-image</small></span></CardContent></Card>
                <div><h4 className="mb-2 text-sm font-black">Что можно сделать</h4><ul className="list-disc space-y-2 pl-5 text-sm text-zinc-400"><li>Перегенерировать выбранное фото</li><li>Создать обложку с помощью Gemini</li><li>Добавить изображение по URL</li><li>Сохранить страницу и опубликовать</li></ul></div>
                <Card className="bg-zinc-950"><CardContent className="flex items-center gap-3 p-3">{(images[selectedImage] || draft.image_url) ? <img className="h-14 w-14 rounded-md bg-zinc-900 object-contain" src={String(images[selectedImage] || draft.image_url)} alt="Выбранное фото" /> : null}<span><strong>Выбрано: {selectedImage === 0 ? 'Обложка' : `Фото ${selectedImage + 1}`}</strong><small className="block text-orange-400">{busy ? 'В работе...' : 'Готово'}</small></span></CardContent></Card>
                <label className="flex items-center gap-2 text-sm font-bold text-zinc-300"><input type="checkbox" checked={Boolean(draft.published)} onChange={(event) => setDraft((current) => ({ ...current, published: event.target.checked }))} />Опубликовано на сайте</label>
              </CardContent>
            </Card>
              </div>
            </TabsContent>
          </div>
        </div>
      </Tabs>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><button className="btn btn-quiet" type="button" onClick={onClose}>Отмена</button><button className="btn btn-primary" type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : article ? 'Сохранить страницу' : 'Создать страницу'}</button></div>
    </section>
    {fullscreenImage ? (
      <div
        className="fixed inset-0 z-[1000] grid place-items-center bg-black/95 p-6"
        role="dialog"
        aria-label="Фото на весь экран"
        onMouseDown={(event) => {
          event.stopPropagation();
          setFullscreenImage(null);
        }}
      >
        <button
          className="absolute right-6 top-6 rounded-md border border-zinc-800 bg-zinc-950 px-4 py-3 text-sm font-black text-zinc-200 hover:border-orange-500 hover:text-white"
          type="button"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={() => setFullscreenImage(null)}
        >
          Закрыть
        </button>
        <img
          className="max-h-[88vh] max-w-[96vw] rounded-md border border-zinc-800 object-contain shadow-2xl"
          src={fullscreenImage}
          alt="Фото на весь экран"
          onMouseDown={(event) => event.stopPropagation()}
        />
      </div>
    ) : null}
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
  const imageReadyCount = props.articles.filter((article) => Boolean(article.image_url)).length;
  return <section className="ops-panel catalog-browser">
    <PanelTitle title="SEO Blog Dima Fomin" icon="seo" action={`${rows.length} из ${props.articles.length}`} />
    <div className="catalog-toolbar"><label><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Заголовок, slug, SEO, категория" /></label><label><span>Категория</span><select value={category} onChange={(event) => setCategory(event.target.value)}><option value="all">Все категории</option>{categories.map((item) => <option key={item} value={item}>{item}</option>)}</select></label><button className="btn btn-primary" type="button" onClick={() => setCreatingArticle(true)}><AppIcon name="sparkles" />Создать SEO страницу</button><button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из backend</button></div>
    <div className="catalog-stats-row"><article><span>Всего страниц</span><strong>{props.articles.length}</strong></article><article><span>Опубликовано</span><strong>{props.articles.filter((article) => article.published).length}</strong></article><article><span>SEO готово</span><strong>{props.articles.filter((article) => article.seo_title || article.seo_title_ru).length}</strong></article><article><span>С фото</span><strong>{imageReadyCount}</strong></article><article><span>Категорий</span><strong>{categories.length}</strong></article></div>
    <div className="table-scroll"><table className="ops-table"><thead><tr><th>Фото</th><th>Название</th><th>Категория</th><th>Slug</th><th>SEO</th><th>Статус</th><th>Действие</th></tr></thead><tbody>{rows.map((article) => <tr key={article.id}><td>{article.image_url ? <img className="catalog-product-thumb" src={article.image_url} alt={articleDisplayTitle(article)} loading="lazy" /> : <span className="catalog-product-thumb empty"><AppIcon name="seo" size={18} /></span>}</td><td><strong>{articleDisplayTitle(article)}</strong><small>{article.title_en}</small></td><td>{article.category || 'blog'}</td><td><code>{article.slug}</code></td><td><StatusPill tone={article.seo_title || article.seo_title_ru ? 'good' : 'warning'} label={article.seo_title || article.seo_title_ru ? 'готово' : 'не заполнено'} /></td><td><StatusPill tone={article.published ? 'good' : 'neutral'} label={article.published ? 'опубликовано' : 'черновик'} /></td><td><button className="table-action" type="button" onClick={() => setEditingArticle(article)}>Редактировать</button></td></tr>)}</tbody></table></div>
    {props.articles.length === 0 ? <p className="empty-state">SEO/Blog страницы не загружены. Нажмите «Обновить».</p> : null}
    {creatingArticle ? <DimaArticleEditorModal onClose={() => setCreatingArticle(false)} onSaved={props.onRefresh} /> : null}
    {editingArticle ? <DimaArticleEditorModal article={editingArticle} onClose={() => setEditingArticle(null)} onSaved={props.onRefresh} /> : null}
  </section>;
}

function SiteSettings({ data }: { data: SiteDataset }) {
  return <div className="site-card-grid"><article className="site-card"><div><h3>{data.name}</h3><StatusPill tone={data.ga4 ? 'good' : 'warning'} label={data.ga4 ? 'аналитика подключена' : 'аналитика не подключена'} /></div><p>{data.domain}</p><dl><dt>Cloudflare</dt><dd>{data.cloudflare}</dd><dt>GitHub</dt><dd>{data.github}</dd><dt>Backend</dt><dd>{data.backend}</dd><dt>Языки</dt><dd>{data.languages}</dd><dt>GA4</dt><dd>{data.ga4 ? 'подключено' : 'не подключено'}</dd><dt>Search Console</dt><dd>{data.searchConsole ? 'подключено' : 'не подключено'}</dd><dt>Sitemap</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd><dt>robots.txt</dt><dd>{data.ga4 ? 'валидно' : 'нужна проверка'}</dd></dl><div className="card-actions"><button>Открыть сайт</button><button>Синхронизировать</button><button>Деплой</button></div></article></div>;
}

function NotConnectedPanel({ title, icon, endpoint, description }: { title: string; icon: AppIconName; endpoint: string; description: string }) {
  return <section className="ops-panel">
    <PanelTitle title={title} icon={icon} action="API не подключён" />
    <div className="empty-state">
      <strong>{description}</strong>
      <p>Чтобы здесь не было демо-данных, нужен backend endpoint: <code>{endpoint}</code></p>
    </div>
  </section>;
}

function LeadsCrm({ props }: { props: OperationsPageProps }) {
  if (props.activeSite !== 'almabuild') {
    return <NotConnectedPanel title="CRM заявок" icon="leads" endpoint="/api/admin/leads" description="Backend пока не отдаёт заявки и стадии воронки для выбранного сайта." />;
  }

  const leads = props.almabuildLeads;
  const todayCount = leads.filter((lead) => isToday(lead.createdAt)).length;
  const withItemsCount = leads.filter((lead) => lead.items.length > 0).length;

  return <section className="ops-panel catalog-browser">
    <PanelTitle title="CRM заявок Kazaxbud" icon="leads" action={`${leads.length} заявок из /api/leads`} />
    <div className="catalog-toolbar">
      <label><span>Источник</span><input value="http://localhost:3000/api/leads/" readOnly /></label>
      <button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из сайта</button>
    </div>
    <div className="catalog-stats-row">
      <article><span>Всего заявок</span><strong>{leads.length}</strong></article>
      <article><span>Сегодня</span><strong>{todayCount}</strong></article>
      <article><span>С позициями сметы</span><strong>{withItemsCount}</strong></article>
      <article><span>С площадью</span><strong>{leads.filter((lead) => lead.area).length}</strong></article>
      <article><span>Телефоны</span><strong>{leads.filter((lead) => lead.phone).length}</strong></article>
    </div>
    <div className="table-scroll">
      <table className="ops-table">
        <thead><tr><th>Дата</th><th>Клиент</th><th>Телефон</th><th>Объект</th><th>Комментарий</th><th>Позиции</th></tr></thead>
        <tbody>{leads.map((lead) => (
          <tr key={lead.id}>
            <td><code>{new Date(lead.createdAt).toLocaleString('ru-RU')}</code></td>
            <td><strong>{lead.name || 'Без имени'}</strong><small>{lead.id.slice(0, 8)}</small></td>
            <td><a className="table-action" href={`tel:${lead.phone}`}>{lead.phone || 'нет телефона'}</a></td>
            <td>{lead.type || 'не указан'}<small>{lead.area ? `${lead.area} м2` : 'площадь не указана'}</small></td>
            <td>{lead.comment || 'Комментарий не указан'}</td>
            <td>{lead.items.length ? lead.items.join(', ') : 'нет позиций'}</td>
          </tr>
        ))}</tbody>
      </table>
    </div>
    {leads.length === 0 ? <p className="empty-state">Заявок пока нет. Когда клиент отправит форму на сайте Kazaxbud, она появится здесь.</p> : null}
  </section>;
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

function CatalogNumberField({ label, value, onChange }: { label: string; value: unknown; onChange: (value: string) => void }) {
  return <label className="grid min-w-0 gap-2 text-sm font-black text-zinc-400">
    <span>{label}</span>
    <Input type="number" step="any" value={value as number ?? ''} onChange={(event) => onChange(event.target.value)} />
  </label>;
}

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
      <div className="catalog-edit-head"><div><p className="eyebrow">Gemini product editor</p><h3>{productDisplayName(product)}</h3><span>{categoryDisplayName(category)} · {product.slug || product.id}</span></div><Button variant="secondary" type="button" onClick={onClose}>Закрыть</Button></div>
      <Tabs value={tab} onValueChange={(value) => setTab(value as CatalogEditTab)} className="grid min-h-0 flex-1 grid-rows-[auto_auto_minmax(0,1fr)]">
        <div className="border-b border-zinc-800 px-4 py-3">
          <TabsList className="max-w-full overflow-x-auto">{tabs.map(([id, label]) => <TabsTrigger key={id} value={id}>{label}</TabsTrigger>)}</TabsList>
        </div>
        <section className="grid gap-3 border-b border-zinc-800 p-4 xl:grid-cols-[minmax(0,1fr)_auto]">
          <SeoEditorField label="Задача для Gemini" value={instruction} onChange={setInstruction} multiline textareaClassName="min-h-16 max-h-28" />
          <div className="grid gap-2 sm:grid-cols-3 xl:min-w-[620px]">
            <Button className="grid min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start bg-orange-500 text-black hover:bg-orange-400" type="button" onClick={() => void runGeminiEdit()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="sparkles" /></span>{busy === 'gemini' ? 'Генерируем...' : 'Текст и профиль'}<small className="text-xs opacity-70">{CATALOG_TEXT_MODEL}</small></Button>
            <Button className="grid min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start" variant="secondary" type="button" onClick={() => void generateImage()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="package" /></span>{busy === 'image' ? 'Фото...' : 'Фото'}<small className="text-xs opacity-70">{CATALOG_IMAGE_MODEL}</small></Button>
            <Button className="grid min-h-16 grid-cols-[auto_1fr] grid-rows-2 content-center justify-items-start" variant="secondary" type="button" onClick={() => void regenerateStates()} disabled={Boolean(busy)}><span className="row-span-2"><AppIcon name="refresh" /></span>{busy === 'states' ? 'Обновляем...' : 'Состояния'}<small className="text-xs opacity-70">{CATALOG_TEXT_MODEL}</small></Button>
          </div>
        </section>
        <div className="grid min-h-0 gap-4 overflow-auto p-4 lg:grid-cols-[240px_minmax(0,1fr)]">
          <aside className="grid content-start gap-3">
            <Card className="overflow-hidden bg-zinc-950">
              {draft.image_url ? <img className="aspect-square w-full object-cover" src={draft.image_url} alt={productDisplayName(product)} /> : <div className="grid aspect-square place-items-center text-zinc-500"><AppIcon name="package" size={28} /></div>}
              <CardContent className="grid gap-2 p-3 text-xs font-bold text-zinc-500"><span>Фото: {CATALOG_IMAGE_MODEL}</span><span className="break-all">{String(draft.image_url || 'Фото не задано')}</span></CardContent>
            </Card>
            <Card className="bg-zinc-950">
              <CardContent className="grid gap-2 p-3 text-xs text-zinc-400">
                <strong className="text-sm text-zinc-100">{String(draft.product_type || 'product')}</strong>
                <span>{String(draft.unit || 'unit не задан')}</span>
              </CardContent>
            </Card>
          </aside>
          <div className="min-w-0">
            <TabsContent value="basic" className="m-0">
              <Card className="bg-zinc-950/60">
                <CardHeader><CardTitle>Основное</CardTitle></CardHeader>
                <CardContent className="grid gap-4 xl:grid-cols-2">
                  <SeoEditorField label="Название RU" value={String(draft.name_ru || '')} onChange={(value) => setText('name_ru', value)} />
                  <SeoEditorField label="Название EN" value={String(draft.name_en || '')} onChange={(value) => setText('name_en', value)} />
                  <SeoEditorField label="Название PL" value={String(draft.name_pl || '')} onChange={(value) => setText('name_pl', value)} />
                  <SeoEditorField label="Название UK" value={String(draft.name_uk || '')} onChange={(value) => setText('name_uk', value)} />
                  <SeoEditorField label="Тип продукта" value={String(draft.product_type || '')} onChange={(value) => setText('product_type', value)} />
                  <SeoEditorField label="Единица" value={String(draft.unit || '')} onChange={(value) => setText('unit', value)} />
                  <SeoEditorField label="Image URL" value={String(draft.image_url || '')} onChange={(value) => setText('image_url', value)} multiline className="xl:col-span-2" textareaClassName="min-h-24" />
                </CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="content" className="m-0">
              <Card className="bg-zinc-950/60">
                <CardHeader><CardTitle>4 языка и SEO</CardTitle></CardHeader>
                <CardContent className="grid gap-4 xl:grid-cols-2">
                  <SeoEditorField label="Описание RU" value={String(draft.description_ru || '')} onChange={(value) => setText('description_ru', value)} multiline textareaClassName="min-h-[180px]" />
                  <SeoEditorField label="Описание EN" value={String(draft.description_en || '')} onChange={(value) => setText('description_en', value)} multiline textareaClassName="min-h-[180px]" />
                  <SeoEditorField label="Описание PL" value={String(draft.description_pl || '')} onChange={(value) => setText('description_pl', value)} multiline textareaClassName="min-h-[180px]" />
                  <SeoEditorField label="Описание UK" value={String(draft.description_uk || '')} onChange={(value) => setText('description_uk', value)} multiline textareaClassName="min-h-[180px]" />
                  <SeoEditorField label="SEO title" value={String(draft.seo_title || '')} onChange={(value) => setText('seo_title', value)} />
                  <SeoEditorField label="SEO H1" value={String(draft.seo_h1 || '')} onChange={(value) => setText('seo_h1', value)} />
                  <SeoEditorField label="SEO description" value={String(draft.seo_description || '')} onChange={(value) => setText('seo_description', value)} multiline className="xl:col-span-2" textareaClassName="min-h-[160px]" />
                </CardContent>
              </Card>
            </TabsContent>
            <TabsContent value="nutrition" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Макросы на 100 г</CardTitle></CardHeader><CardContent className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">{([['calories_per_100g', 'Kalorie/kcal'], ['protein_per_100g', 'Białko'], ['fat_per_100g', 'Tłuszcz'], ['carbs_per_100g', 'Węglowodany'], ['fiber_per_100g', 'Błonnik'], ['sugar_per_100g', 'Cukier'], ['density_g_per_ml', 'Gęstość'], ['typical_portion_g', 'Porcja'], ['shelf_life_days', 'Trwałość'], ['starch_g', 'Starch'], ['water_g', 'Water'], ['alcohol_g', 'Alcohol']] as Array<[string, string]>).map(([field, label]) => field in draft ? <CatalogNumberField key={field} label={label} value={draft[field as keyof CatalogEditDraft]} onChange={(value) => setNumber(field as keyof CatalogEditDraft, value)} /> : <CatalogNumberField key={field} label={label} value={profileValue('macros', field)} onChange={(value) => setProfileNumber('macros', field, value)} />)}</CardContent></Card>
            </TabsContent>
            <TabsContent value="vitamins" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Витамины</CardTitle></CardHeader><CardContent className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">{CATALOG_VITAMINS.map(([field, label]) => <CatalogNumberField key={field} label={label} value={profileValue('vitamins', field)} onChange={(value) => setProfileNumber('vitamins', field, value)} />)}</CardContent></Card>
            </TabsContent>
            <TabsContent value="minerals" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Минералы</CardTitle></CardHeader><CardContent className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">{CATALOG_MINERALS.map(([field, label]) => <CatalogNumberField key={field} label={label} value={profileValue('minerals', field)} onChange={(value) => setProfileNumber('minerals', field, value)} />)}</CardContent></Card>
            </TabsContent>
            <TabsContent value="culinary" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Кулинария</CardTitle></CardHeader><CardContent className="grid gap-4 xl:grid-cols-2"><div className="grid gap-4 sm:grid-cols-2 xl:col-span-2 xl:grid-cols-5">{(['sweetness', 'acidity', 'bitterness', 'umami', 'aroma'] as const).map((field) => <CatalogNumberField key={field} label={field} value={profileValue('culinary', field)} onChange={(value) => setProfileNumber('culinary', field, value)} />)}{(['glycemic_index', 'glycemic_load', 'ph', 'smoke_point', 'water_activity'] as const).map((field) => <CatalogNumberField key={field} label={field} value={profileValue('food_properties', field)} onChange={(value) => setProfileNumber('food_properties', field, value)} />)}</div><SeoEditorField label="Texture" value={String(profileValue('culinary', 'texture') || '')} onChange={(value) => setProfileValue('culinary', 'texture', value)} multiline textareaClassName="min-h-[180px]" /><SeoEditorField label="Processing notes PL" value={String(profileValue('processing_effects', 'processing_notes_pl') || '')} onChange={(value) => setProfileValue('processing_effects', 'processing_notes_pl', value)} multiline textareaClassName="min-h-[180px]" /></CardContent></Card>
            </TabsContent>
            <TabsContent value="health" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Health profile</CardTitle></CardHeader><CardContent className="grid gap-4 xl:grid-cols-2"><div className="grid gap-2 sm:grid-cols-2 xl:col-span-2 xl:grid-cols-4">{CATALOG_DIETS.map(([field, label]) => <label className="flex items-center gap-2 rounded-md border border-zinc-800 bg-zinc-950 p-3 text-sm font-bold text-zinc-300" key={field}><input type="checkbox" checked={Boolean(profileValue('diet_flags', field))} onChange={(event) => setProfileValue('diet_flags', field, event.target.checked)} />{label}</label>)}</div>{(['en', 'ru', 'pl', 'uk'] as const).flatMap((lang) => [<SeoEditorField key={`bio-${lang}`} label={`Bioactive ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `bioactive_compounds_${lang}`))} onChange={(value) => setProfileValue('health_profile', `bioactive_compounds_${lang}`, csvToArray(value))} multiline textareaClassName="min-h-[150px]" />, <SeoEditorField key={`eff-${lang}`} label={`Health effects ${lang.toUpperCase()}`} value={arrayToCsv(profileValue('health_profile', `health_effects_${lang}`))} onChange={(value) => setProfileValue('health_profile', `health_effects_${lang}`, csvToArray(value))} multiline textareaClassName="min-h-[150px]" />])}<SeoEditorField label="Food role" value={String(profileValue('health_profile', 'food_role') || '')} onChange={(value) => setProfileValue('health_profile', 'food_role', value)} className="xl:col-span-2" /></CardContent></Card>
            </TabsContent>
            <TabsContent value="states" className="m-0">
              <Card className="bg-zinc-950/60"><CardHeader><CardTitle>Состояния обработки</CardTitle></CardHeader><CardContent><div className="grid gap-3">{states.map((state) => <Card key={state.id} className="bg-zinc-950"><CardContent className="grid gap-1 p-3"><strong>{state.name_suffix_pl || state.name_suffix_ru || state.state}</strong><span className="text-sm text-zinc-400">{state.calories_per_100g ?? '-'} kcal · {state.storage_temp_c ?? '-'}°C · {state.shelf_life_hours ?? '-'} godz.</span><p className="m-0 text-sm text-zinc-500">{state.notes_pl || state.notes_ru || state.notes_en}</p></CardContent></Card>)}</div></CardContent></Card>
            </TabsContent>
          </div>
        </div>
      </Tabs>
      <div className="catalog-edit-actions"><EditorMessage value={message} /><Button variant="secondary" type="button" onClick={onClose}>Отмена</Button><Button type="button" onClick={() => void save()} disabled={Boolean(busy)}><AppIcon name="check" />{busy === 'save' ? 'Сохраняем...' : 'Сохранить всё'}</Button></div>
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
  const [query, setQuery] = useState('');
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [visionFile, setVisionFile] = useState<File | null>(null);
  const [visionCount, setVisionCount] = useState(4);
  const [visionInstruction, setVisionInstruction] = useState('Определи материалы на фото и создай категории для сайта стройматериалов в Алматы.');
  const [visionBusy, setVisionBusy] = useState(false);
  const [visionMessage, setVisionMessage] = useState<string | null>(null);
  const [visionDrafts, setVisionDrafts] = useState<MaterialCategory[]>([]);
  const [visionPreviewUrl, setVisionPreviewUrl] = useState<string | null>(null);
  const content = props.almabuildContent;
  useEffect(() => {
    if (!visionFile) {
      setVisionPreviewUrl(null);
      return undefined;
    }
    const url = URL.createObjectURL(visionFile);
    setVisionPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [visionFile]);
  if (!content) return <section className="ops-panel"><PanelTitle title="Материалы Kazaxbud" icon="materials" action="контент не загружен" /><p className="empty-state">Нажмите «Обновить», чтобы загрузить контент сайта.</p></section>;
  const loadedContent = content;
  const needle = query.trim().toLowerCase();
  const rows = content.materialCategories
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => {
      const haystack = [item.title, item.slug, item.index, item.text, item.photo, ...item.bullets].join(' ').toLowerCase();
      return !needle || haystack.includes(needle);
    });

  async function runVision() {
    if (!visionFile) {
      setVisionMessage('Сначала загрузите фото материала.');
      return;
    }
    setVisionBusy(true); setVisionMessage(null);
    try {
      const response = await generateAlmabuildMaterialsFromPhoto({
        image: visionFile,
        count: visionCount,
        instruction: visionInstruction,
        existingCount: loadedContent.materialCategories.length,
        existing: loadedContent.materialCategories
      });
      setVisionDrafts(response.materials);
      setVisionMessage(`Gemini Vision подготовил ${response.materials.length} карточек. Проверьте и добавьте в сайт.`);
    } catch (error) {
      setVisionMessage(error instanceof Error ? error.message : 'Gemini Vision не смог создать материалы');
    } finally { setVisionBusy(false); }
  }

  async function saveVisionDrafts() {
    if (!visionDrafts.length) return;
    setVisionBusy(true); setVisionMessage('Сохраняем материалы в backend...');
    try {
      const nextMaterials = withUniqueMaterialSlugs(loadedContent.materialCategories, visionDrafts);
      await saveAlmabuildContent({ ...loadedContent, materialCategories: [...loadedContent.materialCategories, ...nextMaterials] });
      setVisionDrafts([]);
      setVisionFile(null);
      setVisionPreviewUrl(null);
      setVisionMessage('Материалы добавлены и доступны сайту Kazaxbud.');
      await props.onRefresh();
      setCreating(false);
    } catch (error) {
      setVisionMessage(error instanceof Error ? error.message : 'Не удалось сохранить материалы');
    } finally { setVisionBusy(false); }
  }

  return <section className="ops-panel catalog-browser">
    <PanelTitle title="Материалы Kazaxbud" icon="materials" action={`${rows.length} из ${content.materialCategories.length}`} />
    <div className="catalog-toolbar"><label><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Название, slug, индекс, визуальный класс" /></label><button className="btn btn-primary" type="button" onClick={() => setCreating(true)}><AppIcon name="sparkles" />Добавить материал</button><button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из backend</button></div>
    <div className="catalog-stats-row"><article><span>Всего материалов</span><strong>{content.materialCategories.length}</strong></article><article><span>С описанием</span><strong>{content.materialCategories.filter((item) => item.text).length}</strong></article><article><span>С пунктами</span><strong>{content.materialCategories.filter((item) => item.bullets.length).length}</strong></article><article><span>Визуальных классов</span><strong>{new Set(content.materialCategories.map((item) => item.photo).filter(Boolean)).size}</strong></article><article><span>Показано</span><strong>{rows.length}</strong></article></div>
    <div className="table-scroll"><table className="ops-table"><thead><tr><th>Вид</th><th>Название</th><th>Slug</th><th>Индекс</th><th>Пункты</th><th>Статус</th><th>Действие</th></tr></thead><tbody>{rows.map(({ item, index }) => <tr key={`${item.slug}-${index}`}><td>{item.imageUrl ? <img className="catalog-product-thumb" src={item.imageUrl} alt={item.title} loading="lazy" /> : <span className="catalog-product-thumb empty"><AppIcon name="materials" size={18} /></span>}</td><td><strong>{item.title}</strong><small>{item.text}</small>{item.imageUrl ? <small>R2 фото сохранено</small> : null}</td><td><code>{item.slug}</code></td><td><code>{item.index}</code></td><td>{item.bullets.length}</td><td><StatusPill tone={item.title && item.slug && item.text ? 'good' : 'warning'} label={item.title && item.slug && item.text ? 'готово' : 'проверить'} /></td><td><button className="table-action" type="button" onClick={() => setEditingIndex(index)}>Редактировать</button></td></tr>)}</tbody></table></div>
    {content.materialCategories.length === 0 ? <p className="empty-state">Материалы не загружены. Добавьте первый материал.</p> : null}
    {creating ? <KazaxbudMaterialEditorModal content={content} onClose={() => setCreating(false)} onRefresh={props.onRefresh} visionPanel={<Card className="bg-zinc-950"><CardHeader><CardTitle>Gemini Vision по фото</CardTitle></CardHeader><CardContent className="grid gap-3"><Button asChild variant="secondary"><label htmlFor="kazaxbud-material-photo-modal" className="cursor-pointer"><AppIcon name="materials" />Выбрать фото</label></Button><Input id="kazaxbud-material-photo-modal" className="sr-only" type="file" accept="image/*" onChange={(event) => setVisionFile(event.target.files?.[0] || null)} /><div className="overflow-hidden rounded-md border border-zinc-800 bg-black">{visionPreviewUrl ? <img className="h-48 w-full object-cover" src={visionPreviewUrl} alt="Фото для Gemini Vision" /> : <div className="grid h-48 place-items-center text-center text-sm font-bold text-zinc-500">Gemini увидит выбранное фото</div>}</div><span className="truncate text-xs font-bold text-zinc-500">{visionFile?.name || 'Файл не выбран'}</span><div className="grid grid-cols-[44px_minmax(0,1fr)_44px] gap-2"><Button variant="secondary" size="icon" type="button" onClick={() => setVisionCount((value) => clampMaterialCount(value - 1))}>-</Button><Input className="text-center text-lg font-black" type="number" min={1} max={12} value={visionCount} onChange={(event) => setVisionCount(clampMaterialCount(Number(event.target.value) || 1))} /><Button variant="secondary" size="icon" type="button" onClick={() => setVisionCount((value) => clampMaterialCount(value + 1))}>+</Button></div><Textarea className="min-h-32 resize-none text-sm font-bold leading-relaxed" value={visionInstruction} onChange={(event) => setVisionInstruction(event.target.value)} /><Button type="button" onClick={() => void runVision()} disabled={visionBusy || !visionFile}><AppIcon name="sparkles" />{visionBusy ? 'Gemini смотрит...' : 'Создать по фото'}</Button><EditorMessage value={visionMessage} />{visionDrafts.length ? <div className="grid gap-3"><div className="flex items-center justify-between gap-3"><strong className="text-sm text-zinc-100">Предпросмотр: {visionDrafts.length}</strong><Button size="sm" type="button" onClick={() => void saveVisionDrafts()} disabled={visionBusy}><AppIcon name="check" />Добавить все</Button></div><div className="grid max-h-72 gap-3 overflow-auto pr-1">{visionDrafts.map((item, index) => <article key={`${item.slug}-${index}`} className="rounded border border-zinc-800 bg-black p-3">{item.imageUrl ? <img className="mb-3 h-24 w-full rounded object-cover" src={item.imageUrl} alt={item.title} /> : null}<div className="mb-2 flex items-start justify-between gap-2"><strong className="text-zinc-100">{item.title}</strong><code className="text-xs text-zinc-500">{item.index}</code></div><p className="text-sm text-zinc-400">{item.text}</p><small className="mt-2 block text-zinc-500">{item.slug} · {item.photo}</small></article>)}</div></div> : null}</CardContent></Card>} /> : null}
    {editingIndex !== null ? <KazaxbudMaterialEditorModal item={content.materialCategories[editingIndex]} index={editingIndex} content={content} onClose={() => setEditingIndex(null)} onRefresh={props.onRefresh} /> : null}
  </section>;
}

function KazaxbudSeoTable({ props }: { props: OperationsPageProps }) {
  const [query, setQuery] = useState('');
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const content = props.almabuildContent;
  if (!content) return <section className="ops-panel"><PanelTitle title="SEO Kazaxbud" icon="seo" action="контент не загружен" /><p className="empty-state">Нажмите «Обновить», чтобы загрузить SEO-контент сайта.</p></section>;
  const needle = query.trim().toLowerCase();
  const rows = content.kits
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => {
      const haystack = [item.title, item.text, ...item.items].join(' ').toLowerCase();
      return !needle || haystack.includes(needle);
    });
  return <section className="ops-panel catalog-browser">
    <PanelTitle title="SEO Kazaxbud" icon="seo" action={`${rows.length} из ${content.kits.length}`} />
    <div className="catalog-toolbar"><label><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Название, описание, состав" /></label><button className="btn btn-primary" type="button" onClick={() => setCreating(true)}><AppIcon name="sparkles" />Добавить SEO</button><button className="btn btn-quiet" type="button" onClick={props.onRefresh} disabled={props.loading}><AppIcon name="refresh" />Обновить из backend</button></div>
    <div className="catalog-stats-row"><article><span>Всего SEO-блоков</span><strong>{content.kits.length}</strong></article><article><span>С описанием</span><strong>{content.kits.filter((item) => item.text).length}</strong></article><article><span>С составом</span><strong>{content.kits.filter((item) => item.items.length).length}</strong></article><article><span>Позиций</span><strong>{content.kits.reduce((sum, item) => sum + item.items.length, 0)}</strong></article><article><span>Показано</span><strong>{rows.length}</strong></article></div>
    <div className="table-scroll"><table className="ops-table"><thead><tr><th>Вид</th><th>Название</th><th>Описание</th><th>Состав</th><th>SEO</th><th>Действие</th></tr></thead><tbody>{rows.map(({ item, index }) => <tr key={`${item.title}-${index}`}><td><span className="catalog-product-thumb empty"><AppIcon name="seo" size={18} /></span></td><td><strong>{item.title}</strong><small>SEO-комплект Kazaxbud</small></td><td>{item.text}</td><td>{item.items.length} поз.</td><td><StatusPill tone={item.title && item.text && item.items.length ? 'good' : 'warning'} label={item.title && item.text && item.items.length ? 'готово' : 'проверить'} /></td><td><button className="table-action" type="button" onClick={() => setEditingIndex(index)}>Редактировать</button></td></tr>)}</tbody></table></div>
    {content.kits.length === 0 ? <p className="empty-state">SEO-комплекты не загружены. Добавьте первый блок.</p> : null}
    {creating ? <KazaxbudSeoKitEditorModal content={content} onClose={() => setCreating(false)} onRefresh={props.onRefresh} /> : null}
    {editingIndex !== null ? <KazaxbudSeoKitEditorModal item={content.kits[editingIndex]} index={editingIndex} content={content} onClose={() => setEditingIndex(null)} onRefresh={props.onRefresh} /> : null}
  </section>;
}

function SuppliersPanel() {
  return <NotConnectedPanel title="Поставщики" icon="suppliers" endpoint="/api/admin/suppliers" description="Backend пока не отдаёт поставщиков, прайсы и статусы синхронизации." />;
}

function AiStudio() {
  return <NotConnectedPanel title="AI-студия" icon="bot" endpoint="/api/admin/ai/tasks" description="Общая AI-студия пока не имеет backend endpoint. Рабочие AI-действия уже подключены внутри редакторов каталога, SEO и Kazaxbud." />;
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
  return <NotConnectedPanel title="Деплои Cloudflare Pages" icon="deploy" endpoint="/api/admin/deployments" description={`Backend пока не отдаёт деплои для ${data.cloudflare}.`} />;
}

function SettingsMatrix({ data }: { data: SiteDataset }) {
  const rows = [['URL бэкенда', data.backend], ['Домен', data.domain], ['Cloudflare', data.cloudflare], ['Языки', data.languages], ['GA4', data.ga4 ? 'ожидается из Analytics API' : 'не подключено'], ['Search Console', data.searchConsole ? 'ожидается из Search Console API' : 'не подключено']];
  return <section className="ops-panel"><PanelTitle title="Настройки сайта" icon="settings" action="только подключённые значения" /><div className="settings-matrix">{rows.map(([row, value]) => <label key={row}><span>{row}</span><input value={value} readOnly /></label>)}</div><p className="page-muted">Редактирование настроек требует backend endpoint <code>/api/admin/site-settings</code>.</p></section>;
}

function ModuleContent({ props, data }: { props: OperationsPageProps; data: SiteDataset }) {
  if (props.page === 'overview') return <Overview props={props} data={data} />;
  if (props.page === 'sites') return <SiteSettings data={data} />;
  if (props.page === 'leads') return <LeadsCrm props={props} />;
  if (props.page === 'catalog') return <CatalogTable props={props} />;
  if (props.page === 'materials') return <MaterialsTable props={props} />;
  if (props.page === 'suppliers') return <SuppliersPanel />;
  if (props.page === 'projects') return props.activeSite === 'almabuild' ? <AlmabuildEditor props={props} mode="projects" /> : <DimaPagesEditor props={props} />;
  if (props.page === 'seo') return props.activeSite === 'almabuild' ? <KazaxbudSeoTable props={props} /> : <DimaPagesEditor props={props} />;
  if (props.page === 'analytics') return <section className="ops-panel"><PanelTitle title="Аналитика" icon="analytics" action={props.analytics?.configured || props.searchConsole?.overview?.configured ? 'данные из API' : 'нет API-данных'} /><TrafficChart analytics={props.analytics} searchConsole={props.searchConsole} /></section>;
  if (props.page === 'ai') return <AiStudio />;
  if (props.page === 'usb') return <UsbKeyPanel />;
  if (props.page === 'deployments') return <Deployments data={data} />;
  return <SettingsMatrix data={data} />;
}

export function OperationsPage(props: OperationsPageProps) {
  const data = dataset(props);
  return <section className="ops-page" key={props.activeSite + '-' + props.page}><PageHeader props={props} data={data} />{props.error ? <p className="ops-alert"><AppIcon name="shield" />{props.error}</p> : null}<ModuleContent props={props} data={data} /></section>;
}
