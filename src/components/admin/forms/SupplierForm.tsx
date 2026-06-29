import { useState } from 'react';
import type { AdminResourceRow } from '../../../types/admin';
import type { CreateSupplierDto } from '../../../types/adminApi';
import { FieldError, csv, isValidUrl, type FormErrors } from './formUtils';

type BackendSupplier = {
  name?: string; country?: string; city?: string | null; categories?: string[]; contact?: string; website?: string | null; commission_terms?: string | null; type?: string; supplier_type?: string;
};

type SupplierFormState = {
  name: string; country: string; city: string; categories: string; contact: string; website: string; commissionTerms: string; supplierType: string;
};

function initialState(row?: AdminResourceRow | null): SupplierFormState {
  const backend = (row?.backend || {}) as BackendSupplier;
  return {
    name: backend.name || row?.title || '',
    country: backend.country || '',
    city: backend.city || '',
    categories: backend.categories?.join(', ') || row?.type || '',
    contact: backend.contact || row?.owner || '',
    website: backend.website || '',
    commissionTerms: backend.commission_terms || '',
    supplierType: backend.type || backend.supplier_type || row?.type || 'local_supplier'
  };
}

export function SupplierForm({ formId, row, disabled, onSubmit }: { formId: string; row?: AdminResourceRow | null; disabled?: boolean; onSubmit: (payload: CreateSupplierDto) => void }) {
  const [form, setForm] = useState<SupplierFormState>(() => initialState(row));
  const [errors, setErrors] = useState<FormErrors>({});

  function validate() {
    const next: FormErrors = {};
    if (!form.name.trim()) next.name = 'Введите название поставщика.';
    if (!isValidUrl(form.website)) next.website = 'Введите корректный URL сайта.';
    setErrors(next);
    return !Object.keys(next).length;
  }

  return (
    <form id={formId} className="admin-form-grid" onSubmit={(event) => {
      event.preventDefault();
      if (!validate()) return;
      onSubmit({
        siteId: row?.siteId || 'construction',
        title: form.name.trim(),
        name: form.name.trim(),
        country: form.country.trim(),
        city: form.city.trim() || undefined,
        categories: csv(form.categories),
        contact: form.contact.trim() || undefined,
        website: form.website.trim() || undefined,
        commissionTerms: form.commissionTerms.trim() || undefined,
        supplierType: form.supplierType.trim() || undefined,
        type: form.supplierType.trim() || undefined
      });
    }}>
      <label><span>Name</span><input disabled={disabled} value={form.name} onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))} /><FieldError message={errors.name} /></label>
      <label><span>Country</span><input disabled={disabled} value={form.country} onChange={(event) => setForm((current) => ({ ...current, country: event.target.value }))} /></label>
      <label><span>City</span><input disabled={disabled} value={form.city} onChange={(event) => setForm((current) => ({ ...current, city: event.target.value }))} /></label>
      <label><span>Categories</span><input disabled={disabled} value={form.categories} onChange={(event) => setForm((current) => ({ ...current, categories: event.target.value }))} /></label>
      <label><span>Contact</span><input disabled={disabled} value={form.contact} onChange={(event) => setForm((current) => ({ ...current, contact: event.target.value }))} /></label>
      <label><span>Website</span><input disabled={disabled} value={form.website} onChange={(event) => setForm((current) => ({ ...current, website: event.target.value }))} /><FieldError message={errors.website} /></label>
      <label><span>Commission terms</span><textarea disabled={disabled} value={form.commissionTerms} onChange={(event) => setForm((current) => ({ ...current, commissionTerms: event.target.value }))} /></label>
      <label><span>Supplier type</span><select disabled={disabled} value={form.supplierType} onChange={(event) => setForm((current) => ({ ...current, supplierType: event.target.value }))}><option value="local_supplier">local_supplier</option><option value="marketplace">marketplace</option><option value="manufacturer">manufacturer</option><option value="affiliate_merchant">affiliate_merchant</option></select></label>
    </form>
  );
}
