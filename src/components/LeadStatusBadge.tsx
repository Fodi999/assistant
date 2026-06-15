import type { LeadStatus } from '../types/admin';
import { leadStatusLabels } from '../lib/labels';

const tone: Record<LeadStatus, string> = {
  new: 'info',
  contacted: 'warning',
  quoted: 'warning',
  won: 'good',
  lost: 'danger'
};

export function LeadStatusBadge({ status }: { status: LeadStatus }) {
  return <span className={`status-pill ${tone[status]}`}><i />{leadStatusLabels[status]}</span>;
}
