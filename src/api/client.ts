const API_BASE = String(import.meta.env.VITE_API_BASE_URL || 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app').replace(/\/+$/, '');
const ADMIN_TOKEN_KEY = 'admin_token';
const STATIC_ADMIN_TOKEN = (import.meta.env.VITE_ADMIN_STATIC_TOKEN || '').trim();

export function getAdminToken(): string | null {
  return localStorage.getItem(ADMIN_TOKEN_KEY);
}

export function setAdminToken(token: string): void {
  localStorage.setItem(ADMIN_TOKEN_KEY, token);
}

export function bootstrapAdminToken(): boolean {
  if (!STATIC_ADMIN_TOKEN) {
    return false;
  }

  if (localStorage.getItem(ADMIN_TOKEN_KEY) !== STATIC_ADMIN_TOKEN) {
    localStorage.setItem(ADMIN_TOKEN_KEY, STATIC_ADMIN_TOKEN);
  }

  return true;
}

export function clearAdminToken(): void {
  localStorage.removeItem(ADMIN_TOKEN_KEY);
}

export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = getAdminToken();
  const headers = new Headers(init.headers || {});

  if (!headers.has('Content-Type') && init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      ...init,
      headers
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Network error';
    throw new Error(`API недоступен (${API_BASE}). ${message}`);
  }

  if (!response.ok) {
    const payload = (await response.json().catch(() => ({}))) as {
      message?: string;
      error?: string;
      details?: string;
      code?: string;
    };
    throw new Error(payload.details || payload.message || payload.error || payload.code || `HTTP ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}
