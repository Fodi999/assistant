import type { ConstructionBundle } from '../types/admin';
import { publishStatusLabels } from '../lib/labels';

interface BundleEditorProps {
  bundles: ConstructionBundle[];
}

export function BundleEditor({ bundles }: BundleEditorProps) {
  return (
    <div className="card-grid">
      {bundles.map((bundle) => (
        <article key={bundle.id} className="site-card">
          <div><h3>{bundle.title.ru}</h3><span className="status-pill info"><i />{publishStatusLabels[bundle.status]}</span></div>
          <p>{bundle.city} / {bundle.slug}</p>
          <dl>
            <dt>Материалы</dt><dd>{bundle.materials.join(', ')}</dd>
            <dt>Работы</dt><dd>{bundle.works.join(', ')}</dd>
            <dt>Цена</dt><dd>{(bundle.totalPrice ?? 0).toLocaleString('ru-RU')} {bundle.currency}</dd>
            <dt>Форма заявки</dt><dd>{bundle.leadFormEnabled ? 'включена' : 'выключена'}</dd>
          </dl>
        </article>
      ))}
    </div>
  );
}
