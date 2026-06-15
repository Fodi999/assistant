interface ConnectionBadgeProps {
  state: 'online' | 'limited' | 'offline';
}

const LABELS: Record<ConnectionBadgeProps['state'], string> = {
  online: 'Бэкенд онлайн',
  limited: 'Ограниченный режим',
  offline: 'Бэкенд офлайн'
};

export function ConnectionBadge({ state }: ConnectionBadgeProps) {
  return (
    <span className={`connection-badge connection-${state}`}>
      <span className="connection-dot" aria-hidden="true" />
      {LABELS[state]}
    </span>
  );
}
