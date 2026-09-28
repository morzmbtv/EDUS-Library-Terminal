<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { ArrowRight, BookOpen, Check, CircleAlert, Clock3, Keyboard, LibraryBig, X } from '@lucide/vue';
import { api } from '../shared/api/LocalServiceApiClient.ts';
import { CardHidBuffer } from '../shared/lib/cardHid.ts';
import { formatDate, locale, localeSaveFailed, localeTag, setLocale, t } from '../shared/i18n/index.ts';
import TerminalShell from '../shared/ui/TerminalShell.vue';
import SystemHeader from '../shared/ui/SystemHeader.vue';
import TerminalBackButton from '../shared/ui/TerminalBackButton.vue';
import HomeAction from '../shared/ui/HomeAction.vue';
import OperationStepper from '../shared/ui/OperationStepper.vue';
import ReaderSummary from '../shared/ui/ReaderSummary.vue';
import ReaderIdentification from '../shared/ui/ReaderIdentification.vue';
import FaceIdentification from '../shared/ui/FaceIdentification.vue';
import ScannerPanel from '../shared/ui/ScannerPanel.vue';
import BookRow from '../shared/ui/BookRow.vue';
import BookBasket from '../shared/ui/BookBasket.vue';
import ActionBar from '../shared/ui/ActionBar.vue';
import TouchInput from '../shared/ui/TouchInput.vue';
import TouchKeyboard from '../shared/ui/TouchKeyboard.vue';
import QuantityControl from '../shared/ui/QuantityControl.vue';
import ContextHelp from '../shared/ui/ContextHelp.vue';
import ReturnLoanList, { type ReturnLoanListItem } from '../shared/ui/ReturnLoanList.vue';
import LibrarySearchView from '../pages/LibrarySearchView.vue';
import {
  screen,
  operation,
  reader,
  basket,
  busy,
  message,
  info,
  sessionExpired,
  nfcState,
  faceCaptureActive,
  scanValue,
  titleChoice,
  ambiguousTitles,
  legacyQuantity,
  dialog,
  result,
  server,
  count,
  loans,
  readerCount,
  operationName,
  titleFor,
  codeFor,
  start,
  home,
  changeReader,
  confirmChangeReader,
  back,
  openFace,
  openCard,
  openLibrarySearch,
  beginCatalogueIssue,
  beginCatalogueReservation,
  identify,
  scan,
  chooseTitle,
  legacyLimit,
  addLegacy,
  removeItem,
  clearBasket,
  confirm,
  submit,
  checkResult,
  wrongReader,
  setIdentificationTransitionPaused,
  cancelIdentification,
  setReturnLoanQuantity,
  refresh,
  refreshNative,
  scansPending,
  snapshot,
  reservationTitleId,
} from '../features/terminal/useTerminal.ts';
const schoolName = ref('EDUS Library');
let events: EventSource | undefined;
const route = ref(location.hash);
const clock = ref(new Date());
const time = computed(() =>
  clock.value.toLocaleTimeString(localeTag.value, { hour: '2-digit', minute: '2-digit' }),
);
const date = computed(() => formatDate(clock.value, { day: 'numeric', month: 'long', year: 'numeric' }));
const settings = ref(false),
  help = ref(false);
