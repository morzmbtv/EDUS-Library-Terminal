import test from 'node:test'
import assert from 'node:assert/strict'
import { reactive, ref } from 'vue'
import { createMockAdapter, DEMO_CODES, DomainError, type BasketItem, type Title } from '../src/domain/index.ts'

const fresh = () => createMockAdapter({ delay: 0 })
const copyItem = (copyId = 'copy-2', titleId = 'title-1'): BasketItem => ({ id: `basket-${copyId}`, titleId, copyId, quantity: 1, mode: 'COPY' })
const legacyItem = (quantity: number, loanId?: string): BasketItem => ({ id: 'basket-legacy', titleId: 'title-3', quantity, mode: 'LEGACY_TITLE', ...(loanId ? { loanId } : {}) })
const errorCode = (code: string) => (error: unknown): boolean => error instanceof DomainError && error.code === code
const newTitle: Omit<Title, 'id'> = { name: 'География Казахстана', author: 'Б. Әбдіманапов', isbn: '978-601-123456-7', publisher: 'Мектеп', year: 2024, language: 'Қазақша', subject: 'География', grade: '8' }

class MemoryStorage implements Storage {
  private readonly values = new Map<string, string>()
  get length(): number { return this.values.size }
  clear(): void { this.values.clear() }
  getItem(key: string): string | null { return this.values.get(key) ?? null }
  key(index: number): string | null { return [...this.values.keys()][index] ?? null }
  removeItem(key: string): void { this.values.delete(key) }
  setItem(key: string, value: string): void { this.values.set(key, value) }
}

test('ISBN resolves an edition without inventing a physical copy or borrower', () => {
  const adapter = fresh()
  const result = adapter.resolveCode(DEMO_CODES.legacyIsbn)
  assert.equal(result.kind, 'title')
  assert.equal('copy' in result, false)
  assert.equal('loan' in result, false)
  if (result.kind === 'title') assert.equal(result.title.id, 'title-3')
  assert.equal(adapter.resolveCode('978-601-012345-6').kind, 'title')
})

test('inventory lookup preserves leading zeros and accepts alphabetic codes', () => {
  const adapter = fresh()
  const result = adapter.resolveCode(' 000124 ')
  assert.equal(result.kind, 'copy')
  if (result.kind === 'copy') {
    assert.equal(result.copy.code, '000124')
    assert.equal(result.loan, undefined)
  }
  const issued = adapter.resolveCode('kz-0007')
  assert.equal(issued.kind, 'copy')
  if (issued.kind === 'copy') assert.equal(issued.loan?.readerId, 'reader-2')
  assert.equal(adapter.resolveCode('124').kind, 'not-found')
})

test('search supports Kazakh letters, partial names, card numbers and empty results', async () => {
  const adapter = fresh()
  assert.equal(adapter.searchReaders('сәрсенова').length, 2)
  assert.equal(adapter.searchReaders('Әлихан')[0]?.id, 'reader-2')
  assert.equal(adapter.searchReaders('EDUS-1003')[0]?.id, 'reader-3')
  assert.deepEqual(adapter.searchReaders('несуществующий читатель'), [])
  assert.equal(adapter.searchTitles('Қазақ тілі')[0]?.id, 'title-2')
  assert.equal((await adapter.identifyCard(' edus-1001 ')).id, 'reader-1')
  await assert.rejects(adapter.identifyCard('unknown'), errorCode('NOT_FOUND'))
})

test('an ambiguous code explicitly returns candidates', () => {
  const result = fresh().resolveCode(DEMO_CODES.ambiguous)
  assert.equal(result.kind, 'ambiguous')
  if (result.kind === 'ambiguous') assert.equal(result.titles.length, 2)
})

test('issuing a copy changes the actual reader loans and detects duplicate scans', async () => {
  const adapter = fresh()
  const before = adapter.getReaderLoans('reader-1').length
  const result = await adapter.issue('reader-1', [copyItem()], 'issue-1')
  assert.equal(result.quantity, 1)
  assert.equal(adapter.getReaderLoans('reader-1').length, before + 1)
  const resolved = adapter.resolveCode('000124')
  if (resolved.kind === 'copy') assert.equal(resolved.loan?.readerId, 'reader-1')
  await assert.rejects(adapter.issue('reader-2', [copyItem()], 'issue-2'), errorCode('COPY_ISSUED'))
  await assert.rejects(fresh().issue('reader-1', [copyItem(), { ...copyItem(), id: 'duplicate' }], 'duplicate-scan'), errorCode('DUPLICATE_COPY'))
})

