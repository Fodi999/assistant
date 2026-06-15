import { useEffect, useMemo, useState } from 'react';
import { AppIcon } from './AppIcon';
import type { AppPage, ManagedSite } from './Sidebar';

interface TopbarProps {
  activeSite: ManagedSite;
  activePage: AppPage;
  connectionState: 'online' | 'limited' | 'offline';
  onNavigate: (page: AppPage) => void;
  onLogout: () => void;
}

const SITE_META: Record<ManagedSite, { name: string; domain: string; env: string }> = {
  almabuild: { name: 'KAZAXBUD', domain: 'kazaxbud.pages.dev / localhost:3000', env: 'Продакшн + локально' },
  dima: { name: 'Dima Fomin', domain: 'dima-fomin.pl', env: 'Продакшн' }
};

const PAGE_LABELS: Record<AppPage, string> = {
  overview: 'Обзор',
  sites: 'Сайт',
  leads: 'CRM заявок',
  catalog: 'Каталог',
  materials: 'Материалы',
  suppliers: 'Поставщики',
  projects: 'Проекты',
  seo: 'SEO-фабрика',
  analytics: 'Аналитика',
  ai: 'AI-студия',
  usb: 'USB Key',
  deployments: 'Деплои',
  settings: 'Настройки'
};

const COMMANDS: Array<{ page: AppPage; title: string; hint: string }> = [
  { page: 'seo', title: 'Создать SEO-страницу', hint: 'услуга + город' },
  { page: 'catalog', title: 'Добавить товар', hint: 'позиция каталога' },
  { page: 'materials', title: 'Импорт CSV', hint: 'цены материалов' },
  { page: 'ai', title: 'Сгенерировать статью', hint: 'шаблон AI-студии' },
  { page: 'sites', title: 'Синхронизировать сайт', hint: 'загрузить публичный контент' },
  { page: 'usb', title: 'Открыть USB Key', hint: 'локальные тяжёлые задачи' },
  { page: 'deployments', title: 'Открыть деплои Cloudflare', hint: 'лог публикации' },
  { page: 'leads', title: 'Создать заявку', hint: 'карточка CRM' },
  { page: 'seo', title: 'Опубликовать sitemap', hint: 'robots и sitemap' }
];

function commandAllowed(site: ManagedSite, page: AppPage) {
  if (site === 'almabuild' && page === 'catalog') return false;
  if (site === 'dima' && page === 'materials') return false;
  return true;
}

export function Topbar({ activeSite, activePage, connectionState, onNavigate, onLogout }: TopbarProps) {
  const [commandOpen, setCommandOpen] = useState(false);
  const [query, setQuery] = useState('');
  const site = SITE_META[activeSite];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const siteCommands = COMMANDS.filter((command) => commandAllowed(activeSite, command.page));
    if (!needle) return siteCommands;
    return siteCommands.filter((command) => (command.title + ' ' + command.hint).toLowerCase().includes(needle));
  }, [activeSite, query]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setCommandOpen(true);
      }
      if (event.key === 'Escape') setCommandOpen(false);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  return (
    <header className="topbar">
      <div className="topbar-context">
        <p className="eyebrow">{PAGE_LABELS[activePage]}</p>
        <h1>{site.name}</h1>
        <div className="topbar-meta">
          <span><AppIcon name="globe" />{site.domain}</span>
          <span><AppIcon name="cloud" />{site.env}</span>
        </div>
      </div>

      <button className="command-search" type="button" onClick={() => setCommandOpen(true)}>
        <AppIcon name="search" />
        <span>Поиск по сайтам, заявкам, товарам, SEO-страницам...</span>
        <kbd>⌘ K</kbd>
      </button>

      <div className="topbar-actions">
        <button className="btn btn-quiet topbar-action-btn" type="button" aria-label="Синхронизировать" title="Синхронизировать"><AppIcon name="refresh" /><span>Синхронизировать</span></button>
        <button className="btn btn-quiet topbar-action-btn" type="button" aria-label="Генерировать" title="Генерировать"><AppIcon name="sparkles" /><span>Генерировать</span></button>
        <button className="btn btn-primary topbar-action-btn primary" type="button"><AppIcon name="deploy" /><span>Опубликовать</span></button>
        <span className={'deploy-pill ' + connectionState}><i />Деплой готов</span>
        <button className="topbar-icon" type="button"><AppIcon name="settings" /></button>
        <button className="profile-menu" type="button" onClick={onLogout}><span>ДА</span><strong>Дима Админ<small>Выйти</small></strong></button>
      </div>

      {commandOpen ? (
        <div className="command-overlay" role="presentation" onMouseDown={() => setCommandOpen(false)}>
          <section className="command-panel" role="dialog" aria-label="Палитра команд" onMouseDown={(event) => event.stopPropagation()}>
            <label className="command-input"><AppIcon name="command" /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Введите команду или раздел..." /></label>
            <div className="command-list">
              {filtered.map((command) => (
                <button key={command.title} type="button" onClick={() => { onNavigate(command.page); setCommandOpen(false); setQuery(''); }}>
                  <AppIcon name="zap" />
                  <span><strong>{command.title}</strong><small>{command.hint}</small></span>
                  <kbd>↵</kbd>
                </button>
              ))}
            </div>
          </section>
        </div>
      ) : null}
    </header>
  );
}
