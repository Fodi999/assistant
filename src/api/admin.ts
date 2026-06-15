import { apiFetch } from './client';
import type { AdminStats, AdminUsersResponse } from '../types/admin';

export function getAdminStats(): Promise<AdminStats> {
  return apiFetch<AdminStats>('/api/admin/stats');
}

export function listAdminUsers(): Promise<AdminUsersResponse> {
  return apiFetch<AdminUsersResponse>('/api/admin/users');
}

export function deleteAdminUser(userId: string): Promise<{ message: string }> {
  return apiFetch<{ message: string }>(`/api/admin/users/${userId}`, {
    method: 'DELETE'
  });
}
