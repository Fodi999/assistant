import type { AdminUser } from '../types/admin';

interface UsersPageProps {
  users: AdminUser[];
  loading: boolean;
  error: string | null;
  onReload: () => Promise<void>;
  onDelete: (user: AdminUser) => Promise<void>;
}

function formatDate(value: string | null): string {
  if (!value) return '-';
  return new Date(value).toLocaleString('ru-RU');
}

export function UsersPage({ users, loading, error, onReload, onDelete }: UsersPageProps) {
  return (
    <section className="orders-page page-card">
      <div className="section-head">
        <div>
          <h2>Пользователи</h2>
          <p className="page-muted">Рестораны и владельцы аккаунтов: {users.length}</p>
        </div>
        <button className="btn btn-primary" onClick={() => void onReload()} disabled={loading}>Обновить</button>
      </div>

      {loading && <p className="page-muted">Загружаем пользователей...</p>}
      {error && <p className="form-error">{error}</p>}

      {!loading && !error && (
        <div className="orders-table-wrap">
          <table className="orders-table">
            <thead>
              <tr>
                <th>Пользователь</th>
                <th>Ресторан</th>
                <th>Язык</th>
                <th>Входы</th>
                <th>Последний вход</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {users.map((user) => (
                <tr key={user.id}>
                  <td><strong>{user.name || user.email}</strong><br /><span className="page-muted">{user.email}</span></td>
                  <td>{user.restaurant_name}</td>
                  <td>{user.language.toUpperCase()}</td>
                  <td>{user.login_count}</td>
                  <td>{formatDate(user.last_login_at)}</td>
                  <td>
                    <button className="btn btn-danger" onClick={() => void onDelete(user)}>Удалить</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
