import type { ReactNode } from 'react';

export type DataTableColumn<T> = {
  key: string;
  header: ReactNode;
  render: (row: T) => ReactNode;
  align?: 'left' | 'right' | 'center';
};

type DataTableProps<T> = {
  columns: Array<DataTableColumn<T>>;
  rows: T[];
  getRowKey: (row: T, index: number) => string;
  empty?: ReactNode;
};

export function DataTable<T>({ columns, rows, getRowKey, empty }: DataTableProps<T>) {
  if (!rows.length) {
    return <>{empty ?? <p className="admin-table-empty">No data</p>}</>;
  }

  return (
    <div className="admin-table-wrap">
      <table className="admin-data-table">
        <thead>
          <tr>
            {columns.map((column) => <th key={column.key} className={column.align ? 'align-' + column.align : undefined}>{column.header}</th>)}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr key={getRowKey(row, index)}>
              {columns.map((column) => <td key={column.key} className={column.align ? 'align-' + column.align : undefined}>{column.render(row)}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
