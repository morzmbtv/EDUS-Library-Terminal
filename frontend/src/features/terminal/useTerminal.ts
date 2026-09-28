import { computed, ref, watch } from 'vue';
import {
  DomainError,
  type CodeResult,
  type BasketItem,
  type Reader,
  type Title,
  type OperationResult,
} from '../../shared/types/terminalTypes.ts';
import { api, createLocalServiceApiClient } from '../../shared/api/LocalServiceApiClient.ts';

export type Operation = 'issue' | 'accept' | 'reserve';
export type Screen =
  | 'home'
  | 'library-search'
  | 'identify'
  | 'face'
  | 'scan'
  | 'confirm'
  | 'reservation-confirm'
  | 'success'
  | 'unknown';
export const adapter = await createLocalServiceApiClient();
export const screen = ref<Screen>('home');
export const operation = ref<Operation>('issue');
export const reader = ref<Reader | null>(null);
export const basket = ref<BasketItem[]>([]);
export const snapshot = ref(adapter.snapshot());
export const busy = ref(false);
export const scansPending = ref(0);
export const message = ref('');
export const info = ref('');
export const sessionExpired = ref(false);
export const nfcState = ref<'idle' | 'reading' | 'success' | 'error'>('idle');
export const faceCaptureActive = ref(false);
export const wrongReader = ref<{
  readerId: string;
  name: string;
  group: string;
  title: string;
  code: string;
} | null>(null);
export const scanValue = ref('');
export const titleChoice = ref<Title | null>(null);
export const ambiguousTitles = ref<Title[]>([]);
export const legacyQuantity = ref(1);
export const dialog = ref<
  | ''
  | 'manual'
  | 'title'
  | 'change-reader'
  | 'leave'
  | 'clear-basket'
  | 'loans'
  | 'return-item'
  | 'return-search'
