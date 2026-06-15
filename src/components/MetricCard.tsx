import { AppIcon } from './AppIcon';

interface MetricCardProps {
  title: string;
  value: string;
  note?: string;
  icon?: 'users' | 'building' | 'box' | 'trend';
  tone?: 'purple' | 'orange' | 'green';
}

export function MetricCard({ title, value, note, icon = 'trend', tone = 'purple' }: MetricCardProps) {
  return (
    <article className={`metric-card metric-${tone}`}>
      <span className="metric-icon"><AppIcon name={icon} /></span>
      <p className="metric-title">{title}</p>
      <p className="metric-value">{value}</p>
      {note ? <p className="metric-note">{note}</p> : null}
    </article>
  );
}
