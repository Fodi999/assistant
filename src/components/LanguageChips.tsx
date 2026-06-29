import { languages } from '../lib/siteConfig';
import type { LanguageCode } from '../types/admin';

interface LanguageChipsProps {
  value: LanguageCode[];
  onChange?: (value: LanguageCode[]) => void;
}

export function LanguageChips({ value, onChange }: LanguageChipsProps) {
  function toggle(language: LanguageCode) {
    if (!onChange) return;
    onChange(value.includes(language) ? value.filter((item) => item !== language) : [...value, language]);
  }

  return (
    <div className="language-chips">
      {languages.map((language) => (
        <button key={language} className={value.includes(language) ? 'active' : ''} type="button" onClick={() => toggle(language)}>
          {language.toUpperCase()}
        </button>
      ))}
    </div>
  );
}
