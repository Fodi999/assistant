import { useEffect, useMemo, useState } from 'react';
import { siteConfigs } from '../lib/siteConfig';
import { apiStatusLabels } from '../lib/labels';
import type { SiteKey } from '../types/admin';
import { AppIcon } from './AppIcon';
import { pageAllowedForSite, type AppPage } from './Sidebar';

interface TopbarProps {
  activeSite: SiteKey;
  activePage: AppPage;
  connectionState: 'online' | 'limited' | 'offline';
  onNavigate: (page: AppPage) => void;
  onLogout: () => void;
}

const PAGE_LABELS: Record<AppPage, string> = {
  dashboard: 'Панель',
  affiliate: 'Партнерка',
  content: 'Контент',
  construction: 'Стройка',
  culinary: 'Кулинария',
  icons: 'Иконы',
  leads: 'Заявки',
  suppliers: 'Поставщики',
  analytics: 'Аналитика',
  users: 'Пользователи',
  settings: 'Настройки',
  about: 'О системе'
};

const COMMANDS: Array<{ page: AppPage; title: string; hint: string; site?: SiteKey }> = [
  { page: 'content', title: 'Контент кулинарного сайта', hint: 'статьи / обзоры / подборки / фото', site: 'culinary' },
  { page: 'culinary', title: 'Ингредиенты CU', hint: 'каталог ингредиентов и кулинарные данные', site: 'culinary' },
  { page: 'content', title: 'Контент Kazaxbud', hint: 'материалы / товары / комплекты / проекты', site: 'construction' },
  { page: 'suppliers', title: 'Поставщики CO', hint: 'Алматы, контакты, условия, маржа', site: 'construction' },
  { page: 'content', title: 'Контент сайта икон', hint: 'иконы / молитвы / святые / QR / SEO', site: 'icons' },
  { page: 'affiliate', title: 'Affiliate / поставки', hint: 'партнерские товары выбранного сайта' },
  { page: 'leads', title: 'Посмотреть заявки', hint: 'новые / в работе / смета / выиграно / потеряно' },
  { page: 'analytics', title: 'Аналитика сайта', hint: 'GA4, клики, доход, конверсии' }
];

export function Topbar({ activeSite, activePage, connectionState, onNavigate, onLogout }: TopbarProps) {
  const [commandOpen, setCommandOpen] = useState(false);
  const [query, setQuery] = useState('');
  const site = siteConfigs.find((item) => item.key === activeSite) ?? siteConfigs[0];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const siteCommands = COMMANDS.filter((command) => (!command.site || command.site === activeSite) && pageAllowedForSite(activeSite, command.page));
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
    <header className="topbar new-topbar">
      <div className="topbar-context">
        <p className="eyebrow">{PAGE_LABELS[activePage]}</p>
        <h1>{site.name}</h1>
        <div className="topbar-meta">
          <span><AppIcon name="globe" />{site.domain}</span>
          <span><AppIcon name="cloud" />{site.region}</span>
        </div>
      </div>

      <button className="command-search" type="button" onClick={() => setCommandOpen(true)}>
        <AppIcon name="search" />
        <span>Поиск по товарам, статьям, заявкам, поставщикам...</span>
        <kbd>⌘ K</kbd>
      </button>

      <div className="topbar-actions">
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
