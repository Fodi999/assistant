import { currencies } from '../lib/siteConfig';
import type { CurrencyCode } from '../types/admin';

interface CurrencyInputProps {
  value?: number;
  currency: CurrencyCode;
  onChange: (value: number | undefined, currency: CurrencyCode) => void;
}

export function CurrencyInput({ value, currency, onChange }: CurrencyInputProps) {
  return (
    <div className="currency-input">
      <input value={value ?? ''} type="number" min="0" placeholder="0" onChange={(event) => onChange(event.target.value ? Number(event.target.value) : undefined, currency)} />
      <select value={currency} onChange={(event) => onChange(value, event.target.value as CurrencyCode)}>
        {currencies.map((item) => <option key={item} value={item}>{item}</option>)}
      </select>
    </div>
  );
}
