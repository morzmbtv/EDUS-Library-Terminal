import { computed, ref, watch } from 'vue'
import { DomainError, type BasketItem, type Reader, type Title, type OperationResult } from '../domain/terminalTypes.ts'
import { createUnavailableAdapter } from '../infrastructure/unavailableAdapter.ts'
import { apiConfigured, runtimeConfig } from '../infrastructure/runtimeConfig.ts'

export type Operation = 'issue' | 'accept' | 'register'
export type Screen = 'home' | 'identify' | 'search' | 'scan' | 'confirm' | 'success' | 'unknown' | 'registration'
const config = runtimeConfig()
declare const __EDUS_DEMO_RUNTIME__: boolean
declare const __EDUS_TERMINAL_TEST_RUNTIME__: boolean
export const terminalTestRuntime = __EDUS_TERMINAL_TEST_RUNTIME__
/** Development and the separately packaged terminal test target use local data. Production is blocked until a confirmed LAN API is configured. */
export const adapter = __EDUS_DEMO_RUNTIME__
  ? (await import('../domain/index.ts')).createMockAdapter({ storage: localStorage, delay: 500 })
  : __EDUS_TERMINAL_TEST_RUNTIME__
    ? (await import('../domain/index.ts')).createMockAdapter({ storage: localStorage, delay: 500, storageKey: 'edus-library-terminal-test-v2', storageVersion: 2, requirePersistence: true })
  : createUnavailableAdapter(apiConfigured(config)
    ? 'Библиотечный API указан, но его контракт ещё не подтверждён. Операции заблокированы до подключения согласованного адаптера.'
    : 'Библиотечный API не настроен. Укажите адрес API, школу и устройство в runtime-config.js.')
