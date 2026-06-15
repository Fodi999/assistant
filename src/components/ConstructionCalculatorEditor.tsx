import { useMemo, useState } from 'react';
import type { CurrencyCode } from '../types/admin';
import { CurrencyInput } from './CurrencyInput';

export function ConstructionCalculatorEditor() {
  const [areaM2, setAreaM2] = useState(42);
  const [materialCost, setMaterialCost] = useState(690000);
  const [workCost, setWorkCost] = useState(540000);
  const [marginPercent, setMarginPercent] = useState(18);
  const [currency, setCurrency] = useState<CurrencyCode>('KZT');
  const total = useMemo(() => Math.round((materialCost + workCost) * (1 + marginPercent / 100)), [materialCost, workCost, marginPercent]);

  return (
    <section className="editor-card">
      <div className="panel-title"><span>Калькулятор ремонта</span><small>Алматы / {currency}</small></div>
      <div className="editor-grid">
        <label className="editor-field"><span>Площадь, м2</span><input type="number" value={areaM2} onChange={(event) => setAreaM2(Number(event.target.value))} /></label>
        <label className="editor-field"><span>Цена материала</span><CurrencyInput value={materialCost} currency={currency} onChange={(value, nextCurrency) => { setMaterialCost(value ?? 0); setCurrency(nextCurrency); }} /></label>
        <label className="editor-field"><span>Цена работы</span><CurrencyInput value={workCost} currency={currency} onChange={(value, nextCurrency) => { setWorkCost(value ?? 0); setCurrency(nextCurrency); }} /></label>
        <label className="editor-field"><span>Маржа, %</span><input type="number" value={marginPercent} onChange={(event) => setMarginPercent(Number(event.target.value))} /></label>
      </div>
      <div className="total-strip"><span>Итоговая цена для клиента</span><strong>{total.toLocaleString('ru-RU')} {currency}</strong><small>{Math.round(total / Math.max(areaM2, 1)).toLocaleString('ru-RU')} {currency}/м2</small></div>
    </section>
  );
}
