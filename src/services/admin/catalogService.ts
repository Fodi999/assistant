import type { AdminResourceRow, CatalogItem, SiteId } from '../../types/admin';
import { catalogAdapter } from './adapters/catalogAdapter';
import { createAdminResourceService, type AdminResourceCreatePayload, type AdminResourceUpdatePayload } from './resourceServiceFactory';

export const catalogService = createAdminResourceService<CatalogItem, 'catalog'>('catalog', (row) => ({ ...row, resource: 'catalog' }), catalogAdapter);

export const list = catalogService.list;
export const getById = catalogService.getById;
export const create = catalogService.create;
export const update = catalogService.update;
export const remove = catalogService.remove;

export function listCatalogItems(siteId: SiteId): Promise<CatalogItem[]> {
  return catalogService.list(siteId);
}

export function createCatalogItem(payload: AdminResourceCreatePayload): Promise<CatalogItem> {
  return catalogService.create(payload);
}

export function updateCatalogItem(id: string, payload: AdminResourceUpdatePayload): Promise<CatalogItem> {
  return catalogService.update(id, payload);
}

export function removeCatalogItem(id: string): Promise<void> {
  return catalogService.remove(id);
}

export function createLocalCatalogItem(siteId: SiteId, item: Omit<CatalogItem, 'id' | 'siteId' | 'resource' | 'owner' | 'updated' | 'metric'>): CatalogItem {
  const updatedAt = new Date().toISOString();

  return {
    ...item,
    id: `cat-local-${Date.now()}`,
    siteId,
    resource: 'catalog',
    owner: 'Local',
    updated: 'now',
    updatedAt,
    metric: item.language?.toUpperCase() ?? 'local'
  };
}

export function catalogItemToRow(item: CatalogItem): AdminResourceRow {
  return item;
}
