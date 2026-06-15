import { apiFetch, clearAdminToken, setAdminToken } from './client';
import type { LoginResponse } from '../types/admin';

export async function adminLogin(email: string, password: string): Promise<void> {
  const data = await apiFetch<LoginResponse>('/api/admin/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password })
  });

  setAdminToken(data.token);
}

export async function verifyAdminToken(): Promise<boolean> {
  try {
    await apiFetch<{ message: string; role: string }>('/api/admin/auth/verify', { method: 'GET' });
    return true;
  } catch {
    clearAdminToken();
    return false;
  }
}

export function adminLogout(): void {
  clearAdminToken();
}
