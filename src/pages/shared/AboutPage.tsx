import { AppIcon } from '../../components/AppIcon';

export function AboutPage() {
  return (
    <section className="ops-page">
      <div className="ops-header">
        <div className="ops-header-icon"><AppIcon name="shield" /></div>
        <div>
          <p className="eyebrow">О системе</p>
          <h2>Админка партнерской ОС</h2>
          <p>Tauri + React + TypeScript desktop-admin для кулинарного и строительного партнерского бизнеса.</p>
        </div>
      </div>
      <section className="ops-panel">
        <div className="ops-list">
          <div><span>Бизнес-модель</span><strong>партнерский каталог + контент + заявки</strong></div>
          <div><span>Сайты</span><strong>кулинарный / строительный</strong></div>
          <div><span>Языки</span><strong>RU / PL / EN / KK</strong></div>
          <div><span>Валюты</span><strong>KZT / PLN / EUR / USD</strong></div>
        </div>
      </section>
    </section>
  );
}
