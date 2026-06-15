import { useEffect, useState } from 'react';
import { adminLogin, adminLogout, verifyAdminToken } from './api/auth';
import { bootstrapAdminToken, getAdminToken } from './api/client';
import { normalizeSitePage, Sidebar, type AppPage } from './components/Sidebar';
import { Topbar } from './components/Topbar';
import { AboutPage } from './pages/AboutPage';
import { AffiliatePage } from './pages/AffiliatePage';
import { AiStudioPage } from './pages/AiStudioPage';
import { AnalyticsPage } from './pages/AnalyticsPage';
import { ConstructionPage } from './pages/ConstructionPage';
import { ContentPage } from './pages/ContentPage';
import { CulinaryPage } from './pages/CulinaryPage';
import { DashboardPage } from './pages/DashboardPage';
import { LeadsPage } from './pages/LeadsPage';
import { LoginPage } from './pages/LoginPage';
import { SettingsPage } from './pages/SettingsPage';
import { SitesPage } from './pages/SitesPage';
import { SuppliersPage } from './pages/SuppliersPage';
import { UsersPage } from './pages/UsersPage';
import type { SiteKey } from './types/admin';

type PageBySite = Record<SiteKey, AppPage>;

export function App() {
  const [activeSite, setActiveSite] = useState<SiteKey>('construction');
  const [pageBySite, setPageBySite] = useState<PageBySite>({ culinary: 'dashboard', construction: 'dashboard' });
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem('admin_sidebar_collapsed') === 'true');
  const activePage = normalizeSitePage(activeSite, pageBySite[activeSite]);
  const [authState, setAuthState] = useState<'checking' | 'authenticated' | 'anonymous'>('checking');
  const [authError, setAuthError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [dataError] = useState<string | null>(null);

  function navigate(page: AppPage) {
    setPageBySite((current) => ({ ...current, [activeSite]: normalizeSitePage(activeSite, page) }));
  }

  function changeSite(site: SiteKey) {
    setPageBySite((current) => ({ ...current, [site]: normalizeSitePage(site, current[site]) }));
    setActiveSite(site);
  }

  function toggleSidebar() {
    setSidebarCollapsed((current) => {
      const next = !current;
      localStorage.setItem('admin_sidebar_collapsed', String(next));
      return next;
    });
  }

  useEffect(() => {
    bootstrapAdminToken();
    if (!getAdminToken()) {
      setAuthState('anonymous');
      return;
    }
    void verifyAdminToken().then((valid) => {
      setAuthState(valid ? 'authenticated' : 'anonymous');
    });
  }, []);

  async function login(email: string, password: string) {
    setLoading(true);
    setAuthError(null);
    try {
      await adminLogin(email, password);
      const valid = await verifyAdminToken();
      if (!valid) throw new Error('Backend не подтвердил admin token');
      setAuthState('authenticated');
    } catch (error) {
      setAuthError(error instanceof Error ? error.message : 'Ошибка входа');
      setAuthState('anonymous');
    } finally {
      setLoading(false);
    }
  }

  function logout() {
    adminLogout();
    setAuthState('anonymous');
  }

  function renderPage() {
    switch (activePage) {
      case 'dashboard': return <DashboardPage activeSite={activeSite} />;
      case 'sites': return <SitesPage />;
      case 'affiliate': return <AffiliatePage activeSite={activeSite} />;
      case 'content': return <ContentPage activeSite={activeSite} />;
      case 'ai-studio': return <AiStudioPage activeSite={activeSite} />;
      case 'construction': return <ConstructionPage />;
      case 'culinary': return <CulinaryPage />;
      case 'leads': return <LeadsPage activeSite={activeSite} />;
      case 'suppliers': return <SuppliersPage />;
      case 'analytics': return <AnalyticsPage activeSite={activeSite} />;
      case 'users': return <UsersPage />;
      case 'settings': return <SettingsPage />;
      case 'about': return <AboutPage />;
      default: return <DashboardPage activeSite={activeSite} />;
    }
  }

  if (authState === 'checking') return <main className="login-page"><p className="page-muted">Проверяем подключение к backend...</p></main>;
  if (authState === 'anonymous') return <LoginPage loading={loading} error={authError} onLogin={login} />;

  return (
    <div className={'admin-shell' + (sidebarCollapsed ? ' sidebar-collapsed' : '')}>
      <Sidebar activeSite={activeSite} activePage={activePage} collapsed={sidebarCollapsed} onToggleCollapse={toggleSidebar} onSiteChange={changeSite} onNavigate={navigate} />
      <div className="content-shell">
        <Topbar activeSite={activeSite} activePage={activePage} connectionState={dataError ? 'limited' : 'online'} onSiteChange={changeSite} onNavigate={navigate} onLogout={logout} />
        <main className="page-content">{renderPage()}</main>
      </div>
    </div>
  );
}
