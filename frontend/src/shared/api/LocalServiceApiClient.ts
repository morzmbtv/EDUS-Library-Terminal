import { localRequest as request } from './localTransport.ts';
import {
  DomainError,
  type Snapshot,
  type Reader,
  type Loan,
  type BasketItem,
  type CodeResult,
  type OperationResult,
} from '../types/terminalTypes.ts';
import type { CatalogAvailability, CatalogSearchResult, CatalogSort } from '../types/catalogSearch.ts';
export const FRONTEND_VERSION = '2.0.0-rc.1';
export interface ApiVersion {
  backend_version: string;
  local_api_version: string;
  minimum_frontend_version: string;
  maximum_frontend_version: string;
}
export interface RuntimeStatus {
  terminalTest: boolean;
  uat: boolean;
  buildLabel: string;
}
export interface SyncStatus {
  state: string;
  localReady: boolean;
  online: boolean;
  pendingCount: number;
  conflictCount: number;
  lastSyncAt: string | null;
  lastError: string | null;
  testCloud: boolean;
}
const post = <T>(path: string, body: unknown) =>
  request<T>(path, { method: 'POST', body: JSON.stringify(body) }, true);
const wireItems = (items: readonly BasketItem[]) =>
  items.map((item) => ({ ...item, copyId: item.copyId ?? null, loanId: item.loanId ?? null }));
/** One same-origin client. Display snapshots are disposable. Every decision/commit belongs to the backend. */
export class LocalServiceApiClient {
  getVersion() {
    return request<ApiVersion>('/version');
  }
  getRuntimeStatus() {
    return request<RuntimeStatus>('/runtime/capabilities');
  }
  getSnapshot() {
    return request<Snapshot>('/snapshot');
  }
  identifyByCard(code: string) {
    return post<{ reader: Reader }>('/identify/card', { code });
  }
  identifyByFace() {
    return post<Reader>('/identify/face', {});
  }
  getReaderLoans(readerId: string) {
    return post<Loan[]>('/reader-loans', { readerId });
  }
  searchBooks(query: string, sort: CatalogSort = 'relevance') {
    return post<CatalogSearchResult[]>('/catalog/search', { query, sort });
  }
  getBookAvailability(titleId: string) {
    return post<CatalogAvailability>('/catalog/availability', { titleId });
  }
  resolveCode(code: string) {
    return post<CodeResult>('/resolve-code', { code });
  }
  issueBooks(readerId: string, items: readonly BasketItem[], operationId: string) {
    return post<OperationResult>('/issue', { readerId, items: wireItems(items), operationId });
  }
  returnBooks(readerId: string, items: readonly BasketItem[], operationId: string) {
    return post<OperationResult>('/return', { readerId, items: wireItems(items), operationId });
  }
  createReservation(readerId: string, titleId: string, operationId: string) {
    return post<OperationResult>('/reservation', { readerId, titleId, operationId });
  }
  cancelReservation(readerId: string, reservationId: string, operationId: string) {
    return post<OperationResult>('/reservation/cancel', { readerId, reservationId, operationId });
  }
  getSyncStatus() {
    return request<SyncStatus>('/sync/status');
  }
  getSettings() {
    return request<{ locale: 'ru' | 'kk' }>('/settings/ui');
  }
  updateLanguage(locale: 'ru' | 'kk') {
    return post('/settings/ui', { locale });
  }
  checkOperation(operationId: string) {
    return request<OperationResult | null>('/operations/' + encodeURIComponent(operationId));
  }
  subscribe(onUpdate: () => void, onDisconnect: () => void) {
    const events = new EventSource('/api/local/v1/events', { withCredentials: true });
    events.addEventListener('status', onUpdate);
    events.onmessage = onUpdate;
    events.onerror = onDisconnect;
    return events;
  }
}
export const api = new LocalServiceApiClient();
export function compatibleVersion(version: ApiVersion) {
  return (
    version.local_api_version === '1' &&
    version.minimum_frontend_version === FRONTEND_VERSION &&
    version.maximum_frontend_version === FRONTEND_VERSION
  );
}
export async function createLocalServiceApiClient() {
  let cache = await api.getSnapshot();
  const refresh = async () => {
    cache = await api.getSnapshot();
  };
  async function committed(action: Promise<OperationResult>) {
    const result = await action;
    try {
      await refresh();
    } catch {
      throw new DomainError(
        'UNKNOWN',
        'Операция записана. Проверьте сохранённый результат.',
        result.operationId,
      );
    }
    return result;
  }
  return {
    snapshot: () => structuredClone(cache),
    refreshAsync: refresh,
    resolveCodeAsync: (code: string) => api.resolveCode(code),
    identifyCard: async (code: string) => {
      const result = await api.identifyByCard(code);
      await refresh();
      return result.reader;
    },
    issue: (readerId: string, items: readonly BasketItem[], operationId: string) =>
      committed(api.issueBooks(readerId, items, operationId)),
    accept: (readerId: string, items: readonly BasketItem[], operationId: string) =>
      committed(api.returnBooks(readerId, items, operationId)),
    reserve: (readerId: string, titleId: string, operationId: string) =>
      committed(api.createReservation(readerId, titleId, operationId)),
    checkOperation: (id: string) => api.checkOperation(id),
  };
}
