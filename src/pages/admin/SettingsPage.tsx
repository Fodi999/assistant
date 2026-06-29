import { useCallback, useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { AdminPanel } from '../../components/admin/AdminPanel';
import { AdminState } from '../../components/admin/AdminState';
import { StatusBadge } from '../../components/admin/StatusBadge';
import { useAdminAuth } from '../../components/admin/AuthContext';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { adminConfig } from '../../config/adminConfig';
import { useActiveSite } from '../../lib/useActiveSite';
import { DataTable } from '../../components/admin/DataTable';
import {
  getGoogleAnalyticsConnectionStatus,
  getGoogleAnalyticsReconnectUrl,
  runDiagnostics,
  updateGoogleAnalyticsConnection,
  type AdminDiagnosticResult,
  type GoogleAnalyticsConnectionStatus
} from '../../services/admin/diagnosticsService';
import { checkApiHealth, type AdminApiHealth } from '../../services/admin/healthService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';
import { getSiteSettings, listSettingsRows } from '../../services/admin/settingsService';
import type { SiteId, SiteSettings } from '../../types/admin';
import { AdminResourcePage } from './AdminResourcePage';

export function SettingsPage() {
  const { activeSiteId, activeSite } = useActiveSite();
  const { authenticated, authState, logout } = useAdminAuth();
  const toast = useAdminToast();
  const [settings, setSettings] = useState<SiteSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [health, setHealth] = useState<AdminApiHealth | null>(adminConfig.isApiMode ? null : { status: 'mock', ok: true });
  const [checkingHealth, setCheckingHealth] = useState(false);
  const [diagnostics, setDiagnostics] = useState<AdminDiagnosticResult[]>([]);
  const [runningDiagnostics, setRunningDiagnostics] = useState(false);
  const [reconnectingAnalytics, setReconnectingAnalytics] = useState(false);
  const [analyticsConnection, setAnalyticsConnection] = useState<GoogleAnalyticsConnectionStatus | null>(null);
  const [loadingAnalyticsConnection, setLoadingAnalyticsConnection] = useState(false);
  const [savingAnalyticsConnection, setSavingAnalyticsConnection] = useState(false);
  const [analyticsPropertyId, setAnalyticsPropertyId] = useState('');

  const loadSettings = useCallback(() => {
    setLoading(true);
    setError(null);
    void getSiteSettings(activeSiteId as SiteId)
      .then(setSettings)
      .catch((loadError) => setError(loadError instanceof Error ? loadError.message : 'Unknown error'))
      .finally(() => setLoading(false));
  }, [activeSiteId]);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  useEffect(() => {
    setLoadingAnalyticsConnection(true);
    void getGoogleAnalyticsConnectionStatus(activeSiteId as SiteId)
      .then((connection) => {
        setAnalyticsConnection(connection);
        setAnalyticsPropertyId(connection.google_property_id || '');
      })
      .catch((connectionError) => {
        setAnalyticsConnection({
          site_id: activeSiteId,
          status: 'error',
          google_property_id: null,
          connected_at: null,
          has_refresh_token: false,
          source: connectionError instanceof Error ? connectionError.message : 'error'
        });
        setAnalyticsPropertyId('');
      })
      .finally(() => setLoadingAnalyticsConnection(false));
  }, [activeSiteId]);

  async function checkConnection() {
    setCheckingHealth(true);

    try {
      const nextHealth = await checkApiHealth();
      setHealth(nextHealth);

      if (nextHealth.status === 'mock') {
        toast.info('Mock mode активен.');
      } else if (nextHealth.ok) {
        toast.success('API доступен.');
      } else {
        toast.error(nextHealth.message || 'API недоступен.');
      }
    } catch (healthError) {
      const message = healthError instanceof Error ? healthError.message : 'API недоступен.';
      setHealth({ status: 'error', ok: false, message });
      toast.error(message);
    } finally {
      setCheckingHealth(false);
    }
  }

  async function handleSaveAnalyticsConnection() {
    if (!adminConfig.isApiMode) {
      toast.info('Mock mode активен.');
      return;
    }

    setSavingAnalyticsConnection(true);

    try {
      const nextConnection = await updateGoogleAnalyticsConnection(activeSiteId as SiteId, {
        google_property_id: analyticsPropertyId.trim() || null
      });
      setAnalyticsConnection(nextConnection);
      setAnalyticsPropertyId(nextConnection.google_property_id || '');
      toast.success('Google Analytics connection saved.');
    } catch (saveError) {
      toast.error(saveError instanceof Error ? saveError.message : 'Google Analytics connection save failed.');
    } finally {
      setSavingAnalyticsConnection(false);
    }
  }

  async function handleRunDiagnostics() {
    setRunningDiagnostics(true);

    try {
      const results = await runDiagnostics(activeSiteId as SiteId);
      setDiagnostics(results);

      const failed = results.filter((result) => !result.ok);
      const analyticsAuthExpired = results.some((result) => result.code === 'ANALYTICS_AUTH_EXPIRED');
      if (failed.length) {
        toast.error(analyticsAuthExpired ? 'Google Analytics authorization expired.' : `${failed.length} checks failed.`);
      } else {
        toast.success('Backend diagnostics passed.');
      }
    } catch (diagnosticsError) {
      toast.error(diagnosticsError instanceof Error ? diagnosticsError.message : 'Diagnostics failed.');
    } finally {
      setRunningDiagnostics(false);
    }
  }

  async function handleReconnectGoogleAnalytics() {
    if (!adminConfig.isApiMode) {
      toast.info('Mock mode активен.');
      return;
    }

    setReconnectingAnalytics(true);

    try {
      const oauth = await getGoogleAnalyticsReconnectUrl(activeSiteId as SiteId);
      toast.info(`Opening Google Analytics reconnect for ${activeSite.name}.`);
      window.location.assign(oauth.url);
    } catch (reconnectError) {
      toast.error(reconnectError instanceof Error ? reconnectError.message : 'Google Analytics reconnect failed.');
    } finally {
      setReconnectingAnalytics(false);
    }
  }

  return (
    <section className="admin-resource-page">
      <AdminPageHeader
        title="Settings"
        eyebrow="Workspace"
        description="Site configuration prepared for backend settings endpoints."
        icon="settings"
        meta={<StatusBadge status={activeSite.status} label={activeSite.id} />}
      />

      <AdminPanel title="Active site" icon="settings">
        <AdminState loading={loading} error={error} empty={!settings} emptyTitle="No settings" onRetry={loadSettings}>
          {settings ? (
            <dl className="admin-settings-card">
              <div><dt>siteId</dt><dd>{settings.siteId}</dd></div>
              <div><dt>name</dt><dd>{settings.name}</dd></div>
              <div><dt>domain</dt><dd>{settings.domain}</dd></div>
              <div><dt>defaultLanguage</dt><dd>{settings.defaultLanguage}</dd></div>
              <div><dt>ga4Id</dt><dd>{settings.ga4Id}</dd></div>
              <div><dt>searchConsoleProperty</dt><dd>{settings.searchConsoleProperty}</dd></div>
              <div><dt>apiUrl</dt><dd>{settings.apiUrl}</dd></div>
              <div><dt>status</dt><dd><StatusBadge status={settings.status} /></dd></div>
            </dl>
          ) : null}
        </AdminState>
      </AdminPanel>

      <AdminPanel
        title="API подключение"
        icon="settings"
        meta={<ActionButton icon="sparkles" onClick={checkConnection} disabled={checkingHealth}>{checkingHealth ? 'Проверяем' : 'Проверить API'}</ActionButton>}
      >
        <dl className="admin-settings-card">
          <div><dt>Data mode</dt><dd>{adminConfig.dataMode}</dd></div>
          <div><dt>API URL</dt><dd>{adminConfig.apiUrl || 'not configured'}</dd></div>
          <div><dt>Connection status</dt><dd>{health ? `${health.status} (${health.ok ? 'ok' : 'error'})` : 'not checked'}</dd></div>
          <div><dt>Health endpoint</dt><dd>/health</dd></div>
          <div><dt>Version endpoint</dt><dd>/api/admin/version</dd></div>
          <div><dt>Auth status</dt><dd>{authenticated ? 'authenticated' : authState === 'checking' ? 'checking' : 'not authenticated'}</dd></div>
          <div><dt>Session</dt><dd>{authenticated ? <ActionButton onClick={logout}>Logout</ActionButton> : 'login required'}</dd></div>
        </dl>
      </AdminPanel>

      <AdminPanel
        title="Google Analytics"
        icon="analytics"
        meta={
          <ActionButton icon="external" onClick={handleReconnectGoogleAnalytics} disabled={reconnectingAnalytics}>
            {reconnectingAnalytics ? 'Opening' : 'Reconnect Google Analytics'}
          </ActionButton>
        }
      >
        <dl className="admin-settings-card">
          <div><dt>Active site</dt><dd>{activeSite.name}</dd></div>
          <div><dt>Status</dt><dd>{loadingAnalyticsConnection ? 'loading' : analyticsConnection?.status || 'not checked'}</dd></div>
          <div>
            <dt>GA property</dt>
            <dd>
              <input
                className="admin-inline-input"
                value={analyticsPropertyId}
                onChange={(event) => setAnalyticsPropertyId(event.target.value)}
                placeholder="properties/123456789 or 123456789"
              />
            </dd>
          </div>
          <div><dt>Refresh token</dt><dd>{analyticsConnection?.has_refresh_token ? 'stored' : 'missing'}</dd></div>
          <div><dt>Connected at</dt><dd>{analyticsConnection?.connected_at || 'not connected'}</dd></div>
          <div><dt>Config source</dt><dd>{analyticsConnection?.source || 'unknown'}</dd></div>
          <div><dt>Save</dt><dd><ActionButton icon="save" onClick={handleSaveAnalyticsConnection} disabled={savingAnalyticsConnection}>{savingAnalyticsConnection ? 'Saving' : 'Save property'}</ActionButton></dd></div>
        </dl>
      </AdminPanel>

      <AdminPanel
        title="Backend diagnostics"
        icon="settings"
        meta={<ActionButton icon="sparkles" onClick={handleRunDiagnostics} disabled={runningDiagnostics}>{runningDiagnostics ? 'Running' : 'Run diagnostics'}</ActionButton>}
      >
        <DataTable
          rows={diagnostics}
          getRowKey={(row) => row.key}
          empty={<p className="admin-table-empty">Diagnostics have not been run yet.</p>}
          columns={[
            { key: 'service', header: 'Service', render: (row) => <strong>{row.label}</strong> },
            { key: 'status', header: 'Status', render: (row) => <StatusBadge status={row.ok ? 'active' : 'warning'} label={String(row.status)} /> },
            { key: 'message', header: 'Message', render: (row) => row.message }
          ]}
        />
      </AdminPanel>

      <AdminResourcePage
        title="Settings records"
        eyebrow="Workspace"
        description="Local settings rows prepared for API integration."
        icon="settings"
        actionLabel="Add setting"
        loadRows={listSettingsRows}
        capabilities={resourceCapabilities.settings}
      />
    </section>
  );
}
