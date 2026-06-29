import type { SiteId, User } from '../../../types/admin';
import { baseRow, updatedLabel } from './shared';

type BackendUser = {
  id: string;
  email: string;
  name?: string | null;
  restaurant_name?: string;
  language?: string;
  created_at?: string;
  last_login_at?: string | null;
  login_count?: number;
};

type BackendUsersResponse = {
  users: BackendUser[];
  total: number;
};

export const usersAdapter = {
  normalizeList(response: BackendUsersResponse | BackendUser[]) {
    return Array.isArray(response) ? response : response.users;
  },

  fromBackend(user: BackendUser, siteId: SiteId): User {
    const updatedAt = user.last_login_at || user.created_at;
    return {
      ...baseRow({
        id: user.id,
        title: user.name || user.email,
        type: user.language || 'User',
        status: 'active',
        owner: user.restaurant_name || 'Tenant',
        updated: updatedLabel(updatedAt),
        updatedAt: updatedAt || undefined,
        metric: `${user.login_count ?? 0} logins`
      }, siteId),
      resource: 'users' as const,
      email: user.email,
      role: 'admin'
    };
  }
};
