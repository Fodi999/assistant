export interface AiReferenceImage {
  url: string;
  preview: string;
  name: string;
}

interface AiReferenceUploadProps {
  title: string;
  hint: string;
  images: AiReferenceImage[];
  busy?: boolean;
  onAdd: (files: FileList | null) => void;
  onRemove: (index: number) => void;
}

export function AiReferenceUpload({ title, hint, images, busy = false, onAdd, onRemove }: AiReferenceUploadProps) {
  return <div className="ai-reference-upload"><div className="ai-reference-copy"><strong>{title}</strong><span>{hint}</span></div><div className="ai-reference-items">{images.map((image, index) => <div className="ai-reference-thumb" key={image.url}><img src={image.preview} alt={image.name} /><button type="button" onClick={() => onRemove(index)}>×</button></div>)}{images.length < 2 && <label className="ai-reference-add"><input type="file" accept="image/*" multiple onChange={(event) => onAdd(event.target.files)} disabled={busy} /><b>{busy ? '…' : '+'}</b><span>{busy ? 'Загрузка' : 'Добавить фото'}</span></label>}</div></div>;
}
