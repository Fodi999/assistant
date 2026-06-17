import { siteConfigs } from '../../lib/mockData';
import { AppIcon } from '../../components/AppIcon';

export function SettingsPage() {
  return (
    <section className="ops-page">
      <div className="ops-header">
        <div className="ops-header-icon"><AppIcon name="settings" /></div>
        <div>
          <p className="eyebrow">Системные настройки</p>
          <h2>Настройки</h2>
          <p>API endpoints, Gemini, Cloudflare R2, ревалидация, языки, валюты и сайты.</p>
        </div>
      </div>
      <div className="settings-matrix">
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="terminal" />Адреса API</span></div><label className="editor-field"><span>API бэкенда</span><input value={String(import.meta.env.VITE_API_BASE_URL || 'Koyeb default')} readOnly /></label><label className="editor-field"><span>Ревалидация</span><input value="/api/admin/revalidate" readOnly /></label></section>
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="bot" />Gemini</span></div><div className="ops-list"><div><span>Gemini API key</span><span className="status-pill warning"><i />только backend</span></div><div><span>AI-студия</span><span className="status-pill good"><i />включена</span></div></div></section>
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="cloud" />Cloudflare R2</span></div><div className="ops-list"><div><span>Бакет изображений</span><strong>запланировано</strong></div><div><span>Детальные изображения</span><strong>готовы URL-поля</strong></div></div></section>
        <section className="ops-panel"><div className="panel-title"><span><AppIcon name="globe" />Сайты</span></div><div className="ops-list">{siteConfigs.map((site) => <div key={site.key}><span>{site.name}</span><strong>{site.languages.join(' / ')} · {site.defaultCurrency}</strong></div>)}</div></section>
      </div>
    </section>
  );
}