const nativeTerminalTest = ref(false);
const nativeUat = ref(false);
const uatBuildLabel = ref('EDUS Library · UNSIGNED RC BUILD');
watch(() => help.value || settings.value, setIdentificationTransitionPaused, {
  flush: 'sync',
  immediate: true,
});
const overlayOpen = computed(
  () => settings.value || help.value || !!dialog.value || !!wrongReader.value || sessionExpired.value,
);
const keyboard = ref<'' | 'library-search' | 'scan'>('');
const librarySearchQuery = ref('');
const keyboardValue = computed({
  get: () => (keyboard.value === 'library-search' ? librarySearchQuery.value : scanValue.value),
  set: (v: string) => {
    if (keyboard.value === 'library-search') librarySearchQuery.value = v;
    else scanValue.value = v;
  },
});
const librarySearchRef = ref<InstanceType<typeof LibrarySearchView> | null>(null);
const scanInput = ref<InstanceType<typeof TouchInput> | null>(null);
const manualKind = ref<'inventory' | 'isbn'>('inventory');
const workflow = computed(() => !['home', 'library-search', 'success'].includes(screen.value));
const issueScan = computed(() => screen.value === 'scan' && operation.value === 'issue' && !!reader.value);
const acceptScan = computed(() => screen.value === 'scan' && operation.value === 'accept' && !!reader.value);
const returnLoans = computed<ReturnLoanListItem[]>(() =>
  loans.value.map((loan) => ({
    id: loan.id,
    title: titleFor(loan.titleId)?.name ?? 'Неизвестное издание',
    author: titleFor(loan.titleId)?.author,
    code: loan.copyId ? snapshotCopyCode(loan.copyId) : undefined,
    mode: loan.mode,
    quantity: loan.quantity,
    selectedQuantity: basket.value.find((item) => item.loanId === loan.id)?.quantity ?? 0,
  })),
);
const returnLoansState = computed<'loading' | 'ready' | 'error'>(() => (server.value ? 'ready' : 'error'));
const selectedReturnLoan = ref<ReturnLoanListItem | null>(null);
const returnQuantity = ref(1);
const currentStep = computed(() =>
  ['identify', 'face'].includes(screen.value)
    ? 0
    : ['confirm', 'unknown', 'reservation-confirm'].includes(screen.value)
      ? 2
      : 1,
);
const heading = computed(() =>
  screen.value === 'identify'
    ? operation.value === 'issue'
      ? 'Кому выдаём книги?'
      : operation.value === 'reserve'
        ? 'Кого поставить в очередь?'
        : 'Кто возвращает книги?'
    : screen.value === 'reservation-confirm'
      ? 'Подтвердите очередь'
      : screen.value === 'confirm'
        ? 'Всё верно?'
        : screen.value === 'unknown'
          ? 'Проверяем результат операции'
          : operation.value === 'issue'
            ? 'Сканируйте книги'
            : 'Какие книги принимаем?',
);
const titleHelp = computed(() =>
  operation.value === 'issue'
    ? 'Добавьте книги, которые хотите выдать читателю.'
    : 'Сканируйте книги выбранного читателя или выберите их из списка.',
);
function globalBack() {
  back();
}
function openLibrarySearchRoute() {
  openLibrarySearch();
  librarySearchQuery.value = '';
  if (location.hash !== '#/library-search') window.history.pushState(null, '', '#/library-search');
  route.value = location.hash;
}
function leaveLibrarySearch() {
  home();
  if (location.hash === '#/library-search') window.history.pushState(null, '', '#/');
  route.value = location.hash;
}
function openManual() {
  manualKind.value = 'inventory';
  scanValue.value = '';
  dialog.value = 'manual';
  nextTick(() => scanInput.value?.focus());
}
function openIsbn() {
  manualKind.value = 'isbn';
  scanValue.value = '';
  dialog.value = 'manual';
  nextTick(() => scanInput.value?.focus());
}
async function manualSubmit() {
  const code = scanValue.value;
  keyboard.value = '';
  dialog.value = '';
  await scan(code);
}
function openLoans() {
  dialog.value = 'loans';
}
function requestClearBasket() {
  if (basket.value.length && !busy.value) dialog.value = 'clear-basket';
}
function openReturnItem(item: ReturnLoanListItem) {
  selectedReturnLoan.value = item;
  returnQuantity.value = item.selectedQuantity || 1;
  dialog.value = 'return-item';
}
function openReturnSearch() {
  dialog.value = 'return-search';
}
function chooseReturnLoan(item: ReturnLoanListItem) {
  openReturnItem(item);
}
function snapshotCopyCode(copyId: string) {
  return codeFor({ id: copyId, titleId: '', copyId, quantity: 1, mode: 'COPY' });
}
function resetReturnMarks() {
  clearBasket();
  selectedReturnLoan.value = null;
  closeDialog();
}
function retryReturnLoans() {
  refresh();
}
function openSettings() {
  settings.value = true;
}
function closeSettings() {
  settings.value = false;
}
function closeDialog() {
  dialog.value = '';
  keyboard.value = '';
  message.value = '';
}
let helpReturnFocus: HTMLElement | null = null;
function openHelp() {
  helpReturnFocus = document.querySelector<HTMLElement>(
    screen.value === 'library-search'
      ? '.library-search-view input'
      : dialog.value === 'manual'
        ? '.modal input'
        : ':focus',
  );
  scanBuffer = '';
  overlayScanBuffer = '';
  help.value = true;
}
async function closeHelp() {
  help.value = false;
  scanBuffer = '';
  overlayScanBuffer = '';
  await nextTick();
  if (helpReturnFocus?.isConnected) helpReturnFocus.focus({ preventScroll: true });
  else document.querySelector<HTMLElement>('[aria-label="Помощь"]')?.focus({ preventScroll: true });
}
function dismissWrongReader() {
  wrongReader.value = null;
  message.value = '';
}
function switchFromWrongReader() {
  dismissWrongReader();
  dialog.value = '';
  changeReader();
}
const cardHid = new CardHidBuffer();
let scanBuffer = '',
  lastKey = 0,
  lastActivity = Date.now(),
  overlayScanBuffer = '',
  overlayLastKey = 0;
