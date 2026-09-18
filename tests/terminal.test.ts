import test from 'node:test'
import assert from 'node:assert/strict'

class MemoryStorage implements Storage {
  private readonly values = new Map<string,string>()
  get length() { return this.values.size }
  clear() { this.values.clear() }
  getItem(key: string) { return this.values.get(key) ?? null }
  key(index: number) { return [...this.values.keys()][index] ?? null }
  removeItem(key: string) { this.values.delete(key) }
  setItem(key: string, value: string) { this.values.set(key, value) }
}

let fixtureId = 0
async function fixture(): Promise<typeof import('../src/composables/useTerminal.ts')> {
  Object.defineProperty(globalThis, 'localStorage', { configurable:true, value:new MemoryStorage() })
  return import(`../src/composables/useTerminal.ts?fixture=${++fixtureId}`)
}

function selectedReaderId(terminal: Awaited<ReturnType<typeof fixture>>) {
  return terminal.reader.value?.id
}

test('reader-specific return refuses another reader’s copy without changing the basket', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('KZ-0007')
  assert.equal(terminal.basket.value.length, 0)
  assert.match(terminal.message.value, /другому читателю.*Әлихан/)
  assert.equal(terminal.wrongReader.value?.readerId, 'reader-2')
  terminal.scan('000123')
  assert.equal(terminal.basket.value.length, 0, 'the warning stays open until an explicit action')
  terminal.clearFeedback()
  terminal.scan('000123')
  assert.equal(terminal.basket.value.length, 1)
  assert.equal(terminal.basket.value[0]?.loanId, 'loan-1')
})

test('changing a selected reader with books requires explicit basket clearing', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1002')[0]!)
  terminal.scan('KZ-0007')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  assert.equal(terminal.reader.value?.id, 'reader-2')
  assert.equal(terminal.dialog.value, 'change-reader')
  assert.equal(terminal.basket.value.length, 1)
  terminal.confirmChangeReader()
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.basket.value.length, 0)
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  assert.deepEqual(terminal.reader.value, terminal.adapter.searchReaders('EDUS-1001')[0])
  terminal.changeReader()
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.message.value, '', 'an ordinary reader change is not a missing-context error')
})

test('ISBN quantity return allocates across multiple loans and excludes selected quantities', async () => {
  const terminal = await fixture()
  await terminal.adapter.issue('reader-1', [{id:'extra',titleId:'title-3',quantity:3,mode:'LEGACY_TITLE'}], 'extra-loan')
  terminal.refresh();terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('9786010123456')
  assert.equal(terminal.legacyLimit.value, 5)
  terminal.legacyQuantity.value=4
  terminal.addLegacy()
  assert.equal(terminal.message.value, '')
  assert.equal(terminal.count.value, 4)
  assert.equal(terminal.basket.value.length, 2)
  assert.equal(terminal.legacyLimit.value, 1)
  terminal.scan('9786010123456')
  assert.equal(terminal.count.value, 4, 'another scan alone never adds quantity')
  terminal.addLegacy()
  assert.equal(terminal.count.value, 5)
  assert.equal(terminal.basket.value.length, 2, 'explicit addition merges the existing loan row')
  assert.equal(terminal.legacyLimit.value, 0)
  await terminal.adapter.accept('reader-1', terminal.basket.value, 'return-entire-edition')
  assert.equal(terminal.adapter.getReaderLoans('reader-1').some(loan=>loan.titleId==='title-3'), false)
})

test('select all fills the remainder of a partially selected legacy loan', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('9786010123456')
  terminal.legacyQuantity.value=1
  terminal.addLegacy()
  assert.equal(terminal.count.value, 1)
  terminal.allLoans()
  assert.equal(terminal.count.value, terminal.readerCount.value)
  assert.equal(terminal.basket.value.find(item=>item.loanId==='loan-2')?.quantity, 2)
})

