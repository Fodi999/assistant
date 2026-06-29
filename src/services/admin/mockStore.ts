import { getActiveSiteOptions } from '../../lib/useActiveSite';
import type { AdminResourceRow, ResourceStatus, SiteId, SiteSettings } from '../../types/admin';
import { adminMockData, type AdminResourceKey } from './mockData';

const API_URL = String(import.meta.env.VITE_API_URL || import.meta.env.VITE_API_BASE_URL || 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app').replace(/\/+$/, '');

type StoredResourceRow = Omit<AdminResourceRow, 'siteId'>;
type ResourcePayload = Partial<Omit<AdminResourceRow, 'id'>> & { siteId?: SiteId };

const resourcePrefixes: Record<AdminResourceKey, string> = {
  catalog: 'cat',
  cms: 'cms',
  shop: 'shop',
  orders: 'ord',
  suppliers: 'sup',
  leads: 'lead',
  users: 'user',
  analytics: 'ana',
  settings: 'set'
};

function nowIso() {
  return new Date().toISOString();
}

function toPublicRow(row: StoredResourceRow, siteId: SiteId): AdminResourceRow {
  return { ...row, siteId };
}

function findMockRow(resource: AdminResourceKey, id: string): { rows: StoredResourceRow[]; row: StoredResourceRow; siteId: SiteId; index: number } | null {
  const bySite = adminMockData[resource];

  for (const siteId of Object.keys(bySite) as SiteId[]) {
    const rows = bySite[siteId] as StoredResourceRow[];
    const index = rows.findIndex((row) => row.id === id);

    if (index >= 0) {
      return { rows, row: rows[index], siteId, index };
    }
  }

  return null;
}

function createStoredRow(resource: AdminResourceKey, siteId: SiteId, payload: ResourcePayload): StoredResourceRow {
  const timestamp = nowIso();
  return {
    id: `${resourcePrefixes[resource]}-local-${Date.now()}`,
    title: payload.title?.trim() || 'Untitled item',
    slug: payload.slug,
    type: payload.type?.trim() || 'Item',
    status: payload.status ?? 'draft',
    owner: payload.owner || 'Local',
    updated: payload.updated || 'now',
    updatedAt: payload.updatedAt || timestamp,
    metric: payload.metric || payload.language?.toUpperCase() || 'local',
    language: payload.language
  };
}

export async function listMockResource(resource: AdminResourceKey, siteId: SiteId): Promise<AdminResourceRow[]> {
  const rows = adminMockData[resource][siteId] ?? [];
  return rows.map((row) => toPublicRow(row, siteId));
}

export async function getMockResource(resource: AdminResourceKey, id: string): Promise<AdminResourceRow> {
  const match = findMockRow(resource, id);

  if (!match) {
    throw new Error(`Mock ${resource} record not found`);
  }

  return toPublicRow(match.row, match.siteId);
}

export async function createMockResource(resource: AdminResourceKey, payload: ResourcePayload): Promise<AdminResourceRow> {
  const siteId = payload.siteId ?? 'construction';
  const rows = adminMockData[resource][siteId] as StoredResourceRow[];
  const row = createStoredRow(resource, siteId, payload);

  rows.unshift(row);
  return toPublicRow(row, siteId);
}

export async function updateMockResource(resource: AdminResourceKey, id: string, payload: ResourcePayload): Promise<AdminResourceRow> {
  const match = findMockRow(resource, id);

  if (!match) {
    throw new Error(`Mock ${resource} record not found`);
  }

  const updatedAt = nowIso();
  const nextRow: StoredResourceRow = {
    ...match.row,
    ...payload,
    status: (payload.status ?? match.row.status) as ResourceStatus,
    updated: payload.updated || 'now',
    updatedAt
  };

  match.rows[match.index] = nextRow;
  return toPublicRow(nextRow, payload.siteId ?? match.siteId);
}

export async function removeMockResource(resource: AdminResourceKey, id: string): Promise<void> {
  const match = findMockRow(resource, id);

  if (!match) {
    throw new Error(`Mock ${resource} record not found`);
  }

  match.rows.splice(match.index, 1);
}

export async function getMockSiteSettings(siteId: SiteId): Promise<SiteSettings> {
  const site = getActiveSiteOptions().find((item) => item.id === siteId) ?? getActiveSiteOptions()[0];
  return {
    siteId,
    name: site.name,
    domain: site.domain,
    defaultLanguage: site.language.toLowerCase() as SiteSettings['defaultLanguage'],
    ga4Id: `G-${site.shortName}-CRM`,
    searchConsoleProperty: `sc-domain:${site.domain}`,
    apiUrl: API_URL,
    status: site.status
  };
}
