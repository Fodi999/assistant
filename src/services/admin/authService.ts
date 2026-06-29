import { clearAdminToken, getAdminToken, setAdminToken } from '../../api/client';
import { adminConfig } from '../../config/adminConfig';
import type { LoginResponse } from '../../types/admin';
import { adminApiClient } from './adminApiClient';

export type AdminAuthVerifyResponse = {
  message: string;
  role: string;
};

export async function loginAdmin(email: string, password: string): Promise<void> {
  const response = await adminApiClient.post<LoginResponse>('/api/admin/auth/login', { email, password });
  setAdminToken(response.token);
}

export async function verifyAdminSession(): Promise<boolean> {
  if (!adminConfig.isApiMode) {
    return true;
  }

  if (!getAdminToken()) {
    return false;
  }

  try {
    await adminApiClient.get<AdminAuthVerifyResponse>('/api/admin/auth/verify');
    return true;
  } catch {
    clearAdminToken();
    return false;
  }
}

export function logoutAdmin(): void {
  clearAdminToken();
}

export function hasAdminToken(): boolean {
  return Boolean(getAdminToken());
}
