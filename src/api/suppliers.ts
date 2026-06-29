import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import type { Supplier } from '../types/admin';

export function listSuppliers(): Promise<Supplier[]> {
  return apiFetch<Supplier[]>('/api/admin/suppliers');
}

export function listSuppliersWithSource(): Promise<SourcedData<Supplier[]>> {
  return withDataSource(apiFetch<Supplier[]>('/api/admin/suppliers'), []);
}

export function createSupplier(payload: Partial<Supplier>): Promise<Supplier> {
  return apiFetch<Supplier>('/api/admin/suppliers', { method: 'POST', body: JSON.stringify(payload) });
}

export function updateSupplier(id: string, payload: Partial<Supplier>): Promise<Supplier> {
  return apiFetch<Supplier>(`/api/admin/suppliers/${id}`, { method: 'PATCH', body: JSON.stringify(payload) });
}
