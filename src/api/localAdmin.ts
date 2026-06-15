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


export interface PromptTemplateItem {
  name: string;
  path: string;
  type: 'site' | 'image' | 'system' | 'unknown';
}

export interface PromptListResponse {
  ok: boolean;
  command: string;
  usb_root?: string;
  prompts?: PromptTemplateItem[];
  error?: string;
}

export interface PromptReadResponse {
  ok: boolean;
  command: string;
  path?: string;
  content?: string;
  error?: string;
}

export interface GeminiSettingsStatus {
  ok: boolean;
  command: string;
  usb_root?: string;
  settings_path?: string;
  gemini_api_key?: 'configured' | 'missing';
  text_model?: string;
  image_model?: string;
  error?: string;
}

export interface AiHistoryItem {
  id: string;
  created_at: string;
  type: string;
  template: string;
  path: string;
}

export interface AiHistoryListResponse {
  ok: boolean;
  command: string;
  history?: AiHistoryItem[];
  error?: string;
}

export type JsonObject = Record<string, string | number | boolean | null | string[]>;

export function adminKeyPromptList(): Promise<PromptListResponse> {
  return invokeTauri<PromptListResponse>('admin_key_prompt_list');
}

export function adminKeyPromptRead(path: string): Promise<PromptReadResponse> {
  return invokeTauri<PromptReadResponse>('admin_key_prompt_read', { path });
}

export function adminKeyPromptRender(template: string, vars: JsonObject): Promise<Record<string, unknown>> {
  return invokeTauri<Record<string, unknown>>('admin_key_prompt_render', { template, vars });
}

export function adminKeyGeminiGenerateText(template: string, vars: JsonObject): Promise<Record<string, unknown>> {
  return invokeTauri<Record<string, unknown>>('admin_key_gemini_generate_text', { template, vars });
}

export function adminKeyGeminiGenerateImagePrompt(template: string, vars: JsonObject): Promise<Record<string, unknown>> {
  return invokeTauri<Record<string, unknown>>('admin_key_gemini_generate_image_prompt', { template, vars });
}

export function adminKeyAiHistoryList(): Promise<AiHistoryListResponse> {
  return invokeTauri<AiHistoryListResponse>('admin_key_ai_history_list');
}

export function adminKeyAiHistoryRead(id: string): Promise<Record<string, unknown>> {
  return invokeTauri<Record<string, unknown>>('admin_key_ai_history_read', { id });
}

export function adminKeyGeminiSettingsStatus(): Promise<GeminiSettingsStatus> {
  return invokeTauri<GeminiSettingsStatus>('admin_key_gemini_settings_status');
}

export function adminKeyOpenFolder(pathType: 'prompts' | 'exports' | 'history' | 'settings' | 'logs'): Promise<void> {
  return invokeTauri<void>('admin_key_open_folder', { pathType });
}
