import type { RuntimeConfig } from './runtimeConfig.ts'

/** Candidate Safe School library contract from SPEC v1.2. It is intentionally
 * isolated until the server repository confirms each DTO and endpoint. */
export type LibraryApiErrorCode =
  | 'LOCAL_SERVICE_UNAVAILABLE' | 'REQUEST_TIMEOUT' | 'INVALID_RESPONSE'
  | 'READER_NOT_FOUND' | 'BOOK_NOT_FOUND' | 'SCAN_UNKNOWN' | 'SCAN_AMBIGUOUS'
  | 'BOOK_ALREADY_ON_LOAN' | 'BOOK_NOT_ON_LOAN' | 'LEGACY_READER_REQUIRED'
  | 'LEGACY_LOAN_NOT_FOUND' | 'IDENTIFIER_CONFLICT' | 'VALIDATION_ERROR'
  | 'FORBIDDEN' | 'SESSION_EXPIRED' | 'OPERATION_PENDING' | 'OPERATION_ALREADY_APPLIED'

export class LibraryApiError extends Error {
  readonly code: LibraryApiErrorCode
  readonly status?: number
  readonly operationId?: string

  constructor(code: LibraryApiErrorCode, message: string, status?: number, operationId?: string) {
    super(message)
    this.name = 'LibraryApiError'
    this.code = code
    this.status = status
    this.operationId = operationId
  }
}

export interface ApiEnvelope<T> { readonly success: boolean; readonly code?: string; readonly message?: string; readonly details?: T; readonly operation_id?: string }
export interface LibraryApiTransport { fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> }

const knownCodes = new Set<LibraryApiErrorCode>([
  'READER_NOT_FOUND', 'BOOK_NOT_FOUND', 'SCAN_UNKNOWN', 'SCAN_AMBIGUOUS', 'BOOK_ALREADY_ON_LOAN', 'BOOK_NOT_ON_LOAN',
  'LEGACY_READER_REQUIRED', 'LEGACY_LOAN_NOT_FOUND', 'IDENTIFIER_CONFLICT', 'VALIDATION_ERROR', 'FORBIDDEN',
  'SESSION_EXPIRED', 'LOCAL_SERVICE_UNAVAILABLE', 'OPERATION_PENDING', 'OPERATION_ALREADY_APPLIED',
])

function codeFrom(value: unknown, fallback: LibraryApiErrorCode) {
  return typeof value === 'string' && knownCodes.has(value as LibraryApiErrorCode) ? value as LibraryApiErrorCode : fallback
}

export class LibraryApiClient {
  private readonly config: RuntimeConfig
  private readonly transport: LibraryApiTransport

  constructor(config: RuntimeConfig, transport?: LibraryApiTransport) {
    this.config = config
    if (transport) {
      this.transport = transport
    } else if (typeof window !== 'undefined') {
      this.transport = { fetch: window.fetch.bind(window) }
    } else {
      throw new LibraryApiError('LOCAL_SERVICE_UNAVAILABLE', 'Транспорт библиотечного API недоступен вне браузера.')
    }
  }

  private url(path: string) {
    if (!this.config.apiBaseUrl) throw new LibraryApiError('LOCAL_SERVICE_UNAVAILABLE', 'Адрес локального библиотечного API не настроен.')
    return `${this.config.apiBaseUrl}${this.config.apiPrefix}/${this.config.language}/library/${path.replace(/^\/+/, '')}`
  }

  private async request<T>(method: 'GET' | 'POST', path: string, body?: unknown, timeoutMs = this.config.requestTimeoutMs.read): Promise<T> {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), timeoutMs)
    try {
      const response = await this.transport.fetch(this.url(path), {
        method, credentials: 'include', signal: controller.signal,
        headers: {
          Accept: 'application/json', ...(body === undefined ? {} : { 'Content-Type': 'application/json' }),
          ...(this.config.schoolId ? { 'X-Edus-School-Id': this.config.schoolId } : {}),
          ...(this.config.deviceId ? { 'X-Edus-Device-Id': this.config.deviceId } : {}),
          'X-Edus-Terminal-Type': 'LIBRARY',
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      })
      let payload: ApiEnvelope<T>
      try { payload = await response.json() as ApiEnvelope<T> } catch { throw new LibraryApiError('INVALID_RESPONSE', 'Локальный сервер вернул ответ в неподдерживаемом формате.', response.status) }
      if (!response.ok || !payload.success) throw new LibraryApiError(codeFrom(payload.code, response.status === 401 ? 'SESSION_EXPIRED' : response.status === 403 ? 'FORBIDDEN' : 'LOCAL_SERVICE_UNAVAILABLE'), payload.message || 'Локальный сервер не выполнил операцию.', response.status, payload.operation_id)
      if (payload.details === undefined) throw new LibraryApiError('INVALID_RESPONSE', 'В ответе сервера отсутствуют ожидаемые данные.', response.status, payload.operation_id)
      return payload.details
    } catch (error) {
      if (error instanceof LibraryApiError) throw error
      if (error instanceof DOMException && error.name === 'AbortError') throw new LibraryApiError('REQUEST_TIMEOUT', 'Превышено время ожидания локального сервера.')
      throw new LibraryApiError('LOCAL_SERVICE_UNAVAILABLE', 'Нет связи с локальным библиотечным сервером.')
    } finally { clearTimeout(timer) }
  }

  health() { return this.request<unknown>('GET', 'health', undefined, this.config.requestTimeoutMs.read) }
  readerByCard(rawCardCode: string) { return this.request<unknown>('GET', `readers/by-card/${encodeURIComponent(rawCardCode)}`) }
  searchReaders(query: string) { return this.request<unknown>('GET', `readers/search?q=${encodeURIComponent(query)}`) }
  resolveScan(rawCode: string) { return this.request<unknown>('POST', 'scan/resolve', { code: rawCode }) }
  /** The candidate API accepts the operation id in the transaction body so a
   * retry can be reconciled without replaying a write. Confirm this field with
   * the real library server before wiring this client into the terminal. */
  issue(payload: Record<string, unknown>, operationId: string) {
    return this.request<unknown>('POST', 'loans/issue', { ...payload, operation_id: operationId }, this.config.requestTimeoutMs.write)
  }
  accept(payload: Record<string, unknown>, operationId: string) {
    return this.request<unknown>('POST', 'loans/return', { ...payload, operation_id: operationId }, this.config.requestTimeoutMs.write)
  }
  operation(operationId: string) { return this.request<unknown>('GET', `operations/${encodeURIComponent(operationId)}`, undefined, this.config.requestTimeoutMs.reconcile) }
}
