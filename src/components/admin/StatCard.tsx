import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';

type StatCardProps = {
  title: string;
  value: ReactNode;
  hint?: string;
  icon: AppIconName;
  tone?: 'default' | 'good' | 'warning' | 'danger';
};

export function StatCard({ title, value, hint, icon, tone = 'default' }: StatCardProps) {
  return (
    <article className={'admin-stat-card ' + tone}>
      <div className="admin-stat-head">
        <span><AppIcon name={icon} /></span>
        {hint ? <small>{hint}</small> : null}
      </div>
      <p>{title}</p>
      <strong>{value}</strong>
    </article>
  );
}
