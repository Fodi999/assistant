import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { AppIcon, type AppIconName } from '../AppIcon';

type ActionButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  icon?: AppIconName;
  tone?: 'primary' | 'secondary' | 'danger';
  children: ReactNode;
};

export function ActionButton({ icon, tone = 'secondary', children, className = '', type = 'button', ...props }: ActionButtonProps) {
  return (
    <button className={`admin-btn ${tone} ${className}`.trim()} type={type} {...props}>
      {icon ? <AppIcon name={icon} /> : null}
      <span>{children}</span>
    </button>
  );
}
