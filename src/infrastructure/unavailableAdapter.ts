import { DomainError, type TerminalAdapter, type Snapshot } from '../domain/terminalTypes.ts'

/** Blocks production transactions until a confirmed Safe School library API adapter exists. */
export function createUnavailableAdapter(message = 'Библиотечный API не настроен. Обратитесь к администратору локального сервера.'): TerminalAdapter {
  const offline = () => { throw new DomainError('SERVER_OFFLINE', message) }
  const snapshot = (): Snapshot => ({ readers: [], titles: [], copies: [], loans: [], legacyStock: {}, connection: { server: false, internet: true }, persistence: 'none' })
  return {
    snapshot, searchReaders: () => [], searchTitles: () => [],
    resolveCode: (code) => ({ kind: 'not-found', code: code.trim() }), getReaderLoans: () => [], getTitle: () => undefined,
    identifyCard: async () => offline(), issue: async () => offline(), accept: async () => offline(), register: async () => offline(), checkOperation: async () => offline(),
    setConnection: () => undefined, setNextOutcome: () => undefined,
  }
}
