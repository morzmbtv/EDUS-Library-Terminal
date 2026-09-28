import test from 'node:test';
import assert from 'node:assert/strict';
import type { Snapshot } from '../src/shared/types/terminalTypes.ts';
const fixture: Snapshot = {
  readers: [{ id: 'reader', name: 'TEST READER', group: '7A', card: '***' }],
  titles: [
    {
      id: 'title',
      name: 'TEST BOOK',
      author: 'TEST AUTHOR',
      isbn: '123',
      publisher: '',
      year: 2026,
      language: 'kk',
      subject: '',
      grade: '',
    },
  ],
  copies: [{ id: 'copy', titleId: 'title', code: '0007', status: 'AVAILABLE' }],
  loans: [],
  reservations: [],
  legacyStock: {},
  connection: { server: true, internet: false },
  persistence: 'sqlcipher',
};
test('reader-first navigation, scan draft, explicit commit and uncertain receipt recovery use HTTP only', async () => {
  const original = globalThis.fetch;
  let issueCalls = 0,
    unknown = false,
    committed = false;
  let lastOperation = '';
  globalThis.fetch = async (url, options) => {
    const path = String(url),
      input = options?.body ? JSON.parse(String(options.body)) : {};
    if (path.endsWith('/session/renew')) return Response.json({ csrfToken: 'test' });
    if (path.endsWith('/snapshot')) return Response.json(fixture);
    if (path.endsWith('/identify/card')) {
      assert.equal(input.code, '000001');
      return Response.json({ reader: fixture.readers[0] });
    }
    if (path.endsWith('/resolve-code'))
      return Response.json({ kind: 'copy', copy: fixture.copies[0], title: fixture.titles[0] });
    if (path.endsWith('/issue')) {
      issueCalls++;
      lastOperation = input.operationId;
      assert.equal(input.readerId, 'reader');
      assert.equal(input.items.length, 1);
      committed = true;
      if (unknown) throw new TypeError('lost acknowledgement');
      return Response.json({
        operationId: input.operationId,
        type: 'issue',
        quantity: 1,
        copyIds: ['copy'],
        loanIds: ['loan'],
      });
    }
    if (path.includes('/operations/'))
      return Response.json(
        committed
          ? { operationId: lastOperation, type: 'issue', quantity: 1, copyIds: ['copy'], loanIds: ['loan'] }
          : null,
      );
    throw new Error('Unexpected endpoint ' + path);
  };
  try {
    const ui = await import('../src/features/terminal/useTerminal.ts');
    ui.start('issue');
    assert.equal(ui.screen.value, 'identify');
    ui.openFace();
    assert.equal(ui.screen.value, 'face');
    assert.equal(ui.operation.value, 'issue');
    ui.back();
    assert.equal(ui.screen.value, 'identify');
    assert.equal(ui.faceCaptureActive.value, false);
    ui.back();
    assert.equal(ui.screen.value, 'home');
    ui.start('accept');
    ui.openFace();
    ui.openCard();
    assert.equal(ui.operation.value, 'accept');
    assert.equal(ui.screen.value, 'identify');
    ui.back();
    ui.start('issue');
    await ui.identify('000001');
    assert.equal(ui.screen.value, 'scan');
    await ui.scan('0007');
    assert.equal(ui.basket.value.length, 1);
    await ui.scan('0007');
    assert.equal(ui.basket.value.length, 1);
    assert.equal(fixture.loans.length, 0);
    assert.equal(issueCalls, 0);
    ui.confirm();
    assert.equal(ui.screen.value, 'confirm');
    unknown = true;
    await ui.submit();
    assert.equal(ui.screen.value, 'unknown');
    assert.equal(issueCalls, 1);
    assert.equal(ui.basket.value.length, 1);
    await ui.submit();
    assert.equal(issueCalls, 1);
    await ui.checkResult();
    assert.equal(ui.screen.value, 'success');
    assert.equal(ui.result.value?.operationId, lastOperation);
    ui.home();
    ui.openLibrarySearch();
    ui.beginCatalogueReservation('title');
    assert.equal(ui.screen.value, 'identify');
    ui.back();
    assert.equal(ui.screen.value, 'library-search');
  } finally {
    globalThis.fetch = original;
  }
});
