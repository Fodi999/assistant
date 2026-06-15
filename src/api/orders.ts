import { apiFetch } from './client';
import type { RecordSaleRequest } from '../types/admin';

// The backend currently records order events through menu-engineering sales.
export function recordOrderSale(payload: RecordSaleRequest): Promise<void> {
  return apiFetch<void>('/api/menu-engineering/sales', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}