test('return draft marks a single legacy copy without changing active loans, and can be removed', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  assert.equal(terminal.readerCount.value, 3)
  assert.equal(terminal.setReturnLoanQuantity('loan-2', 1), true)
  assert.equal(terminal.count.value, 1)
  assert.equal(terminal.basket.value.find(item => item.loanId === 'loan-2')?.quantity, 1)
  assert.equal(terminal.readerCount.value, 3, 'active loans do not change before confirmation')
  assert.equal(terminal.setReturnLoanQuantity('loan-2', 0), true)
  assert.equal(terminal.count.value, 0)
  assert.equal(terminal.readerCount.value, 3)
})

test('a repeated copy scan keeps the return selection at one copy', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000123')
  terminal.scan('000123')
  assert.equal(terminal.count.value, 1)
  assert.match(terminal.message.value, /уже отмечена к приёму/)
})

test('late card identification cannot navigate away from a newly started operation', async () => {
  const terminal = await fixture()
  terminal.start('issue')
  const pending=terminal.identify('EDUS-1001')
  terminal.start('accept')
  await pending
  assert.equal(terminal.operation.value, 'accept')
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.busy.value, false)
})

test('a session pause prevents late card results from selecting a reader', async () => {
  const terminal = await fixture()
  terminal.start('issue')
  const pending=terminal.identify('EDUS-1001')
  terminal.sessionExpired.value=true
  await pending
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.busy.value, false)
})

test('every return begins with identification and rejects codes until a reader is selected', async () => {
  const terminal = await fixture()
  terminal.start('issue')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000124')
  assert.equal(terminal.basket.value.length, 1)
  terminal.start('accept')
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.basket.value.length, 0)
  terminal.scan('000123')
  terminal.scan('9786010123456')
  terminal.addLoan('loan-1')
  terminal.titleChoice.value=terminal.adapter.getTitle('title-3')!
  terminal.addLegacy()
  terminal.confirm()
  await terminal.submit()
  assert.equal(terminal.screen.value, 'identify')
  assert.equal(terminal.reader.value, null)
  assert.equal(terminal.basket.value.length, 0)
  assert.equal(terminal.adapter.getReaderLoans('reader-1').length, 2)
})

test('direct scan and confirmation screen changes cannot bypass reader context', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.screen.value='scan'
  assert.equal(terminal.screen.value, 'identify')
  terminal.screen.value='confirm'
  assert.equal(terminal.screen.value, 'identify')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000123');terminal.confirm()
  assert.equal(terminal.screen.value, 'confirm')
  terminal.reader.value=null
  assert.equal(terminal.screen.value, 'identify')
  await terminal.submit()
  assert.equal(terminal.adapter.getReaderLoans('reader-1').some(loan=>loan.id==='loan-1'), true)
})

test('reader-first return commits once on double press and cannot replay success', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000123');terminal.confirm()
  const id=terminal.operationId.value
  await Promise.all([terminal.submit(),terminal.submit()])
  assert.equal(terminal.screen.value, 'success')
  assert.equal(terminal.result.value?.quantity, 1)
  assert.equal(terminal.result.value?.type, 'accept')
  await terminal.submit()
  assert.equal(terminal.operationId.value, id)
  assert.equal(terminal.message.value, '')
  assert.equal(terminal.adapter.getReaderLoans('reader-1').some(loan=>loan.id==='loan-1'), false)
})

test('unknown return retains reader and basket and blocks resubmission until checked', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000123');terminal.confirm();terminal.setOutcome('unknown')
  await terminal.submit()
  const id=terminal.operationId.value
  assert.equal(terminal.screen.value, 'unknown')
  terminal.start('accept');terminal.home();terminal.confirmChangeReader();terminal.confirm();await terminal.submit()
  assert.equal(terminal.screen.value, 'unknown')
  assert.equal(terminal.reader.value?.id, 'reader-1')
  assert.equal(terminal.basket.value.length, 1)
  assert.equal(terminal.operationId.value, id)
  await Promise.all([terminal.checkResult(),terminal.checkResult()])
  assert.equal(terminal.screen.value, 'success')
  assert.equal(terminal.result.value?.type, 'accept')
  assert.equal(terminal.result.value?.quantity, 1)
})

