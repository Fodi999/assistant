import { apiFetch } from './client';
import { calculatorPresets, constructionBundles, constructionMaterials } from '../lib/mockData';
import type { ConstructionBundle, ConstructionCalculatorPreset, ConstructionMaterial } from '../types/admin';

export async function listMaterials(): Promise<ConstructionMaterial[]> {
  try {
    return await apiFetch<ConstructionMaterial[]>('/api/admin/construction/materials');
  } catch {
    return constructionMaterials;
  }
}

export async function listBundles(): Promise<ConstructionBundle[]> {
  try {
    return await apiFetch<ConstructionBundle[]>('/api/admin/construction/bundles');
  } catch {
    return constructionBundles;
  }
}

export async function calculateRepair(payload: Pick<ConstructionCalculatorPreset, 'areaM2' | 'materialCost' | 'workCost' | 'marginPercent' | 'currency'>): Promise<ConstructionCalculatorPreset> {
  const totalPrice = Math.round((payload.materialCost + payload.workCost) * (1 + payload.marginPercent / 100));
  try {
    return await apiFetch<ConstructionCalculatorPreset>('/api/admin/construction/calculate', { method: 'POST', body: JSON.stringify(payload) });
  } catch {
    return { ...calculatorPresets[0], ...payload, totalPrice };
  }
}

export async function createBundle(payload: Partial<ConstructionBundle>): Promise<ConstructionBundle> {
  const mock = { ...constructionBundles[0], ...payload, id: `bundle-${Date.now()}` } as ConstructionBundle;
  try {
    return await apiFetch<ConstructionBundle>('/api/admin/construction/bundles', { method: 'POST', body: JSON.stringify(payload) });
  } catch {
    return mock;
  }
}