export const screen = ref<Screen>('home')
export const operation = ref<Operation>('issue')
export const reader = ref<Reader | null>(null)
export const basket = ref<BasketItem[]>([])
export const snapshot = ref(adapter.snapshot())
export const busy = ref(false)
export const message = ref('')
export const info = ref('')
export const sessionExpired = ref(false)
export const nfcState = ref<'idle'|'reading'|'success'|'error'>('idle')
export const wrongReader = ref<{readerId:string;name:string;group:string;title:string;code:string}|null>(null)
export const query = ref('')
export const searching = ref(false)
export const searchResults = ref<Reader[]>([])
export const searched = ref(false)
export const scanValue = ref('')
export const titleChoice = ref<Title | null>(null)
export const ambiguousTitles = ref<Title[]>([])
export const legacyQuantity = ref(1)
export const dialog = ref<''|'manual'|'title'|'change-reader'|'leave'|'clear-basket'|'loans'|'return-item'|'return-search'>('')
export const registrationTitle = ref<Title | null>(null)
export const result = ref<OperationResult | null>(null)
export const operationId = ref('')
export const server = computed(() => snapshot.value.connection.server)
export const internet = computed(() => snapshot.value.connection.internet)
export const count = computed(() => basket.value.reduce((n,x)=>n+x.quantity,0))
export const loans = computed(() => reader.value ? snapshot.value.loans.filter(x=>x.readerId===reader.value!.id) : [])
export const readerCount = computed(() => loans.value.reduce((n,x)=>n+x.quantity,0))
export const operationName = computed(() => ({issue:'Выдача книг',accept:'Приём книг',register:'Добавление книг'})[operation.value])
export function titleFor(id: string) { return snapshot.value.titles.find(x=>x.id===id) }
export function codeFor(item: BasketItem) { return item.copyId ? snapshot.value.copies.find(x=>x.id===item.copyId)?.code : undefined }
export function refresh() { snapshot.value = adapter.snapshot() }
export function clearFeedback() { message.value=''; info.value=''; wrongReader.value=null }
let workflowVersion = 0
let pendingSubmission = false
let basketReaderId: string | null = null
let identificationTransitionPaused = false
type IdentificationAttempt = {
  version: number
  operation: Operation
  reader: Reader | null
  ready: boolean
  timer?: ReturnType<typeof setTimeout>
  finishDelay?: () => void
}
let activeIdentification: IdentificationAttempt | null = null
function identificationIsCurrent(attempt: IdentificationAttempt) {
  return activeIdentification===attempt && attempt.version===workflowVersion && attempt.operation===operation.value && screen.value==='identify' && !sessionExpired.value
}
export function cancelIdentification() {
  if(!activeIdentification)return
  const attempt=activeIdentification
  activeIdentification=null
  clearTimeout(attempt.timer)
  attempt.finishDelay?.()
  busy.value=false;nfcState.value='idle'
}
function invalidateWorkflow() { workflowVersion++;cancelIdentification() }
function finishIdentification() {
  const attempt=activeIdentification
  if(!attempt?.reader||!attempt.ready||identificationTransitionPaused||!identificationIsCurrent(attempt))return
  activeIdentification=null;busy.value=false
  chooseReader(attempt.reader)
}
export function setIdentificationTransitionPaused(paused: boolean) {
  identificationTransitionPaused=paused
  if(!paused)finishIdentification()
}
watch([screen,operation,sessionExpired],()=>{
  if(activeIdentification&&!identificationIsCurrent(activeIdentification))cancelIdentification()
},{flush:'sync'})
watch(basket, items=>{
  if(!items.length)basketReaderId=null
  else if(!basketReaderId)basketReaderId=reader.value?.id??null
},{deep:true,flush:'sync'})
watch([screen,reader,operation],()=>{
  const known=reader.value&&snapshot.value.readers.some(candidate=>candidate.id===reader.value?.id)
  if(operation.value!=='register'&&['scan','confirm'].includes(screen.value)&&!known){
    screen.value='identify';dialog.value='';nfcState.value='idle'
    message.value='Сначала выберите читателя: приложите карту или воспользуйтесь поиском.'
  }
},{flush:'sync'})
function navigationLocked(){return screen.value==='unknown'||pendingSubmission||(busy.value&&screen.value!=='identify')}
function hasReaderContext(): boolean {
  if(operation.value==='register')return false
  if(!reader.value||!snapshot.value.readers.some(candidate=>candidate.id===reader.value?.id)){
    reader.value=null;screen.value='identify';dialog.value=''
    message.value='Сначала выберите читателя: приложите карту или воспользуйтесь поиском.';return false
  }
  if(basketReaderId&&basketReaderId!==reader.value.id){
    message.value='Список относится к другому читателю. Подтвердите очистку списка перед сменой читателя.'
    dialog.value='change-reader';return false
  }
  return true
}
function canEditBasket(): boolean {
  if(busy.value||sessionExpired.value||wrongReader.value||screen.value==='unknown'||screen.value==='success')return false
  if(!hasReaderContext())return false
  return screen.value==='scan'
}
export function start(op: Operation) {
  if(navigationLocked())return
  invalidateWorkflow()
  basket.value=[];operation.value=op;reader.value=null;result.value=null;operationId.value='';clearFeedback();scanValue.value='';query.value='';titleChoice.value=null;ambiguousTitles.value=[];dialog.value='';sessionExpired.value=false;nfcState.value='idle'
  screen.value=op==='register'?'registration':'identify'
}
export function home() { if(navigationLocked())return;invalidateWorkflow();screen.value='home';reader.value=null;basket.value=[];clearFeedback();dialog.value='' }
export function requestHome() { if(navigationLocked())return;if(basket.value.length&&screen.value!=='success')dialog.value='leave';else home() }
export function changeReader() { if(navigationLocked()||sessionExpired.value)return;clearFeedback();if(basket.value.length)dialog.value='change-reader';else{invalidateWorkflow();screen.value='identify';reader.value=null;nfcState.value='idle'} }
export function confirmChangeReader() { if(navigationLocked()||sessionExpired.value)return;invalidateWorkflow();basket.value=[];reader.value=null;clearFeedback();dialog.value='';screen.value='identify';nfcState.value='idle' }
export function back() {
  if(navigationLocked())return
  clearFeedback()
  if (screen.value==='search') screen.value='identify'
  else if (screen.value==='confirm') screen.value='scan'
  else requestHome()
}
export function openSearch() { if(navigationLocked()||busy.value||sessionExpired.value)return;screen.value='search';query.value='';searchResults.value=[];searched.value=false;clearFeedback() }
let searchTimer: ReturnType<typeof setTimeout> | undefined
watch(query, (q) => {
  clearTimeout(searchTimer); searching.value=!!q.trim(); searched.value=false
  searchTimer=setTimeout(()=>{ searchResults.value=q.trim()?adapter.searchReaders(q):[]; searching.value=false; searched.value=!!q.trim() },250)
})
export function chooseReader(r: Reader) {
  if(sessionExpired.value||navigationLocked()||!['identify','search','scan'].includes(screen.value))return
  const known=snapshot.value.readers.find(candidate=>candidate.id===r.id)
  if(!known){message.value='Читатель не найден. Повторите поиск.';return}
  if(basket.value.length&&(reader.value?.id??basketReaderId)!==known.id){
    message.value='Текущий список будет очищен только после подтверждения смены читателя.';dialog.value='change-reader';return
  }
  if (operation.value==='accept' && basket.value.some(item=>{
    const loan=snapshot.value.loans.find(candidate=>candidate.id===item.loanId)
    return loan && loan.readerId!==r.id
  })) {message.value='В списке есть книги другого читателя. Завершите их приём или уберите их из списка перед выбором читателя.';return}
  reader.value=known;screen.value='scan';clearFeedback()
}
export async function identify(card: string) {
  if(busy.value || activeIdentification || identificationTransitionPaused || sessionExpired.value || screen.value!=='identify') return
  const attempt: IdentificationAttempt={version:workflowVersion,operation:operation.value,reader:null,ready:false}
  activeIdentification=attemp
  busy.value=true; nfcState.value='reading';clearFeedback()
  try {
    const r=await adapter.identifyCard(card)
    if(!identificationIsCurrent(attempt))return
    attempt.reader=r;nfcState.value='success'
    await new Promise<void>(resolve=>{
      attempt.finishDelay=resolve
      attempt.timer=setTimeout(()=>{attempt.timer=undefined;attempt.finishDelay=undefined;resolve()},400)
    })
    if(identificationIsCurrent(attempt)){attempt.ready=true;finishIdentification()}
  }
  catch(e) { if(identificationIsCurrent(attempt)){nfcState.value='error';message.value=errorText(e)} }
  finally {
    if(activeIdentification===attempt){
      busy.value=false
      if(!attempt.reader)activeIdentification=null
    }
  }
}
export function errorText(e: unknown) { return e instanceof Error?e.message:'Операция не выполнена. Повторите попытку.' }
export function addItem(item: BasketItem) {
  if(!canEditBasket())return false
  if(operation.value==='accept'){
    const loan=snapshot.value.loans.find(candidate=>candidate.id===item.loanId)
    if(!loan||loan.readerId!==reader.value!.id){message.value='Эта выдача не относится к выбранному читателю.';return false}
  }
  if(basket.value.some(x=>x.id===item.id || (item.copyId && x.copyId===item.copyId))) {message.value=operation.value==='accept'?'Эта книга уже отмечена к приёму.':'Эта книга уже в списке. Повторный скан не добавляет ещё один экземпляр.';return false}
  basket.value.push(item); info.value='Книга добавлена в список';return true
}
export function scan(code: string) {
  if(wrongReader.value||!canEditBasket())return
  clearFeedback(); if(!server.value){message.value='Нет связи с локальным сервером. Восстановите соединение — список сохранён.';return}
  if (!code.trim()) {message.value='Введите код книги.';return}
  const found=adapter.resolveCode(code.trim());scanValue.value=''
  if(found.kind==='not-found') {message.value='Код не найден. Проверьте номер или добавьте книгу в фонд.';return}
  if(found.kind==='ambiguous') {ambiguousTitles.value=[...found.titles];dialog.value='title';titleChoice.value=null;return}
  if(found.kind==='title') { chooseTitle(found.title);return }
  if(operation.value==='issue') {
    if(found.loan){message.value='Эта книга уже выдана. Сначала оформите её приём.';return}
    addItem({id:found.copy.id,titleId:found.title.id,copyId:found.copy.id,quantity:1,mode:'COPY'})
  } else {
    if(!found.loan){message.value='У этой книги нет активной выдачи. Возможно, её уже приняли.';return}
    if(found.loan.readerId!==reader.value!.id){
      const owner=snapshot.value.readers.find(candidate=>candidate.id===found.loan!.readerId)
      wrongReader.value={readerId:found.loan.readerId,name:owner?.name??'Другой читатель',group:owner?.group??'',title:found.title.name,code:found.copy.code}
      message.value=`Эта книга выдана другому читателю${owner?`: ${owner.name}`:''}. Выберите его для приёма этой книги.`;return
    }
    addItem({id:found.loan.id,titleId:found.title.id,copyId:found.copy.id,quantity:1,mode:'COPY',loanId:found.loan.id})
  }
  dialog.value=''
}
export function chooseTitle(t: Title) {
  if(!canEditBasket())return
  ambiguousTitles.value=[]
  titleChoice.value=t;legacyQuantity.value=1;dialog.value='title'
}
export const legacyLimit=computed(()=>{
  if(!titleChoice.value)return 0
  if(operation.value==='accept')return loans.value.filter(l=>l.titleId===titleChoice.value!.id&&l.mode==='LEGACY_TITLE').reduce((n,l)=>{
    const selected=basket.value.find(item=>item.loanId===l.id)?.quantity??0
    return n+Math.max(0,l.quantity-selected)
  },0)
  const lent=snapshot.value.loans.filter(l=>l.titleId===titleChoice.value!.id&&l.mode==='LEGACY_TITLE').reduce((n,l)=>n+l.quantity,0)
  return Math.max(0,(snapshot.value.legacyStock[titleChoice.value.id]??0)-lent)
})
export function addLegacy() {
  if(!canEditBasket()||!titleChoice.value)return
  const t=titleChoice.value
  if(operation.value==='accept'){
    const matching=loans.value.filter(x=>x.titleId===t.id&&x.mode==='LEGACY_TITLE')
    if(!matching.length){message.value='У выбранного читателя нет выдачи этого издания из старого фонда.';return}
    if(!Number.isSafeInteger(legacyQuantity.value)||legacyQuantity.value<1||legacyQuantity.value>legacyLimit.value){message.value='Укажите количество в пределах ещё не выбранных книг на руках.';return}
    let remaining=legacyQuantity.value
    const updated=basket.value.map(item=>({...item}))
    for(const loan of matching){
      const selected=updated.find(item=>item.loanId===loan.id)
      const quantity=Math.min(remaining,Math.max(0,loan.quantity-(selected?.quantity??0)))
      if(!quantity)continue
      if(selected)updated[updated.indexOf(selected)]={...selected,quantity:selected.quantity+quantity}
      else updated.push({id:loan.id,titleId:t.id,quantity,mode:'LEGACY_TITLE',loanId:loan.id})
      remaining-=quantity
      if(!remaining)break
    }
    basket.value=updated;message.value='';info.value='Книги добавлены к приёму';dialog.value=''
  } else {
    if(legacyQuantity.value>legacyLimit.value){message.value='Недостаточно книг в старом фонде.';return}
    if(addItem({id:`legacy-${t.id}`,titleId:t.id,quantity:legacyQuantity.value,mode:'LEGACY_TITLE'}))dialog.value=''
  }
}
export function addLoan(id:string) {
  if(operation.value!=='accept'||!canEditBasket())return
  const l=loans.value.find(x=>x.id===id);if(!l)return
  const selected=basket.value.find(item=>item.loanId===l.id)
  if(selected){
    if(selected.quantity<l.quantity){basket.value=basket.value.map(item=>item.loanId===l.id?{...item,quantity:l.quantity}:item);clearFeedback();info.value='Все книги этой выдачи добавлены к приёму'}
    return
  }
  clearFeedback();addItem({id:l.id,titleId:l.titleId,copyId:l.copyId,quantity:l.quantity,mode:l.mode,loanId:l.id})
}
export function allLoans() { if(operation.value!=='accept'||!canEditBasket())return;for(const l of loans.value)addLoan(l.id) }
export function removeItem(id:string) {if(!canEditBasket())return;basket.value=basket.value.filter(x=>x.id!==id);clearFeedback()}
/**
 * Updates only the current return draft.  Active loans stay untouched until the
 * shared confirmation transaction succeeds, so the on-hand counter remains
 * truthful while an operator reviews the return.
 */
