import { useEffect, useState } from 'react';
import { ActionButton } from '../../components/admin/ActionButton';
import { AdminPageHeader } from '../../components/admin/AdminPageHeader';
import { AdminPanel } from '../../components/admin/AdminPanel';
import { DataTable } from '../../components/admin/DataTable';
import { useAdminToast } from '../../components/admin/useAdminToast';
import { useActiveSite } from '../../lib/useActiveSite';
import {
  addMediaUrl,
  addUploadedMedia,
  isValidImageUrl,
  listMediaItems,
  removeMediaItem,
  updateMediaAltText,
  type MediaLibraryItem
} from '../../services/admin/mediaLibraryService';
import type { SiteId } from '../../types/admin';

export function MediaPage() {
  const { activeSiteId, activeSite } = useActiveSite();
  const toast = useAdminToast();
  const [items, setItems] = useState<MediaLibraryItem[]>([]);
  const [url, setUrl] = useState('');
  const [altText, setAltText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const siteId = activeSiteId as SiteId;

  useEffect(() => {
    void listMediaItems(siteId).then(setItems);
  }, [siteId]);

  async function handleAddUrl() {
    setError(null);
    if (!isValidImageUrl(url)) {
      setError('Введите корректный URL изображения.');
      return;
    }

    try {
      const nextItems = await addMediaUrl(siteId, url, altText);
      setItems(nextItems);
      setUrl('');
      setAltText('');
      toast.success('Image URL added.');
    } catch (addError) {
      const message = addError instanceof Error ? addError.message : 'Cannot add image.';
      setError(message);
      toast.error(message);
    }
  }

  async function handleUpload(file?: File | null) {
    if (!file) return;

    try {
      const nextItems = await addUploadedMedia(siteId, file, altText);
      setItems(nextItems);
      setAltText('');
      toast.success('Image uploaded to local library.');
    } catch (uploadError) {
      toast.error(uploadError instanceof Error ? uploadError.message : 'Upload failed.');
    }
  }

  async function handleCopy(item: MediaLibraryItem) {
    await navigator.clipboard?.writeText(item.url);
    toast.info('Image URL copied.');
  }

  async function handleAltText(item: MediaLibraryItem, nextAltText: string) {
    setItems(await updateMediaAltText(siteId, item.id, nextAltText));
  }

  async function handleRemove(item: MediaLibraryItem) {
    if (!window.confirm(`Remove "${item.altText || item.url}"?`)) return;
    setItems(await removeMediaItem(siteId, item.id));
    toast.success('Image removed.');
  }

  return (
    <section className="admin-resource-page">
      <AdminPageHeader
        title="Media Library"
        eyebrow="Publishing"
        description="Upload, validate and select images for the active site."
        icon="image"
        meta={`${items.length} assets`}
      />

      <AdminPanel title="Add image" icon="image" meta={activeSite.id}>
        <div className="media-library-form">
          <label>
            <span>Image URL</span>
            <input value={url} onChange={(event) => setUrl(event.target.value)} placeholder="https://..." />
          </label>
          <label>
            <span>Alt text</span>
            <input value={altText} onChange={(event) => setAltText(event.target.value)} placeholder="Describe the image" />
          </label>
          <div className="admin-panel-actions">
            <ActionButton icon="save" onClick={() => void handleAddUrl()}>Add URL</ActionButton>
            <label className="admin-btn secondary">
              <span>Upload</span>
              <input type="file" accept="image/*" hidden onChange={(event) => void handleUpload(event.target.files?.[0])} />
            </label>
          </div>
          {error ? <p className="form-error">{error}</p> : null}
        </div>
      </AdminPanel>

      <AdminPanel title="Assets" icon="image">
        <DataTable
          rows={items}
          getRowKey={(item) => item.id}
          empty={<p className="admin-table-empty">No media assets for this site yet.</p>}
          columns={[
            { key: 'preview', header: 'Preview', render: (item) => <img className="media-thumb" src={item.url} alt="" /> },
            { key: 'alt', header: 'Alt text', render: (item) => <input className="admin-inline-input" value={item.altText} onChange={(event) => void handleAltText(item, event.target.value)} /> },
            { key: 'source', header: 'Source', render: (item) => item.source },
            { key: 'created', header: 'Created', render: (item) => new Date(item.createdAt).toLocaleDateString() },
            { key: 'actions', header: '', align: 'right', render: (item) => (
              <div className="admin-table-actions">
                <button className="table-action" type="button" onClick={() => void handleCopy(item)}>Select</button>
                <button className="table-action danger" type="button" onClick={() => void handleRemove(item)}>Remove</button>
              </div>
            ) }
          ]}
        />
      </AdminPanel>
    </section>
  );
}
