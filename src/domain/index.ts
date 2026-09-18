/** Isolated demonstration adapter. It never calls production services or hardware. */
export * from './terminalTypes.ts'
import { DomainError, type AccountingMode, type BasketItem, type CodeResult, type Copy, type DomainErrorCode, type Loan, type OperationResult, type Reader, type Snapshot, type TerminalAdapter, type Title, type TitleInput } from './terminalTypes.ts'

export const DEMO_CODES = Object.freeze({
  readerCard: 'EDUS-1001',
  secondReaderCard: 'EDUS-1002',
  availableCopy: '000124',
  secondAvailableCopy: '000125',
  issuedCopy: '000123',
  secondIssuedCopy: 'KZ-0007',
  legacyIsbn: '9786010123456',
  ambiguous: 'DEMO-MULTI',
  unknown: 'BOOK-NOT-FOUND',
})

export const SEED_READERS: readonly Reader[] = [
  { id: 'reader-1', name: 'Сәрсенова Айша Ерланқызы', group: '7 «А» класс', card: 'EDUS-1001' },
  { id: 'reader-2', name: 'Нұрланұлы Әлихан', group: '8 «Б» класс', card: 'EDUS-1002' },
  { id: 'reader-3', name: 'Ахметова Динара Сергеевна', group: 'Учитель казахского языка', card: 'EDUS-1003' },
  { id: 'reader-4', name: 'Сәрсенова Әсем Ерланқызы', group: '5 «А» класс', card: 'EDUS-1004' },
]

export const SEED_TITLES: readonly Title[] = [
  { id: 'title-1', name: 'Алгебра. 7 класс', author: 'А. Е. Әбілқасымова, Т. П. Кучер', isbn: '9786013411239', publisher: 'Мектеп', year: 2023, language: 'Русский', subject: 'Математика', grade: '7' },
  { id: 'title-2', name: 'Қазақ тілі. 7-сынып', author: 'Г. Қосымова, Р. Рахметова', isbn: '9786013411246', publisher: 'Атамұра', year: 2023, language: 'Қазақша', subject: 'Казахский язык', grade: '7' },
  { id: 'title-3', name: 'Қазақстан тарихы. 8-сынып', author: 'З. Е. Қабылдинов, А. Т. Қайыпбаева', isbn: '9786010123456', publisher: 'Атамұра', year: 2022, language: 'Қазақша', subject: 'История Казахстана', grade: '8' },
  { id: 'title-4', name: 'Абай жолы. Бірінші кітап', author: 'Мұхтар Әуезов', isbn: '9786012710043', publisher: 'Жазушы', year: 2021, language: 'Қазақша', subject: 'Литература', grade: '10–11' },
  { id: 'title-5', name: 'Естествознание. 5 класс', author: 'А. А. Плешаков', isbn: '', publisher: 'Алматыкітап', year: 2020, language: 'Русский', subject: 'Естествознание', grade: '5' },
]

export const SEED_COPIES: readonly Copy[] = [
  { id: 'copy-1', titleId: 'title-1', code: '000123' },
  { id: 'copy-2', titleId: 'title-1', code: '000124' },
  { id: 'copy-3', titleId: 'title-2', code: '000125' },
  { id: 'copy-4', titleId: 'title-4', code: 'KZ-0007' },
  { id: 'copy-5', titleId: 'title-4', code: 'KZ-0008' },
]

export const SEED_LOANS: readonly Loan[] = [
  { id: 'loan-1', readerId: 'reader-1', titleId: 'title-1', copyId: 'copy-1', quantity: 1, mode: 'COPY' },
  { id: 'loan-2', readerId: 'reader-1', titleId: 'title-3', quantity: 2, mode: 'LEGACY_TITLE' },
  { id: 'loan-3', readerId: 'reader-2', titleId: 'title-4', copyId: 'copy-4', quantity: 1, mode: 'COPY' },
  { id: 'loan-4', readerId: 'reader-2', titleId: 'title-3', quantity: 1, mode: 'LEGACY_TITLE' },
]

