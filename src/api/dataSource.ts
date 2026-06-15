import type { DataSource } from '../components/DataSourceBadge';

export interface SourcedData<T> {
  data: T;
  source: DataSource;
  error?: string;
}

export async function withDataSource<T>(request: Promise<T>, mock: T): Promise<SourcedData<T>> {
  try {
    return { data: await request, source: 'api' };
  } catch (error) {
    return {
      data: mock,
      source: 'mock',
      error: error instanceof Error ? error.message : 'API недоступен'
    };
  }
}
