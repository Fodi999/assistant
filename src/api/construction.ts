import { apiFetch } from './client';
import type { ConstructionBundle, ConstructionCalculatorPreset, ConstructionMaterial } from '../types/admin';

export function listMaterials(): Promise<ConstructionMaterial[]> {
  return apiFetch<ConstructionMaterial[]>('/api/admin/construction/materials');
}

export function listBundles(): Promise<ConstructionBundle[]> {
  return apiFetch<ConstructionBundle[]>('/api/admin/construction/bundles');
}

export function calculateRepair(payload: Pick<ConstructionCalculatorPreset, 'areaM2' | 'materialCost' | 'workCost' | 'marginPercent' | 'currency'>): Promise<ConstructionCalculatorPreset> {
  return apiFetch<ConstructionCalculatorPreset>('/api/admin/construction/calculate', { method: 'POST', body: JSON.stringify(payload) });
}

export function createBundle(payload: Partial<ConstructionBundle>): Promise<ConstructionBundle> {
  return apiFetch<ConstructionBundle>('/api/admin/construction/bundles', { method: 'POST', body: JSON.stringify(payload) });
}
