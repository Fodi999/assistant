interface ConnectionBadgeProps {
  state: 'online' | 'limited' | 'offline';
}

const LABELS: Record<ConnectionBadgeProps['state'], string> = {
  online: 'Backend online',
  limited: 'Limited mode',
  offline: 'Backend offline'
};

export function ConnectionBadge({ state }: ConnectionBadgeProps) {
  return (
    <span className={`connection-badge connection-${state}`}>
      <span className="connection-dot" aria-hidden="true" />
      {LABELS[state]}
    </span>
  );
}
