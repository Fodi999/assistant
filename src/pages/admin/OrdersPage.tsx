import { AdminResourcePage } from './AdminResourcePage';
import { listOrders } from '../../services/admin/ordersService';
import { resourceCapabilities } from '../../services/admin/resourceCapabilities';

export function OrdersPage() {
  return (
    <AdminResourcePage
      title="Orders"
      eyebrow="Operations"
      description="Orders, estimates and purchase requests for the active site."
      icon="package"
      actionLabel="Add order"
      loadRows={listOrders}
      capabilities={resourceCapabilities.orders}
    />
  );
}
