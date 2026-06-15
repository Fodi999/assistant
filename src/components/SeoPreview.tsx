import type { LanguageCode, LocalizedText } from '../types/admin';

interface SeoPreviewProps {
  title?: Partial<LocalizedText>;
  description?: Partial<LocalizedText>;
  slug: string;
  language?: LanguageCode;
}

export function SeoPreview({ title, description, slug, language = 'ru' }: SeoPreviewProps) {
  return (
    <section className="seo-preview">
      <p>SEO-превью</p>
      <strong>{title?.[language] || title?.ru || 'Заголовок еще не сгенерирован'}</strong>
      <span>/{slug || 'draft-slug'}</span>
      <small>{description?.[language] || description?.ru || 'Описание появится после генерации или ручного ввода.'}</small>
    </section>
  );
}