test('a failing issue batch is atomic even when an earlier line is valid', async () => {
  const adapter = fresh()
  const before = adapter.snapshot()
  await assert.rejects(adapter.issue('reader-1', [copyItem(), copyItem('copy-1')], 'atomic-issue'), errorCode('COPY_ISSUED'))
  assert.deepEqual(adapter.snapshot(), before)
  assert.equal(await adapter.checkOperation('atomic-issue'), null)
})

test('legacy quantities are explicit and cannot exceed available stock', async () => {
  const adapter = fresh()
  await adapter.issue('reader-1', [legacyItem(3)], 'legacy-issue')
  assert.equal(adapter.getReaderLoans('reader-1').filter((loan) => loan.titleId === 'title-3').reduce((sum, loan) => sum + loan.quantity, 0), 5)
  await assert.rejects(adapter.issue('reader-1', [legacyItem(13)], 'too-many'), errorCode('INSUFFICIENT_STOCK'))
  await assert.rejects(adapter.issue('reader-1', [legacyItem(0)], 'zero'), errorCode('INVALID_INPUT'))
  await assert.rejects(adapter.issue('reader-1', [legacyItem(1.5)], 'fraction'), errorCode('INVALID_INPUT'))
  await assert.rejects(adapter.issue('reader-1', [legacyItem(1), { ...legacyItem(1), id: 'other' }], 'repeat-isbn'), errorCode('INVALID_INPUT'))
})

test('copy return verifies the selected reader and frees the physical book', async () => {
  const adapter = fresh()
  const result = await adapter.accept('reader-1', [copyItem('copy-1')], 'return-copy')
  assert.equal(result.quantity, 1)
  assert.equal(adapter.getReaderLoans('reader-1').some((loan) => loan.copyId === 'copy-1'), false)
  await assert.rejects(adapter.accept('reader-1', [copyItem('copy-1')], 'return-again'), errorCode('NO_LOAN'))
  await adapter.issue('reader-2', [copyItem('copy-1')], 'reissue')
  assert.equal(adapter.getReaderLoans('reader-2').some((loan) => loan.copyId === 'copy-1'), true)
})

test('legacy return requires an exact reader loan and supports partial return', async () => {
  const adapter = fresh()
  await assert.rejects(adapter.accept('reader-1', [legacyItem(1)], 'no-reader'), errorCode('NO_LOAN'))
  await assert.rejects(adapter.accept('reader-1', [legacyItem(3, 'loan-2')], 'excess'), errorCode('EXCESS_QUANTITY'))
  await adapter.accept('reader-1', [legacyItem(1, 'loan-2')], 'partial')
  assert.equal(adapter.getReaderLoans('reader-1').find((loan) => loan.id === 'loan-2')?.quantity, 1)
  assert.equal(adapter.getReaderLoans('reader-2').find((loan) => loan.id === 'loan-4')?.quantity, 1)
  await adapter.accept('reader-1', [legacyItem(1, 'loan-2')], 'remainder')
  assert.equal(adapter.getReaderLoans('reader-1').some((loan) => loan.id === 'loan-2'), false)
})

test('a failed mass return leaves all original loans intact', async () => {
  const adapter = fresh()
  const before = adapter.snapshot()
  await assert.rejects(adapter.accept('reader-1', [copyItem('copy-1'), legacyItem(3, 'loan-2')], 'atomic-return'), errorCode('EXCESS_QUANTITY'))
  assert.deepEqual(adapter.snapshot(), before)
})