type StoredOperation = { fingerprint: string; result: OperationResult }
interface DataState {
  readers: Reader[]
  titles: Title[]
  copies: Copy[]
  loans: Loan[]
  legacyStock: Record<string, number>
  operations: Record<string, StoredOperation>
  /** Local-only mappings made by the terminal hardware test panel. */
  cardBindings: Record<string, string>
  scanBindings: Record<string, { kind: 'copy' | 'title'; id: string }>
  sequence: number
}

const STORAGE_KEY = 'edus-library-demo-v1'
const clone = <T>(value: T): T => structuredClone(value)
const normalizedCode = (value: string): string => value.trim().toLocaleUpperCase('ru')
const normalizedIsbn = (value: string): string => value.replace(/[\s-]/g, '').toUpperCase()
function fail(code: DomainErrorCode, message: string): never { throw new DomainError(code, message) }

/** Copy DTO fields explicitly: UI frameworks may wrap input objects in proxies. */
function basketInput(items: readonly BasketItem[]): BasketItem[] {
  return Array.from(items, (item) => ({
    id: item.id, titleId: item.titleId, copyId: item.copyId,
    quantity: item.quantity, mode: item.mode, loanId: item.loanId,
  }))
}

function titleInput(title: TitleInput): TitleInput {
  if (typeof title === 'string') return title
  return {
    name: title.name, author: title.author, isbn: title.isbn, publisher: title.publisher,
    year: title.year, language: title.language, subject: title.subject, grade: title.grade,
  }
}

