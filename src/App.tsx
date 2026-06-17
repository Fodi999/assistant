import { useEffect, useState } from 'react';
import { adminLogin, adminLogout, verifyAdminToken } from './api/auth';
import { bootstrapAdminToken, getAdminToken } from './api/client';
import { defaultPageForSite, normalizeSitePage, Sidebar, type AppPage } from './components/Sidebar';
import { Topbar } from './components/Topbar';
import { AlmabuildPage } from './pages/construction/AlmabuildPage';
import { ContentPage } from './pages/culinary/ContentPage';
import { CulinaryPage } from './pages/culinary/CulinaryPage';
import { AboutPage } from './pages/shared/AboutPage';
import { AffiliatePage } from './pages/shared/AffiliatePage';
import { AiStudioPage } from './pages/shared/AiStudioPage';
import { AnalyticsPage } from './pages/shared/AnalyticsPage';
import { DashboardPage } from './pages/shared/DashboardPage';
import { LeadsPage } from './pages/shared/LeadsPage';
import { LoginPage } from './pages/shared/LoginPage';
import { SettingsPage } from './pages/shared/SettingsPage';
import { SitesPage } from './pages/shared/SitesPage';
import { SuppliersPage } from './pages/shared/SuppliersPage';
import { UsersPage } from './pages/shared/UsersPage';
import type { SiteKey } from './types/admin';

type PageBySite = Record<SiteKey, AppPage>;

export function App() {
  const [activeSite, setActiveSite] = useState<SiteKey>('construction');
  const [pageBySite, setPageBySite] = useState<PageBySite>({
    culinary: defaultPageForSite('culinary'),
    construction: defaultPageForSite('construction')
  });
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
    setPageBySite((current) => ({
      ...current,
      [activeSite]: normalizeSitePage(activeSite, current[activeSite]),
      [site]: normalizeSitePage(site, current[site] || defaultPageForSite(site))
    }));
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
      case 'construction': return <AlmabuildPage />;
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