test('new edition and its copies register together with real inventory numbers', async () => {
  const adapter = fresh()
  const result = await adapter.register(newTitle, 'COPY', ['000012', 'Қ-008'], 2, 'new-title')
  assert.equal(result.quantity, 2)
  assert.ok(result.titleId)
  assert.equal(adapter.getTitle(result.titleId)?.isbn, '9786011234567')
  assert.equal(adapter.resolveCode('000012').kind, 'copy')
  assert.equal(adapter.resolveCode('Қ-008').kind, 'copy')
})

test('existing inventory codes of the same edition reuse copies', async () => {
  const adapter = fresh()
  const before = adapter.snapshot().copies.length
  const result = await adapter.register('title-1', 'COPY', ['000124', '001999'], 2, 'reuse')
  assert.equal(result.copyIds[0], 'copy-2')
  assert.equal(adapter.snapshot().copies.length, before + 1)
  assert.equal(adapter.snapshot().copies.filter((copy) => copy.code === '000124').length, 1)
})

test('registration rejects duplicate inventory codes and preserves atomicity', async () => {
  const adapter = fresh()
  const before = adapter.snapshot()
  await assert.rejects(adapter.register(newTitle, 'COPY', ['new-1', '000124'], 2, 'collision'), errorCode('DUPLICATE_CODE'))
  assert.deepEqual(adapter.snapshot(), before)
  await assert.rejects(adapter.register(newTitle, 'COPY', ['Book-1', 'book-1'], 2, 'repeated'), errorCode('DUPLICATE_CODE'))
  assert.deepEqual(adapter.snapshot(), before)
  await assert.rejects(adapter.register(newTitle, 'COPY', ['new-1'], 2, 'count-mismatch'), errorCode('INVALID_INPUT'))
  assert.deepEqual(adapter.snapshot(), before)
})

test('new edition duplicate ISBN is rejected while missing ISBN is allowed', async () => {
  const adapter = fresh()
  await assert.rejects(adapter.register({ ...newTitle, isbn: DEMO_CODES.legacyIsbn }, 'LEGACY_TITLE', [], 1, 'duplicate-isbn'), errorCode('DUPLICATE_ISBN'))
  const result = await adapter.register({ ...newTitle, isbn: '' }, 'LEGACY_TITLE', [], 6, 'without-isbn')
  assert.ok(result.titleId)
  assert.equal(adapter.getTitle(result.titleId)?.isbn, '')
  assert.equal(adapter.snapshot().legacyStock[result.titleId], 6)
})

test('unlabelled registration increases stock without inventing individual numbers', async () => {
  const adapter = fresh()
  const before = adapter.snapshot()
  await adapter.register('title-3', 'LEGACY_TITLE', [], 7, 'old-stock')
  assert.equal(adapter.snapshot().legacyStock['title-3'], 25)
  assert.deepEqual(adapter.snapshot().copies, before.copies)
  await assert.rejects(adapter.register('title-3', 'LEGACY_TITLE', ['invented'], 1, 'no-code'), errorCode('INVALID_INPUT'))
})

test('missing local server blocks mutations but missing internet permits local work', async () => {
  const adapter = fresh()
  const before = adapter.snapshot().loans
  adapter.setConnection({ server: false })
  await assert.rejects(adapter.issue('reader-1', [copyItem()], 'offline'), errorCode('SERVER_OFFLINE'))
  await assert.rejects(adapter.identifyCard(DEMO_CODES.readerCard), errorCode('SERVER_OFFLINE'))
  assert.deepEqual(adapter.snapshot().loans, before)
  adapter.setConnection({ server: true, internet: false })
  await adapter.issue('reader-1', [copyItem()], 'offline')
  assert.equal(adapter.snapshot().connection.internet, false)
  assert.equal(adapter.getReaderLoans('reader-1').length, before.filter((loan) => loan.readerId === 'reader-1').length + 1)
})

test('unknown response commits once, status lookup resolves it and retry is idempotent', async () => {
  const adapter = fresh()
  adapter.setNextOutcome('unknown')
  await assert.rejects(adapter.issue('reader-1', [copyItem()], 'uncertain'), errorCode('UNKNOWN'))
  const after = adapter.snapshot()
  const verified = await adapter.checkOperation('uncertain')
  assert.equal(verified?.type, 'issue')
  assert.equal(verified?.quantity, 1)
  assert.deepEqual(await adapter.issue('reader-1', [copyItem()], 'uncertain'), verified)
  assert.deepEqual(adapter.snapshot(), after)
  assert.equal(await adapter.checkOperation('never-sent'), null)
})