test('server outage retains return context and retry works without internet', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  terminal.chooseReader(terminal.adapter.searchReaders('EDUS-1001')[0]!)
  terminal.scan('000123');terminal.confirm()
  const id=terminal.operationId.value
  terminal.setConnection('server',false)
  await terminal.submit()
  assert.equal(terminal.screen.value,'confirm')
  assert.equal(terminal.basket.value.length,1)
  assert.equal(terminal.reader.value?.id,'reader-1')
  assert.equal(terminal.adapter.getReaderLoans('reader-1').some(loan=>loan.id==='loan-1'),true)
  terminal.setConnection('server',true);terminal.setConnection('internet',false)
  await terminal.submit()
  assert.equal(terminal.screen.value,'success')
  assert.equal(terminal.operationId.value,id)
})

test('NFC reading starts immediately and success requires the adapter response', async () => {
  const terminal = await fixture()
  terminal.start('accept')
  const pending=terminal.identify('EDUS-1001')
  assert.equal(terminal.nfcState.value,'reading')
  assert.equal(terminal.reader.value,null)
  await pending
  assert.equal(terminal.nfcState.value,'success')
  assert.equal(terminal.screen.value,'scan')
  assert.deepEqual(terminal.reader.value,terminal.adapter.searchReaders('EDUS-1001')[0])
  terminal.start('accept')
  await terminal.identify('UNKNOWN-CARD')
  assert.equal(terminal.nfcState.value,'error')
  assert.equal(terminal.reader.value,null)
  assert.equal(terminal.screen.value,'identify')
})

test('help retains an in-flight card result and selects that reader only after closing', async (context) => {
  const terminal=await fixture()
  const found=terminal.adapter.searchReaders('EDUS-1002')[0]!
  let resolveCard!: (reader: typeof found) => void
  const response=new Promise<typeof found>(resolve=>{resolveCard=resolve})
  const request=context.mock.method(terminal.adapter,'identifyCard',()=>response)
  terminal.start('accept')
  const pending=terminal.identify('EDUS-1002')
  terminal.setIdentificationTransitionPaused(true)
  resolveCard(found)
  await pending
  assert.equal(terminal.nfcState.value,'success')
  assert.equal(terminal.screen.value,'identify','the open instruction stays on its screen')
  assert.equal(terminal.reader.value,null)
  assert.equal(terminal.operation.value,'accept')
  assert.equal(terminal.basket.value.length,0)
  await terminal.identify('EDUS-1001')
  assert.equal(request.mock.callCount(),1,'new scans do not replace the retained result')
  terminal.setIdentificationTransitionPaused(false)
  assert.equal(terminal.screen.value,'scan')
  assert.equal(selectedReaderId(terminal),'reader-2','the actual adapter result is retained')
  assert.equal(terminal.operation.value,'accept')
})

test('help opened during the success hold pauses the eventual transition', async () => {
  const terminal=await fixture()
  terminal.start('issue')
  terminal.adapter.identifyCard=async()=>terminal.adapter.searchReaders('EDUS-1001')[0]!
  const pending=terminal.identify('EDUS-1001')
  await Promise.resolve()
  assert.equal(terminal.nfcState.value,'success')
  terminal.setIdentificationTransitionPaused(true)
  await pending
  assert.equal(terminal.screen.value,'identify')
  terminal.setIdentificationTransitionPaused(false)
  assert.equal(terminal.screen.value,'scan')
  assert.equal(selectedReaderId(terminal),'reader-1')
})

