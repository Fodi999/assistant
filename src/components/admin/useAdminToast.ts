import { createContext, useContext } from 'react';

export type AdminToastType = 'success' | 'error' | 'info';

export type AdminToastInput = {
  type: AdminToastType;
  message: string;
  title?: string;
};

export type AdminToastContextValue = {
  showToast: (toast: AdminToastInput) => void;
  success: (message: string, title?: string) => void;
  error: (message: string, title?: string) => void;
  info: (message: string, title?: string) => void;
};

export const AdminToastContext = createContext<AdminToastContextValue | null>(null);

export function useAdminToast() {
  const context = useContext(AdminToastContext);

  if (!context) {
    throw new Error('useAdminToast must be used inside AdminToastProvider');
  }

  return context;
}
