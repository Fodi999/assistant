import type { SiteId, User } from '../../types/admin';
import { usersAdapter } from './adapters/usersAdapter';
import { createAdminResourceService } from './resourceServiceFactory';

export const usersService = createAdminResourceService<User, 'users'>('users', (row) => ({ ...row, resource: 'users' }), usersAdapter);

export const list = usersService.list;
export const getById = usersService.getById;
export const create = usersService.create;
export const update = usersService.update;
export const remove = usersService.remove;

export function listUsers(siteId: SiteId): Promise<User[]> {
  return usersService.list(siteId);
}
