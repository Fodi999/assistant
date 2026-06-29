import { useState } from 'react';
import type { LocalizedAdminTextDto } from '../../../types/adminApi';

export type LangTab = 'uk' | 'ru' | 'en';

export const langTabs: Array<{ id: LangTab; label: string }> = [
  { id: 'uk', label: 'UK' },
  { id: 'ru', label: 'RU' },
  { id: 'en', label: 'EN' }
];

export type FormErrors = Record<string, string>;

export function useLangTab() {
  return useState<LangTab>('uk');
}

export function isValidSlug(slug: string) {
  return !slug || /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug);
}

export function isValidUrl(url: string) {
  if (!url) return true;
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

export function optionalNumber(value: string): number | undefined {
  if (!value.trim()) return undefined;
  return Number(value);
}

export function optionalInteger(value: string): number | undefined {
  if (!value.trim()) return undefined;
  return Number.parseInt(value, 10);
}

export function csv(value: string): string[] {
  return value.split(',').map((item) => item.trim()).filter(Boolean);
}

export function firstText(value?: LocalizedAdminTextDto): string {
  return value?.uk || value?.ru || value?.en || '';
}

export function FieldError({ message }: { message?: string }) {
  return message ? <small className="admin-form-error">{message}</small> : null;
}

export function LanguageTabs({ active, onChange }: { active: LangTab; onChange: (lang: LangTab) => void }) {
  return (
    <div className="admin-form-tabs" role="tablist" aria-label="Language">
      {langTabs.map((tab) => (
        <button key={tab.id} type="button" className={active === tab.id ? 'active' : ''} onClick={() => onChange(tab.id)}>
          {tab.label}
        </button>
      ))}
    </div>
  );
}
