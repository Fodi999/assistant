import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { AdminToastContext, type AdminToastInput } from './useAdminToast';

type AdminToast = AdminToastInput & {
  id: string;
};

type AdminToastProviderProps = {
  children: ReactNode;
};

const toastTitles: Record<AdminToast['type'], string> = {
  success: 'Готово',
  error: 'Ошибка',
  info: 'Информация'
};

export function AdminToastProvider({ children }: AdminToastProviderProps) {
  const [toasts, setToasts] = useState<AdminToast[]>([]);
  const timers = useRef<number[]>([]);

  const dismiss = useCallback((id: string) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const showToast = useCallback((toast: AdminToastInput) => {
    const id = `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    setToasts((current) => [...current, { ...toast, id }].slice(-4));

    const timer = window.setTimeout(() => dismiss(id), 4200);
    timers.current.push(timer);
  }, [dismiss]);

  useEffect(() => () => {
    timers.current.forEach((timer) => window.clearTimeout(timer));
  }, []);

  const value = useMemo(() => ({
    showToast,
    success: (message: string, title?: string) => showToast({ type: 'success', message, title }),
    error: (message: string, title?: string) => showToast({ type: 'error', message, title }),
    info: (message: string, title?: string) => showToast({ type: 'info', message, title })
  }), [showToast]);

  return (
    <AdminToastContext.Provider value={value}>
      {children}
      <div className="admin-toast-viewport" aria-live="polite" aria-atomic="true">
        {toasts.map((toast) => (
          <div className="admin-toast" data-type={toast.type} role="status" key={toast.id}>
            <strong>{toast.title || toastTitles[toast.type]}</strong>
            <span>{toast.message}</span>
            <button type="button" aria-label="Close notification" onClick={() => dismiss(toast.id)}>x</button>
          </div>
        ))}
      </div>
    </AdminToastContext.Provider>
  );
}
