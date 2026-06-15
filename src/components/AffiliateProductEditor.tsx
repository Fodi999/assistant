import { useState } from 'react';
import type { AffiliateNetwork, AffiliateProduct, LanguageCode, PublishStatus, SiteKey } from '../types/admin';
import { AiGenerationPanel } from './AiGenerationPanel';
import { AffiliateOfferCards } from './AffiliateOfferCards';
import { CurrencyInput } from './CurrencyInput';
import { LanguageChips } from './LanguageChips';
import { SeoPreview } from './SeoPreview';
import { publishStatusLabels } from '../lib/labels';

interface AffiliateProductEditorProps {
  site: SiteKey;
  product?: AffiliateProduct;
  onClose: () => void;
}

const networks: AffiliateNetwork[] = ['amazon', 'allegro', 'ceneo', 'awin', 'custom'];
const statuses: PublishStatus[] = ['draft', 'active', 'published', 'archived'];

function emptyProduct(site: SiteKey): AffiliateProduct {
  return {
    id: 'new',
    site,
    title: { ru: '', pl: '', en: '', kk: '' },
    slug: '',
    category: '',
    network: 'custom',
    merchant: '',
    affiliateUrl: '',
    currency: site === 'construction' ? 'KZT' : 'PLN',
    status: 'draft',
    languages: ['ru'],
    seoTitle: { ru: '', pl: '', en: '', kk: '' },
    seoDescription: { ru: '', pl: '', en: '', kk: '' },
    offers: []
  };
}

export function AffiliateProductEditor({ site, product, onClose }: AffiliateProductEditorProps) {
  const [draft, setDraft] = useState<AffiliateProduct>(product ?? emptyProduct(site));
  const primaryLanguage: LanguageCode = draft.languages[0] ?? 'ru';

  return (
    <div className="modal-overlay" role="presentation" onMouseDown={onClose}>
      <section className="editor-modal affiliate-editor" role="dialog" aria-label="Редактор партнерского товара" onMouseDown={(event) => event.stopPropagation()}>
        <div className="editor-modal-head">
          <div>
            <p className="eyebrow">Партнерский товар</p>
            <h2>{product ? 'Редактор партнерского товара' : 'Создать партнерский товар'}</h2>
          </div>
          <button className="btn btn-quiet" type="button" onClick={onClose}>Закрыть</button>
        </div>
        <div className="affiliate-editor-layout">
          <div className="editor-card">
            <div className="editor-grid">
              <label className="editor-field"><span>Название RU</span><input value={draft.title.ru} onChange={(event) => setDraft({ ...draft, title: { ...draft.title, ru: event.target.value } })} /></label>
              <label className="editor-field"><span>Slug</span><input value={draft.slug} onChange={(event) => setDraft({ ...draft, slug: event.target.value })} /></label>
              <label className="editor-field"><span>Категория</span><input value={draft.category} onChange={(event) => setDraft({ ...draft, category: event.target.value })} /></label>
              <label className="editor-field"><span>Продавец</span><input value={draft.merchant} onChange={(event) => setDraft({ ...draft, merchant: event.target.value })} /></label>
              <label className="editor-field"><span>Партнерская ссылка</span><input value={draft.affiliateUrl} onChange={(event) => setDraft({ ...draft, affiliateUrl: event.target.value })} /></label>
              <label className="editor-field"><span>Сеть</span><select value={draft.network} onChange={(event) => setDraft({ ...draft, network: event.target.value as AffiliateNetwork })}>{networks.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>
              <label className="editor-field"><span>Цена</span><CurrencyInput value={draft.price} currency={draft.currency} onChange={(price, currency) => setDraft({ ...draft, price, currency })} /></label>
              <label className="editor-field"><span>Статус</span><select value={draft.status} onChange={(event) => setDraft({ ...draft, status: event.target.value as PublishStatus })}>{statuses.map((item) => <option key={item} value={item}>{publishStatusLabels[item]}</option>)}</select></label>
              <label className="editor-field"><span>URL изображения</span><input value={draft.imageUrl ?? ''} onChange={(event) => setDraft({ ...draft, imageUrl: event.target.value })} /></label>
              <label className="editor-field"><span>URL детального изображения</span><input value={draft.detailImageUrl ?? ''} onChange={(event) => setDraft({ ...draft, detailImageUrl: event.target.value })} /></label>
            </div>
            <LanguageChips value={draft.languages} onChange={(languages) => setDraft({ ...draft, languages })} />
            <SeoPreview title={draft.seoTitle} description={draft.seoDescription} slug={draft.slug} language={primaryLanguage} />
            <AffiliateOfferCards offers={draft.offers} />
            <div className="editor-actions">
              <button className="btn btn-quiet" type="button">Импорт ссылки</button>
              <button className="btn btn-quiet" type="button">Предпросмотр</button>
              <button className="btn btn-primary" type="button" onClick={onClose}>Создать партнерский товар</button>
            </div>
          </div>
          <AiGenerationPanel site={draft.site} language={primaryLanguage} defaultText={draft.title.ru || draft.affiliateUrl} />
        </div>
      </section>
    </div>
  );
}
