export type AccountingMode = 'COPY' | 'LEGACY_TITLE'

export interface Reader { readonly id: string; readonly name: string; readonly group: string; readonly card: string }
export interface Title { readonly id: string; readonly name: string; readonly author: string; readonly isbn: string; readonly publisher: string; readonly year: number; readonly language: string; readonly subject: string; readonly grade: string }
export interface Copy { readonly id: string; readonly titleId: string; readonly code: string }
export interface Loan { readonly id: string; readonly readerId: string; readonly titleId: string; readonly copyId?: string; readonly quantity: number; readonly mode: AccountingMode }
export interface BasketItem { readonly id: string; readonly titleId: string; readonly copyId?: string; readonly quantity: number; readonly mode: AccountingMode; readonly loanId?: string }
export interface OperationResult { readonly operationId: string; readonly type: 'issue' | 'accept' | 'register'; readonly quantity: number; readonly titleId?: string; readonly copyIds: readonly string[]; readonly loanIds: readonly string[] }
export type DomainErrorCode = 'UNKNOWN' | 'SERVER_OFFLINE' | 'NOT_FOUND' | 'DUPLICATE_COPY' | 'COPY_ISSUED' | 'INVALID_INPUT' | 'INSUFFICIENT_STOCK' | 'NO_LOAN' | 'EXCESS_QUANTITY' | 'DUPLICATE_CODE' | 'DUPLICATE_ISBN' | 'OPERATION_CONFLICT' | 'SIMULATED_ERROR' | 'WRONG_READER' | 'PERSISTENCE_UNAVAILABLE'
export class DomainError extends Error {
  readonly code: DomainErrorCode
  readonly operationId?: string
  constructor(code: DomainErrorCode, message: string, operationId?: string) { super(message); this.name = 'DomainError'; this.code = code; this.operationId = operationId }
}
export type CodeResult = { readonly kind: 'copy'; readonly copy: Copy; readonly title: Title; readonly loan?: Loan } | { readonly kind: 'title'; readonly title: Title } | { readonly kind: 'ambiguous'; readonly titles: readonly Title[] } | { readonly kind: 'not-found'; readonly code: string }
export interface Snapshot { readonly readers: readonly Reader[]; readonly titles: readonly Title[]; readonly copies: readonly Copy[]; readonly loans: readonly Loan[]; readonly legacyStock: Readonly<Record<string, number>>; readonly connection: Readonly<{ server: boolean; internet: boolean }>; readonly persistence: 'memory' | 'local-storage' | 'none' }
export type TitleInput = string | Omit<Title, 'id'>
export interface TerminalAdapter {
  snapshot(): Snapshot
  searchReaders(query: string): Reader[]
  searchTitles(query: string): Title[]
  resolveCode(code: string): CodeResult
  getReaderLoans(readerId: string): Loan[]
  getTitle(titleId: string): Title | undefined
  identifyCard(card: string): Promise<Reader>
  issue(readerId: string, items: readonly BasketItem[], operationId: string): Promise<OperationResult>
  accept(readerId: string, items: readonly BasketItem[], operationId: string): Promise<OperationResult>
  register(title: TitleInput, mode: AccountingMode, codes: readonly string[], quantity: number, operationId: string): Promise<OperationResult>
  checkOperation(operationId: string): Promise<OperationResult | null>
  setConnection(connection: Partial<{ server: boolean; internet: boolean }>): void
  setNextOutcome(outcome: 'success' | 'unknown' | 'error'): void
}
