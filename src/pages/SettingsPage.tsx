import { StatusBadge } from '../components/StatusBadge';

interface SettingsPageProps {
  tokenReady: boolean;
  status: string;
  onReload: () => Promise<void>;
  onLogout: () => void;
}

export function SettingsPage({ tokenReady, status, onReload, onLogout }: SettingsPageProps) {
  return (
    <section className="settings-page">
      <article className="page-card">
        <div className="section-head"><h2>Диагностика подключения</h2></div>
        <div className="diagnostics-grid">
          <div className="diag-item"><p className="diag-label">Admin JWT</p><StatusBadge tone={tokenReady ? 'ok' : 'danger'} label={tokenReady ? 'Проверен' : 'Отсутствует'} /></div>
          <div className="diag-item"><p className="diag-label">Backend</p><p className="diag-value">{status}</p></div>
          <div className="diag-item"><p className="diag-label">Действия</p><button className="btn btn-primary" onClick={() => void onReload()}>Обновить данные</button></div>
        </div>
      </article>
      <article className="page-card">
        <div className="section-head"><h2>Сессия</h2></div>
        <p className="page-muted">Токен хранится локально под ключом admin_token и отправляется как Bearer token.</p>
        <div className="settings-actions"><button className="btn btn-secondary" onClick={onLogout}>Выйти из admin API</button></div>
      </article>
    </section>
  );
}