test('cancelling a request does not apply its result or unlock a newer identification', async (context) => {
  const terminal=await fixture()
  const firstReader=terminal.adapter.searchReaders('EDUS-1001')[0]!
  const nextReader=terminal.adapter.searchReaders('EDUS-1002')[0]!
  let resolveFirst!: (reader: typeof firstReader) => void
  let resolveNext!: (reader: typeof nextReader) => void
  const firstResponse=new Promise<typeof firstReader>(resolve=>{resolveFirst=resolve})
  const nextResponse=new Promise<typeof nextReader>(resolve=>{resolveNext=resolve})
  context.mock.method(terminal.adapter,'identifyCard',(card: string)=>card==='EDUS-1001'?firstResponse:nextResponse)
  terminal.start('issue')
  const firstPending=terminal.identify('EDUS-1001')
  terminal.home()
  assert.equal(terminal.screen.value,'home')
  assert.equal(terminal.busy.value,false)
  terminal.start('accept')
  const nextPending=terminal.identify('EDUS-1002')
  resolveFirst(firstReader)
  await firstPending
  assert.equal(terminal.busy.value,true,'the old request cannot clear the new request lock')
  assert.equal(terminal.nfcState.value,'reading')
  assert.equal(terminal.reader.value,null)
  resolveNext(nextReader)
  await nextPending
  assert.equal(selectedReaderId(terminal),'reader-2')
  assert.equal(terminal.operation.value,'accept')
})

test('cancelling during the success hold clears its timer and cannot select the old reader', async () => {
  const terminal=await fixture()
  terminal.start('issue')
  terminal.adapter.identifyCard=async()=>terminal.adapter.searchReaders('EDUS-1001')[0]!
  const pending=terminal.identify('EDUS-1001')
  await Promise.resolve()
  assert.equal(terminal.nfcState.value,'success')
  terminal.home()
  await pending
  terminal.start('accept')
  assert.equal(terminal.screen.value,'identify')
  assert.equal(terminal.reader.value,null)
  assert.equal(terminal.nfcState.value,'idle')
  assert.equal(terminal.busy.value,false)
})

test('identification view cleanup resets the drawing and ignores a late hidden-route response', async (context) => {
  const terminal=await fixture()
  const found=terminal.adapter.searchReaders('EDUS-1001')[0]!
  let resolveCard!: (reader: typeof found) => void
  const response=new Promise<typeof found>(resolve=>{resolveCard=resolve})
  context.mock.method(terminal.adapter,'identifyCard',()=>response)
  terminal.start('issue')
  const pending=terminal.identify('EDUS-1001')
  assert.equal(terminal.nfcState.value,'reading')
  terminal.cancelIdentification()
  assert.equal(terminal.screen.value,'identify','route cleanup does not manufacture workflow navigation')
  assert.equal(terminal.nfcState.value,'idle')
  assert.equal(terminal.busy.value,false)
  resolveCard(found)
  await pending
  assert.equal(terminal.screen.value,'identify')
  assert.equal(terminal.reader.value,null)
  assert.equal(terminal.nfcState.value,'idle')
})

test('leaving with a completed result held by help discards it before help closes', async () => {
  const terminal=await fixture()
  terminal.start('issue')
  terminal.adapter.identifyCard=async()=>terminal.adapter.searchReaders('EDUS-1001')[0]!
  const pending=terminal.identify('EDUS-1001')
  terminal.setIdentificationTransitionPaused(true)
  await pending
  assert.equal(terminal.nfcState.value,'success')
  terminal.start('accept')
  terminal.setIdentificationTransitionPaused(false)
  assert.equal(terminal.operation.value,'accept')
  assert.equal(terminal.screen.value,'identify')
  assert.equal(terminal.reader.value,null)
  assert.equal(terminal.nfcState.value,'idle')
})

test('an error completing behind help stays an error and can be retried after close', async () => {
  const terminal=await fixture()
  terminal.start('accept')
  const pending=terminal.identify('UNKNOWN-CARD')
  terminal.setIdentificationTransitionPaused(true)
  await pending
  assert.equal(terminal.nfcState.value,'error')
  assert.notEqual(terminal.message.value,'')
  terminal.setIdentificationTransitionPaused(false)
  assert.equal(terminal.screen.value,'identify')
  assert.equal(terminal.reader.value,null)
  await terminal.identify('EDUS-1001')
  assert.equal(selectedReaderId(terminal),'reader-1')
})
