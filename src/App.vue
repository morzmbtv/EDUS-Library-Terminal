<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { ArrowLeft, ArrowRight, BookOpen, Check, CircleAlert, Clock3, Home, Keyboard, LibraryBig, Monitor, ShieldCheck, SlidersHorizontal, X } from '@lucide/vue'
import TerminalShell from './components/TerminalShell.vue'
import SystemHeader from './components/SystemHeader.vue'
import HomeAction from './components/HomeAction.vue'
import OperationStepper from './components/OperationStepper.vue'
import ReaderSummary from './components/ReaderSummary.vue'
import ReaderSearch from './components/ReaderSearch.vue'
import ReaderIdentification from './components/ReaderIdentification.vue'
import ScannerPanel from './components/ScannerPanel.vue'
import BookRow from './components/BookRow.vue'
import BookBasket from './components/BookBasket.vue'
import ActionBar from './components/ActionBar.vue'
import TouchInput from './components/TouchInput.vue'
import TouchKeyboard from './components/TouchKeyboard.vue'
import QuantityControl from './components/QuantityControl.vue'
import ContextHelp from './components/ContextHelp.vue'
import ReturnLoanList, { type ReturnLoanListItem } from './components/ReturnLoanList.vue'
import RegistrationView from './views/RegistrationView.vue'
import { screen,operation,reader,basket,busy,message,info,sessionExpired,nfcState,query,searching,searchResults,searched,scanValue,titleChoice,ambiguousTitles,legacyQuantity,dialog,result,operationId,server,internet,count,loans,readerCount,operationName,titleFor,codeFor,start,home,requestHome,changeReader,confirmChangeReader,back,openSearch,chooseReader,identify,scan,chooseTitle,legacyLimit,addLegacy,removeItem,clearBasket,confirm,submit,checkResult,setConnection,setOutcome,wrongReader,setIdentificationTransitionPaused,cancelIdentification,setReturnLoanQuantity,refresh,bindTerminalTestCard,bindTerminalTestCode,resetTerminalTestData,setTerminalTestDelay,snapshot } from './composables/useTerminal'
const StudentDisplay=defineAsyncComponent(()=>import('./views/StudentDisplay.vue'))
declare const __EDUS_DEMO_RUNTIME__: boolean
declare const __EDUS_TERMINAL_TEST_RUNTIME__: boolean
const demoRuntime=__EDUS_DEMO_RUNTIME__
const terminalTestRuntime=__EDUS_TERMINAL_TEST_RUNTIME__
/** Demo-only chunks are absent from the production dependency graph. */
const ComponentGallery=__EDUS_DEMO_RUNTIME__?defineAsyncComponent(()=>import('./views/ComponentGallery.vue')):undefined
const DemoScenarioPanel=__EDUS_DEMO_RUNTIME__?defineAsyncComponent(()=>import('./components/DemoScenarioPanel.vue')):undefined
const TerminalTestPanel=__EDUS_TERMINAL_TEST_RUNTIME__?defineAsyncComponent(()=>import('./components/TerminalTestPanel.vue')):undefined
const route=ref(location.hash)
const clock=ref(new Date())
const time=computed(()=>clock.value.toLocaleTimeString('ru-RU',{hour:'2-digit',minute:'2-digit'}))
const date=computed(()=>clock.value.toLocaleDateString('ru-RU',{day:'numeric',month:'long',year:'numeric'}))
const settings=ref(false),help=ref(false),demoPanel=ref(false),terminalTestPanel=ref(false)
const terminalCaptureKind=ref<''|'card'|'book'>('')
const terminalCapturedValue=ref(''),terminalCapturedSuffix=ref('')
watch(help, setIdentificationTransitionPaused, { flush: 'sync', immediate: true })
const overlayOpen=computed(()=>settings.value||help.value||!!dialog.value||!!wrongReader.value||sessionExpired.value)
const keyboard=ref<''|'search'|'scan'>('')
const keyboardValue=computed({get:()=>keyboard.value==='search'?query.value:scanValue.value,set:(v:string)=>{if(keyboard.value==='search')query.value=v;else scanValue.value=v}})
const registrationRef=ref<InstanceType<typeof RegistrationView>|null>(null)
const scanInput=ref<InstanceType<typeof TouchInput>|null>(null)
const manualKind=ref<'inventory'|'isbn'>('inventory')
const workflow=computed(()=>!['home','success'].includes(screen.value))
const issueScan=computed(()=>screen.value==='scan'&&operation.value==='issue'&&!!reader.value)
const acceptScan=computed(()=>screen.value==='scan'&&operation.value==='accept'&&!!reader.value)
const returnLoans=computed<ReturnLoanListItem[]>(()=>loans.value.map(loan=>({
  id:loan.id,title:titleFor(loan.titleId)?.name??'Неизвестное издание',author:titleFor(loan.titleId)?.author,
  code:loan.copyId?snapshotCopyCode(loan.copyId):undefined,mode:loan.mode,quantity:loan.quantity,
  selectedQuantity:basket.value.find(item=>item.loanId===loan.id)?.quantity??0,
})))
const returnLoansState=computed<'loading'|'ready'|'error'>(()=>server.value?'ready':'error')
const selectedReturnLoan=ref<ReturnLoanListItem|null>(null)
const returnQuantity=ref(1)
const currentStep=computed(()=>['identify','search'].includes(screen.value)?0:screen.value==='confirm'||screen.value==='unknown'?2:1)
const heading=computed(()=>screen.value==='identify'?(operation.value==='issue'?'Кому выдаём книги?':'Кто возвращает книги?'):screen.value==='search'?'Найдите читателя':screen.value==='confirm'?'Всё верно?':screen.value==='unknown'?'Проверяем результат операции':operation.value==='issue'?'Сканируйте книги':'Какие книги принимаем?')
const titleHelp=computed(()=>operation.value==='issue'?'Добавьте книги, которые хотите выдать читателю.':'Сканируйте книги выбранного читателя или выберите их из списка.')
function globalBack(){if(screen.value==='registration')registrationRef.value?.back();else back()}
function globalHome(){if(screen.value==='registration')registrationRef.value?.requestLeave();else requestHome()}
function openManual(){manualKind.value='inventory';scanValue.value='';dialog.value='manual';nextTick(()=>scanInput.value?.focus())}
function openIsbn(){manualKind.value='isbn';scanValue.value='';dialog.value='manual';nextTick(()=>scanInput.value?.focus())}
function manualSubmit(){keyboard.value='';scan(scanValue.value);if(wrongReader.value)dialog.value=''}
function openLoans(){dialog.value='loans'}
function requestClearBasket(){if(basket.value.length&&!busy.value)dialog.value='clear-basket'}
function openReturnItem(item:ReturnLoanListItem){selectedReturnLoan.value=item;returnQuantity.value=item.selectedQuantity||1;dialog.value='return-item'}
function openReturnSearch(){dialog.value='return-search'}
function chooseReturnLoan(item:ReturnLoanListItem){openReturnItem(item)}
function snapshotCopyCode(copyId:string){return codeFor({id:copyId,titleId:'',copyId,quantity:1,mode:'COPY'})}
function resetReturnMarks(){clearBasket();selectedReturnLoan.value=null;closeDialog()}
function retryReturnLoans(){refresh()}
function demoCard(card:string){settings.value=false;identify(card)}
function demoScan(code:string){settings.value=false;if(screen.value==='registration')registrationRef.value?.scan(code);else if(screen.value==='scan')scan(code)}
function beginTerminalCapture(kind:'card'|'book'){
  if(!terminalTestRuntime)return
  keyboard.value='';scanBuffer='';overlayScanBuffer='';terminalCapturedValue.value='';terminalCapturedSuffix.value='';terminalCaptureBuffer='';terminalCaptureLastKey=0
  terminalCaptureKind.value=kind
  nextTick(()=>document.querySelector<HTMLElement>('.terminal-test-panel')?.focus({preventScroll:true}))
}
function cancelTerminalCapture(){terminalCaptureKind.value='';terminalCaptureBuffer='';terminalCaptureLastKey=0}
function bindTerminalCard(readerId:string,rawCode:string){if(bindTerminalTestCard(readerId,rawCode))terminalCapturedValue.value=''}
function bindTerminalCode(kind:'copy'|'title',id:string,rawCode:string){if(bindTerminalTestCode(kind,id,rawCode))terminalCapturedValue.value=''}
function resetTerminalData(){if(resetTerminalTestData()){cancelTerminalCapture();home();info.value='Тестовые данные сброшены локально.'}}
function setTestDelay(delayMs:number){if(setTerminalTestDelay(delayMs))info.value='Для следующей операции включена тестовая задержка.'}
function closeDialog(){dialog.value='';keyboard.value='';message.value=''}
let helpReturnFocus:HTMLElement|null=null
function openHelp(){
  helpReturnFocus=document.querySelector<HTMLElement>(screen.value==='search'?'.reader-search input':dialog.value==='manual'?'.modal input':':focus')
  scanBuffer='';overlayScanBuffer='';help.value=true
}
async function closeHelp(){
  help.value=false;scanBuffer='';overlayScanBuffer='';await nextTick()
  if(helpReturnFocus?.isConnected)helpReturnFocus.focus({preventScroll:true})
  else document.querySelector<HTMLElement>('[aria-label="Помощь"]')?.focus({preventScroll:true})
}
function dismissWrongReader(){wrongReader.value=null;message.value=''}
function switchFromWrongReader(){dismissWrongReader();dialog.value='';changeReader()}
function toggleConnection(key:'server'|'internet'){setConnection(key,key==='server'?!server.value:!internet.value)}
function expire(){settings.value=false;sessionExpired.value=true}
const channel= typeof BroadcastChannel==='undefined'?null:new BroadcastChannel('edus-library-display')
let displayExpiry=0
function broadcast(){
  if(route.value==='#/display'||route.value==='#/components')return
  const active=['scan','confirm','success'].includes(screen.value)&&operation.value!=='register'&&!sessionExpired.value
  if(screen.value==='success'&&!displayExpiry)displayExpiry=Date.now()+15000
  if(screen.value!=='success')displayExpiry=0
  channel?.postMessage({type:'state',state:!active?'idle':screen.value==='success'?'success':'active',operation:operation.value,quantity:screen.value==='success'?(result.value?.quantity??count.value):count.value,reader:active&&reader.value?reader.value.name.split(' ').slice(0,2).join(' '):'',books:active?basket.value.map(x=>({name:titleFor(x.titleId)?.name??'',quantity:x.quantity})):[],expiresAt:displayExpiry||Date.now()+45000})
}
watch([screen,basket,reader,sessionExpired],broadcast,{deep:true})
let scanBuffer='',lastKey=0,lastActivity=Date.now(),overlayScanBuffer='',overlayLastKey=0,terminalCaptureBuffer='',terminalCaptureLastKey=0
function onKey(event:KeyboardEvent){
  if(route.value==='#/display'||route.value==='#/components')return
  lastActivity=Date.now()
  if(event.defaultPrevented)return
  if(help.value&&event.key==='Escape'){closeHelp();return}
  if(terminalCaptureKind.value){
    if(event.key==='Escape'){event.preventDefault();cancelTerminalCapture();return}
    if(Date.now()-terminalCaptureLastKey>1000)terminalCaptureBuffer=''
    terminalCaptureLastKey=Date.now()
    if(event.key==='Enter'){
      if(terminalCaptureBuffer.length){terminalCapturedValue.value=terminalCaptureBuffer;terminalCapturedSuffix.value='Enter';terminalCaptureBuffer='';terminalCaptureKind.value='';event.preventDefault()}
      return
    }
    if(event.key.length===1&&!event.ctrlKey&&!event.metaKey){terminalCaptureBuffer+=event.key;event.preventDefault()}
    return
  }
  if(event.key==='Escape'){keyboard.value='';if(!busy.value){settings.value=false;dialog.value='';dismissWrongReader()};return}
  const target=event.target as HTMLElemen
  const editable=['INPUT','TEXTAREA','SELECT'].includes(target.tagName)||target.isContentEditable
  if(overlayOpen.value&&!editable){
    scanBuffer=''
    if(Date.now()-overlayLastKey>1000)overlayScanBuffer=''
    overlayLastKey=Date.now()
    // A scanner's trailing Enter must not activate the focused modal button.
    if(event.key==='Enter'){if(overlayScanBuffer.length>=3)event.preventDefault();overlayScanBuffer=''}
    else if(event.key.length===1&&!event.ctrlKey&&!event.metaKey)overlayScanBuffer+=event.key
    return
  }
  if(editable||overlayOpen.value||busy.value){scanBuffer='';return}
  if(Date.now()-lastKey>1000)scanBuffer='';lastKey=Date.now()
  if(event.key==='Enter'&&scanBuffer.length>=3){const value=scanBuffer;scanBuffer='';event.preventDefault();if(screen.value==='identify')identify(value);else if(screen.value==='scan')scan(value);else if(screen.value==='registration')registrationRef.value?.scan(value)}
  else if(event.key.length===1&&!event.ctrlKey&&!event.metaKey)scanBuffer+=event.key
}
function updateHash(){
  route.value=location.hash
  if(route.value==='#/display'||route.value==='#/components')cancelIdentification()
  // Workflow deep links start with identification; they never manufacture a reader.
  const requested=route.value.match(/^#\/(accept|issue)(?:\/.*)?$/)?.[1]
  if(requested&&operation.value!==requested&&screen.value==='identify'&&!basket.value.length)cancelIdentification()
  if(requested&&(screen.value==='home'||operation.value!==requested||!reader.value)&&!busy.value&&screen.value!=='unknown'&&!basket.value.length)start(requested as 'accept'|'issue')
}
function pointerActivity(){lastActivity=Date.now()}
let timer:ReturnType<typeof setInterval>
onMounted(()=>{
  window.addEventListener('hashchange',updateHash);window.addEventListener('keydown',onKey);window.addEventListener('pointerdown',pointerActivity)
  channel?.addEventListener('message',event=>{if(event.data?.type==='request')broadcast()})
  timer=setInterval(()=>{clock.value=new Date();if(workflow.value&&!busy.value&&Date.now()-lastActivity>300000)sessionExpired.value=true;broadcast()},10000)
  if(route.value!=='#/display'&&route.value!=='#/components')try { const pending=JSON.parse(localStorage.getItem('edus-library-pending')??'null');if(pending&&typeof pending.id==='string'&&['issue','accept','register'].includes(pending.operation)){operationId.value=pending.id;operation.value=pending.operation;reader.value=pending.reader??null;basket.value=Array.isArray(pending.basket)?pending.basket:[];screen.value='unknown'} } catch { /* Unavailable or invalid demo recovery storage: the adapter still preserves receipts when possible. */ }
  updateHash()
})
onUnmounted(()=>{cancelIdentification();setIdentificationTransitionPaused(false);clearInterval(timer);channel?.close();window.removeEventListener('hashchange',updateHash);window.removeEventListener('keydown',onKey);window.removeEventListener('pointerdown',pointerActivity)})
watch(screen,(s)=>{keyboard.value='';try{if(s==='unknown')localStorage.setItem('edus-library-pending',JSON.stringify({id:operationId.value,operation:operation.value,reader:reader.value,basket:basket.value}));if(s==='success')localStorage.removeItem('edus-library-pending')}catch{message.value='Хранилище браузера недоступно. До проверки результата не закрывайте окно.'}})
watch([settings,dialog,sessionExpired,wrongReader],async()=>{
  scanBuffer='';overlayScanBuffer='';await nextTick()
  if(sessionExpired.value)document.querySelector<HTMLElement>('.session-backdrop button')?.focus()
  else if(help.value)return
  else if(wrongReader.value)document.querySelector<HTMLElement>('.wrong-reader-dialog button')?.focus()
  else if(dialog.value==='manual')scanInput.value?.focus()
  else document.querySelector<HTMLElement>('.modal button')?.focus()
})
function trapFocus(e:KeyboardEvent){
  if(e.key!=='Tab')return
  const items=Array.from((e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('button:not([disabled]),a[href],input:not([disabled]),select:not([disabled])'))
  const first=items[0],last=items.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus()}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus()}
}
</script>

