import type { AdminResourceRow, SiteId } from '../../../types/admin';
import type { CreateLeadDto, UpdateLeadDto } from '../../../types/adminApi';
import { baseRow, mapStatus, pickText, updatedLabel } from './shared';

type BackendLead = {
  id: string;
  client_name?: string;
  clientName?: string;
  name?: string;
  contact?: string;
  source_site?: string;
  category?: string;
  city?: string | null;
  message?: string;
  status?: string;
  potential_value?: number | null;
  currency?: string;
  created_at?: string;
  updated_at?: string;
};

export const leadsAdapter = {
  fromBackend(lead: BackendLead, siteId: SiteId): AdminResourceRow {
    const updatedAt = lead.updated_at || lead.created_at;
    const value = lead.potential_value ? `${lead.currency || ''} ${lead.potential_value}`.trim() : lead.city;

    return baseRow({
      id: lead.id,
      title: pickText(lead.client_name, lead.clientName, lead.name, lead.contact),
      type: lead.category || lead.source_site || 'Lead',
      status: mapStatus(lead.status, 'new'),
      owner: lead.contact || 'Lead',
      updated: updatedLabel(updatedAt),
      updatedAt,
      metric: value || lead.status || '-'
    }, siteId);
  },

  toCreate(payload: CreateLeadDto) {
    return {
      clientName: payload.clientName || payload.title,
      contact: payload.contact || payload.owner,
      source: payload.source || payload.siteId,
      status: payload.status
    };
  },

  toUpdate(payload: UpdateLeadDto) {
    return {
      status: payload.status || 'new'
    };
  }
};