function onKey(event: KeyboardEvent) {
  lastActivity = Date.now();
  if (event.defaultPrevented) return;
  if (help.value && event.key === 'Escape') {
    closeHelp();
    return;
  }
  if (event.key === 'Escape') {
    keyboard.value = '';
    if (!busy.value) {
      dialog.value = '';
      dismissWrongReader();
    }
    return;
  }
  const target = event.target as HTMLElement;
  const editable = ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable;
  if (overlayOpen.value && !editable) {
    cardHid.reset();
    scanBuffer = '';
    if (Date.now() - overlayLastKey > 1000) overlayScanBuffer = '';
    overlayLastKey = Date.now();
    // A scanner's trailing Enter must not activate the focused modal button.
    if (event.key === 'Enter') {
      if (overlayScanBuffer.length >= 3) event.preventDefault();
      overlayScanBuffer = '';
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) overlayScanBuffer += event.key;
    return;
  }
  if (editable || overlayOpen.value || busy.value) {
    scanBuffer = '';
    cardHid.reset();
    return;
  }
  if (screen.value === 'identify') {
    const frame = cardHid.feed(event.key, Date.now(), event.ctrlKey || event.metaKey || event.altKey);
    if (frame) {
      event.preventDefault();
      void identify(frame.code);
    }
    return;
  }
  if (Date.now() - lastKey > 1000) scanBuffer = '';
  lastKey = Date.now();
  if (event.key === 'Enter' && scanBuffer.length >= 3) {
    const value = scanBuffer;
    scanBuffer = '';
    event.preventDefault();
    if (screen.value === 'scan') scan(value);
  } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) scanBuffer += event.key;
}
function updateHash() {
  route.value = location.hash;
  if (route.value === '#/library-search') {
    if (screen.value !== 'library-search') openLibrarySearch();
    return;
  }
  if ((!route.value || route.value === '#/') && screen.value === 'library-search') {
    home();
    return;
  }
  // Workflow deep links start with identification; they never manufacture a reader.
  const requested = route.value.match(/^#\/(accept|issue)(?:\/.*)?$/)?.[1];
  if (requested && operation.value !== requested && screen.value === 'identify' && !basket.value.length)
    cancelIdentification();
  if (
    requested &&
    (screen.value === 'home' || operation.value !== requested || !reader.value) &&
    !busy.value &&
    screen.value !== 'unknown' &&
    !basket.value.length
  )
    start(requested as 'accept' | 'issue');
}
function pointerActivity() {
  lastActivity = Date.now();
}
let timer: ReturnType<typeof setInterval>;
onMounted(() => {
  void api
    .getRuntimeStatus()
    .then((value) => {
      nativeTerminalTest.value = value.terminalTest;
      nativeUat.value = value.uat;
      uatBuildLabel.value = value.buildLabel;
    })
    .catch(() => {
      message.value = t('Локальная служба EDUS недоступна');
    });
  events = api.subscribe(
    () => {
      if (!busy.value) void refreshNative().catch(() => undefined);
    },
    () => {
      void refreshNative().catch(() => undefined);
    },
  );
  window.addEventListener('hashchange', updateHash);
  window.addEventListener('keydown', onKey);
  window.addEventListener('pointerdown', pointerActivity);
  timer = setInterval(() => {
    clock.value = new Date();
    if (workflow.value && !busy.value && Date.now() - lastActivity > 300000) sessionExpired.value = true;
  }, 10000);
  updateHash();
});
onUnmounted(() => {
  events?.close();
  cancelIdentification();
  setIdentificationTransitionPaused(false);
  clearInterval(timer);
  window.removeEventListener('hashchange', updateHash);
  window.removeEventListener('keydown', onKey);
  window.removeEventListener('pointerdown', pointerActivity);
});
watch(screen, (s) => {
  cardHid.reset();
  if (s === 'home' && settings.value) closeSettings();
  keyboard.value = '';
  if (s === 'library-search') nextTick(() => librarySearchRef.value?.submit());
});
watch([settings, dialog, sessionExpired, wrongReader], async () => {
  scanBuffer = '';
  overlayScanBuffer = '';
  cardHid.reset();
  await nextTick();
  if (sessionExpired.value) document.querySelector<HTMLElement>('.session-backdrop button')?.focus();
  else if (help.value) return;
  else if (wrongReader.value) document.querySelector<HTMLElement>('.wrong-reader-dialog button')?.focus();
  else if (dialog.value === 'manual') scanInput.value?.focus();
  else document.querySelector<HTMLElement>('.modal button')?.focus();
});
function trapFocus(e: KeyboardEvent) {
  if (e.key !== 'Tab') return;
  const items = Array.from(
    (e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>(
      'button:not([disabled]),a[href],input:not([disabled]),select:not([disabled])',
    ),
  );
  const first = items[0],
    last = items.at(-1);
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault();
    last?.focus();
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault();
    first?.focus();
  }
}
</script>

<template>
  <TerminalShell :class="{ 'keyboard-open': keyboard && !help }">
    <template #header
      ><SystemHeader
        :school-name="schoolName"
        :time="time"
        :date="date"
        :terminal-test="nativeTerminalTest"
        :inert="overlayOpen"
        @settings="openSettings"
        @help="openHelp"
      />
      <div v-if="!server && workflow" class="server-banner" role="alert">
        <CircleAlert :size="24" />{{ t('Нет связи с локальным сервером. Текущий список сохранён.') }}
      </div>
    </template>
    <main
      class="app-main"
      :class="[{ 'workflow-main': workflow }, `screen-${screen}`]"
      :aria-busy="busy"
      :inert="overlayOpen"
    >
      <template v-if="screen === 'home'">
        <div class="home-content">
          <div class="home-heading">
            <div>
              <h1>{{ t('Библиотечный терминал') }}</h1>
              <p>{{ t('Выберите действие для работы с книгами') }}</p>
            </div>
          </div>
          <div class="home-actions">
            <HomeAction
              :title="t('Выдать книги')"
              :description="t('Оформить выдачу книг ученику по карте')"
              icon="issue"
              :action="t('Начать')"
              @click="start('issue')"
            />
            <HomeAction
              :title="t('Принять книги')"
              :description="t('Оформить возврат книг от ученика')"
              icon="return"
              :action="t('Начать')"
              @click="start('accept')"
            />
            <HomeAction
              :title="t('Найти книгу')"
              :description="t('Проверить наличие в фонде и посмотреть информацию')"
              icon="search"
              :action="t('Поиск')"
              @click="openLibrarySearchRoute"
            />
          </div>
        </div>
      </template>
      <template v-else-if="screen === 'library-search'">
        <LibrarySearchView
          ref="librarySearchRef"
          v-model="librarySearchQuery"
          :snapshot="snapshot"
          :server-available="server"
          @back="leaveLibrarySearch"
          @keyboard="keyboard = 'library-search'"
          @issue="beginCatalogueIssue($event.titleId, $event.copyId)"
          @reserve="beginCatalogueReservation"
        />
      </template>
      <template v-else-if="screen === 'success'">
        <div class="topline">
          <TerminalBackButton @click="globalBack" /><span class="label">{{ t(operationName) }}</span>
        </div>
        <div class="success-content">
          <div class="success-mark"><Check :stroke-width="2" /></div>
          <h1>
            {{
              operation === 'issue'
                ? t('Книги выданы')
                : operation === 'accept'
                  ? t('Книги приняты')
                  : operation === 'reserve'
                    ? t('Вы записаны в очередь')
                    : t('Книги добавлены')
            }}
          </h1>
          <p>
            {{
              operation === 'issue'
                ? t('Всё готово. Приятного чтения!')
                : operation === 'accept'
                  ? t('Книги снова доступны в библиотеке.')
                  : t('Когда книга станет доступна, библиотекарь увидит запись в очереди.')
            }}
          </p>
          <div class="success-details">
            <BookOpen :stroke-width="1.8" />
            <div>
              <strong>{{
                operation === 'reserve'
                  ? `Ваша позиция: ${result?.queuePosition ?? '—'}`
                  : `${result?.quantity ?? count} ${operation === 'issue' ? t('выдано') : operation === 'accept' ? t('принято') : t('зарегистрировано')}`
              }}</strong
              ><span>{{ reader?.name || t('Возврат в библиотечный фонд') }}</span>
            </div>
          </div>
          <div class="success-actions">
            <button v-if="operation !== 'reserve'" class="btn primary" @click="start(operation)">
              {{ operation === 'accept' ? t('Следующий возврат') : t('Следующий читатель')
              }}<ArrowRight /></button
            ><button v-else class="btn primary" @click="home">{{ t('Готово') }}</button>
          </div>
          <span class="small muted">{{
            nativeTerminalTest
              ? t('Операция сохранена в отдельной тестовой базе')
              : t('Операция сохранена в локальной базе')
          }}</span>
        </div>
      </template>
      <template v-else>
        <div v-if="issueScan || acceptScan" class="issue-scan-navigation">
          <TerminalBackButton :disabled="busy || screen === 'unknown'" @click="globalBack" /><OperationStepper
            :steps="[t('Читатель'), acceptScan ? t('Приём книг') : t('Книги'), t('Подтверждение')]"
            :active="1"
          /><button
            class="btn secondary change-reader-button"
            :disabled="busy || sessionExpired"
            @click="changeReader"
          >
            <ArrowRight class="change-reader-icon" />{{ t('Сменить читателя') }}
          </button>
        </div>
        <div v-else class="topline">
          <TerminalBackButton
            :disabled="(busy && screen !== 'identify') || screen === 'unknown'"
            @click="globalBack"
          /><span class="label">{{ t(operationName) }}</span>
        </div>
        <div
          v-if="screen !== 'identify' && screen !== 'face' && !issueScan && !acceptScan"
          class="operation-heading"
        >
          <div>
            <h1>{{ t(heading) }}</h1>
            <p v-if="screen === 'scan'">{{ t(titleHelp) }}</p>
            <p v-if="screen === 'confirm'">
              {{ t('Проверьте список перед') }} {{ operation === 'issue' ? t('выдачей') : t('приёмом') }}.
            </p>
          </div>
          <OperationStepper :steps="[t('Читатель'), t('Книги'), t('Подтверждение')]" :active="currentStep" />
        </div>
        <div v-if="message && !wrongReader && screen !== 'identify'" class="feedback error" role="alert">
          <CircleAlert /><span>{{ t(message) }}</span
          ><button :aria-label="t('Закрыть сообщение')" @click="message = ''"><X :size="20" /></button>
        </div>
        <div v-else-if="info && screen !== 'scan' && screen !== 'identify'" class="feedback" role="status">
          <Check /><span>{{ t(info) }}</span>
        </div>
        <div class="workflow-body">
          <ReaderIdentification
            v-if="screen === 'identify'"
            :state="nfcState"
            :busy="busy || scansPending > 0"
            :message="message"
            @face="openFace"
            @dismiss="message = ''"
          />
          <FaceIdentification
            v-else-if="screen === 'face'"
            :terminal-test="nativeTerminalTest"
            :camera-active="faceCaptureActive && !overlayOpen"
            @card="openCard"
          />
          <section
            v-else-if="screen === 'reservation-confirm' && reader && reservationTitleId"
            class="reservation-confirmation"
          >
            <div class="reservation-icon"><BookOpen :size="42" /></div>
            <h2>{{ t('Поставить в очередь?') }}</h2>
            <p>{{ t('Запись будет создана для выбранного читателя после подтверждения.') }}</p>
            <div class="reservation-summary">
              <strong>{{ titleFor(reservationTitleId)?.name }}</strong
              ><span>{{ titleFor(reservationTitleId)?.author }}</span>
              <hr />
              <strong>{{ reader.name }}</strong
              ><span>{{ reader.group }}</span>
            </div>
          </section>
          <div v-else-if="issueScan" class="issue-scan-layout">
            <ReaderSummary
              :name="reader!.name"
              :group="reader!.group"
              :card="reader!.card"
              :count="readerCount"
              variant="scan"
              @loans="openLoans"
            />
            <div class="issue-scan-workspace">
              <div class="issue-scanner-column">
                <ScannerPanel
                  issue
                  :description="t('Поднесите штрихкод книги к сканеру или введите инвентарный номер вручную')"
                  @manual="openManual"
                  @isbn="openIsbn"
                />
              </div>
              <div class="issue-basket-column">
                <BookBasket :title="t('Список книг')" :count="count"
                  ><template v-if="basket.length"
                    ><div v-for="item in basket" :key="item.id">
                      <BookRow
                        :title="titleFor(item.titleId)?.name ?? ''"
                        :author="titleFor(item.titleId)?.author"
                        :code="codeFor(item)"
                        :mode="item.mode === 'COPY' ? t('Индивидуальный экземпляр') : t('Учёт количеством')"
                        :quantity="item.quantity"
                        @remove="removeItem(item.id)"
                      /></div></template
                  ><template #empty
                    ><div class="issue-basket-empty">
                      <BookOpen :stroke-width="1.5" />
                      <p>{{ t('Здесь появятся') }}<br />{{ t('отсканированные книги') }}</p>
                    </div></template
                  ></BookBasket
                >
              </div>
            </div>
          </div>
          <div v-else-if="acceptScan" class="accept-scan-layout">
            <ReaderSummary
              :name="reader!.name"
              :group="reader!.group"
              :card="reader!.card"
              :count="readerCount"
              variant="scan"
              @loans="openLoans"
            />
            <div class="accept-scan-workspace">
              <div class="accept-scanner-column">
                <ScannerPanel
                  accept
                  :title="t('Сканируйте книги для приёма')"
                  :description="t('Поднесите штрихкод книги к сканеру или введите инвентарный номер вручную')"
                  @manual="openManual"
                  @on-hand-search="openReturnSearch"
                />
              </div>
              <ReturnLoanList
                :items="returnLoans"
                :state="returnLoansState"
                @details="openReturnItem"
                @retry="retryReturnLoans"
              />
            </div>
          </div>
          <div v-else-if="screen === 'scan' && reader" class="scan-layout">
            <div class="scanner-column">
              <ScannerPanel @manual="openManual" />
              <p class="small muted scanner-note">
                {{ t('Поддерживаются штрихкоды экземпляров и ISBN изданий.') }}
              </p>
            </div>
            <div class="basket-column">
              <ReaderSummary
                v-if="reader"
                :name="reader.name"
                :group="reader.group"
                :count="readerCount"
                @change="changeReader"
              />
              <BookBasket :title="operation === 'issue' ? t('К выдаче') : t('К приёму')" :count="count"
                ><template v-if="basket.length"
                  ><div v-for="item in basket" :key="item.id">
                    <BookRow
                      :title="titleFor(item.titleId)?.name ?? ''"
                      :author="titleFor(item.titleId)?.author"
                      :code="codeFor(item)"
                      :mode="item.mode === 'COPY' ? t('Индивидуальный экземпляр') : t('Старый фонд')"
                      :quantity="item.quantity"
                      @remove="removeItem(item.id)"
                    /></div></template
                ><template #empty
                  ><div class="plain-empty">
                    <LibraryBig :stroke-width="1.5" />
                    <h3>{{ t('Здесь появятся книги') }}</h3>
                    <p>{{ t('Отсканируйте штрихкод или введите номер вручную.') }}</p>
                  </div></template
                ></BookBasket
              >
            </div>
          </div>
          <div v-else-if="screen === 'confirm'" class="confirm-layout">
            <ReaderSummary
              v-if="reader"
              :name="reader.name"
              :group="reader.group"
              :count="readerCount"
              :changeable="false"
            />
            <div class="review-list">
              <div class="review-title">
                <h3>{{ operation === 'issue' ? t('К выдаче') : t('К приёму') }} · {{ count }}</h3>
                <span>{{ t('Позиций:') }} {{ basket.length }}</span>
              </div>
              <div v-for="item in basket" :key="item.id">
                <BookRow
                  :title="titleFor(item.titleId)?.name ?? ''"
                  :author="titleFor(item.titleId)?.author"
                  :code="codeFor(item)"
                  :mode="item.mode === 'COPY' ? t('Индивидуальный экземпляр') : t('Старый фонд')"
                  :quantity="item.quantity"
                  :removable="false"
                />
              </div>
            </div>
          </div>
          <div v-else-if="screen === 'unknown'" class="success-content">
            <div class="success-mark unknown-mark"><Clock3 /></div>
            <h2>{{ t('Ответ ещё не получен') }}</h2>
            <p>
              {{ t('Операция могла выполниться. Проверьте результат,') }}<br />{{
                t('чтобы не оформить её повторно.')
              }}
            </p>
            <button class="btn primary" :disabled="busy || !server" @click="checkResult">
              {{ busy ? t('Проверяем…') : t('Проверить результат') }}</button
            ><span class="small muted">{{ t('Список сохранён. Повторная отправка заблокирована.') }}</span>
          </div>
        </div>
      </template>
    </main>
    <template #footer>
      <ActionBar
        v-if="(['scan', 'confirm'].includes(screen) || screen === 'reservation-confirm') && reader"
        :inert="overlayOpen"
        ><template v-if="screen === 'reservation-confirm'"
          ><div class="action-count">
            {{ t('Очередь на выбранное издание') }}<small>{{ t('Читатель:') }} {{ reader.name }}</small>
          </div></template
        ><template v-else-if="issueScan"
          ><button
            class="btn secondary clear-basket-button"
            :disabled="!count || busy"
            @click="requestClearBasket"
          >
            <X />{{ t('Очистить список') }}
          </button></template
        ><template v-else-if="acceptScan"
          ><button
            class="btn secondary clear-basket-button"
            :disabled="!count || busy"
            @click="requestClearBasket"
          >
            <X />{{ t('Сбросить отметки') }}
          </button></template
        ><template v-else
          ><div class="action-count">
            {{ count }} {{ operation === 'issue' ? t('к выдаче') : t('к приёму')
            }}<small v-if="!count">{{ t('Добавьте первую книгу') }}</small>
          </div></template
        ><template #right
          ><button
            v-if="screen === 'reservation-confirm'"
            class="btn primary"
            :disabled="busy || !server || sessionExpired"
            @click="submit"
          >
            {{ busy ? t('Оформляем…') : t('Поставить в очередь') }}<ArrowRight v-if="!busy" /></button
          ><button
            v-else-if="issueScan"
            class="btn primary issue-confirm-button"
            :disabled="!count || busy || !server || sessionExpired"
            @click="confirm"
          >
            {{ t('Перейти к подтверждению') }}<ArrowRight /></button
          ><button
            v-else-if="acceptScan"
            class="btn primary accept-confirm-button"
            :disabled="!count || busy || !server || sessionExpired"
            @click="confirm"
          >
            <span
              >{{ t('Перейти к подтверждению') }}<small>{{ t('К приёму:') }} {{ count }}</small></span
            ><ArrowRight /></button
          ><template v-else
            ><button
              v-if="screen === 'confirm'"
              class="btn secondary"
              :disabled="busy"
              @click="screen = 'scan'"
            >
              {{ t('Изменить список') }}</button
            ><button
              class="btn primary"
              :disabled="!count || busy || !server || sessionExpired"
              @click="screen === 'scan' ? confirm() : submit()"
            >
              {{
                busy
                  ? t('Выполняем…')
                  : screen === 'scan'
                    ? t('Продолжить')
                    : `${operation === 'issue' ? t('Выдать') : t('Принять')} ${count} ${count === 1 ? t('книгу') : count < 5 ? t('книги') : t('книг')}`
              }}<ArrowRight v-if="!busy" /></button></template></template
      ></ActionBar>
    </template>
    <div v-if="dialog" class="modal-backdrop" @click.self="closeDialog">
      <section
        class="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-heading"
        @keydown="trapFocus"
      >
        <div class="modal-header">
          <h2 id="dialog-heading">
            {{
              dialog === 'manual'
                ? manualKind === 'isbn'
                  ? t('Введите ISBN')
                  : t('Введите инвентарный номер')
                : dialog === 'title'
                  ? t('Найдено издание')
                  : dialog === 'leave'
                    ? t('Завершить без сохранения?')
                    : dialog === 'clear-basket'
                      ? acceptScan
                        ? t('Сбросить отметки?')
                        : t('Очистить список?')
                      : dialog === 'loans'
                        ? t('Книги на руках')
                        : dialog === 'return-search'
                          ? t('Найти книгу на руках')
                          : dialog === 'return-item'
                            ? t('Книга к приёму')
                            : t('Выбрать другого читателя?')
            }}
          </h2>
          <button class="btn" :aria-label="t('Закрыть')" @click="closeDialog"><X /></button>
        </div>
        <template v-if="dialog === 'manual'"
          ><p>
            {{
              manualKind === 'isbn'
                ? t('ISBN определяет издание. Выберите допустимый способ учёта на следующем шаге.')
                : t('Буквы, цифры, ведущие нули и разделители сохраняются.')
            }}
          </p>
          <TouchInput
            ref="scanInput"
            v-model="scanValue"
            :label="manualKind === 'isbn' ? 'ISBN' : t('Инвентарный номер')"
            :placeholder="
              manualKind === 'isbn' ? t('Например, 9786010123456') : t('Например, 000124 или KZ-0008')
            "
            @keyboard="keyboard = 'scan'"
            @enter="manualSubmit" />
          <div class="modal-actions">
            <button class="btn primary" :disabled="!scanValue.trim() || !server" @click="manualSubmit">
              {{ t('Найти книгу') }}<ArrowRight />
            </button></div
        ></template>
        <template v-else-if="dialog === 'title'">
          <template v-if="ambiguousTitles.length"
            ><p>{{ t('Код соответствует нескольким изданиям. Выберите нужное.') }}</p>
            <button v-for="t in ambiguousTitles" :key="t.id" class="loan-option" @click="chooseTitle(t)">
              <BookOpen /><span
                >{{ t.name }}<small>{{ t.author }}</small></span
              ><ArrowRight /></button
          ></template>
          <template v-else-if="titleChoice"
            ><div class="title-detail">
              <BookOpen />
              <div>
                <h3>{{ titleChoice.name }}</h3>
                <p>{{ titleChoice.author }}</p>
                <p class="small">ISBN {{ titleChoice.isbn }}</p>
              </div>
            </div>
            <p class="isbn-note">{{ t('ISBN обозначает издание, а не отдельную книгу.') }}</p>
            <button v-if="operation === 'issue'" class="btn secondary full" @click="openManual">
              <Keyboard />{{ t('Указать инвентарный номер') }}
            </button>
            <div class="modal-section">
              <h3>{{ t('Книги без индивидуального номера') }}</h3>
              <p class="small">
                {{
                  operation === 'issue'
                    ? t('Существующий старый фонд')
                    : t('Активная выдача выбранного читателя')
                }}
                {{ t('· доступно') }} {{ legacyLimit }}
              </p>
              <div v-if="legacyLimit" class="quantity-line">
                <span>{{ t('Количество') }}</span
                ><QuantityControl v-model="legacyQuantity" :max="legacyLimit" />
              </div>
              <p v-else class="feedback error">
                {{
                  operation === 'accept'
                    ? t('У этого читателя нет выдачи такого издания.')
                    : t('В старом фонде нет доступных книг этого издания.')
                }}
              </p>
            </div>
            <button class="btn primary" :disabled="!legacyLimit" @click="addLegacy">
              {{ operation === 'issue' ? t('Добавить из старого фонда') : t('Добавить к приёму')
              }}<ArrowRight />
            </button>
          </template>
        </template>
        <template v-else-if="dialog === 'return-search'"
          ><p>
            {{
              t('Выберите книгу из активных выдач читателя. Отметка появится только после явного действия.')
            }}
          </p>
          <div v-if="returnLoans.length" class="return-search-options">
            <button
              v-for="item in returnLoans"
              :key="item.id"
              class="loan-option"
              type="button"
              @click="chooseReturnLoan(item)"
            >
              <BookOpen :size="25" /><span
                >{{ item.title
                }}<small
                  >{{ item.code ? `Инв. № ${item.code}` : t('Учёт количеством') }} {{ t('· на руках:') }}
                  {{ item.quantity }}</small
                ></span
              ><ArrowRight />
            </button>
          </div>
          <p v-else>{{ t('У читателя нет книг на руках.') }}</p></template
        >
        <template v-else-if="dialog === 'return-item' && selectedReturnLoan"
          ><div class="title-detail">
            <BookOpen />
            <div>
              <h3>{{ selectedReturnLoan.title }}</h3>
              <p>
                {{
                  selectedReturnLoan.code
                    ? `Инвентарный номер ${selectedReturnLoan.code}`
                    : t('Книга учитывается количеством')
                }}
              </p>
              <p class="small">{{ t('На руках:') }} {{ selectedReturnLoan.quantity }}</p>
            </div>
          </div>
          <div v-if="selectedReturnLoan.mode === 'LEGACY_TITLE'" class="modal-section">
            <h3>{{ t('Количество к приёму') }}</h3>
            <QuantityControl v-model="returnQuantity" :max="selectedReturnLoan.quantity" />
          </div>
          <p v-else class="isbn-note">{{ t('Экземпляр с этим номером будет отмечен один раз.') }}</p>
          <div class="modal-actions">
            <button
              v-if="selectedReturnLoan.selectedQuantity"
              class="btn secondary"
              @click="
                setReturnLoanQuantity(selectedReturnLoan.id, 0);
                closeDialog();
              "
            >
              {{ t('Убрать из приёма') }}</button
            ><button
              v-if="!selectedReturnLoan.selectedQuantity || selectedReturnLoan.mode === 'LEGACY_TITLE'"
              class="btn primary"
              @click="
                setReturnLoanQuantity(
                  selectedReturnLoan.id,
                  selectedReturnLoan.mode === 'LEGACY_TITLE' ? returnQuantity : 1,
                );
                closeDialog();
              "
            >
              {{ selectedReturnLoan.selectedQuantity ? t('Сохранить количество') : t('Отметить к приёму')
              }}<Check />
            </button></div
        ></template>
        <template v-else-if="dialog === 'loans'"
          ><div v-if="loans.length" class="loan-options">
            <div v-for="loan in loans" :key="loan.id" class="loan-option">
              <BookOpen :size="24" /><span
                >{{ titleFor(loan.titleId)?.name
                }}<small>{{
                  loan.mode === 'COPY'
                    ? t('Индивидуальный экземпляр')
                    : `Учёт количеством · ${loan.quantity} шт.`
                }}</small></span
              >
            </div>
          </div>
          <p v-else>{{ t('У выбранного читателя сейчас нет книг на руках.') }}</p>
          <div class="modal-actions">
            <button class="btn primary" @click="closeDialog">{{ t('Понятно') }}</button>
          </div></template
        >
        <template v-else
          ><p>
            {{
              dialog === 'leave'
                ? t('Книги будут убраны только из текущего списка. Фонд и выдачи не изменятся.')
                : dialog === 'clear-basket'
                  ? acceptScan
                    ? t(
                        'Будут сняты только отметки к приёму. Книги на руках и выбранный читатель останутся на экране.',
                      )
                    : t('Текущий список книг будет очищен. Выбранный читатель сохранится.')
                  : t('Текущий список книг будет очищен. Затем можно выбрать другого читателя.')
            }}
          </p>
          <div class="modal-actions">
            <button class="btn secondary" @click="closeDialog">{{ t('Продолжить работу') }}</button
            ><button
              class="btn primary"
              @click="
                dialog === 'leave'
                  ? home()
                  : dialog === 'clear-basket'
                    ? acceptScan
                      ? resetReturnMarks()
                      : (clearBasket(), closeDialog())
                    : confirmChangeReader()
              "
            >
              {{
                dialog === 'leave'
                  ? t('Завершить сеанс')
                  : dialog === 'clear-basket'
                    ? acceptScan
                      ? t('Сбросить отметки')
                      : t('Очистить список')
                    : t('Очистить и выбрать')
              }}
            </button>
          </div></template
        >
        <p v-if="message" class="feedback error" role="alert">{{ t(message) }}</p>
      </section>
    </div>
    <div v-if="settings" class="modal-backdrop" @click.self="closeSettings">
      <section
        class="modal wide"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-heading"
        @keydown="trapFocus"
      >
        <div class="modal-header">
          <h2 id="settings-heading">{{ t('Настройки и диагностика') }}</h2>
          <button class="btn" :aria-label="t('Закрыть настройки')" @click="closeSettings"><X /></button>
        </div>
        <section class="language-settings" aria-labelledby="language-settings-title">
          <h3 id="language-settings-title">{{ t('Язык интерфейса') }}</h3>
          <div class="language-options">
            <button class="btn secondary" :class="{ active: locale === 'ru' }" @click="setLocale('ru')">
              {{ t('Русский') }}</button
            ><button class="btn secondary" :class="{ active: locale === 'kk' }" @click="setLocale('kk')">
              {{ t('Қазақша') }}
            </button>
          </div>
        </section>
        <p class="small">
          {{ t('EDUS · Библиотека. Состояние оборудования и API проверяется на установленном терминале.') }}
        </p>
        <p v-if="nativeUat" class="feedback">
          {{ t('Только тестовый режим · данные тестового рабочего пространства · Cloud не используется.') }}
        </p>
        <p v-if="nativeUat" class="small muted">{{ uatBuildLabel }}</p>
        <div class="edge-service-settings">
          <p v-if="localeSaveFailed" role="alert">
            {{ t('Язык не сохранён. Проверьте связь со службой и выберите язык повторно.') }}
          </p>
          <h3>{{ t('Состояние EDUS Library') }}</h3>
          <p>
            {{
              nativeUat
                ? t('Локальный UAT-режим. Cloud не используется.')
                : t(
                    'Локальная служба работает. Подробная настройка доступна только в EDUS Terminal Configurator.',
                  )
            }}
          </p>
          <p class="small muted">{{ uatBuildLabel }}</p>
        </div>
      </section>
    </div>
    <div v-if="wrongReader" class="modal-backdrop" @click.self="dismissWrongReader">
      <section
        class="modal wrong-reader-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="wrong-reader-heading"
        @keydown="trapFocus"
      >
        <div class="modal-header">
          <h2 id="wrong-reader-heading">{{ t('Книга другого читателя') }}</h2>
          <button class="btn" :aria-label="t('Закрыть предупреждение')" @click="dismissWrongReader">
            <X />
          </button>
        </div>
        <div class="wrong-book">
          <BookOpen :size="32" />
          <div>
            <h3>{{ wrongReader.title }}</h3>
            <span>{{ t('Код') }} {{ wrongReader.code }}</span>
          </div>
        </div>
        <p>{{ t('Книга числится за другим читателем:') }}</p>
        <div class="wrong-reader-owner">
          <strong>{{ wrongReader.name }}</strong
          ><span>{{ wrongReader.group }}</span>
        </div>
        <p>{{ t('Книга не добавлена к приёму. Текущий читатель и список сохранены.') }}</p>
        <div class="modal-actions">
          <button class="btn secondary" @click="switchFromWrongReader">{{ t('Сменить читателя') }}</button
          ><button class="btn primary" @click="dismissWrongReader">{{ t('Не добавлять книгу') }}</button>
        </div>
      </section>
    </div>
    <ContextHelp v-if="help" :screen="screen" :operation="operation" @close="closeHelp" />
    <div v-if="sessionExpired" class="modal-backdrop session-backdrop">
      <section
        class="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="session-heading"
        @keydown="trapFocus"
      >
        <h2 id="session-heading">{{ t('Сессия приостановлена') }}</h2>
        <p>{{ t('Текущий список сохранён. Подтвердите продолжение работы.') }}</p>
        <button
          class="btn primary"
          @click="
            sessionExpired = false;
            lastActivity = Date.now();
          "
        >
          {{ t('Продолжить сессию') }}
        </button>
      </section>
    </div>
    <TouchKeyboard
      v-if="keyboard && !help"
      v-model="keyboardValue"
      :label="keyboard === 'library-search' ? t('Поиск книги') : t('Код книги')"
      @close="keyboard = ''"
      @enter="
        keyboard === 'scan'
          ? manualSubmit()
          : keyboard === 'library-search'
            ? (librarySearchRef?.submit(), (keyboard = ''))
            : (keyboard = '')
      "
    />
  </TerminalShell>
</template>

<style scoped>
.screen-identify .topline {
  flex-shrink: 0;
}
.reservation-confirmation {
  width: min(640px, 100%);
  margin: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  text-align: center;
}
.reservation-icon {
  width: 88px;
  height: 88px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background: var(--blue-soft);
  color: var(--edus-blue);
}
.reservation-confirmation h2 {
  font-size: 34px;
}
.reservation-confirmation > p {
  font-size: 21px;
  color: var(--muted);
}
.reservation-summary {
  width: 100%;
  padding: 22px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}
.reservation-summary strong {
  font-size: 21px;
}
.reservation-summary span {
  color: var(--muted);
  font-size: 18px;
}
.reservation-summary hr {
  width: 100%;
  border: 0;
  border-top: 1px solid var(--line);
  margin: 10px 0;
}
.language-settings {
  margin: 12px 0 20px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--blue-soft);
}
.language-settings h3 {
  margin: 0;
  font-size: 22px;
}
.language-options {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 12px;
}
.language-options .btn {
  min-height: 56px;
}
.language-options .active {
  border-color: var(--edus-blue);
  background: var(--surface);
  box-shadow: inset 0 0 0 1px var(--edus-blue);
}
.service-access {
  margin: 20px 0;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.service-access h3 {
  margin: 0;
  font-size: 22px;
}
.service-access p {
  max-width: 760px;
  margin: 8px 0 14px;
  font-size: 18px;
  line-height: 1.42;
  color: var(--muted);
}
.service-access .btn {
  min-height: 56px;
}
.unknown-mark {
  background: var(--gold-soft);
  color: var(--warning);
}
.settings-actions {
  margin-top: 20px;
}
.edge-service-settings {
  margin: 18px 0;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.edge-service-settings h3 {
  margin: 0;
  font-size: 22px;
}
.edge-service-settings p {
  margin: 8px 0 0;
  font-size: 18px;
  line-height: 1.45;
  color: var(--muted);
}
</style>
