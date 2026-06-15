import { useState } from 'react';
import { analyzePhotoWithGemini, generateAffiliateProduct, generateAiImage, generatePhotoPrompt, generateSeo, improveProductCard, type AiImageResult, type AiVisionResult } from '../api/ai';
import type { AiGenerationResult, AiGenerationType, LanguageCode, SiteKey } from '../types/admin';
import { AppIcon } from './AppIcon';
import { siteNames } from '../lib/labels';

interface AiGenerationPanelProps {
  site: SiteKey;
  language: LanguageCode;
  defaultText?: string;
  onResult?: (result: AiGenerationResult) => void;
}

const generationTypes: Array<{ value: AiGenerationType; label: string }> = [
  { value: 'product_description', label: 'Описание товара' },
  { value: 'seo', label: 'SEO title/description' },
  { value: 'slug', label: 'Slug' },
  { value: 'photo_prompt', label: 'Фото-промт' },
  { value: 'translation', label: 'RU / PL / EN / KK' },
  { value: 'quality_check', label: 'Проверка качества' }
];

export function AiGenerationPanel({ site, language, defaultText = '', onResult }: AiGenerationPanelProps) {
  const [type, setType] = useState<AiGenerationType>('product_description');
  const [sourceText, setSourceText] = useState(defaultText);
  const [busy, setBusy] = useState(false);
  const [visionBusy, setVisionBusy] = useState(false);
  const [imageBusy, setImageBusy] = useState(false);
  const [result, setResult] = useState<AiGenerationResult | null>(null);
  const [visionResult, setVisionResult] = useState<AiVisionResult | null>(null);
  const [imageResult, setImageResult] = useState<AiImageResult | null>(null);

  async function run() {
    setBusy(true);
    const payload = { site, language, type, sourceText, tone: 'seo' as const };
    try {
      const next = type === 'seo'
        ? await generateSeo(payload)
        : type === 'photo_prompt'
          ? await generatePhotoPrompt(payload)
          : type === 'quality_check'
            ? await improveProductCard(payload)
            : await generateAffiliateProduct(payload);
      setResult(next);
      onResult?.(next);
    } finally {
      setBusy(false);
    }
  }

  async function analyzePhoto(file?: File | null) {
    if (!file) return;
    setVisionBusy(true);
    try {
      const next = await analyzePhotoWithGemini(file, site, language, sourceText);
      setVisionResult(next);
      setSourceText([next.title, next.description, next.seoTitle, next.seoDescription].filter(Boolean).join('\n\n'));
    } finally {
      setVisionBusy(false);
    }
  }

  async function createImage() {
    const title = result?.title || visionResult?.title || sourceText.split('\n')[0] || defaultText;
    if (!title.trim()) return;
    setImageBusy(true);
    try {
      const next = await generateAiImage({
        site,
        title,
        description: result?.description || visionResult?.description || sourceText,
        scene: result?.photoPrompt || (site === 'construction' ? 'real construction material scene for commercial renovation' : 'premium ecommerce or editorial culinary product photo'),
        imageType: site === 'construction' ? 'construction' : type === 'photo_prompt' ? 'product' : 'article'
      });
      setImageResult(next);
    } finally {
      setImageBusy(false);
    }
  }

  return (
    <aside className="ai-generation-panel">
      <div className="panel-title"><span><AppIcon name="sparkles" />AI / Gemini</span><small>{siteNames[site]}</small></div>
      <label className="editor-field">
        <span>Тип генерации</span>
        <select value={type} onChange={(event) => setType(event.target.value as AiGenerationType)}>
          {generationTypes.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
        </select>
      </label>
      <label className="editor-field">
        <span>Контекст</span>
        <textarea value={sourceText} onChange={(event) => setSourceText(event.target.value)} placeholder="Название товара, ключевые слова, URL или черновик текста" />
      </label>
      <label className="editor-field">
        <span>Фото для Gemini Vision</span>
        <input type="file" accept="image/*" onChange={(event) => void analyzePhoto(event.target.files?.[0])} disabled={visionBusy} />
      </label>
      <div className="form-actions">
        <button className="btn btn-primary" type="button" onClick={run} disabled={busy}><AppIcon name="bot" />{busy ? 'Генерируем...' : 'Текст с Gemini'}</button>
        <button className="btn" type="button" onClick={createImage} disabled={imageBusy}><AppIcon name="cloud" />{imageBusy ? 'Фото...' : 'Сгенерировать фото'}</button>
      </div>
      {visionResult ? (
        <div className="ai-result">
          <strong>{visionResult.title || 'Gemini Vision'}</strong>
          <p>{visionResult.description}</p>
          {visionResult.imageUrl ? <small>Фото сохранено: {visionResult.imageUrl}</small> : null}
          {visionResult.category ? <small>Категория: {visionResult.category}</small> : null}
          {visionResult.priceHint ? <small>{visionResult.priceHint}</small> : null}
          {visionResult.suggestions.map((item) => <small key={item}>{item}</small>)}
        </div>
      ) : null}
      {result ? (
        <div className="ai-result">
          <strong>{result.title || result.slug || 'AI результат'}</strong>
          <p>{result.description || result.photoPrompt}</p>
          {result.suggestions.map((item) => <small key={item}>{item}</small>)}
        </div>
      ) : null}
      {imageResult ? (
        <div className="ai-result">
          <strong>Gemini Image</strong>
          {imageResult.imageUrl ? <img src={imageResult.imageUrl} alt="AI generated" /> : <p>Фото не создано: проверь GEMINI_API_KEY и R2.</p>}
          <small>{imageResult.prompt}</small>
        </div>
      ) : null}
    </aside>
  );
}
