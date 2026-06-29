import type { SiteId } from '../../types/admin';
import { isApiMode } from '../../config/adminConfig';
import { adminApiClient, AdminApiError } from './adminApiClient';
import { adminAnalyticsConnectionRoute, adminAnalyticsOAuthUrlRoute, adminApiRoutes, adminHealthRoute, adminVersionRoute } from './adminApiRoutes';

export type AdminDiagnosticResult = {
  key: string;
  label: string;
  ok: boolean;
  status: number | string;
  message: string;
  code?: string;
};

type DiagnosticCheck = {
  key: string;
  label: string;
  path: string;
};

function okMessage(response: unknown): string {
  if (typeof response === 'string') return response || 'OK';
  return 'OK';
}

function errorResult(check: DiagnosticCheck, error: unknown): AdminDiagnosticResult {
  if (error instanceof AdminApiError) {
    if (check.key === 'analytics' && isAnalyticsAuthExpired(error)) {
      return {
        key: check.key,
        label: check.label,
        ok: false,
        status: 'auth_expired',
        code: 'ANALYTICS_AUTH_EXPIRED',
        message: 'Google Analytics authorization expired. Reconnect Google Analytics.'
      };
    }

    return {
      key: check.key,
      label: check.label,
      ok: false,
      status: error.status || error.code,
      message: error.message,
      code: error.code
    };
  }

  return {
    key: check.key,
    label: check.label,
    ok: false,
    status: 'error',
    message: error instanceof Error ? error.message : 'Unknown error'
  };
}

function isAnalyticsAuthExpired(error: AdminApiError): boolean {
  const message = `${error.message} ${error.code}`.toLowerCase();
  return error.status === 400 && (
    message.includes('expired') ||
    message.includes('revoked') ||
    message.includes('invalid_grant') ||
    message.includes('token refresh')
  );
}

async function runCheck(check: DiagnosticCheck): Promise<AdminDiagnosticResult> {
  try {
    const response = await adminApiClient.get<unknown>(check.path);
    return {
      key: check.key,
      label: check.label,
      ok: true,
      status: 200,
      message: okMessage(response)
    };
  } catch (error) {
    return errorResult(check, error);
  }
}

export async function runDiagnostics(activeSiteId: SiteId): Promise<AdminDiagnosticResult[]> {
  const checks: DiagnosticCheck[] = [
    { key: 'health', label: 'Health', path: adminHealthRoute },
    { key: 'version', label: 'Backend version', path: adminVersionRoute },
    { key: 'auth', label: 'Auth verify', path: '/api/admin/auth/verify' },
    { key: 'catalog', label: 'Catalog products', path: adminApiRoutes.catalog.list(activeSiteId) },
    { key: 'cms', label: 'CMS articles', path: adminApiRoutes.cms.list(activeSiteId) },
    { key: 'shop', label: 'Shop products', path: adminApiRoutes.shop.list(activeSiteId) },
    { key: 'suppliers', label: 'Suppliers', path: adminApiRoutes.suppliers.list(activeSiteId) },
    { key: 'leads', label: 'Leads', path: adminApiRoutes.leads.list(activeSiteId) },
    { key: 'users', label: 'Users', path: adminApiRoutes.users.list(activeSiteId) },
    { key: 'analytics', label: 'Analytics overview', path: adminApiRoutes.analytics.list(activeSiteId) }
  ];

  if (!isApiMode) {
    return checks.map((check) => ({
      key: check.key,
      label: check.label,
      ok: true,
      status: 'mock',
      message: 'Mock mode active'
    }));
  }

  return Promise.all(checks.map(runCheck));
}

export type GoogleAnalyticsReconnectUrl = {
  url: string;
  redirect_uri: string;
  scope: string;
  site_id: string;
};

export type GoogleAnalyticsConnectionStatus = {
  site_id: string;
  status: 'not_connected' | 'connected' | 'expired' | 'error' | string;
  google_property_id?: string | null;
  connected_at?: string | null;
  has_refresh_token: boolean;
  connection_id?: string | null;
  source: string;
};

export async function getGoogleAnalyticsReconnectUrl(siteId: SiteId): Promise<GoogleAnalyticsReconnectUrl> {
  return adminApiClient.get<GoogleAnalyticsReconnectUrl>(adminAnalyticsOAuthUrlRoute(siteId));
}

export async function getGoogleAnalyticsConnectionStatus(siteId: SiteId): Promise<GoogleAnalyticsConnectionStatus> {
  if (!isApiMode) {
    return {
      site_id: siteId,
      status: 'mock',
      google_property_id: `G-${siteId.toUpperCase()}-MOCK`,
      connected_at: null,
      has_refresh_token: false,
      source: 'mock'
    };
  }

  return adminApiClient.get<GoogleAnalyticsConnectionStatus>(adminAnalyticsConnectionRoute(siteId));
}

export async function updateGoogleAnalyticsConnection(
  siteId: SiteId,
  payload: { google_property_id?: string | null }
): Promise<GoogleAnalyticsConnectionStatus> {
  return adminApiClient.patch<GoogleAnalyticsConnectionStatus>(adminAnalyticsConnectionRoute(siteId), payload);
}
