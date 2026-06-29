import { useEffect, useState } from 'react';
import { listAdminUsers } from '../../api/admin';
import { AppIcon } from '../../components/AppIcon';
import { DataSourceBadge, type DataSource } from '../../components/DataSourceBadge';
import type { AdminUser } from '../../types/admin';

export function UsersPage() {
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [source, setSource] = useState<DataSource>('unavailable');
  const [sourceError, setSourceError] = useState<string | undefined>();

  useEffect(() => {
    void listAdminUsers()
      .then((response) => {
        setUsers(response.users);
        setSource('api');
        setSourceError(undefined);
      })
      .catch((error) => {
        setUsers([]);
        setSource('unavailable');
        setSourceError(error instanceof Error ? error.message : 'API недоступен');
      });
  }, []);

  return (
    <section className="ops-page">
      <div className="ops-header">
        <div className="ops-header-icon"><AppIcon name="users" /></div>
        <div>
          <p className="eyebrow">Управление доступом</p>
          <h2>Пользователи</h2>
          <p>Суперадминистраторы, редакторы партнерского каталога, контент-редакторы и операторы заявок.</p>
        </div>
        <div className="ops-header-actions"><DataSourceBadge source={source} label="Пользователи" /></div>
      </div>
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул пользователей: {sourceError}. Демо-данные отключены.</p> : null}
      <section className="ops-panel">
        <table className="ops-table">
          <thead><tr><th>Пользователь</th><th>Ресторан / объект</th><th>Язык</th><th>Входы</th><th>Последний вход</th></tr></thead>
          <tbody>
            {users.map((user) => (
              <tr key={user.id}>
                <td><strong>{user.name || user.email}</strong><small>{user.email}</small></td>
                <td>{user.restaurant_name}</td>
                <td>{user.language.toUpperCase()}</td>
                <td>{user.login_count}</td>
                <td>{user.last_login_at ? new Date(user.last_login_at).toLocaleString('ru-RU') : '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {users.length === 0 ? <p className="empty-state">Пользователей из backend нет.</p> : null}
      </section>
    </section>
  );
}