<template>
  <StudentDisplay v-if="route==='#/display'" />
  <ComponentGallery v-else-if="demoRuntime && route==='#/components'" />
  <TerminalShell v-else :class="{ 'keyboard-open': keyboard && !help }">
    <template #header><SystemHeader :time="time" :date="date" :identification="screen==='identify'||issueScan||acceptScan" :terminal-test="terminalTestRuntime" :inert="overlayOpen" @settings="settings=true" @help="openHelp" />
      <div v-if="!server && workflow" class="server-banner" role="alert"><CircleAlert :size="24" />Нет связи с локальным сервером. Текущий список сохранён.</div>
    </template>
    <main class="app-main" :class="[{'workflow-main':workflow},`screen-${screen}`]" :aria-busy="busy" :inert="overlayOpen">
      <template v-if="screen==='home'">
        <div class="home-content">
          <div class="home-heading"><div><h1>Что будем делать?</h1><p>Выберите операцию, чтобы начать работу.</p></div><span class="home-date">{{ date }}</span></div>
          <div class="home-actions">
            <HomeAction title="Выдать книги" description="Выберите читателя и отсканируйте книги" icon="issue" accent @click="start('issue')" />
            <HomeAction title="Принять книги" description="Определите читателя и примите его книги" icon="return" @click="start('accept')" />
            <HomeAction title="Добавить книги" description="Зарегистрируйте издания и экземпляры" icon="add" @click="start('register')" />
          </div>
          <div class="home-note"><ShieldCheck :stroke-width="1.8" /><span>Карта EDUS для читателя. Штрихкод — для книги.</span></div>
        </div>
      </template>
      <template v-else-if="screen==='success'">
        <div class="success-content">
          <div class="success-mark"><Check :stroke-width="2" /></div>
          <h1>{{ operation==='issue'?'Книги выданы':operation==='accept'?'Книги приняты':'Книги добавлены' }}</h1>
          <p>{{ operation==='register'?'Библиотечный фонд обновлён.':operation==='issue'?'Всё готово. Приятного чтения!':'Книги снова доступны в библиотеке.' }}</p>
          <div class="success-details"><BookOpen :stroke-width="1.8" /><div><strong>{{ result?.quantity??count }} {{ operation==='issue'?'выдано':operation==='accept'?'принято':'зарегистрировано' }}</strong><span>{{ reader?.name || (operation==='register'?'Регистрация в фонде':'Возврат в библиотечный фонд') }}</span></div></div>
          <div class="success-actions"><button class="btn secondary" @click="home"><Home />На главную</button><button class="btn primary" @click="start(operation)">{{ operation==='register'?'Добавить ещё книги':operation==='accept'?'Следующий возврат':'Следующий читатель' }}<ArrowRight /></button></div>
          <span class="small muted">Демонстрационная операция выполнена</span>
        </div>
      </template>
      <template v-else>
        <div v-if="issueScan||acceptScan" class="issue-scan-navigation"><button class="btn ghost back-button" :disabled="busy||screen==='unknown'" @click="globalBack"><ArrowLeft />Назад</button><OperationStepper :steps="['Читатель',acceptScan?'Приём книг':'Книги','Подтверждение']" :active="1" /><button class="btn secondary change-reader-button" :disabled="busy||sessionExpired" @click="changeReader"><ArrowRight class="change-reader-icon" />Сменить читателя</button></div>
        <div v-else class="topline"><button class="btn ghost back-button" :disabled="(busy&&screen!=='identify')||screen==='unknown'" @click="globalBack"><ArrowLeft />Назад</button><span class="label">{{ operationName }}</span><button class="btn ghost" :disabled="(busy&&screen!=='identify')||screen==='unknown'" @click="globalHome"><Home :size="22" />На главную</button></div>
        <template v-if="screen==='registration'"><RegistrationView ref="registrationRef" /></template>
        <template v-else>
          <div v-if="screen!=='identify'&&!issueScan&&!acceptScan" class="operation-heading"><div><h1>{{ heading }}</h1><p v-if="screen==='scan'">{{ titleHelp }}</p><p v-if="screen==='confirm'">Проверьте список перед {{operation==='issue'?'выдачей':'приёмом'}}.</p></div><OperationStepper :steps="['Читатель','Книги','Подтверждение']" :active="currentStep" /></div>
          <div v-if="message && !wrongReader && screen!=='identify'" class="feedback error" role="alert"><CircleAlert /><span>{{ message }}</span><button aria-label="Закрыть сообщение" @click="message=''"><X :size="20" /></button></div>
          <div v-else-if="info && screen!=='scan' && screen!=='identify'" class="feedback" role="status"><Check /><span>{{ info }}</span></div>
          <div class="workflow-body">
            <ReaderIdentification v-if="screen==='identify'" :state="nfcState" :busy="busy" :message="message" @search="openSearch" @dismiss="message=''" />
            <div v-else-if="screen==='search'" class="search-layout"><ReaderSearch v-model="query" :results="searchResults" :loading="searching" :searched="searched" @select="chooseReader" @keyboard="keyboard='search'" /></div>
            <div v-else-if="issueScan" class="issue-scan-layout">
              <ReaderSummary :name="reader!.name" :group="reader!.group" :card="reader!.card" :count="readerCount" variant="scan" @loans="openLoans" />
              <div class="issue-scan-workspace">
                <div class="issue-scanner-column"><ScannerPanel issue description="Поднесите штрихкод книги к сканеру или введите инвентарный номер вручную" @manual="openManual" @isbn="openIsbn" /></div>
                <div class="issue-basket-column"><BookBasket title="Список книг" :count="count"><template v-if="basket.length"><div v-for="item in basket" :key="item.id"><BookRow :title="titleFor(item.titleId)?.name??''" :author="titleFor(item.titleId)?.author" :code="codeFor(item)" :mode="item.mode==='COPY'?'Индивидуальный экземпляр':'Учёт количеством'" :quantity="item.quantity" @remove="removeItem(item.id)"/></div></template><template #empty><div class="issue-basket-empty"><BookOpen :stroke-width="1.5" /><p>Здесь появятся<br />отсканированные книги</p></div></template></BookBasket></div>
              </div>
            </div>
            <div v-else-if="acceptScan" class="accept-scan-layout">
              <ReaderSummary :name="reader!.name" :group="reader!.group" :card="reader!.card" :count="readerCount" variant="scan" @loans="openLoans" />
              <div class="accept-scan-workspace">
                <div class="accept-scanner-column">
                  <ScannerPanel accept title="Сканируйте книги для приёма" description="Поднесите штрихкод книги к сканеру или введите инвентарный номер вручную" @manual="openManual" @on-hand-search="openReturnSearch" />
                </div>
                <ReturnLoanList :items="returnLoans" :state="returnLoansState" @details="openReturnItem" @retry="retryReturnLoans" />
              </div>
            </div>
            <div v-else-if="screen==='scan' && reader" class="scan-layout">
              <div class="scanner-column"><ScannerPanel @manual="openManual" /><p class="small muted scanner-note">Поддерживаются штрихкоды экземпляров и ISBN изданий.</p></div>
              <div class="basket-column">
                <ReaderSummary v-if="reader" :name="reader.name" :group="reader.group" :count="readerCount" @change="changeReader" />
                <BookBasket :title="operation==='issue'?'К выдаче':'К приёму'" :count="count"><template v-if="basket.length"><div v-for="item in basket" :key="item.id"><BookRow :title="titleFor(item.titleId)?.name??''" :author="titleFor(item.titleId)?.author" :code="codeFor(item)" :mode="item.mode==='COPY'?'Индивидуальный экземпляр':'Старый фонд'" :quantity="item.quantity" @remove="removeItem(item.id)"/></div></template><template #empty><div class="plain-empty"><LibraryBig :stroke-width="1.5" /><h3>Здесь появятся книги</h3><p>Отсканируйте штрихкод или введите номер вручную.</p></div></template></BookBasket>
              </div>
            </div>
            <div v-else-if="screen==='confirm'" class="confirm-layout"><ReaderSummary v-if="reader" :name="reader.name" :group="reader.group" :count="readerCount" :changeable="false" />
              <div class="review-list"><div class="review-title"><h3>{{operation==='issue'?'К выдаче':'К приёму'}} · {{count}}</h3><span>Позиций: {{basket.length}}</span></div><div v-for="item in basket" :key="item.id"><BookRow :title="titleFor(item.titleId)?.name??''" :author="titleFor(item.titleId)?.author" :code="codeFor(item)" :mode="item.mode==='COPY'?'Индивидуальный экземпляр':'Старый фонд'" :quantity="item.quantity" :removable="false" /></div></div>
            </div>
            <div v-else-if="screen==='unknown'" class="success-content"><div class="success-mark" style="background:var(--gold-soft);color:var(--warning)"><Clock3 /></div><h2>Ответ ещё не получен</h2><p>Операция могла выполниться. Проверьте результат,<br />чтобы не оформить её повторно.</p><button class="btn primary" :disabled="busy||!server" @click="checkResult">{{busy?'Проверяем…':'Проверить результат'}}</button><span class="small muted">Список сохранён. Повторная отправка заблокирована.</span></div>
          </div>
        </template>
      </template>
    </main>
    <template #footer>
      <ActionBar v-if="['scan','confirm'].includes(screen) && reader" :inert="overlayOpen"><template v-if="issueScan"><button class="btn secondary clear-basket-button" :disabled="!count||busy" @click="requestClearBasket"><X />Очистить список</button></template><template v-else-if="acceptScan"><button class="btn secondary clear-basket-button" :disabled="!count||busy" @click="requestClearBasket"><X />Сбросить отметки</button></template><template v-else><div class="action-count">{{count}} {{operation==='issue'?'к выдаче':'к приёму'}}<small v-if="!count">Добавьте первую книгу</small></div></template><template #right><button v-if="issueScan" class="btn primary issue-confirm-button" :disabled="!count||busy||!server||sessionExpired" @click="confirm">Перейти к подтверждению<ArrowRight /></button><button v-else-if="acceptScan" class="btn primary accept-confirm-button" :disabled="!count||busy||!server||sessionExpired" @click="confirm"><span>Перейти к подтверждению<small>К приёму: {{ count }}</small></span><ArrowRight /></button><template v-else><button v-if="screen==='confirm'" class="btn secondary" :disabled="busy" @click="screen='scan'">Изменить список</button><button class="btn primary" :disabled="!count||busy||!server||sessionExpired" @click="screen==='scan'?confirm():submit()">{{busy?'Выполняем…':screen==='scan'?'Продолжить':`${operation==='issue'?'Выдать':'Принять'} ${count} ${count===1?'книгу':count<5?'книги':'книг'}`}}<ArrowRight v-if="!busy" /></button></template></template></ActionBar>
    </template>
    <div v-if="dialog" class="modal-backdrop" @click.self="closeDialog"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="dialog-heading" @keydown="trapFocus">
      <div class="modal-header"><h2 id="dialog-heading">{{dialog==='manual'?(manualKind==='isbn'?'Введите ISBN':'Введите инвентарный номер'):dialog==='title'?'Найдено издание':dialog==='leave'?'Завершить без сохранения?':dialog==='clear-basket'?(acceptScan?'Сбросить отметки?':'Очистить список?'):dialog==='loans'?'Книги на руках':dialog==='return-search'?'Найти книгу на руках':dialog==='return-item'?'Книга к приёму':'Выбрать другого читателя?'}}</h2><button class="btn" aria-label="Закрыть" @click="closeDialog"><X /></button></div>
      <template v-if="dialog==='manual'"><p>{{manualKind==='isbn'?'ISBN определяет издание. Выберите допустимый способ учёта на следующем шаге.':'Буквы, цифры, ведущие нули и разделители сохраняются.'}}</p><TouchInput ref="scanInput" v-model="scanValue" :label="manualKind==='isbn'?'ISBN':'Инвентарный номер'" :placeholder="manualKind==='isbn'?'Например, 9786010123456':'Например, 000124 или KZ-0008'" @keyboard="keyboard='scan'" @enter="manualSubmit" /><div class="modal-actions"><button class="btn primary" :disabled="!scanValue.trim()||!server" @click="manualSubmit">Найти книгу<ArrowRight /></button></div></template>
      <template v-else-if="dialog==='title'">
        <template v-if="ambiguousTitles.length"><p>Код соответствует нескольким изданиям. Выберите нужное.</p><button v-for="t in ambiguousTitles" :key="t.id" class="loan-option" @click="chooseTitle(t)"><BookOpen /><span>{{t.name}}<small>{{t.author}}</small></span><ArrowRight /></button></template>
        <template v-else-if="titleChoice"><div class="title-detail"><BookOpen /><div><h3>{{titleChoice.name}}</h3><p>{{titleChoice.author}}</p><p class="small">ISBN {{titleChoice.isbn}}</p></div></div><p class="isbn-note">ISBN обозначает издание, а не отдельную книгу.</p>
          <button v-if="operation==='issue'" class="btn secondary full" @click="openManual"><Keyboard />Указать инвентарный номер</button>
          <div class="modal-section"><h3>Книги без индивидуального номера</h3><p class="small">{{operation==='issue'?'Существующий старый фонд':'Активная выдача выбранного читателя'}} · доступно {{legacyLimit}}</p><div v-if="legacyLimit" class="quantity-line"><span>Количество</span><QuantityControl v-model="legacyQuantity" :max="legacyLimit" /></div><p v-else class="feedback error">{{operation==='accept'?'У этого читателя нет выдачи такого издания.':'В старом фонде нет доступных книг этого издания.'}}</p></div>
          <button class="btn primary" :disabled="!legacyLimit" @click="addLegacy">{{operation==='issue'?'Добавить из старого фонда':'Добавить к приёму'}}<ArrowRight /></button>
        </template>
      </template>
      <template v-else-if="dialog==='return-search'"><p>Выберите книгу из активных выдач читателя. Отметка появится только после явного действия.</p><div v-if="returnLoans.length" class="return-search-options"><button v-for="item in returnLoans" :key="item.id" class="loan-option" type="button" @click="chooseReturnLoan(item)"><BookOpen :size="25" /><span>{{ item.title }}<small>{{ item.code ? `Инв. № ${item.code}` : 'Учёт количеством' }} · на руках: {{ item.quantity }}</small></span><ArrowRight /></button></div><p v-else>У читателя нет книг на руках.</p></template>
      <template v-else-if="dialog==='return-item' && selectedReturnLoan"><div class="title-detail"><BookOpen /><div><h3>{{ selectedReturnLoan.title }}</h3><p>{{ selectedReturnLoan.code ? `Инвентарный номер ${selectedReturnLoan.code}` : 'Книга учитывается количеством' }}</p><p class="small">На руках: {{ selectedReturnLoan.quantity }}</p></div></div><div v-if="selectedReturnLoan.mode==='LEGACY_TITLE'" class="modal-section"><h3>Количество к приёму</h3><QuantityControl v-model="returnQuantity" :max="selectedReturnLoan.quantity" /></div><p v-else class="isbn-note">Экземпляр с этим номером будет отмечен один раз.</p><div class="modal-actions"><button v-if="selectedReturnLoan.selectedQuantity" class="btn secondary" @click="setReturnLoanQuantity(selectedReturnLoan.id,0);closeDialog()">Убрать из приёма</button><button v-if="!selectedReturnLoan.selectedQuantity || selectedReturnLoan.mode==='LEGACY_TITLE'" class="btn primary" @click="setReturnLoanQuantity(selectedReturnLoan.id,selectedReturnLoan.mode==='LEGACY_TITLE'?returnQuantity:1);closeDialog()">{{ selectedReturnLoan.selectedQuantity ? 'Сохранить количество' : 'Отметить к приёму' }}<Check /></button></div></template>
      <template v-else-if="dialog==='loans'"><div v-if="loans.length" class="loan-options"><div v-for="loan in loans" :key="loan.id" class="loan-option"><BookOpen :size="24" /><span>{{ titleFor(loan.titleId)?.name }}<small>{{ loan.mode==='COPY'?'Индивидуальный экземпляр':`Учёт количеством · ${loan.quantity} шт.` }}</small></span></div></div><p v-else>У выбранного читателя сейчас нет книг на руках.</p><div class="modal-actions"><button class="btn primary" @click="closeDialog">Понятно</button></div></template>
      <template v-else><p>{{dialog==='leave'?'Книги будут убраны только из текущего списка. Фонд и выдачи не изменятся.':dialog==='clear-basket'?(acceptScan?'Будут сняты только отметки к приёму. Книги на руках и выбранный читатель останутся на экране.':'Текущий список книг будет очищен. Выбранный читатель сохранится.'):'Текущий список книг будет очищен. Затем можно выбрать другого читателя.'}}</p><div class="modal-actions"><button class="btn secondary" @click="closeDialog">Продолжить работу</button><button class="btn primary" @click="dialog==='leave'?home():dialog==='clear-basket'?(acceptScan?resetReturnMarks():(clearBasket(),closeDialog())):confirmChangeReader()">{{dialog==='leave'?'На главную':dialog==='clear-basket'?(acceptScan?'Сбросить отметки':'Очистить список'):'Очистить и выбрать'}}</button></div></template>
      <p v-if="message" class="feedback error" role="alert">{{message}}</p>
    </section></div>
    <div v-if="settings" class="modal-backdrop" @click.self="settings=false"><section class="modal wide" role="dialog" aria-modal="true" aria-labelledby="settings-heading" @keydown="trapFocus">
      <div class="modal-header"><h2 id="settings-heading">Настройки и диагностика</h2><button class="btn" aria-label="Закрыть настройки" @click="settings=false"><X /></button></div>
      <p class="small">EDUS · Библиотека. Состояние оборудования и API проверяется на установленном терминале.</p>
      <div class="settings-grid"><div><h3>Состояние системы</h3><template v-if="demoRuntime"><div class="setting-row"><span>Локальный сервер<p>Демонстрационное соединение</p></span><button class="toggle" :class="{on:server}" :aria-pressed="server" @click="toggleConnection('server')">{{server?'Доступен':'Нет связи'}}</button></div><div class="setting-row"><span>Интернет<p>Не нужен для локальных операций</p></span><button class="toggle" :class="{on:internet}" :aria-pressed="internet" @click="toggleConnection('internet')">{{internet?'Доступен':'Нет связи'}}</button></div></template><div class="setting-row"><span>Сканер / NFC</span><span class="muted small">Проверяется диагностикой терминала</span></div><div class="setting-row"><span>Автоматический приём</span><span class="muted small">Выключен</span></div></div><div><h3>Просмотр и проверка</h3><div class="form-section" style="margin-top:20px"><a class="btn secondary" href="#/display" target="_blank" rel="noopener"><Monitor />Второй экран</a><a v-if="demoRuntime" class="btn secondary" href="#/components" target="_blank" rel="noopener"><LibraryBig />Компоненты и состояния</a><button v-if="demoRuntime" class="btn secondary" :aria-expanded="demoPanel" @click="demoPanel=!demoPanel"><SlidersHorizontal />Панель сценариев</button><button v-if="terminalTestRuntime" class="btn secondary" :aria-expanded="terminalTestPanel" @click="terminalTestPanel=!terminalTestPanel"><SlidersHorizontal />Панель тестирования</button></div></div></div>
      <DemoScenarioPanel v-if="demoRuntime && demoPanel" :screen="screen" @card="demoCard" @scan="demoScan" @outcome="setOutcome($event);settings=false;info=$event==='unknown'?'Следующая операция: ответ задержится. Результат можно будет проверить.':'Следующая операция завершится демонстрационной ошибкой.'" @expire="expire" />
      <TerminalTestPanel v-if="terminalTestRuntime && terminalTestPanel" :screen="screen" :readers="snapshot.readers" :titles="snapshot.titles" :copies="snapshot.copies" :capture-kind="terminalCaptureKind" :captured-value="terminalCapturedValue" :captured-suffix="terminalCapturedSuffix" :persistence="snapshot.persistence" @capture="beginTerminalCapture" @cancel-capture="cancelTerminalCapture" @bind-card="bindTerminalCard" @bind-code="bindTerminalCode" @card="demoCard" @scan="demoScan" @outcome="setOutcome($event);info=$event==='unknown'?'Следующая операция вернёт неопределённый результат.':'Следующая операция вернёт тестовую ошибку.'" @delay="setTestDelay" @reset="resetTerminalData" />
    </section></div>
    <div v-if="wrongReader" class="modal-backdrop" @click.self="dismissWrongReader"><section class="modal wrong-reader-dialog" role="dialog" aria-modal="true" aria-labelledby="wrong-reader-heading" @keydown="trapFocus">
      <div class="modal-header"><h2 id="wrong-reader-heading">Книга другого читателя</h2><button class="btn" aria-label="Закрыть предупреждение" @click="dismissWrongReader"><X /></button></div>
      <div class="wrong-book"><BookOpen :size="32" /><div><h3>{{ wrongReader.title }}</h3><span>Код {{ wrongReader.code }}</span></div></div>
      <p>Книга числится за другим читателем:</p>
      <div class="wrong-reader-owner"><strong>{{ wrongReader.name }}</strong><span>{{ wrongReader.group }}</span></div>
      <p>Книга не добавлена к приёму. Текущий читатель и список сохранены.</p>
      <div class="modal-actions"><button class="btn secondary" @click="switchFromWrongReader">Сменить читателя</button><button class="btn primary" @click="dismissWrongReader">Не добавлять книгу</button></div>
    </section></div>
    <ContextHelp v-if="help" :screen="screen" :operation="operation" @close="closeHelp" />
    <div v-if="sessionExpired" class="modal-backdrop session-backdrop"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="session-heading" @keydown="trapFocus"><h2 id="session-heading">Сессия приостановлена</h2><p>Текущий список сохранён. Подтвердите продолжение работы с демонстрационными данными.</p><button class="btn primary" @click="sessionExpired=false;lastActivity=Date.now()">Продолжить сессию</button></section></div>
    <TouchKeyboard v-if="keyboard && !help" v-model="keyboardValue" :label="keyboard==='search'?'Поиск читателя':'Код книги'" @close="keyboard=''" @enter="keyboard==='scan'?manualSubmit():keyboard=''" />
  </TerminalShell>
</template>

<style scoped>
.app-main.screen-identify { max-width: none; padding: 16px 56px 24px; gap: 12px; }
.screen-identify .topline { flex-shrink: 0; }
@media(max-height:820px) { .app-main.screen-identify { padding: 8px 40px 16px; gap: 8px; } }
@media(max-width:760px) { .app-main.screen-identify { padding: 12px 20px; } }
</style>

\n