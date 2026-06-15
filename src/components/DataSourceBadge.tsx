export type DataSource = 'api' | 'mock';

interface DataSourceBadgeProps {
  source: DataSource;
  label?: string;
}

export function DataSourceBadge({ source, label }: DataSourceBadgeProps) {
  return (
    <span className={`data-source-badge ${source}`}>
      <i />
      {label ? `${label}: ` : null}{source === 'api' ? 'API' : 'MOCK'}
    </span>
  );
}