test('an operation ID cannot be reused for another reader or payload', async () => {
  const adapter = fresh()
  await adapter.issue('reader-1', [copyItem()], 'same-id')
  await assert.rejects(adapter.issue('reader-2', [copyItem()], 'same-id'), errorCode('OPERATION_CONFLICT'))
  await assert.rejects(adapter.accept('reader-1', [copyItem()], 'same-id'), errorCode('OPERATION_CONFLICT'))
})

test('concurrent identical submissions create a single loan', async () => {
  const adapter = createMockAdapter({ delay: 5 })
  const [first, second] = await Promise.all([
    adapter.issue('reader-1', [copyItem()], 'double-tap'),
    adapter.issue('reader-1', [copyItem()], 'double-tap'),
  ])
  assert.deepEqual(first, second)
  assert.equal(adapter.snapshot().loans.filter((loan) => loan.copyId === 'copy-2').length, 1)
})

test('simulated failure commits nothing and a retry remains possible', async () => {
  const adapter = fresh()
  const before = adapter.snapshot()
  adapter.setNextOutcome('error')
  await assert.rejects(adapter.issue('reader-1', [copyItem()], 'retryable'), errorCode('SIMULATED_ERROR'))
  assert.deepEqual(adapter.snapshot(), before)
  await adapter.issue('reader-1', [copyItem()], 'retryable')
  assert.equal((await adapter.checkOperation('retryable'))?.quantity, 1)
})

test('persisted loans and operation receipts survive adapter recreation', async () => {
  const storage = new MemoryStorage()
  const first = createMockAdapter({ storage, delay: 0 })
  first.setNextOutcome('unknown')
  await assert.rejects(first.issue('reader-1', [copyItem()], 'persistent'), errorCode('UNKNOWN'))
  const second = createMockAdapter({ storage, delay: 0 })
  assert.equal(second.snapshot().persistence, 'local-storage')
  assert.equal((await second.checkOperation('persistent'))?.quantity, 1)
  await second.issue('reader-1', [copyItem()], 'persistent')
  assert.equal(second.snapshot().loans.filter((loan) => loan.copyId === 'copy-2').length, 1)
})

test('corrupt storage falls back to a complete valid demonstration dataset', () => {
  const storage = new MemoryStorage()
  storage.setItem('edus-library-demo-v1', '{invalid json')
  assert.equal(createMockAdapter({ storage }).snapshot().readers.length, 4)
  storage.setItem('edus-library-demo-v1', JSON.stringify({ version: 1, data: { readers: [] } }))
  assert.equal(createMockAdapter({ storage }).snapshot().readers.length, 4)
})

test('storage denial degrades to memory while keeping successful operations visible', async () => {
  const storage = new MemoryStorage()
  storage.setItem = () => { throw new Error('Quota exceeded') }
  const adapter = createMockAdapter({ storage, delay: 0 })
  await adapter.issue('reader-1', [copyItem()], 'memory-fallback')
  assert.equal(adapter.snapshot().persistence, 'memory')
  assert.equal((await adapter.checkOperation('memory-fallback'))?.quantity, 1)
})

test('terminal-test mappings persist under their own namespace and ISBN maps to a title', async () => {
  const storage = new MemoryStorage()
  const first = createMockAdapter({ storage, delay: 0, storageKey: 'edus-library-terminal-test-v2', storageVersion: 2, requirePersistence: true })
  first.bindTestCard('reader-2', 'raw-card-42')
  first.bindTestCode('title', 'title-3', 'raw-isbn-42')
  assert.equal((await first.identifyCard('raw-card-42')).id, 'reader-2')
  assert.equal(first.resolveCode('raw-isbn-42').kind, 'title')
  const second = createMockAdapter({ storage, delay: 0, storageKey: 'edus-library-terminal-test-v2', storageVersion: 2, requirePersistence: true })
  assert.equal((await second.identifyCard('raw-card-42')).id, 'reader-2')
  assert.equal(second.resolveCode('raw-isbn-42').kind, 'title')
  assert.equal(storage.getItem('edus-library-demo-v1'), null)
})

