import type { ResourceStatus } from '../../types/admin';

type StatusBadgeProps = {
  status: ResourceStatus | 'online' | 'limited' | 'offline' | 'danger';
  label?: string;
};

export function StatusBadge({ status, label = status }: StatusBadgeProps) {
  return <span className={'admin-status-chip ' + status}><i />{label}</span>;
}
