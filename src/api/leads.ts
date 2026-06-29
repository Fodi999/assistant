import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import type { Lead, LeadStatus, SiteKey } from '../types/admin';

export function listLeads(site?: SiteKey): Promise<Lead[]> {
  return apiFetch<Lead[]>(`/api/admin/leads${site ? `?site=${site}` : ''}`);
}

export function listLeadsWithSource(site?: SiteKey): Promise<SourcedData<Lead[]>> {
  return withDataSource(apiFetch<Lead[]>(`/api/admin/leads${site ? `?site=${site}` : ''}`), []);
}

export function updateLeadStatus(id: string, status: LeadStatus): Promise<Lead> {
  return apiFetch<Lead>(`/api/admin/leads/${id}/status`, { method: 'PATCH', body: JSON.stringify({ status }) });
}
