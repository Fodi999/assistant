import type { AdminResourceKey } from './mockData';

export type AdminResourceCapabilities = {
  canCreate?: boolean;
  canEdit?: boolean;
  canDelete?: boolean;
  canChangeStatus?: boolean;
  mockOnly?: boolean;
};

export const resourceCapabilities: Record<AdminResourceKey, AdminResourceCapabilities> = {
  catalog: { canCreate: true, canEdit: true, canDelete: true, canChangeStatus: false },
  cms: { canCreate: true, canEdit: true, canDelete: true, canChangeStatus: false },
  shop: { canCreate: true, canEdit: true, canDelete: true, canChangeStatus: false },
  suppliers: { canCreate: true, canEdit: true, canDelete: false, canChangeStatus: false },
  leads: { canCreate: false, canEdit: false, canDelete: false, canChangeStatus: true },
  users: { canCreate: false, canEdit: false, canDelete: true, canChangeStatus: false },
  orders: { canCreate: false, canEdit: false, canDelete: false, canChangeStatus: false, mockOnly: true },
  analytics: { canCreate: false, canEdit: false, canDelete: false, canChangeStatus: false },
  settings: { canCreate: false, canEdit: false, canDelete: false, canChangeStatus: false, mockOnly: true }
};

export function resourceNotice(capabilities?: AdminResourceCapabilities): string | null {
  if (!capabilities) return null;
  if (capabilities.mockOnly) return 'Mock-only resource: backend endpoint is not available yet.';
  if (!capabilities.canCreate && !capabilities.canEdit && !capabilities.canDelete) {
    return capabilities.canChangeStatus
      ? 'Read-only resource: backend allows status changes only.'
      : 'Read-only resource: backend does not expose write actions yet.';
  }
  if (capabilities.canCreate === false || capabilities.canDelete === false) {
    return 'Limited backend resource: some actions are intentionally disabled.';
  }
  return null;
}
