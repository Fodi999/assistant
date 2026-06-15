import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import { leads } from '../lib/mockData';
import type { Lead, LeadStatus, SiteKey } from '../types/admin';

export async function listLeads(site?: SiteKey): Promise<Lead[]> {
  const mock = site ? leads.filter((lead) => lead.sourceSite === site) : leads;
  try {
    return await apiFetch<Lead[]>(`/api/admin/leads${site ? `?site=${site}` : ''}`);
  } catch {
    return mock;
  }
}

export function listLeadsWithSource(site?: SiteKey): Promise<SourcedData<Lead[]>> {
  const mock = site ? leads.filter((lead) => lead.sourceSite === site) : leads;
  return withDataSource(apiFetch<Lead[]>(`/api/admin/leads${site ? `?site=${site}` : ''}`), mock);
}

export async function updateLeadStatus(id: string, status: LeadStatus): Promise<Lead> {
  const current = leads.find((lead) => lead.id === id) ?? leads[0];
  try {
    return await apiFetch<Lead>(`/api/admin/leads/${id}/status`, { method: 'PATCH', body: JSON.stringify({ status }) });
  } catch {
    return { ...current, status };
  }
}
