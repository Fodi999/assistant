import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { adminConfig } from '../../config/adminConfig';
import { hasAdminToken, loginAdmin, logoutAdmin, verifyAdminSession } from '../../services/admin/authService';

type AdminAuthState = 'checking' | 'authenticated' | 'anonymous';

type AdminAuthContextValue = {
  authState: AdminAuthState;
  authenticated: boolean;
  authError: string | null;
  loading: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
  refresh: () => Promise<void>;
};

const AdminAuthContext = createContext<AdminAuthContextValue | null>(null);

export function AdminAuthProvider({ children }: { children: ReactNode }) {
  const [authState, setAuthState] = useState<AdminAuthState>('checking');
  const [authError, setAuthError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!adminConfig.isApiMode) {
      setAuthState('authenticated');
      setAuthError(null);
      return;
    }

    if (!hasAdminToken()) {
      setAuthState('anonymous');
      return;
    }

    setAuthState('checking');
    const valid = await verifyAdminSession();
    setAuthState(valid ? 'authenticated' : 'anonymous');
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    function requireAuth() {
      if (adminConfig.isApiMode) {
        setAuthState('anonymous');
      }
    }

    window.addEventListener('admin-auth-required', requireAuth);
    return () => window.removeEventListener('admin-auth-required', requireAuth);
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    setLoading(true);
    setAuthError(null);

    try {
      await loginAdmin(email, password);
      const valid = await verifyAdminSession();
      if (!valid) throw new Error('Backend не подтвердил admin token');
      setAuthState('authenticated');
    } catch (error) {
      setAuthError(error instanceof Error ? error.message : 'Ошибка входа');
      setAuthState('anonymous');
    } finally {
      setLoading(false);
    }
  }, []);

  const logout = useCallback(() => {
    logoutAdmin();
    setAuthState(adminConfig.isApiMode ? 'anonymous' : 'authenticated');
  }, []);

  const value = useMemo(() => ({
    authState,
    authenticated: authState === 'authenticated',
    authError,
    loading,
    login,
    logout,
    refresh
  }), [authError, authState, loading, login, logout, refresh]);

  return <AdminAuthContext.Provider value={value}>{children}</AdminAuthContext.Provider>;
}

export function useAdminAuth() {
  const context = useContext(AdminAuthContext);

  if (!context) {
    throw new Error('useAdminAuth must be used inside AdminAuthProvider');
  }

  return context;
}
