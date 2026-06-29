export type DataSource = 'api' | 'unavailable';

interface DataSourceBadgeProps {
  source: DataSource;
  label?: string;
}

export function DataSourceBadge({ source, label }: DataSourceBadgeProps) {
  return (
    <span className={`data-source-badge ${source}`}>
      <i />
      {label ? `${label}: ` : null}{source === 'api' ? 'API' : 'Нет API'}
    </span>
  );
}
