import { type ReactNode, useEffect, useState } from 'react';

interface AiPhotoGalleryProps {
  images: string[];
  selectedIndex: number;
  title: string;
  heading?: string;
  subtitle?: string;
  actions?: ReactNode;
  emptyText: string;
  busyText: string;
  busy?: boolean;
  addBusy?: boolean;
  maxImages?: number;
  itemLabel?: (index: number) => string;
  onSelect: (index: number) => void;
  onAdd?: () => void;
  onRemove?: (index: number) => void;
}

export function AiPhotoGallery({
  images,
  selectedIndex,
  title,
  heading = 'AI-фотографии',
  subtitle = 'Gemini image generation',
  actions,
  emptyText,
  busyText,
  busy = false,
  addBusy = false,
  maxImages = 12,
  itemLabel = (index) => index === 0 ? 'Главное' : `Фото ${index + 1}`,
  onSelect,
  onAdd,
  onRemove
}: AiPhotoGalleryProps) {
  const [imageSizes, setImageSizes] = useState<Record<string, { width: number; height: number }>>({});
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const selectedImage = images[selectedIndex];

  useEffect(() => {
    if (!lightboxOpen) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setLightboxOpen(false);
      if (event.key === '+' || event.key === '=') changeZoom(.25);
      if (event.key === '-') changeZoom(-.25);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [lightboxOpen]);

  function rememberSize(url: string, image: HTMLImageElement) {
    if (!url || !image.naturalWidth || !image.naturalHeight) return;
    setImageSizes((current) => current[url] ? current : { ...current, [url]: { width: image.naturalWidth, height: image.naturalHeight } });
  }

  function changeZoom(delta: number) {
    setZoom((current) => Math.min(4, Math.max(.5, current + delta)));
  }

  function openLightbox() {
    setZoom(1);
    setLightboxOpen(true);
  }

  const resolution = selectedImage && imageSizes[selectedImage]
    ? `${imageSizes[selectedImage].width} × ${imageSizes[selectedImage].height} px`
    : 'Определяем разрешение...';

  return <>
    <div className="ai-photo-head"><div><h4>{heading}</h4><p>{subtitle}</p></div>{actions && <div className="ai-photo-actions">{actions}</div>}</div>
    <div className="ai-photo-main">
      {selectedImage ? <><img src={selectedImage} alt={title} onLoad={(event) => rememberSize(selectedImage, event.currentTarget)} /><div className="ai-photo-overlay"><span>{resolution}</span><button type="button" onClick={openLightbox} aria-label="Увеличить фото" title="Посмотреть на весь экран">⛶</button></div></> : <div className="ai-photo-empty"><b>AI</b><strong>{busy || addBusy ? busyText : emptyText}</strong></div>}
    </div>
    <div className="ai-photo-options">
      {images.map((image, index) => <div className={`ai-photo-card ${selectedIndex === index ? 'active' : ''}`} key={image}>
        <button type="button" className="ai-photo-select" onClick={() => onSelect(index)}><img src={image} alt={itemLabel(index)} /><small>{itemLabel(index)}</small></button>
        {onRemove && <button type="button" className="ai-photo-delete" onClick={() => onRemove(index)} disabled={busy || addBusy} aria-label={`Удалить ${itemLabel(index)}`} title="Удалить фото">×</button>}
      </div>)}
      {onAdd && images.length < maxImages && <button type="button" className="ai-photo-add" onClick={onAdd} disabled={busy || addBusy}><span>{addBusy ? '…' : '+'}</span><strong>{addBusy ? 'Генерируем' : 'Ещё фото'}</strong><small>Добавить кадр</small></button>}
    </div>
    {lightboxOpen && selectedImage && <div className="ai-lightbox" role="dialog" aria-modal="true" aria-label="Полноэкранный просмотр изображения" onClick={() => setLightboxOpen(false)}>
      <div className="ai-lightbox-toolbar" onClick={(event) => event.stopPropagation()}><div><strong>{title}</strong><span>{resolution} · {Math.round(zoom * 100)}%</span></div><div className="ai-lightbox-controls"><button type="button" onClick={() => changeZoom(-.25)} disabled={zoom <= .5}>−</button><button type="button" onClick={() => setZoom(1)}>100%</button><button type="button" onClick={() => changeZoom(.25)} disabled={zoom >= 4}>+</button><button type="button" onClick={() => setLightboxOpen(false)}>Закрыть</button></div></div>
      <div className="ai-lightbox-canvas" onClick={(event) => event.stopPropagation()} onWheel={(event) => { event.preventDefault(); changeZoom(event.deltaY < 0 ? .25 : -.25); }}><img src={selectedImage} alt={title} style={{ width: `${zoom * 100}%` }} onLoad={(event) => rememberSize(selectedImage, event.currentTarget)} /></div>
    </div>}
  </>;
}