export function setReturnLoanQuantity(loanId:string, quantity:number) {
  if(operation.value!=='accept'||!canEditBasket())return false
  const loan=loans.value.find(candidate=>candidate.id===loanId)
  if(!loan){message.value='Эта выдача больше не доступна. Обновите список книг.';return false}
  if(!Number.isSafeInteger(quantity)||quantity<0||quantity>loan.quantity){message.value='Укажите количество в пределах книг на руках.';return false}
  if(loan.mode==='COPY'&&quantity>1){message.value='Для экземпляра с номером можно отметить только одну книгу.';return false}
  const existing=basket.value.find(item=>item.loanId===loanId)
  if(quantity===0){
    if(existing)basket.value=basket.value.filter(item=>item.loanId!==loanId)
    clearFeedback();return true
  }
  const item: BasketItem={id:loan.id,titleId:loan.titleId,copyId:loan.copyId,quantity,mode:loan.mode,loanId:loan.id}
  basket.value=existing?basket.value.map(candidate=>candidate.loanId===loanId?item:candidate):[...basket.value,item]
  clearFeedback();info.value='Книга отмечена к приёму';return true
}
export function clearBasket() {
  if(!canEditBasket()||!basket.value.length)return false
  basket.value=[]
  clearFeedback()
  return true
}
export function confirm() {if(canEditBasket()&&count.value){screen.value='confirm';clearFeedback();operationId.value=crypto.randomUUID()}}
export async function submit() {
  if(busy.value||sessionExpired.value||screen.value==='unknown'||screen.value==='success')return
  if(!hasReaderContext()||screen.value!=='confirm'||!count.value||!operationId.value)return
  busy.value=true;pendingSubmission=true;clearFeedback()
  try{
    if(operation.value==='issue'&&reader.value)result.value=await adapter.issue(reader.value.id,basket.value,operationId.value)
    else if(operation.value==='accept'&&reader.value)result.value=await adapter.accept(reader.value.id,basket.value,operationId.value)
    else throw new Error('Выберите читателя.')
    refresh();screen.value='success'
  } catch(e) { if(e instanceof DomainError && e.code==='UNKNOWN')screen.value='unknown';else message.value=errorText(e) }
  finally{busy.value=false;pendingSubmission=false}
}
export async function checkResult(){
  if(busy.value||sessionExpired.value||screen.value!=='unknown'||!operationId.value)return
  busy.value=true;clearFeedback()
  try{const r=await adapter.checkOperation(operationId.value);if(r){result.value=r;operation.value=r.type;refresh();screen.value='success'}else message.value='Результат пока не найден. Повторите проверку после восстановления связи.'}
  catch(e){message.value=errorText(e)}finally{busy.value=false}
}
export function setConnection(key:'server'|'internet',value:boolean){adapter.setConnection({[key]:value});refresh()}
export function setOutcome(value:'success'|'unknown'|'error'){adapter.setNextOutcome(value)}

