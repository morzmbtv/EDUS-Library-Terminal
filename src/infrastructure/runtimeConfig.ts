export interface RuntimeConfig {
  readonly apiBaseUrl: string
  readonly apiPrefix: string
  readonly language: 'ru' | 'kk'
  readonly schoolId: string
  readonly deviceId: string
  readonly terminalType: 'LIBRARY'
  readonly requestTimeoutMs: Readonly<{ read: number; write: number; reconcile: number }>
}

declare global {
  interface Window { __EDUS_LIBRARY_RUNTIME_CONFIG__?: Partial<RuntimeConfig> }
}

const DEFAULTS: RuntimeConfig = {
  apiBaseUrl: '', apiPrefix: '/api', language: 'ru', schoolId: '', deviceId: '', terminalType: 'LIBRARY',
  requestTimeoutMs: { read: 8000, write: 15000, reconcile: 8000 },
}

function cleanPath(value: unknown, fallback: string) {
  if (typeof value !== 'string' || !value.startsWith('/') || value.includes('..')) return fallback
  return value.replace(/\/+$/, '') || fallback
}
function cleanUrl(value: unknown) {
  if (typeof value !== 'string' || !value.trim()) return ''
  try {
    const parsed = new URL(value)
    return ['http:', 'https:'].includes(parsed.protocol) ? parsed.origin : ''
  } catch { return '' }
}
function timeout(value: unknown, fallback: number) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 1000 && value <= 60000 ? value : fallback
}

export function runtimeConfig(): RuntimeConfig {
  const raw = typeof window === 'undefined' ? undefined : window.__EDUS_LIBRARY_RUNTIME_CONFIG__
  const source = raw && typeof raw === 'object' ? raw : {}
  const timing: Record<string, unknown> = source.requestTimeoutMs && typeof source.requestTimeoutMs === 'object'
    ? source.requestTimeoutMs as Record<string, unknown> : {}
  return Object.freeze({
    apiBaseUrl: cleanUrl(source.apiBaseUrl),
    apiPrefix: cleanPath(source.apiPrefix, DEFAULTS.apiPrefix),
    language: source.language === 'kk' ? 'kk' : 'ru',
    schoolId: typeof source.schoolId === 'string' ? source.schoolId.trim() : '',
    deviceId: typeof source.deviceId === 'string' ? source.deviceId.trim() : '',
    terminalType: 'LIBRARY',
    requestTimeoutMs: Object.freeze({
      read: timeout(timing.read, DEFAULTS.requestTimeoutMs.read),
      write: timeout(timing.write, DEFAULTS.requestTimeoutMs.write),
      reconcile: timeout(timing.reconcile, DEFAULTS.requestTimeoutMs.reconcile),
    }),
  })
}

export function apiConfigured(config = runtimeConfig()) { return Boolean(config.apiBaseUrl && config.schoolId && config.deviceId) }
