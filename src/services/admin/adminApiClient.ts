import { clearAdminToken, getAdminToken } from '../../api/client';
import { adminConfig } from '../../config/adminConfig';

export type RequestBody = BodyInit | Record<string, unknown> | unknown[] | null | undefined;

export class AdminApiError extends Error {
  status?: number;
  code: string;

  constructor(message: string, code: string, status?: number) {
    super(message);
    this.name = 'AdminApiError';
    this.code = code;
    this.status = status;
  }
}

function buildUrl(path: string): string {
  if (!adminConfig.apiUrl) {
    throw new Error('VITE_API_URL is required when VITE_DATA_MODE=api');
  }

  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  return `${adminConfig.apiUrl}${normalizedPath}`;
}

function buildInit(method: string, body?: RequestBody): RequestInit {
  const headers = new Headers();
  const token = getAdminToken();
  let requestBody: BodyInit | undefined;

  if (body instanceof FormData || body instanceof URLSearchParams || body instanceof Blob) {
    requestBody = body;
  } else if (body !== undefined && body !== null) {
    headers.set('Content-Type', 'application/json');
    requestBody = JSON.stringify(body);
  }

  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  return {
    method,
    headers,
    body: requestBody
  };
}

async function request<T>(method: string, path: string, body?: RequestBody): Promise<T> {
  let response: Response;

  try {
    response = await fetch(buildUrl(path), buildInit(method, body));
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Network error';
    throw new AdminApiError(`Backend недоступен. ${message}`, 'NETWORK');
  }

  if (!response.ok) {
    const text = await response.text().catch(() => '');
    let payload: { message?: string; error?: string; details?: string; code?: string } = {};

    if (text) {
      try {
        payload = JSON.parse(text) as typeof payload;
      } catch {
        payload = { message: text };
      }
    }

    if (response.status === 401) {
      clearAdminToken();
      window.dispatchEvent(new Event('admin-auth-required'));
      throw new AdminApiError('Нужно войти в админку', 'AUTH_REQUIRED', 401);
    }

    if (response.status === 404) {
      throw new AdminApiError('Endpoint не найден', payload.code || 'NOT_FOUND', 404);
    }

    const message = payload.details || payload.message || payload.error || payload.code || response.statusText;

    if (response.status === 400) {
      throw new AdminApiError(message || 'Validation error', payload.code || 'VALIDATION_ERROR', 400);
    }

    if (response.status >= 500) {
      throw new AdminApiError('Ошибка backend', payload.code || 'BACKEND_ERROR', response.status);
    }

    throw new AdminApiError(message, payload.code || 'API_ERROR', response.status);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get('content-type') || '';
  const text = await response.text();

  if (!text) {
    return undefined as T;
  }

  if (contentType.includes('application/json')) {
    return JSON.parse(text) as T;
  }

  return text as T;
}

export const adminApiClient = {
  get: <T>(path: string) => request<T>('GET', path),
  post: <T>(path: string, body?: RequestBody) => request<T>('POST', path, body),
  patch: <T>(path: string, body?: RequestBody) => request<T>('PATCH', path, body),
  put: <T>(path: string, body?: RequestBody) => request<T>('PUT', path, body),
  delete: <T>(path: string) => request<T>('DELETE', path)
};