>('');
export const reservationTitleId = ref<string | null>(null);
export const pendingCatalogueIssue = ref<{ titleId: string; copyId?: string } | null>(null);
export const result = ref<OperationResult | null>(null);
export const operationId = ref('');
export const server = computed(() => snapshot.value.connection.server);
export const internet = computed(() => snapshot.value.connection.internet);
export const count = computed(() => basket.value.reduce((n, x) => n + x.quantity, 0));
export const loans = computed(() =>
  reader.value ? snapshot.value.loans.filter((x) => x.readerId === reader.value!.id) : [],
);
export const readerCount = computed(() => loans.value.reduce((n, x) => n + x.quantity, 0));
export const operationName = computed(
  () => ({ issue: 'Выдача книг', accept: 'Приём книг', reserve: 'Очередь на книгу' })[operation.value],
);
export function titleFor(id: string) {
  return snapshot.value.titles.find((x) => x.id === id);
}
export function codeFor(item: BasketItem) {
  return item.copyId ? snapshot.value.copies.find((x) => x.id === item.copyId)?.code : undefined;
}
export function refresh() {
  snapshot.value = adapter.snapshot();
}
export async function refreshNative() {
  try {
    await adapter.refreshAsync?.();
    refresh();
  } catch (error) {
    snapshot.value = { ...snapshot.value, connection: { ...snapshot.value.connection, server: false } };
    throw error;
  }
}
export function clearFeedback() {
  message.value = '';
  info.value = '';
  wrongReader.value = null;
}
let workflowVersion = 0;
let pendingSubmission = false;
let basketReaderId: string | null = null;
const identificationTransitionPaused = ref(false);
type IdentificationAttempt = {
  version: number;
  operation: Operation;
  reader: Reader | null;
  ready: boolean;
  timer?: ReturnType<typeof setTimeout>;
  finishDelay?: () => void;
};
let activeIdentification: IdentificationAttempt | null = null;
let identificationOrigin: 'home' | 'library-search' = 'home';
function stopFaceCapture() {
  faceCaptureActive.value = false;
}
function identificationIsCurrent(attempt: IdentificationAttempt) {
  return (
    activeIdentification === attempt &&
    attempt.version === workflowVersion &&
    attempt.operation === operation.value &&
    screen.value === 'identify' &&
    !sessionExpired.value
  );
}
export function cancelIdentification() {
  if (!activeIdentification) return;
  const attempt = activeIdentification;
  activeIdentification = null;
  clearTimeout(attempt.timer);
  attempt.finishDelay?.();
  busy.value = false;
  nfcState.value = 'idle';
}
function invalidateWorkflow() {
  workflowVersion++;
  workflowChanged.value++;
  cancelIdentification();
}
function finishIdentification() {
  const attempt = activeIdentification;
  if (
    !attempt?.reader ||
    !attempt.ready ||
    identificationTransitionPaused.value ||
    !identificationIsCurrent(attempt)
  )
    return;
  activeIdentification = null;
  busy.value = false;
  chooseReader(attempt.reader);
}
export function setIdentificationTransitionPaused(paused: boolean) {
  identificationTransitionPaused.value = paused;
  if (!paused) finishIdentification();
}
watch(
  [screen, operation, sessionExpired],
  () => {
    if (activeIdentification && !identificationIsCurrent(activeIdentification)) cancelIdentification();
    if (screen.value !== 'face') stopFaceCapture();
  },
  { flush: 'sync' },
);
watch(
  basket,
  (items) => {
    if (!items.length) basketReaderId = null;
    else if (!basketReaderId) basketReaderId = reader.value?.id ?? null;
  },
  { deep: true, flush: 'sync' },
);
watch(
  [screen, reader, operation],
  () => {
    const known =
      reader.value && snapshot.value.readers.some((candidate) => candidate.id === reader.value?.id);
    if (['scan', 'confirm', 'reservation-confirm'].includes(screen.value) && !known) {
      screen.value = 'identify';
      dialog.value = '';
      nfcState.value = 'idle';
      message.value = 'Сначала определите читателя по карте или по лицу.';
    }
  },
  { flush: 'sync' },
);
function navigationLocked() {
  return screen.value === 'unknown' || pendingSubmission || (busy.value && screen.value !== 'identify');
}
function hasReaderContext(): boolean {
  if (!reader.value || !snapshot.value.readers.some((candidate) => candidate.id === reader.value?.id)) {
    reader.value = null;
    screen.value = 'identify';
    dialog.value = '';
    message.value = 'Сначала определите читателя по карте или по лицу.';
    return false;
  }
  if (basketReaderId && basketReaderId !== reader.value.id) {
    message.value = 'Список относится к другому читателю. Подтвердите очистку списка перед сменой читателя.';
    dialog.value = 'change-reader';
    return false;
  }
  return true;
}
function canEditBasket(): boolean {
  if (
    busy.value ||
    sessionExpired.value ||
    wrongReader.value ||
    screen.value === 'unknown' ||
    screen.value === 'success'
  )
    return false;
  if (!hasReaderContext()) return false;
  return screen.value === 'scan';
}
export function start(op: Operation, origin: 'home' | 'library-search' = 'home') {
  if (navigationLocked()) return;
  invalidateWorkflow();
  identificationOrigin = origin;
  stopFaceCapture();
  basket.value = [];
  operation.value = op;
  reader.value = null;
  result.value = null;
  operationId.value = '';
  reservationTitleId.value = null;
  pendingCatalogueIssue.value = null;
  clearFeedback();
  scanValue.value = '';
  titleChoice.value = null;
  ambiguousTitles.value = [];
  dialog.value = '';
  sessionExpired.value = false;
  nfcState.value = 'idle';
  screen.value = 'identify';
}
/** Opens the catalogue without carrying a reader or an operation draft into it. */
export function openLibrarySearch() {
  if (navigationLocked()) return;
  invalidateWorkflow();
  basket.value = [];
  reader.value = null;
  result.value = null;
  operationId.value = '';
  reservationTitleId.value = null;
  pendingCatalogueIssue.value = null;
  clearFeedback();
  scanValue.value = '';
  titleChoice.value = null;
  ambiguousTitles.value = [];
  dialog.value = '';
  sessionExpired.value = false;
  nfcState.value = 'idle';
  screen.value = 'library-search';
}
export function home() {
  if (navigationLocked()) return;
  invalidateWorkflow();
  screen.value = 'home';
  reader.value = null;
  basket.value = [];
  reservationTitleId.value = null;
  pendingCatalogueIssue.value = null;
  clearFeedback();
  dialog.value = '';
}
/** Starts the normal reader-first issue flow with a concrete available copy when one exists. */
export function beginCatalogueIssue(titleId: string, copyId?: string) {
  if (navigationLocked() || !snapshot.value.titles.some((title) => title.id === titleId)) return;
  start('issue', 'library-search');
  pendingCatalogueIssue.value = { titleId, copyId };
}
/** A reservation never mutates stock before the reader is explicitly identified. */
export function beginCatalogueReservation(titleId: string) {
  if (navigationLocked() || !snapshot.value.titles.some((title) => title.id === titleId)) return;
  start('reserve', 'library-search');
  reservationTitleId.value = titleId;
}
export function requestHome() {
  if (navigationLocked()) return;
  if (basket.value.length && screen.value !== 'success') dialog.value = 'leave';
  else home();
}
export function changeReader() {
  if (navigationLocked() || sessionExpired.value) return;
  clearFeedback();
  if (basket.value.length) dialog.value = 'change-reader';
  else {
    invalidateWorkflow();
    screen.value = 'identify';
    reader.value = null;
    nfcState.value = 'idle';
  }
}
export function confirmChangeReader() {
  if (navigationLocked() || sessionExpired.value) return;
  invalidateWorkflow();
  basket.value = [];
  reader.value = null;
  clearFeedback();
  dialog.value = '';
  screen.value = 'identify';
  nfcState.value = 'idle';
}
export function back() {
  if (navigationLocked()) return;
  clearFeedback();
  if (screen.value === 'success') {
    basket.value = [];
    operationId.value = '';
    result.value = null;
    if (operation.value === 'reserve') {
      reader.value = null;
      screen.value = 'library-search';
    } else screen.value = 'scan';
  } else if (screen.value === 'library-search') home();
  else if (screen.value === 'face') {
    stopFaceCapture();
    screen.value = 'identify';
  } else if (screen.value === 'identify') {
    if (identificationOrigin === 'library-search') {
      pendingCatalogueIssue.value = null;
      reservationTitleId.value = null;
      reader.value = null;
      screen.value = 'library-search';
    } else home();
  } else if (screen.value === 'confirm') screen.value = 'scan';
  else requestHome();
}
/** Face matching remains controlled until a protected camera provider is configured. */
export function openFace() {
  if (navigationLocked() || busy.value || sessionExpired.value || screen.value !== 'identify') return;
  cancelIdentification();
  clearFeedback();
  faceCaptureActive.value = true;
  screen.value = 'face';
}
/** Switches identification method without navigating back or resetting ISSUE/RETURN context. */
export function openCard() {
  if (navigationLocked() || screen.value !== 'face') return;
  stopFaceCapture();
  screen.value = 'identify';
  nfcState.value = 'idle';
}
export function chooseReader(r: Reader) {
  if (sessionExpired.value || navigationLocked() || !['identify', 'scan'].includes(screen.value)) return;
  const known = snapshot.value.readers.find((candidate) => candidate.id === r.id);
  if (!known) {
    message.value = 'Читатель не найден. Повторите поиск.';
    return;
  }
  if (basket.value.length && (reader.value?.id ?? basketReaderId) !== known.id) {
    message.value = 'Текущий список будет очищен только после подтверждения смены читателя.';
    dialog.value = 'change-reader';
    return;
  }
  if (
    operation.value === 'accept' &&
    basket.value.some((item) => {
      const loan = snapshot.value.loans.find((candidate) => candidate.id === item.loanId);
      return loan && loan.readerId !== r.id;
    })
  ) {
    message.value =
      'В списке есть книги другого читателя. Завершите их приём или уберите их из списка перед выбором читателя.';
    return;
  }
  reader.value = known;
  clearFeedback();
  if (operation.value === 'reserve') {
    if (
      !reservationTitleId.value ||
      !snapshot.value.titles.some((title) => title.id === reservationTitleId.value)
    ) {
      message.value = 'Не удалось определить издание для очереди. Выберите книгу ещё раз.';
      screen.value = 'library-search';
      return;
    }
    operationId.value = crypto.randomUUID();
    screen.value = 'reservation-confirm';
    return;
  }
  screen.value = 'scan';
  const intent = pendingCatalogueIssue.value;
  pendingCatalogueIssue.value = null;
  if (operation.value === 'issue' && intent) {
    const title = titleFor(intent.titleId);
    if (!title) return;
    if (intent.copyId)
      addItem({
        id: `catalogue-${intent.copyId}`,
        titleId: title.id,
        copyId: intent.copyId,
        quantity: 1,
        mode: 'COPY',
      });
    else {
      void chooseTitle(title);
      info.value = 'Выберите количество книги из старого фонда.';
    }
  }
}
export async function identify(card: string) {
  if (
    busy.value ||
    activeIdentification ||
    identificationTransitionPaused.value ||
    sessionExpired.value ||
    screen.value !== 'identify'
  )
    return;
  const attempt: IdentificationAttempt = {
    version: workflowVersion,
    operation: operation.value,
    reader: null,
    ready: false,
  };
  activeIdentification = attempt;
  busy.value = true;
  nfcState.value = 'reading';
  clearFeedback();
  try {
    const r = await adapter.identifyCard(card);
    refresh();
    if (!identificationIsCurrent(attempt)) return;
    attempt.reader = r;
    nfcState.value = 'success';
    await new Promise<void>((resolve) => {
      attempt.finishDelay = resolve;
      attempt.timer = setTimeout(() => {
        attempt.timer = undefined;
        attempt.finishDelay = undefined;
        resolve();
      }, 400);
    });
    if (identificationIsCurrent(attempt)) {
      attempt.ready = true;
      finishIdentification();
    }
  } catch (e) {
    if (identificationIsCurrent(attempt)) {
      nfcState.value = 'error';
      message.value = errorText(e);
    }
  } finally {
    if (activeIdentification === attempt) {
      busy.value = false;
      if (!attempt.reader) activeIdentification = null;
    }
  }
}
export function errorText(e: unknown) {
  return e instanceof Error ? e.message : 'Операция не выполнена. Повторите попытку.';
}
export function addItem(item: BasketItem) {
  if (!canEditBasket()) return false;
  if (operation.value === 'accept') {
    const loan = snapshot.value.loans.find((candidate) => candidate.id === item.loanId);
    if (!loan || loan.readerId !== reader.value!.id) {
      message.value = 'Эта выдача не относится к выбранному читателю.';
      return false;
    }
  }
  if (basket.value.some((x) => x.id === item.id || (item.copyId && x.copyId === item.copyId))) {
    message.value =
      operation.value === 'accept'
        ? 'Эта книга уже отмечена к приёму.'
        : 'Эта книга уже в списке. Повторный скан не добавляет ещё один экземпляр.';
    return false;
  }
  basket.value.push(item);
  info.value = 'Книга добавлена в список';
  return true;
}
let scanQueue: Promise<void> = Promise.resolve();
const workflowChanged = ref(0);
/** Already accepted HID scans wait while a dialog is visible; they are never silently dropped. */
function awaitScanReady(version: number, readerId: string | undefined): Promise<boolean> {
  const current = () =>
    version === workflowVersion &&
    readerId === reader.value?.id &&
    screen.value === 'scan' &&
    !sessionExpired.value;
  const ready = () =>
    !dialog.value && !wrongReader.value && !identificationTransitionPaused.value && !busy.value;
  if (!current()) return Promise.resolve(false);
  if (ready()) return Promise.resolve(true);
  return new Promise((resolve) => {
    const stop = watch(
      [
        dialog,
        wrongReader,
        identificationTransitionPaused,
        busy,
        reader,
        screen,
        sessionExpired,
        workflowChanged,
      ],
      () => {
        if (!current() || ready()) {
          stop();
          resolve(current());
        }
      },
      { flush: 'sync' },
    );
  });
}
export function scan(code: string): void | Promise<void> {
  if (wrongReader.value || !canEditBasket() || identificationTransitionPaused.value) return;
  clearFeedback();
  if (!server.value) {
    message.value = 'Локальная база недоступна. Список сохранён.';
    return;
  }
  if (!code.trim()) {
    message.value = 'Введите код книги.';
    return;
  }
  const version = workflowVersion,
    readerId = reader.value?.id;
  scansPending.value++;
  const resolve = adapter.resolveCodeAsync;
  scanQueue = scanQueue
    .then(async () => {
      if (!(await awaitScanReady(version, readerId))) return;
      const found = await resolve(code.trim());
      if (await awaitScanReady(version, readerId)) applyScan(found);
    })
    .catch((error: unknown) => {
      if (version === workflowVersion) message.value = errorText(error);
    })
    .finally(() => {
      scansPending.value--;
    });
  return scanQueue;
}
function applyScan(found: CodeResult) {
  scanValue.value = '';
  if (found.kind === 'not-found') {
    message.value = 'Код не найден. Проверьте номер или добавьте книгу в фонд.';
    return;
  }
  if (found.kind === 'ambiguous') {
    ambiguousTitles.value = [...found.titles];
    dialog.value = 'title';
    titleChoice.value = null;
    return;
  }
  if (found.kind === 'title') {
    chooseTitle(found.title);
    return;
  }
  if (operation.value === 'issue') {
    if (found.loan) {
      message.value = 'Эта книга уже выдана. Сначала оформите её приём.';
      return;
    }
    addItem({ id: found.copy.id, titleId: found.title.id, copyId: found.copy.id, quantity: 1, mode: 'COPY' });
  } else {
    if (!found.loan) {
      message.value = 'У этой книги нет активной выдачи. Возможно, её уже приняли.';
      return;
    }
    if (found.loan.readerId !== reader.value!.id) {
      const owner = snapshot.value.readers.find((candidate) => candidate.id === found.loan!.readerId);
      wrongReader.value = {
        readerId: found.loan.readerId,
        name: owner?.name ?? 'Другой читатель',
        group: owner?.group ?? '',
        title: found.title.name,
        code: found.copy.code,
      };
      message.value = `Эта книга выдана другому читателю${owner ? `: ${owner.name}` : ''}. Выберите его для приёма этой книги.`;
      return;
    }
    addItem({
      id: found.loan.id,
      titleId: found.title.id,
      copyId: found.copy.id,
      quantity: 1,
      mode: 'COPY',
      loanId: found.loan.id,
    });
  }
  dialog.value = '';
}
const serverLegacyAvailable = ref(0);
export async function chooseTitle(t: Title) {
  if (!canEditBasket()) return;
  ambiguousTitles.value = [];
  titleChoice.value = t;
  legacyQuantity.value = 1;
  dialog.value = 'title';
  serverLegacyAvailable.value = 0;
  if (operation.value === 'issue') {
    try {
      const availability = await api.getBookAvailability(t.id);
      if (titleChoice.value?.id === t.id) serverLegacyAvailable.value = availability.legacyAvailable;
    } catch (error) {
      message.value = errorText(error);
    }
  }
}
export const legacyLimit = computed(() => {
  if (!titleChoice.value) return 0;
  if (operation.value === 'accept')
    return loans.value
      .filter((l) => l.titleId === titleChoice.value!.id && l.mode === 'LEGACY_TITLE')
      .reduce((n, l) => {
        const selected = basket.value.find((item) => item.loanId === l.id)?.quantity ?? 0;
        return n + Math.max(0, l.quantity - selected);
      }, 0);
  return serverLegacyAvailable.value;
});
export function addLegacy() {
  if (!canEditBasket() || !titleChoice.value) return;
  const t = titleChoice.value;
  if (operation.value === 'accept') {
    const matching = loans.value.filter((x) => x.titleId === t.id && x.mode === 'LEGACY_TITLE');
    if (!matching.length) {
      message.value = 'У выбранного читателя нет выдачи этого издания из старого фонда.';
      return;
    }
    if (
      !Number.isSafeInteger(legacyQuantity.value) ||
      legacyQuantity.value < 1 ||
      legacyQuantity.value > legacyLimit.value
    ) {
      message.value = 'Укажите количество в пределах ещё не выбранных книг на руках.';
      return;
    }
    let remaining = legacyQuantity.value;
    const updated = basket.value.map((item) => ({ ...item }));
    for (const loan of matching) {
      const selected = updated.find((item) => item.loanId === loan.id);
      const quantity = Math.min(remaining, Math.max(0, loan.quantity - (selected?.quantity ?? 0)));
      if (!quantity) continue;
      if (selected)
        updated[updated.indexOf(selected)] = { ...selected, quantity: selected.quantity + quantity };
      else updated.push({ id: loan.id, titleId: t.id, quantity, mode: 'LEGACY_TITLE', loanId: loan.id });
      remaining -= quantity;
      if (!remaining) break;
    }
    basket.value = updated;
    message.value = '';
    info.value = 'Книги добавлены к приёму';
    dialog.value = '';
  } else {
    if (legacyQuantity.value > legacyLimit.value) {
      message.value = 'Недостаточно книг в старом фонде.';
      return;
    }
    if (
      addItem({ id: `legacy-${t.id}`, titleId: t.id, quantity: legacyQuantity.value, mode: 'LEGACY_TITLE' })
    )
      dialog.value = '';
  }
}
export function addLoan(id: string) {
  if (operation.value !== 'accept' || !canEditBasket()) return;
  const l = loans.value.find((x) => x.id === id);
  if (!l) return;
  const selected = basket.value.find((item) => item.loanId === l.id);
  if (selected) {
    if (selected.quantity < l.quantity) {
      basket.value = basket.value.map((item) =>
        item.loanId === l.id ? { ...item, quantity: l.quantity } : item,
      );
      clearFeedback();
      info.value = 'Все книги этой выдачи добавлены к приёму';
    }
    return;
  }
  clearFeedback();
  addItem({
    id: l.id,
    titleId: l.titleId,
    copyId: l.copyId,
    quantity: l.quantity,
    mode: l.mode,
    loanId: l.id,
  });
}
export function allLoans() {
  if (operation.value !== 'accept' || !canEditBasket()) return;
  for (const l of loans.value) addLoan(l.id);
}
export function removeItem(id: string) {
  if (!canEditBasket()) return;
  basket.value = basket.value.filter((x) => x.id !== id);
  clearFeedback();
}
/**
 * Updates only the current return draft.  Active loans stay untouched until the
 * shared confirmation transaction succeeds, so the on-hand counter remains
 * truthful while an operator reviews the return.
 */
