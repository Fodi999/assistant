import { isApiMode } from '../../config/adminConfig';
import { adminApiClient } from './adminApiClient';
import { adminHealthRoute } from './adminApiRoutes';

export type AdminApiHealth = {
  status: string;
  ok: boolean;
  message?: string;
};

export async function checkApiHealth(): Promise<AdminApiHealth> {
  if (!isApiMode) {
    return { status: 'mock', ok: true };
  }

  const response = await adminApiClient.get<AdminApiHealth | string>(adminHealthRoute);

  if (typeof response === 'string') {
    return { status: response.trim() || 'unknown', ok: response.trim().toUpperCase() === 'OK' };
  }

  return {
    status: response.status || 'healthy',
    ok: response.ok ?? true,
    message: response.message
  };
}
