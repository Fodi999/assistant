export interface UsbStorageInfo {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  total_label: string;
  used_label: string;
  available_label: string;
}

export interface UsbDataPaths {
  config: string;
  backups: string;
  exports: string;
  local_db: string;
  logs: string;
}

export interface UsbKeyStatus {
  found: boolean;
  root?: string | null;
  admin_tool?: string | null;
  config?: string | null;
  storage?: UsbStorageInfo | null;
  data_paths?: UsbDataPaths | null;
}

export interface AdminToolOutput {
  command: string;
  status: number;
  stdout: string;
  stderr: string;
  key_root: string;
}

type TauriWindow = Window & {
  __TAURI__?: { core?: { invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> } };
  __TAURI_INTERNALS__?: { invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> };
};

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const tauriWindow = window as TauriWindow;
  const invoke = tauriWindow.__TAURI__?.core?.invoke || tauriWindow.__TAURI_INTERNALS__?.invoke;
  if (!invoke) {
    throw new Error('Tauri bridge недоступен. Открой приложение через Chef Admin.app, не через браузер.');
  }
  return invoke<T>(cmd, args);
}

export function findUsbKey(): Promise<UsbKeyStatus> {
  return invokeTauri<UsbKeyStatus>('find_usb_key');
}

export function runAdminTool(args: string[]): Promise<AdminToolOutput> {
  return invokeTauri<AdminToolOutput>('run_admin_tool', { args });
}
