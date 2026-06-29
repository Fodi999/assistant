import type { IconPage, QrPage } from '../../api/iconsSite';

export const QR_GALLERY_MARKER = 'data-admin-kind="qr-code"';

const ICONS_PUBLIC_BASE_URL = String(import.meta.env.VITE_ICONS_PUBLIC_BASE_URL || 'https://svet-ikony.fodi85999.workers.dev').replace(/\/+$/, '');

export function qrPageUrlForIcon(item: Pick<IconPage, 'id' | 'slug'> | undefined) {
  if (!item) return '';
  return `/icons/${item.slug || item.id}`;
}

export function qrPageUrlForQrPage(item: QrPage) {
  return `/qr/${item.slug || item.qrId}`;
}

export function absolutePublicUrl(path: string) {
  if (/^https?:\/\//i.test(path)) return path;
  return new URL(path.startsWith('/') ? path : `/${path}`, `${ICONS_PUBLIC_BASE_URL}/`).toString();
}

export function qrPreviewPath(item: IconPage | QrPage) {
  if ('qrCodeUrl' in item) return item.qrCodeUrl || qrPageUrlForIcon(item);
  return qrPageUrlForQrPage(item);
}

export function qrBackendPath(item: IconPage | QrPage) {
  const qrId = 'qrCodeUrl' in item ? item.slug || item.id : item.qrId;
  return `/api/qr/${qrId}`;
}

export function isGeneratedQrImage(url: string) {
  return url.includes('data-admin-kind%3D%22qr-code%22') || url.includes(QR_GALLERY_MARKER);
}
