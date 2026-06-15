import { useEffect, useState } from 'react';
import { deleteAdminUser, getAdminStats, listAdminUsers } from './api/admin';
import { getAnalyticsOverview, getAnalyticsRealtime, getSearchConsoleBundle, type AnalyticsOverview, type AnalyticsRealtime, type SearchConsoleBundle } from './api/analytics';
import { getAlmabuildContent, type AlmabuildContent } from './api/almabuild';
import { adminLogin, adminLogout, verifyAdminToken } from './api/auth';
import { listAdminCategories, listAdminProducts } from './api/catalog';
import { listArticles } from './api/cms';
import { listShopProducts } from './api/shop';
import { bootstrapAdminToken, getAdminToken } from './api/client';
import { Sidebar, type AppPage, type ManagedSite } from './components/Sidebar';
import { Topbar } from './components/Topbar';
import { LoginPage } from './pages/LoginPage';
import { OperationsPage } from './pages/OperationsPage';
import type { AdminCategory, AdminProduct, AdminStats, AdminUser, CmsArticle, ShopProduct } from './types/admin';

type AnalyticsPeriod = 7 | 28 | 90;
type PageBySite = Record<ManagedSite, AppPage>;

function pageAllowedForSite(site: ManagedSite, page: AppPage) {
  if (site === 'almabuild' && page === 'catalog') return false;
  if (site === 'dima' && page === 'materials') return false;
  if (site === 'dima' && page === 'projects') return false;
  return true;
}

function normalizeSitePage(site: ManagedSite, page: AppPage): AppPage {
  if (pageAllowedForSite(site, page)) return page;
  return site === 'almabuild' ? 'materials' : 'catalog';
}

export function App() {
  const [activeSite, setActiveSite] = useState<ManagedSite>('almabuild');
  const [pageBySite, setPageBySite] = useState<PageBySite>({ almabuild: 'overview', dima: 'overview' });
  const activePage = normalizeSitePage(activeSite, pageBySite[activeSite]);
  const [authState, setAuthState] = useState<'checking' | 'authenticated' | 'anonymous'>('checking');
  const [authError, setAuthError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [dataError, setDataError] = useState<string | null>(null);
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [products, setProducts] = useState<AdminProduct[]>([]);
  const [categories, setCategories] = useState<AdminCategory[]>([]);
  const [articles, setArticles] = useState<CmsArticle[]>([]);
  const [shopProducts, setShopProducts] = useState<ShopProduct[]>([]);
  const [almabuildContent, setAlmabuildContent] = useState<AlmabuildContent | null>(null);
  const [analytics, setAnalytics] = useState<AnalyticsOverview | null>(null);
  const [realtimeAnalytics, setRealtimeAnalytics] = useState<AnalyticsRealtime | null>(null);
  const [searchConsole, setSearchConsole] = useState<SearchConsoleBundle | null>(null);
  const [analyticsPeriod] = useState<AnalyticsPeriod>(28);

  function navigate(page: AppPage) {
    if (!pageAllowedForSite(activeSite, page)) return;
    setPageBySite((current) => ({ ...current, [activeSite]: page }));
  }

  function changeSite(site: ManagedSite) {
    setPageBySite((current) => ({ ...current, [site]: normalizeSitePage(site, current[site]) }));
    setActiveSite(site);
  }

  async function loadRealtimeAnalytics() {
    if (activeSite !== 'dima') return;
    try {
      setRealtimeAnalytics(await getAnalyticsRealtime());
    } catch {
      setRealtimeAnalytics(null);
    }
  }

  async function loadAdminData(days = analyticsPeriod) {
    setLoading(true);
    setDataError(null);
    try {
      const [nextStats, nextUsers, nextProducts, nextCategories, nextArticles, nextShopProducts, nextAlmabuild, nextAnalytics, nextRealtime, nextSearchConsole] = await Promise.all([
        getAdminStats(),
        listAdminUsers(),
        listAdminProducts(),
        listAdminCategories(),
        listArticles(),
        listShopProducts(),
        getAlmabuildContent().catch(() => null),
        getAnalyticsOverview(days).catch(() => null),
        getAnalyticsRealtime().catch(() => null),
        getSearchConsoleBundle(days).catch(() => null)
      ]);
      setStats(nextStats);
      setUsers(nextUsers.users);
      setProducts(nextProducts);
      setCategories(nextCategories);
      setArticles(nextArticles);
      setShopProducts(nextShopProducts);
      setAlmabuildContent(nextAlmabuild);
      setAnalytics(nextAnalytics);
      setRealtimeAnalytics(nextRealtime);
      setSearchConsole(nextSearchConsole);
    } catch (error) {
      setDataError(error instanceof Error ? error.message : 'Не удалось загрузить данные');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    bootstrapAdminToken();
    if (!getAdminToken()) {
      setAuthState('anonymous');
      return;
    }
    void verifyAdminToken().then((valid) => {
      setAuthState(valid ? 'authenticated' : 'anonymous');
      if (valid) void loadAdminData();
    });
  }, []);

  useEffect(() => {
    if (authState !== 'authenticated') return;
    const timer = window.setInterval(() => {
      void loadRealtimeAnalytics();
    }, 30000);
    return () => window.clearInterval(timer);
  }, [authState, activeSite]);

  async function login(email: string, password: string) {
    setLoading(true);
    setAuthError(null);
    try {
      await adminLogin(email, password);
      const valid = await verifyAdminToken();
      if (!valid) throw new Error('Backend не подтвердил admin token');
      setAuthState('authenticated');
      await loadAdminData();
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

  async function removeUser(user: AdminUser) {
    if (!window.confirm('Удалить ' + user.email + ', ресторан и все связанные данные? Это необратимо.')) return;
    setLoading(true);
    setDataError(null);
    try {
      await deleteAdminUser(user.id);
      await loadAdminData();
    } catch (error) {
      setDataError(error instanceof Error ? error.message : 'Не удалось удалить пользователя');
    } finally {
      setLoading(false);
    }
  }

  void removeUser;

  if (authState === 'checking') return <main className="login-page"><p className="page-muted">Проверяем подключение к backend...</p></main>;
  if (authState === 'anonymous') return <LoginPage loading={loading} error={authError} onLogin={login} />;

  const reload = async () => { await loadAdminData(analyticsPeriod); };

  return (
    <div className="admin-shell">
      <Sidebar activeSite={activeSite} activePage={activePage} onSiteChange={changeSite} onNavigate={navigate} />
      <div className="content-shell">
        <Topbar activeSite={activeSite} activePage={activePage} connectionState={dataError ? 'limited' : 'online'} onNavigate={navigate} onLogout={logout} />
        <main className="page-content">
          <OperationsPage
            page={activePage}
            activeSite={activeSite}
            stats={stats}
            users={users}
            products={products}
            categories={categories}
            articles={articles}
            shopProducts={shopProducts}
            almabuildContent={almabuildContent}
            analytics={activeSite === 'dima' ? analytics : null}
            realtime={activeSite === 'dima' ? realtimeAnalytics : null}
            searchConsole={activeSite === 'dima' ? searchConsole : null}
            loading={loading}
            error={dataError}
            onRefresh={reload}
          />
        </main>
      </div>
    </div>
  );
}