type TerminalTestControls = {
  bindTestCard(readerId: string, rawCode: string): void
  bindTestCode(kind: 'copy' | 'title', id: string, rawCode: string): void
  resetTestData(): void
  setNextDelay(delayMs: number): void
}
function testControls(): TerminalTestControls | null {
  if (!terminalTestRuntime) return null
  const candidate = adapter as Partial<TerminalTestControls>
  return typeof candidate.bindTestCard === 'function' && typeof candidate.bindTestCode === 'function'
    && typeof candidate.resetTestData === 'function' && typeof candidate.setNextDelay === 'function'
    ? candidate as TerminalTestControls : null
}
function terminalTestAction(action: (controls: TerminalTestControls) => void): boolean {
  const controls = testControls()
  if (!controls) return false
  try { action(controls); refresh(); clearFeedback(); return true }
  catch (error) { message.value = errorText(error); return false }
}
/** These controls only exist in the explicit terminal-test bundle. */
export function bindTerminalTestCard(readerId: string, rawCode: string) { return terminalTestAction((controls) => controls.bindTestCard(readerId, rawCode)) }
export function bindTerminalTestCode(kind: 'copy' | 'title', id: string, rawCode: string) { return terminalTestAction((controls) => controls.bindTestCode(kind, id, rawCode)) }
export function resetTerminalTestData() { return terminalTestAction((controls) => controls.resetTestData()) }
export function setTerminalTestDelay(delayMs: number) { return terminalTestAction((controls) => controls.setNextDelay(delayMs)) }

\n