import assert from 'node:assert/strict'
import test from 'node:test'
import { LibraryApiClient, LibraryApiError } from '../src/infrastructure/libraryApi.ts'
import { apiConfigured, runtimeConfig, type RuntimeConfig } from '../src/infrastructure/runtimeConfig.ts'

const config: RuntimeConfig = {
  apiBaseUrl: 'https://library.school.local', apiPrefix: '/api', language: 'ru',
  schoolId: 'school-01', deviceId: 'terminal-01', terminalType: 'LIBRARY',
  requestTimeoutMs: { read: 1000, write: 1000, reconcile: 1000 },
}

test('candidate library client sends terminal context and idempotency key without canteen routes', async () => {
  let url = ''
  let init: RequestInit | undefined
  const client = new LibraryApiClient(config, {
    fetch: async (input, options) => {
      url = String(input); init = options
      return new Response(JSON.stringify({ success: true, details: { operation_id: 'op-1' } }), { status: 200 })
    },
  })
  await client.accept({ reader_id: 'reader-1', items: [] }, 'op-1')
  assert.equal(url, 'https://library.school.local/api/ru/library/loans/return')
  assert.equal((init?.headers as Record<string, string>)['X-Edus-School-Id'], 'school-01')
  assert.equal((init?.headers as Record<string, string>)['X-Edus-Device-Id'], 'terminal-01')
  assert.equal((init?.headers as Record<string, string>)['X-Edus-Terminal-Type'], 'LIBRARY')
  assert.deepEqual(JSON.parse(String(init?.body)), { reader_id: 'reader-1', items: [], operation_id: 'op-1' })
})

test('candidate library client keeps backend business errors explicit', async () => {
  const client = new LibraryApiClient(config, {
    fetch: async () => new Response(JSON.stringify({ success: false, code: 'SCAN_AMBIGUOUS', message: 'Выберите издание.' }), { status: 409 }),
  })
  await assert.rejects(client.resolveScan('9786010123456'), (error: unknown) => error instanceof LibraryApiError && error.code === 'SCAN_AMBIGUOUS')
})

test('runtime config is safe by default and requires the three deployment identifiers', () => {
  const safeDefault = runtimeConfig()
  assert.equal(apiConfigured(safeDefault), false)
  assert.equal(safeDefault.apiBaseUrl, '')
  assert.equal(safeDefault.terminalType, 'LIBRARY')
})
