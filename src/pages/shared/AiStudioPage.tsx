import { useState } from 'react';
import { AiGenerationPanel } from '../../components/AiGenerationPanel';
import type { LanguageCode, SiteKey } from '../../types/admin';
import { AppIcon } from '../../components/AppIcon';

export function AiStudioPage({ activeSite }: { activeSite: SiteKey }) {
  const [site, setSite] = useState<SiteKey>(activeSite);
  const [language, setLanguage] = useState<LanguageCode>('ru');
  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="bot" /></div><div><p className="eyebrow">Рабочая зона Gemini</p><h2>AI-студия</h2><p>Генерация описаний, SEO, slug, фото-промтов, переводов RU / PL / EN / KK и проверка качества карточек.</p></div></div>
      <div className="ai-studio new">
        <aside><button className={site === 'culinary' ? 'active' : ''} type="button" onClick={() => setSite('culinary')}>Кулинарный сайт</button><button className={site === 'construction' ? 'active' : ''} type="button" onClick={() => setSite('construction')}>Строительный сайт</button><select value={language} onChange={(event) => setLanguage(event.target.value as LanguageCode)}><option value="ru">RU</option><option value="pl">PL</option><option value="en">EN</option><option value="kk">KK</option></select></aside>
        <AiGenerationPanel site={site} language={language} defaultText={site === 'construction' ? 'Керамогранит для ванной в Алматы, цена за м2, монтаж, поставщик' : 'Поварской нож для дома и ресторана, обзор, плюсы, цена'} />
        <section className="ops-panel preview-panel"><div className="panel-title"><span><AppIcon name="check" />Чеклист качества</span></div><p>Проверить: партнерскую ссылку, цену и валюту, продавца, cookie days, SEO title/description, локальные языки, фото-промт, schema.org Product/Review.</p></section>
      </div>
    </section>
  );
}
