import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { api, compatibleVersion, FRONTEND_VERSION } from '../src/shared/api/LocalServiceApiClient.ts';
type Schema = {
  type?: string | string[];
  $ref?: string;
  required?: string[];
  properties?: Record<string, Schema>;
  additionalProperties?: boolean;
  items?: Schema;
  enum?: unknown[];
  anyOf?: Schema[];
};
type Operation = {
  requestBody?: { content: Record<string, { schema: Schema }> };
  responses: Record<string, { content?: Record<string, { schema: Schema }> }>;
};
const contract = JSON.parse(
  fs.readFileSync(new URL('../openapi/local-service-v1.openapi.json', import.meta.url), 'utf8'),
) as { paths: Record<string, Record<string, Operation>>; components: { schemas: Record<string, Schema> } };
function validate(value: unknown, schema: Schema) {
  if (schema.$ref) return validate(value, contract.components.schemas[schema.$ref.split('/').at(-1)!]!);
  if (schema.anyOf) {
    assert.ok(
      schema.anyOf.some((s) => {
        try {
          validate(value, s);
          return true;
        } catch {
          return false;
        }
      }),
    );
    return;
  }
  if (Array.isArray(schema.type)) {
    assert.ok(
      schema.type.some((type) => {
        try {
          validate(value, { ...schema, type });
          return true;
        } catch {
          return false;
        }
      }),
    );
    return;
  }
  if (schema.enum) assert.ok(schema.enum.includes(value));
  if (schema.type === 'null') {
    assert.equal(value, null);
    return;
  }
  if (schema.type === 'string') {
    assert.equal(typeof value, 'string');
    return;
  }
  if (schema.type === 'integer') {
    assert.ok(Number.isSafeInteger(value));
    return;
  }
  if (schema.type === 'boolean') {
    assert.equal(typeof value, 'boolean');
    return;
  }
  if (schema.type === 'array') {
    assert.ok(Array.isArray(value));
    for (const item of value) validate(item, schema.items!);
    return;
  }
  if (schema.type === 'object') {
    assert.ok(value && typeof value === 'object' && !Array.isArray(value));
    for (const key of schema.required ?? []) assert.ok(key in value, key);
    for (const [key, item] of Object.entries(value)) {
      if (schema.properties?.[key]) validate(item, schema.properties[key]!);
      else if (schema.additionalProperties === false) assert.fail('Unknown property ' + key);
    }
  }
}
test('version compatibility fails closed for unsupported protocol and release', () => {
  const good = {
    backend_version: FRONTEND_VERSION,
    local_api_version: '1',
    minimum_frontend_version: FRONTEND_VERSION,
    maximum_frontend_version: FRONTEND_VERSION,
  };
  assert.equal(compatibleVersion(good), true);
  assert.equal(compatibleVersion({ ...good, local_api_version: '2' }), false);
  assert.equal(compatibleVersion({ ...good, minimum_frontend_version: '3.0.0' }), false);
});
test('client sends domain requests matching backend-owned OpenAPI without privileged commands', async () => {
  const original = globalThis.fetch;
  const visited = new Set<string>();
  globalThis.fetch = async (url, options) => {
    const path = String(url),
      verb = options?.method?.toLowerCase() ?? 'get';
    const operation = contract.paths[path]?.[verb];
    assert.ok(operation, 'Undocumented ' + verb + ' ' + path);
    visited.add(path);
    assert.equal(options?.credentials, 'same-origin');
    if (operation.requestBody)
      validate(JSON.parse(String(options?.body)), operation.requestBody.content['application/json']!.schema);
    return Response.json(path.endsWith('/session/renew') ? { csrfToken: 'test-only' } : {});
  };
  try {
    await api.getRuntimeStatus();
    await api.getSnapshot();
    await api.identifyByCard('0009');
    await api.getReaderLoans('reader');
    await api.identifyByFace();
    await api.searchBooks('Абай', 'title');
    await api.getBookAvailability('title');
    await api.issueBooks(
      'reader',
      [{ id: 'copy', titleId: 'title', copyId: 'copy', quantity: 1, mode: 'COPY' }],
      'issue-op',
    );
    await api.returnBooks(
      'reader',
      [{ id: 'loan', loanId: 'loan', titleId: 'title', quantity: 1, mode: 'LEGACY_TITLE' }],
      'return-op',
    );
    await api.createReservation('reader', 'title', 'reserve-op');
    await api.cancelReservation('reader', 'reservation', 'cancel-op');
    await api.updateLanguage('kk');
    await api.getSettings();
    await api.getSyncStatus();
    assert.equal(visited.size, 14);
  } finally {
    globalThis.fetch = original;
  }
});
test('schema validator rejects missing IDs, wrong quantity types, and unwanted fields', () => {
  const schema =
    contract.paths['/api/local/v1/issue']!.post!.requestBody!.content['application/json']!.schema;
  assert.throws(() => validate({}, schema));
  assert.throws(() =>
    validate(
      { readerId: 'r', operationId: 'o', items: [{ id: 'a', titleId: 'b', quantity: '1', mode: 'COPY' }] },
      schema,
    ),
  );
  assert.throws(() => validate({ readerId: 'r', operationId: 'o', items: [], sql: 'DELETE' }, schema));
});
test('SSE uses documented status event and owns closeable subscription', () => {
  const original = globalThis.EventSource;
  let updates = 0,
    errors = 0,
    closed = false;
  const handlers = new Map<string, () => void>();
  class Events {
    onmessage: (() => void) | null = null;
    onerror: (() => void) | null = null;
    constructor(url: string) {
      assert.equal(url, '/api/local/v1/events');
    }
    addEventListener(name: string, fn: () => void) {
      handlers.set(name, fn);
    }
    close() {
      closed = true;
    }
  }
  globalThis.EventSource = Events as unknown as typeof EventSource;
  try {
    const events = api.subscribe(
      () => updates++,
      () => errors++,
    );
    handlers.get('status')!();
    events.onerror!(new Event('error'));
    events.close();
    assert.equal(updates, 1);
    assert.equal(errors, 1);
    assert.equal(closed, true);
  } finally {
    globalThis.EventSource = original;
  }
});