export function setReturnLoanQuantity(loanId: string, quantity: number) {
  if (operation.value !== 'accept' || !canEditBasket()) return false;
  const loan = loans.value.find((candidate) => candidate.id === loanId);
  if (!loan) {
    message.value = 'Эта выдача больше не доступна. Обновите список книг.';
    return false;
  }
  if (!Number.isSafeInteger(quantity) || quantity < 0 || quantity > loan.quantity) {
    message.value = 'Укажите количество в пределах книг на руках.';
    return false;
  }
  if (loan.mode === 'COPY' && quantity > 1) {
    message.value = 'Для экземпляра с номером можно отметить только одну книгу.';
    return false;
  }
  const existing = basket.value.find((item) => item.loanId === loanId);
  if (quantity === 0) {
    if (existing) basket.value = basket.value.filter((item) => item.loanId !== loanId);
    clearFeedback();
    return true;
  }
  const item: BasketItem = {
    id: loan.id,
    titleId: loan.titleId,
    copyId: loan.copyId,
    quantity,
    mode: loan.mode,
    loanId: loan.id,
  };
  basket.value = existing
    ? basket.value.map((candidate) => (candidate.loanId === loanId ? item : candidate))
    : [...basket.value, item];
  clearFeedback();
  info.value = 'Книга отмечена к приёму';
  return true;
}
export function clearBasket() {
  if (!canEditBasket() || !basket.value.length) return false;
  basket.value = [];
  clearFeedback();
  return true;
}
export function confirm() {
  if (scansPending.value === 0 && canEditBasket() && count.value) {
    screen.value = 'confirm';
    clearFeedback();
    operationId.value = crypto.randomUUID();
  }
}
export async function submit() {
  if (busy.value || sessionExpired.value || screen.value === 'unknown' || screen.value === 'success') return;
  const reserving = operation.value === 'reserve';
  if (
    !hasReaderContext() ||
    !operationId.value ||
    (reserving
      ? screen.value !== 'reservation-confirm' || !reservationTitleId.value
      : screen.value !== 'confirm' || !count.value)
  )
    return;
  busy.value = true;
  pendingSubmission = true;
  clearFeedback();
  try {
    if (operation.value === 'issue' && reader.value)
      result.value = await adapter.issue(reader.value.id, basket.value, operationId.value);
    else if (operation.value === 'accept' && reader.value)
      result.value = await adapter.accept(reader.value.id, basket.value, operationId.value);
    else if (operation.value === 'reserve' && reader.value && reservationTitleId.value)
      result.value = await adapter.reserve(reader.value.id, reservationTitleId.value, operationId.value);
    else throw new Error('Выберите читателя.');
    refresh();
    screen.value = 'success';
  } catch (e) {
    if (e instanceof DomainError && e.code === 'UNKNOWN') screen.value = 'unknown';
    else {
      message.value = errorText(e);
    }
  } finally {
    busy.value = false;
    pendingSubmission = false;
  }
}
export async function checkResult() {
  if (busy.value || sessionExpired.value || screen.value !== 'unknown' || !operationId.value) return;
  busy.value = true;
  clearFeedback();
  try {
    const r = await adapter.checkOperation(operationId.value);
    if (r) {
      result.value = r;
      operation.value = r.type;
      refresh();
      screen.value = 'success';
    } else message.value = 'Результат пока не найден. Повторите проверку после восстановления связи.';
  } catch (e) {
    message.value = errorText(e);
  } finally {
    busy.value = false;
  }
}
