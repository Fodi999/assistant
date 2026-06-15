import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import { suppliers } from '../lib/mockData';
import type { Supplier } from '../types/admin';

export async function listSuppliers(): Promise<Supplier[]> {
  try {
    return await apiFetch<Supplier[]>('/api/admin/suppliers');
  } catch {
    return suppliers;
  }
}

export function listSuppliersWithSource(): Promise<SourcedData<Supplier[]>> {
  return withDataSource(apiFetch<Supplier[]>('/api/admin/suppliers'), suppliers);
}

export async function createSupplier(payload: Partial<Supplier>): Promise<Supplier> {
  const mock = { ...suppliers[0], ...payload, id: `supplier-${Date.now()}` } as Supplier;
  try {
    return await apiFetch<Supplier>('/api/admin/suppliers', { method: 'POST', body: JSON.stringify(payload) });
  } catch {
    return mock;
  }
}

export async function updateSupplier(id: string, payload: Partial<Supplier>): Promise<Supplier> {
  const current = suppliers.find((supplier) => supplier.id === id) ?? suppliers[0];
  try {
    return await apiFetch<Supplier>(`/api/admin/suppliers/${id}`, { method: 'PATCH', body: JSON.stringify(payload) });
  } catch {
    return { ...current, ...payload };
  }
}