test('terminal-test rejects mutations when local persistence cannot save them', async () => {
  const storage = new MemoryStorage()
  storage.setItem = () => { throw new Error('Quota exceeded') }
  const adapter = createMockAdapter({ storage, delay: 0, storageKey: 'edus-library-terminal-test-v2', storageVersion: 2, requirePersistence: true })
  await assert.rejects(adapter.issue('reader-1', [copyItem()], 'must-persist'), errorCode('PERSISTENCE_UNAVAILABLE'))
  assert.equal(adapter.snapshot().loans.some((loan) => loan.copyId === 'copy-2'), false)
})

test('callers cannot mutate internal adapter data through query results', () => {
  const adapter = fresh()
  const titles = adapter.searchTitles('Алгебра')
  titles.pop()
  const readers = adapter.searchReaders('')
  readers.splice(0, readers.length)
  assert.equal(adapter.searchTitles('Алгебра').length, 1)
  assert.equal(adapter.searchReaders('').length, 4)
})

test('pending operations snapshot inputs before waiting', async () => {
  const adapter = createMockAdapter({ delay: 5 })
  const items = [copyItem()]
  const pending = adapter.issue('reader-1', items, 'stable-input')
  items.push(copyItem('copy-3', 'title-2'))
  assert.equal((await pending).quantity, 1)
})

test('malformed operation IDs do not access prototype keys', async () => {
  const adapter = fresh()
  await assert.rejects(adapter.issue('reader-1', [copyItem()], '__proto__'), errorCode('INVALID_INPUT'))
  assert.equal(await adapter.checkOperation('__proto__'), null)
})

test('reactive Vue baskets can be issued and returned without cloning proxy objects', async () => {
  const adapter = fresh()
  const basket = ref<BasketItem[]>([copyItem()])
  const issued = await adapter.issue('reader-1', basket.value, 'reactive-issue')
  assert.equal(issued.quantity, 1)
  const returned = await adapter.accept('reader-1', basket.value, 'reactive-return')
  assert.equal(returned.quantity, 1)
  assert.equal(adapter.snapshot().loans.some((loan) => loan.copyId === 'copy-2'), false)
})

test('reactive Vue metadata and code arrays become plain registration DTOs', async () => {
  const adapter = fresh()
  const metadata = reactive({ ...newTitle })
  const codes = ref(['NEW-001'])
  const registered = await adapter.register(metadata, 'COPY', codes.value, 1, 'reactive-register')
  assert.equal(registered.quantity, 1)
  assert.equal(adapter.resolveCode('NEW-001').kind, 'copy')
})

test('accept rejects missing reader context and wrong owners in either accounting mode', async () => {
  const adapter=fresh()
  const before=adapter.snapshot()
  await assert.rejects(adapter.accept('',[copyItem('copy-1')],'reader-required'),errorCode('NOT_FOUND'))
  await assert.rejects(adapter.accept('reader-2',[copyItem('copy-1')],'wrong-copy-owner'),errorCode('WRONG_READER'))
  await assert.rejects(adapter.accept('reader-2',[legacyItem(1,'loan-2')],'wrong-legacy-owner'),errorCode('WRONG_READER'))
  assert.deepEqual(adapter.snapshot(),before)
})

test('a batch containing another reader’s copy is rejected atomically', async () => {
  const adapter=fresh()
  const before=adapter.snapshot()
  await assert.rejects(adapter.accept('reader-1',[copyItem('copy-1'),copyItem('copy-4','title-4')],'mixed-readers'),errorCode('WRONG_READER'))
  assert.deepEqual(adapter.snapshot(),before)
})

test('return idempotency includes reader context', async () => {
  const adapter=fresh()
  const result=await adapter.accept('reader-1',[copyItem('copy-1')],'same-return')
  assert.deepEqual(await adapter.accept('reader-1',[copyItem('copy-1')],'same-return'),result)
  await assert.rejects(adapter.accept('reader-2',[copyItem('copy-1')],'same-return'),errorCode('OPERATION_CONFLICT'))
})
