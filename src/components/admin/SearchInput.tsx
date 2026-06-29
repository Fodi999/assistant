import type { ChangeEvent } from 'react';
import { AppIcon } from '../AppIcon';

type SearchInputProps = {
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
};

export function SearchInput({ value, placeholder = 'Search', onChange }: SearchInputProps) {
  function handleChange(event: ChangeEvent<HTMLInputElement>) {
    onChange(event.target.value);
  }

  return (
    <label className="admin-search-input">
      <AppIcon name="search" />
      <input value={value} placeholder={placeholder} onChange={handleChange} />
    </label>
  );
}
