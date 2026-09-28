import test from 'node:test';
import assert from 'node:assert/strict';
import { localRequest } from '../src/shared/api/localTransport.ts';

test('session expires before dispatch: renew once and preserve mutation operationId', async () => {
  const original = globalThis.fetch;
  const bodies: string[] = [];
  let mutation = 0;
  globalThis.fetch = async (url, options) => {
    if (String(url).endsWith('/session/renew')) return Response.json({ csrfToken: 'fresh' });
    bodies.push(String(options?.body));
    mutation++;
    return mutation === 1
      ? Response.json({ code: 'LOCAL_SESSION_REQUIRED' }, { status: 401 })
      : Response.json({ operationId: 'stable-id' });
  };
  try {
    const result = await localRequest(
      '/issue',
      { method: 'POST', body: JSON.stringify({ operationId: 'stable-id' }) },
      true,
    );
    assert.deepEqual(result, { operationId: 'stable-id' });
    assert.equal(bodies.length, 2);
    assert.equal(bodies[0], bodies[1]);
  } finally {
    globalThis.fetch = original;
  }
});
test('uncertain network response never blindly repeats issue', async () => {
  const original = globalThis.fetch;
  let mutations = 0;
  globalThis.fetch = async () => {
    mutations++;
    throw new TypeError('connection lost after commit');
  };
  try {
    await assert.rejects(localRequest('/issue', { method: 'POST', body: '{"operationId":"stable"}' }, true), {
      code: 'UNKNOWN',
      operationId: 'stable',
    });
    assert.equal(mutations, 1);
  } finally {
    globalThis.fetch = original;
  }
});
