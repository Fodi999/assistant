import { useEffect, useMemo, useState } from 'react';
import { siteConfigs } from '../lib/mockData';
import { apiStatusLabels } from '../lib/labels';
import type { SiteKey } from '../types/admin';
import { AppIcon } from './AppIcon';
import { SiteSwitcher } from './SiteSwitcher';
import type { AppPage } from './Sidebar';

interface TopbarProps {
  activeSite: SiteKey;
  activePage: AppPage;
  connectionState: 'online' | 'limited' | 'offline';
  onSiteChange: (site: SiteKey) => void;
  onNavigate: (page: AppPage) => void;
  onLogout: () => void;
}

const PAGE_LABELS: Record<AppPage, string> = {
  dashboard: 'Панель',
  overview: 'Обзор',
  sites: 'Сайты',
  affiliate: 'Партнерка',
  content: 'Контент',
  'ai-studio': 'AI-студия',
  construction: 'Стройка',
  culinary: 'Кулинария',
  leads: 'Заявки',
  catalog: 'Каталог',
  materials: 'Материалы',
  projects: 'Проекты',
  seo: 'SEO',
  suppliers: 'Поставщики',
  analytics: 'Аналитика',
  ai: 'AI',
  usb: 'USB Key',
  deployments: 'Деплои',
  users: 'Пользователи',
  settings: 'Настройки',
  about: 'О системе'
};

const COMMANDS: Array<{ page: AppPage; title: string; hint: string }> = [
  { page: 'affiliate', title: 'Создать партнерский товар', hint: 'Amazon / Allegro / Ceneo / Awin / своя сеть' },
  { page: 'ai-studio', title: 'Создать через AI', hint: 'описание, SEO, slug, фото-промт' },
  { page: 'content', title: 'Новая статья или обзор', hint: 'статья / обзор / подборка' },
  { page: 'construction', title: 'Открыть калькулятор ремонта', hint: 'Алматы, KZT, маржа' },
  { page: 'leads', title: 'Посмотреть заявки', hint: 'новые / в работе / смета / выиграно / потеряно' },
  { page: 'analytics', title: 'Партнерская аналитика', hint: 'CTR, EPC, доход' }
];

export function Topbar({ activeSite, activePage, connectionState, onSiteChange, onNavigate, onLogout }: TopbarProps) {
  const [commandOpen, setCommandOpen] = useState(false);
  const [query, setQuery] = useState('');
  const site = siteConfigs.find((item) => item.key === activeSite) ?? siteConfigs[0];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return COMMANDS;
    return COMMANDS.filter((command) => (command.title + ' ' + command.hint).toLowerCase().includes(needle));
  }, [query]);

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
    <header className="topbar new-topbar">
      <div className="topbar-context">
        <p className="eyebrow">{PAGE_LABELS[activePage]}</p>
        <h1>{site.name}</h1>
        <div className="topbar-meta">
          <span><AppIcon name="globe" />{site.domain}</span>
          <span><AppIcon name="cloud" />{site.region}</span>
        </div>
      </div>

      <SiteSwitcher activeSite={activeSite} onSiteChange={onSiteChange} />

      <button className="command-search" type="button" onClick={() => setCommandOpen(true)}>
        <AppIcon name="search" />
        <span>Поиск по товарам, статьям, заявкам, поставщикам...</span>
        <kbd>⌘ K</kbd>
      </button>

      <div className="topbar-actions">
        <button className="btn btn-quiet topbar-action-btn" type="button" onClick={() => onNavigate('ai-studio')}><AppIcon name="sparkles" /><span>Создать AI</span></button>
        <span className={'deploy-pill ' + connectionState}><i />Бэкенд: {apiStatusLabels[connectionState]}</span>
        <button className="topbar-icon" type="button" onClick={() => onNavigate('settings')} aria-label="Настройки"><AppIcon name="settings" /></button>
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
