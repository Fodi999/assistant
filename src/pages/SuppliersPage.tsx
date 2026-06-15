import { useEffect, useState } from 'react';
import { listSuppliersWithSource } from '../api/suppliers';
import { DataSourceBadge, type DataSource } from '../components/DataSourceBadge';
import { suppliers } from '../lib/mockData';
import { AppIcon } from '../components/AppIcon';
import { supplierTypeLabels } from '../lib/labels';

export function SuppliersPage() {
  const [rows, setRows] = useState(suppliers);
  const [source, setSource] = useState<DataSource>('mock');
  const [sourceError, setSourceError] = useState<string | undefined>();

  useEffect(() => {
    void listSuppliersWithSource().then((result) => {
      setRows(result.data);
      setSource(result.source);
      setSourceError(result.error);
    });
  }, []);

  return (
    <section className="ops-page">
      <div className="ops-header"><div className="ops-header-icon"><AppIcon name="suppliers" /></div><div><p className="eyebrow">Партнерская сеть</p><h2>Поставщики</h2><p>Маркетплейсы, локальные поставщики, производители и партнерские продавцы с комиссиями и контактами.</p></div><div className="ops-header-actions"><DataSourceBadge source={source} label="Поставщики" /></div></div>
      {sourceError ? <p className="ops-alert"><AppIcon name="terminal" />API не вернул поставщиков: {sourceError}. Показаны mock-данные.</p> : null}
      <section className="ops-panel"><table className="ops-table"><thead><tr><th>Поставщик</th><th>Тип</th><th>Локация</th><th>Категории</th><th>Комиссия</th></tr></thead><tbody>{rows.map((supplier) => <tr key={supplier.id}><td><strong>{supplier.name}</strong><small>{supplier.contact}</small></td><td>{supplierTypeLabels[supplier.type]}</td><td>{supplier.country} {supplier.city}</td><td>{supplier.categories.join(', ')}</td><td>{supplier.commissionTerms}</td></tr>)}</tbody></table></section>
    </section>
  );
}
