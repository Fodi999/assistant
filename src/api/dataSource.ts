import type { DataSource } from '../components/DataSourceBadge';

export interface SourcedData<T> {
  data: T;
  source: DataSource;
  error?: string;
}

export async function withDataSource<T>(request: Promise<T>, emptyData: T): Promise<SourcedData<T>> {
  try {
    return { data: await request, source: 'api' };
  } catch (error) {
    return {
      data: emptyData,
      source: 'unavailable',
      error: error instanceof Error ? error.message : 'API недоступен'
    };
  }
}
