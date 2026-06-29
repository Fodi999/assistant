import { AdminResourcePage } from './AdminResourcePage';
import { listAnalyticsRows } from '../../services/admin/analyticsService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';

export function AnalyticsPage() {
  return (
    <AdminResourcePage
      title="Analytics"
      eyebrow="Insights"
      description="Traffic, conversion and operational metrics."
      icon="analytics"
      actionLabel="Add report"
      loadRows={listAnalyticsRows}
      capabilities={resourceCapabilities.analytics}
    />
  );
}