function seedState(): DataState {
  return clone({
    readers: [...SEED_READERS], titles: [...SEED_TITLES], copies: [...SEED_COPIES], loans: [...SEED_LOANS],
    legacyStock: { 'title-1': 12, 'title-2': 15, 'title-3': 18, 'title-4': 4, 'title-5': 10 },
    operations: {}, cardBindings: {}, scanBindings: {}, sequence: 100,
  })
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function strings(value: Record<string, unknown>, keys: string[]): boolean {
  return keys.every((key) => typeof value[key] === 'string')
}

/** Reject stale/corrupt browser data instead of trusting a JSON type assertion. */
function validStoredState(value: unknown): value is DataState {
  if (!isRecord(value) || !Array.isArray(value.readers) || !Array.isArray(value.titles)
    || !Array.isArray(value.copies) || !Array.isArray(value.loans) || !isRecord(value.legacyStock)
    || !isRecord(value.operations) || !isRecord(value.cardBindings) || !isRecord(value.scanBindings)
    || !Number.isSafeInteger(value.sequence) || Number(value.sequence) < 100) return false
  if (!value.readers.every((r: unknown) => isRecord(r) && strings(r, ['id', 'name', 'group', 'card']))) return false
  if (!value.titles.every((t: unknown) => isRecord(t) && strings(t, ['id', 'name', 'author', 'isbn', 'publisher', 'language', 'subject', 'grade']) && Number.isInteger(t.year))) return false
  const titles = new Set(value.titles.map((t: Title) => t.id))
  const readers = new Set(value.readers.map((r: Reader) => r.id))
  if (!value.copies.every((c: unknown) => isRecord(c) && strings(c, ['id', 'titleId', 'code']) && titles.has(String(c.titleId)))) return false
  const copies = new Map(value.copies.map((c: Copy) => [c.id, c]))
  const copyCodes = new Set(value.copies.map((c: Copy) => normalizedCode(c.code)))
  if (copyCodes.size !== value.copies.length || copies.size !== value.copies.length) return false
  const usedCopies = new Set<string>()
  for (const loan of value.loans) {
    if (!isRecord(loan) || !strings(loan, ['id', 'readerId', 'titleId']) || !readers.has(String(loan.readerId))
      || !titles.has(String(loan.titleId)) || !Number.isSafeInteger(loan.quantity) || Number(loan.quantity) < 1) return false
    if (loan.mode === 'COPY') {
      const copy = typeof loan.copyId === 'string' ? copies.get(loan.copyId) : undefined
      if (!copy || copy.titleId !== loan.titleId || loan.quantity !== 1 || usedCopies.has(copy.id)) return false
      usedCopies.add(copy.id)
    } else if (loan.mode !== 'LEGACY_TITLE' || loan.copyId !== undefined) return false
  }
  if (!Object.entries(value.legacyStock).every(([id, count]) => titles.has(id) && Number.isSafeInteger(count) && Number(count) >= 0)) return false
  if (!Object.entries(value.cardBindings).every(([code, readerId]) => Boolean(normalizedCode(code)) && typeof readerId === 'string' && readers.has(readerId))) return false
  if (!Object.entries(value.scanBindings).every(([code, binding]) => Boolean(normalizedCode(code)) && isRecord(binding)
    && (binding.kind === 'copy' || binding.kind === 'title') && typeof binding.id === 'string'
    && (binding.kind === 'copy' ? copies.has(binding.id) : titles.has(binding.id)))) return false
  for (const titleId of titles) {
    const issued = value.loans.reduce((sum: number, loan: Loan) => sum + (loan.titleId === titleId && loan.mode === 'LEGACY_TITLE' ? loan.quantity : 0), 0)
    if (issued > Number(value.legacyStock[titleId] ?? 0)) return false
  }
  return Object.values(value.operations).every((entry) => {
    if (!isRecord(entry) || typeof entry.fingerprint !== 'string' || !isRecord(entry.result)) return false
    const result = entry.resul
    return typeof result.operationId === 'string' && ['issue', 'accept', 'register'].includes(String(result.type))
      && Number.isSafeInteger(result.quantity) && Number(result.quantity) > 0
      && Array.isArray(result.copyIds) && result.copyIds.every((id: unknown) => typeof id === 'string')
      && Array.isArray(result.loanIds) && result.loanIds.every((id: unknown) => typeof id === 'string')
  })
}

function quantityValid(quantity: number): void {
  if (!Number.isSafeInteger(quantity) || quantity < 1 || quantity > 999) {
    fail('INVALID_INPUT', 'Укажите целое количество от 1 до 999.')
  }
}

function validateBasket(state: DataState, items: readonly BasketItem[]): void {
  if (!items.length || items.length > 200) fail('INVALID_INPUT', 'Добавьте от 1 до 200 позиций.')
  const copyIds = new Set<string>()
  const legacyIds = new Set<string>()
  const itemIds = new Set<string>()
  for (const item of items) {
    quantityValid(item.quantity)
    if (!item.id || itemIds.has(item.id)) fail('INVALID_INPUT', 'В списке есть повторяющаяся позиция.')
    itemIds.add(item.id)
    if (!state.titles.some((title) => title.id === item.titleId)) fail('NOT_FOUND', 'Издание не найдено.')
    if (item.mode === 'COPY') {
      const copy = state.copies.find((candidate) => candidate.id === item.copyId)
      if (!copy || copy.titleId !== item.titleId) fail('NOT_FOUND', 'Экземпляр не найден у этого издания.')
      if (item.quantity !== 1) fail('INVALID_INPUT', 'Индивидуальный экземпляр учитывается по одной книге.')
      if (copyIds.has(copy.id)) fail('DUPLICATE_COPY', 'Этот экземпляр уже добавлен в список.')
      copyIds.add(copy.id)
    } else if (item.mode === 'LEGACY_TITLE') {
      if (item.copyId !== undefined) fail('INVALID_INPUT', 'Для учёта количеством не нужен номер экземпляра.')
      const key = item.loanId ?? item.titleId
      if (legacyIds.has(key)) fail('INVALID_INPUT', 'Измените количество в существующей строке.')
      legacyIds.add(key)
    } else fail('INVALID_INPUT', 'Выберите способ учёта книги.')
  }
}

export interface TerminalTestAdapter extends TerminalAdapter {
  bindTestCard(readerId: string, rawCode: string): void
  bindTestCode(kind: 'copy' | 'title', id: string, rawCode: string): void
  resetTestData(): void
  setNextDelay(delayMs: number): void
}

export function createMockAdapter(options: {
  storage?: Storage
  delay?: number
  storageKey?: string
  storageVersion?: number
  /** Terminal test must reject a mutation when local storage cannot record it. */
  requirePersistence?: boolean
} = {}): TerminalTestAdapter {
  const storageKey = options.storageKey ?? STORAGE_KEY
  const storageVersion = options.storageVersion ?? 1
  let state = seedState()
  let persistence: Snapshot['persistence'] = options.storage ? 'local-storage' : 'memory'
  let connection = { server: true, internet: true }
  let nextOutcome: 'success' | 'unknown' | 'error' = 'success'
  let nextDelayMs: number | undefined
  const delay = Math.max(0, options.delay ?? 450)

  if (options.storage) {
    try {
      const raw = options.storage.getItem(storageKey)
      if (raw) {
        const parsed: unknown = JSON.parse(raw)
        if (isRecord(parsed) && parsed.version === storageVersion && validStoredState(parsed.data)) state = clone(parsed.data)
      }
    } catch { persistence = 'memory' }
  }

  const pause = async (): Promise<void> => {
    const wait = nextDelayMs ?? delay
    nextDelayMs = undefined
    if (wait) await new Promise<void>((resolve) => setTimeout(resolve, wait))
  }
  const requireServer = (): void => {
    if (!connection.server) fail('SERVER_OFFLINE', 'Нет связи с локальным сервером. Восстановите соединение и повторите.')
  }
  const save = (next: DataState): void => {
    if (!options.storage) {
      if (options.requirePersistence) fail('PERSISTENCE_UNAVAILABLE', 'Тестовые данные нельзя сохранить: локальное хранилище недоступно.')
      return
    }
    try {
      options.storage.setItem(storageKey, JSON.stringify({ version: storageVersion, data: next }))
      persistence = 'local-storage'
    } catch {
      persistence = 'memory'
      if (options.requirePersistence) fail('PERSISTENCE_UNAVAILABLE', 'Тестовые данные не сохранены. Проверьте доступность локального хранилища и повторите операцию.')
    }
  }
  const nextId = (draft: DataState, prefix: string): string => `${prefix}-${++draft.sequence}`

  async function mutate(
    operationId: string,
    fingerprint: string,
    apply: (draft: DataState) => OperationResult,
  ): Promise<OperationResult> {
    if (!operationId.trim() || operationId.length > 200 || ['__proto__', 'constructor', 'prototype'].includes(operationId)) {
      fail('INVALID_INPUT', 'Не указан корректный идентификатор операции.')
    }
    await pause()
    requireServer()
    const previous = Object.hasOwn(state.operations, operationId) ? state.operations[operationId] : undefined
    if (previous) {
      if (previous.fingerprint !== fingerprint) fail('OPERATION_CONFLICT', 'Этот идентификатор уже использован для другой операции.')
      return clone(previous.result)
    }
    const outcome = nextOutcome
    nextOutcome = 'success'
    if (outcome === 'error') fail('SIMULATED_ERROR', 'Операция не выполнена. Данные сохранены, попробуйте ещё раз.')
    const draft = clone(state)
    const result = apply(draft)
    draft.operations[operationId] = { fingerprint, result }
    save(draft)
    state = draf
    if (outcome === 'unknown') {
      throw new DomainError('UNKNOWN', 'Ответ не получен. Проверьте результат операции перед повторной отправкой.', operationId)
    }
    return clone(result)
  }

  const adapter: TerminalTestAdapter = {
    snapshot(): Snapshot {
      return clone({ readers: state.readers, titles: state.titles, copies: state.copies, loans: state.loans, legacyStock: state.legacyStock, connection, persistence })
    },

    searchReaders(query: string): Reader[] {
      const words = query.trim().toLocaleLowerCase('ru').split(/\s+/).filter(Boolean)
      return clone(state.readers.filter((reader) => {
        const haystack = `${reader.name} ${reader.card} ${reader.group}`.toLocaleLowerCase('ru')
        return words.every((word) => haystack.includes(word))
      }))
    },

    searchTitles(query: string): Title[] {
      const words = query.trim().toLocaleLowerCase('ru').split(/\s+/).filter(Boolean)
      const isbn = normalizedIsbn(query)
      return clone(state.titles.filter((title) => {
        const haystack = `${title.name} ${title.author} ${title.isbn} ${title.subject}`.toLocaleLowerCase('ru')
        return words.every((word) => haystack.includes(word)) || Boolean(isbn && title.isbn && normalizedIsbn(title.isbn).includes(isbn))
      }))
    },

    resolveCode(code: string): CodeResult {
      const normalized = normalizedCode(code)
      const binding = state.scanBindings[normalized]
      if (binding?.kind === 'copy') {
        const boundCopy = state.copies.find((candidate) => candidate.id === binding.id)
        const boundTitle = boundCopy && state.titles.find((title) => title.id === boundCopy.titleId)
        if (boundCopy && boundTitle) return clone({ kind: 'copy', copy: boundCopy, title: boundTitle, loan: state.loans.find((loan) => loan.copyId === boundCopy.id) })
      }
      if (binding?.kind === 'title') {
        const boundTitle = state.titles.find((title) => title.id === binding.id)
        if (boundTitle) return clone({ kind: 'title', title: boundTitle })
      }
      const copy = state.copies.find((candidate) => normalizedCode(candidate.code) === normalized)
      const copyTitle = copy && state.titles.find((title) => title.id === copy.titleId)
      if (copy && copyTitle) {
        return clone({ kind: 'copy', copy, title: copyTitle, loan: state.loans.find((loan) => loan.copyId === copy.id) })
      }
      if (normalized === DEMO_CODES.ambiguous) {
        return clone({ kind: 'ambiguous', titles: state.titles.filter((title) => ['title-1', 'title-2'].includes(title.id)) })
      }
      const titles = normalized ? state.titles.filter((title) => title.isbn && normalizedIsbn(title.isbn) === normalizedIsbn(code)) : []
      if (titles.length === 1) return clone({ kind: 'title', title: titles[0]! })
      if (titles.length > 1) return clone({ kind: 'ambiguous', titles })
      return { kind: 'not-found', code: code.trim() }
    },

    getReaderLoans(readerId: string): Loan[] {
      return clone(state.loans.filter((loan) => loan.readerId === readerId))
    },

    getTitle(titleId: string): Title | undefined {
      const title = state.titles.find((candidate) => candidate.id === titleId)
      return title ? clone(title) : undefined
    },

    async identifyCard(card: string): Promise<Reader> {
      await pause()
      requireServer()
      const normalized = normalizedCode(card)
      const boundReaderId = state.cardBindings[normalized]
      const reader = state.readers.find((candidate) => candidate.id === boundReaderId || normalizedCode(candidate.card) === normalized)
      if (!reader) fail('NOT_FOUND', 'Карта не найдена. Попробуйте ручной поиск читателя.')
      return clone(reader)
    },

    issue(readerId: string, items: readonly BasketItem[], operationId: string): Promise<OperationResult> {
      const input = basketInput(items)
      return mutate(operationId, JSON.stringify(['issue', readerId, input]), (draft) => {
        if (!draft.readers.some((reader) => reader.id === readerId)) fail('NOT_FOUND', 'Читатель не найден.')
        validateBasket(draft, input)
        const loanIds: string[] = []
        const copyIds: string[] = []
        for (const item of input) {
          if (item.mode === 'COPY') {
            if (draft.loans.some((loan) => loan.copyId === item.copyId)) fail('COPY_ISSUED', 'Этот экземпляр уже выдан читателю.')
            copyIds.push(item.copyId!)
          } else {
            const issued = draft.loans.filter((loan) => loan.titleId === item.titleId && loan.mode === 'LEGACY_TITLE').reduce((sum, loan) => sum + loan.quantity, 0)
            if (issued + item.quantity > (draft.legacyStock[item.titleId] ?? 0)) fail('INSUFFICIENT_STOCK', 'В фонде недостаточно свободных книг этого издания.')
          }
          const id = nextId(draft, 'loan')
          draft.loans.push({ id, readerId, titleId: item.titleId, copyId: item.copyId, quantity: item.quantity, mode: item.mode })
          loanIds.push(id)
        }
        return { operationId, type: 'issue', quantity: input.reduce((sum, item) => sum + item.quantity, 0), copyIds, loanIds }
      })
    },

    accept(readerId: string, items: readonly BasketItem[], operationId: string): Promise<OperationResult> {
      const input = basketInput(items)
      return mutate(operationId, JSON.stringify(['accept', readerId, input]), (draft) => {
        if (!readerId || !draft.readers.some(reader=>reader.id===readerId)) fail('NOT_FOUND', 'Сначала выберите читателя для приёма книг.')
        validateBasket(draft, input)
        const loanIds: string[] = []
        const copyIds: string[] = []
        for (const item of input) {
          const loan = item.mode === 'COPY'
            ? draft.loans.find((candidate) => candidate.copyId === item.copyId && (!item.loanId || candidate.id === item.loanId))
            : draft.loans.find((candidate) => candidate.id === item.loanId && candidate.mode === 'LEGACY_TITLE' && candidate.titleId === item.titleId)
          if (!loan) fail('NO_LOAN', item.mode === 'LEGACY_TITLE' ? 'Выберите читателя и его активную выдачу для этого издания.' : 'У экземпляра нет активной выдачи.')
          if (loan.readerId !== readerId) fail('WRONG_READER', 'Книга выдана другому читателю. Текущий читатель и список не изменены.')
          if (item.quantity > loan.quantity) fail('EXCESS_QUANTITY', 'Нельзя принять больше книг, чем выдано читателю.')
          if (item.quantity === loan.quantity) draft.loans = draft.loans.filter((candidate) => candidate.id !== loan.id)
          else draft.loans = draft.loans.map((candidate) => candidate.id === loan.id ? { ...loan, quantity: loan.quantity - item.quantity } : candidate)
          loanIds.push(loan.id)
          if (loan.copyId) copyIds.push(loan.copyId)
        }
        return { operationId, type: 'accept', quantity: input.reduce((sum, item) => sum + item.quantity, 0), copyIds, loanIds }
      })
    },

    register(title: TitleInput, mode: AccountingMode, codes: readonly string[], quantity: number, operationId: string): Promise<OperationResult> {
      const inputTitle = titleInput(title)
      const inputCodes = [...codes]
      return mutate(operationId, JSON.stringify(['register', inputTitle, mode, inputCodes, quantity]), (draft) => {
        quantityValid(quantity)
        let titleId: string
        if (typeof inputTitle === 'string') {
          if (!draft.titles.some((candidate) => candidate.id === inputTitle)) fail('NOT_FOUND', 'Издание не найдено.')
          titleId = inputTitle
        } else {
          const textFields = ['name', 'author', 'publisher', 'language', 'subject', 'grade', 'isbn'] as cons
          if (textFields.some((key) => typeof inputTitle[key] !== 'string' || inputTitle[key].length > 300)
            || !inputTitle.name.trim() || !inputTitle.author.trim() || !inputTitle.language.trim()
            || !Number.isInteger(inputTitle.year) || inputTitle.year < 1400 || inputTitle.year > new Date().getFullYear() + 1) {
            fail('INVALID_INPUT', 'Укажите название, автора, язык и корректный год издания.')
          }
          const isbn = normalizedIsbn(inputTitle.isbn)
          if (isbn && !/^(\d{13}|\d{9}[\dX])$/.test(isbn)) fail('INVALID_INPUT', 'ISBN должен содержать 10 или 13 знаков.')
          if (isbn && draft.titles.some((candidate) => normalizedIsbn(candidate.isbn) === isbn)) fail('DUPLICATE_ISBN', 'Издание с этим ISBN уже есть. Выберите существующую карточку.')
          titleId = nextId(draft, 'title')
          draft.titles.push({ ...inputTitle, id: titleId, name: inputTitle.name.trim(), author: inputTitle.author.trim(), isbn })
        }
        const copyIds: string[] = []
        if (mode === 'COPY') {
          if (inputCodes.length !== quantity) fail('INVALID_INPUT', 'Количество номеров должно совпадать с количеством экземпляров.')
          const seen = new Set<string>()
          for (const raw of inputCodes) {
            const code = raw.trim()
            const key = normalizedCode(code)
            if (!code || code.length > 80 || [...code].some(char=>char.charCodeAt(0)<32||char.charCodeAt(0)===127)) fail('INVALID_INPUT', 'Введите существующий инвентарный номер без управляющих символов.')
            if (seen.has(key)) fail('DUPLICATE_CODE', 'Один инвентарный номер указан несколько раз.')
            seen.add(key)
            const existing = draft.copies.find((copy) => normalizedCode(copy.code) === key)
            if (existing) {
              if (existing.titleId !== titleId) fail('DUPLICATE_CODE', 'Этот номер уже принадлежит другому изданию.')
              copyIds.push(existing.id)
            } else {
              const id = nextId(draft, 'copy')
              draft.copies.push({ id, titleId, code })
              copyIds.push(id)
            }
          }
        } else if (mode === 'LEGACY_TITLE') {
          if (inputCodes.length) fail('INVALID_INPUT', 'Для немаркированного фонда укажите только количество.')
          draft.legacyStock[titleId] = (draft.legacyStock[titleId] ?? 0) + quantity
        } else fail('INVALID_INPUT', 'Выберите способ регистрации.')
        return { operationId, type: 'register', quantity, titleId, copyIds, loanIds: [] }
      })
    },

    async checkOperation(operationId: string): Promise<OperationResult | null> {
      await pause()
      requireServer()
      const entry = Object.hasOwn(state.operations, operationId) ? state.operations[operationId] : undefined
      return entry ? clone(entry.result) : null
    },

    setConnection(update: { server?: boolean; internet?: boolean }): void {
      connection = { server: update.server ?? connection.server, internet: update.internet ?? connection.internet }
    },

    setNextOutcome(outcome: 'success' | 'unknown' | 'error'): void { nextOutcome = outcome },

    bindTestCard(readerId: string, rawCode: string): void {
      const code = normalizedCode(rawCode)
      if (!code || code.length > 80 || [...code].some((char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127)) fail('INVALID_INPUT', 'Введите код карты без управляющих символов.')
      if (!state.readers.some((reader) => reader.id === readerId)) fail('NOT_FOUND', 'Выберите читателя для привязки карты.')
      const draft = clone(state)
      draft.cardBindings[code] = readerId
      save(draft)
      state = draf
    },

    bindTestCode(kind: 'copy' | 'title', id: string, rawCode: string): void {
      const code = normalizedCode(rawCode)
      if (!code || code.length > 80 || [...code].some((char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127)) fail('INVALID_INPUT', 'Введите код сканера без управляющих символов.')
      const exists = kind === 'copy' ? state.copies.some((copy) => copy.id === id) : state.titles.some((title) => title.id === id)
      if (!exists) fail('NOT_FOUND', kind === 'copy' ? 'Экземпляр для привязки не найден.' : 'Издание для привязки не найдено.')
      const draft = clone(state)
      draft.scanBindings[code] = { kind, id }
      save(draft)
      state = draf
    },

    resetTestData(): void {
      const draft = seedState()
      if (options.storage) {
        try { options.storage.removeItem(storageKey) }
        catch {
          persistence = 'memory'
          if (options.requirePersistence) fail('PERSISTENCE_UNAVAILABLE', 'Тестовые данные не удалось сбросить: локальное хранилище недоступно.')
        }
      }
      save(draft)
      state = draf
      nextOutcome = 'success'
      nextDelayMs = undefined
    },

    setNextDelay(delayMs: number): void {
      if (!Number.isSafeInteger(delayMs) || delayMs < 0 || delayMs > 10_000) fail('INVALID_INPUT', 'Укажите задержку от 0 до 10 секунд.')
      nextDelayMs = delayMs
    },
  }
  return adapter
}

export type MockAdapter = TerminalAdapter

\n