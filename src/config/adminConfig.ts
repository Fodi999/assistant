export type AdminDataMode = 'mock' | 'api';

const rawDataMode = String(import.meta.env.VITE_DATA_MODE || 'mock').trim().toLowerCase();
const apiUrl = String(import.meta.env.VITE_API_URL || '').trim().replace(/\/+$/, '');

export const VITE_DATA_MODE: AdminDataMode = rawDataMode === 'api' ? 'api' : 'mock';
export const VITE_API_URL = apiUrl;
export const isApiMode = VITE_DATA_MODE === 'api';

export const adminConfig = {
  dataMode: VITE_DATA_MODE,
  apiUrl: VITE_API_URL,
  isApiMode
};
