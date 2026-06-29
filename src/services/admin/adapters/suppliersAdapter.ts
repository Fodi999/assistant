import type { AdminResourceRow, SiteId } from '../../../types/admin';
import type { CreateSupplierDto, UpdateSupplierDto } from '../../../types/adminApi';
import { baseRow, mapStatus, pickText } from './shared';

type BackendSupplier = {
  id: string;
  name?: string;
  country?: string;
  city?: string | null;
  categories?: string[];
  contact?: string;
  website?: string | null;
  commission_terms?: string | null;
  type?: string;
  supplier_type?: string;
  status?: string;
};

export const suppliersAdapter = {
  fromBackend(supplier: BackendSupplier, siteId: SiteId): AdminResourceRow {
    return baseRow({
      id: supplier.id,
      title: pickText(supplier.name, supplier.website),
      type: supplier.type || supplier.supplier_type || 'Supplier',
      status: mapStatus(supplier.status, 'active'),
      owner: supplier.contact || 'Network',
      updated: 'backend',
      metric: [supplier.country, supplier.city].filter(Boolean).join(', ') || supplier.categories?.join(', ') || '-'
      ,
      backend: supplier
    }, siteId);
  },

  toCreate(payload: CreateSupplierDto) {
    return {
      name: payload.name || payload.title || 'Новый поставщик',
      country: payload.country || payload.metric || '',
      city: payload.city,
      categories: payload.categories || (payload.type ? [payload.type] : undefined),
      contact: payload.contact || payload.owner,
      website: payload.website,
      commissionTerms: payload.commissionTerms,
      type: payload.supplierType || payload.type
    };
  },

  toUpdate(payload: UpdateSupplierDto) {
    return {
      name: payload.name || payload.title,
      country: payload.country || payload.metric,
      city: payload.city,
      categories: payload.categories || (payload.type ? [payload.type] : undefined),
      contact: payload.contact || payload.owner,
      website: payload.website,
      commissionTerms: payload.commissionTerms,
      type: payload.supplierType || payload.type
    };
  }
};
