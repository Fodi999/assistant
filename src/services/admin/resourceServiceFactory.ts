import { isApiMode } from '../../config/adminConfig';
import type { AdminResourceRow, SiteId } from '../../types/admin';
import type {
  CreateCMSPageDto,
  CreateCatalogItemDto,
  CreateLeadDto,
  CreateShopProductDto,
  CreateSupplierDto,
  UpdateCMSPageDto,
  UpdateCatalogItemDto,
  UpdateLeadDto,
  UpdateShopProductDto,
  UpdateSiteSettingsDto,
  UpdateSupplierDto
} from '../../types/adminApi';
import { adminApiClient, type RequestBody } from './adminApiClient';
import { adminApiRoutes } from './adminApiRoutes';
import type { AdminResourceKey } from './mockData';
import {
  createMockResource,
  getMockResource,
  listMockResource,
  removeMockResource,
  updateMockResource
} from './mockStore';

export type AdminResourceCreatePayload = Partial<Omit<AdminResourceRow, 'id'>> & { siteId: SiteId };
export type AdminResourceUpdatePayload = Partial<Omit<AdminResourceRow, 'id'>>;
export type AdminCreateDtoByResource = {
  catalog: CreateCatalogItemDto;
  cms: CreateCMSPageDto;
  shop: CreateShopProductDto;
  orders: AdminResourceCreatePayload;
  suppliers: CreateSupplierDto;
  leads: CreateLeadDto;
  users: AdminResourceCreatePayload;
  analytics: AdminResourceCreatePayload;
  settings: AdminResourceCreatePayload;
};
export type AdminUpdateDtoByResource = {
  catalog: UpdateCatalogItemDto;
  cms: UpdateCMSPageDto;
  shop: UpdateShopProductDto;
  orders: AdminResourceUpdatePayload;
  suppliers: UpdateSupplierDto;
  leads: UpdateLeadDto;
  users: AdminResourceUpdatePayload;
  analytics: AdminResourceUpdatePayload;
  settings: UpdateSiteSettingsDto;
};

type ResourceListResponse<T> = T[] | { items: T[] };
type DecorateRow<T extends AdminResourceRow> = (row: AdminResourceRow) => T;
type AdminBackendAdapter<T extends AdminResourceRow, CreateDto, UpdateDto> = {
  normalizeList?: (response: any, siteId: SiteId) => any[];
  fromBackend: (item: any, siteId: SiteId) => T;
  toCreate?: (payload: CreateDto) => RequestBody;
  toUpdate?: (payload: UpdateDto) => RequestBody;
};

function normalizeList<T>(response: ResourceListResponse<T>): T[] {
  return Array.isArray(response) ? response : response.items;
}

function identity<T extends AdminResourceRow>(row: AdminResourceRow): T {
  return row as T;
}

export function createAdminResourceService<T extends AdminResourceRow, Resource extends AdminResourceKey>(
  resource: Resource,
  decorate: DecorateRow<T> = identity,
  adapter?: AdminBackendAdapter<T, AdminCreateDtoByResource[Resource], AdminUpdateDtoByResource[Resource]>
) {
  const routes = adminApiRoutes[resource];

  return {
    async list(siteId: SiteId): Promise<T[]> {
      if (isApiMode) {
        const response = await adminApiClient.get<unknown>(routes.list(siteId));

        if (adapter) {
          const items = adapter.normalizeList
            ? adapter.normalizeList(response as never, siteId)
            : normalizeList(response as ResourceListResponse<unknown>);
          return items.map((item) => adapter.fromBackend(item, siteId));
        }

        return normalizeList(response as ResourceListResponse<T>);
      }

      const rows = await listMockResource(resource, siteId);
      return rows.map(decorate);
    },

    async getById(id: string): Promise<T> {
      if (isApiMode) {
        const path = routes.getById(id, 'construction');
        if (!path) throw new Error('Endpoint не найден');
        const response = await adminApiClient.get<unknown>(path);
        return adapter ? adapter.fromBackend(response, 'construction') : response as T;
      }

      return decorate(await getMockResource(resource, id));
    },

    async create(payload: AdminCreateDtoByResource[Resource]): Promise<T> {
      if (isApiMode) {
        const siteId = 'siteId' in (payload as object) ? (payload as AdminResourceCreatePayload).siteId : 'construction';
        const path = routes.create(siteId);
        if (!path) throw new Error('Endpoint не найден');
        const response = await adminApiClient.post<unknown>(path, adapter?.toCreate ? adapter.toCreate(payload) : payload as RequestBody);
        return adapter ? adapter.fromBackend(response, siteId) : response as T;
      }

      return decorate(await createMockResource(resource, payload as AdminResourceCreatePayload));
    },

    async update(id: string, payload: AdminUpdateDtoByResource[Resource]): Promise<T> {
      if (isApiMode) {
        const siteId = 'siteId' in (payload as object) ? (payload as AdminResourceUpdatePayload).siteId ?? 'construction' : 'construction';
        const path = routes.update(id, siteId);
        if (!path) throw new Error('Endpoint не найден');
        const response = await adminApiClient.patch<unknown>(path, adapter?.toUpdate ? adapter.toUpdate(payload) : payload as RequestBody);
        return adapter ? adapter.fromBackend(response, siteId) : response as T;
      }

      return decorate(await updateMockResource(resource, id, payload as AdminResourceUpdatePayload));
    },

    async remove(id: string, siteId: SiteId = 'construction'): Promise<void> {
      if (isApiMode) {
        const path = routes.remove(id, siteId);
        if (!path) throw new Error('Endpoint не найден');
        await adminApiClient.delete<void>(path);
        return;
      }

      await removeMockResource(resource, id);
    }
  };
}
