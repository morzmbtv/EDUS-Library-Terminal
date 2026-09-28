export type AccountingMode = 'COPY' | 'LEGACY_TITLE';
export type CopyStatus =
  'AVAILABLE' | 'ON_LOAN' | 'RESERVED' | 'REPAIR' | 'LOST' | 'WRITTEN_OFF' | 'VERIFYING';
export type ReservationStatus = 'WAITING' | 'FULFILLED' | 'CANCELLED';

export interface Reader {
  readonly id: string;
  readonly name: string;
  readonly group: string;
  readonly card: string;
}
export interface Title {
  readonly id: string;
  readonly name: string;
  readonly author: string;
  readonly isbn: string;
  readonly publisher: string;
  readonly year: number;
  readonly language: string;
  readonly subject: string;
  readonly grade: string;
}
export interface Copy {
  readonly id: string;
  readonly titleId: string;
  readonly code: string;
  readonly status?: CopyStatus;
  readonly location?: string | null;
}
/** A due date belongs to an active loan, never to a copy itself. */
export interface Loan {
  readonly id: string;
  readonly readerId: string;
  readonly titleId: string;
  readonly copyId?: string | null;
  readonly quantity: number;
  readonly mode: AccountingMode;
  readonly dueDate?: string | null;
}
export interface Reservation {
  readonly id: string;
  readonly readerId: string;
  readonly titleId: string;
  readonly createdAt: string;
  readonly status: ReservationStatus;
}
export interface BasketItem {
  readonly id: string;
  readonly titleId: string;
  readonly copyId?: string | null;
  readonly quantity: number;
  readonly mode: AccountingMode;
  readonly loanId?: string | null;
}
export interface OperationResult {
  readonly operationId: string;
  readonly type: 'issue' | 'accept' | 'reserve';
  readonly quantity: number;
  readonly titleId?: string;
  readonly copyIds: readonly string[];
  readonly loanIds: readonly string[];
  readonly reservationId?: string;
  readonly queuePosition?: number;
}
export type DomainErrorCode =
  | 'UNKNOWN'
  | 'SERVER_OFFLINE'
  | 'NOT_FOUND'
  | 'DUPLICATE_COPY'
  | 'COPY_ISSUED'
  | 'INVALID_INPUT'
  | 'INSUFFICIENT_STOCK'
  | 'NO_LOAN'
  | 'EXCESS_QUANTITY'
  | 'DUPLICATE_CODE'
  | 'DUPLICATE_ISBN'
  | 'DUPLICATE_RESERVATION'
  | 'OPERATION_CONFLICT'
  | 'WRONG_READER'
  | 'PERSISTENCE_UNAVAILABLE'
  | 'LOCAL_DATABASE_ERROR'
  | 'CREDENTIAL_STORE_ERROR'
  | 'FACE_PROVIDER_UNAVAILABLE'
  | 'SYNC_ERROR';
export class DomainError extends Error {
  readonly code: DomainErrorCode;
  readonly operationId?: string;
  constructor(code: DomainErrorCode, message: string, operationId?: string) {
    super(message);
    this.name = 'DomainError';
    this.code = code;
    this.operationId = operationId;
  }
}
export type CodeResult =
  | { readonly kind: 'copy'; readonly copy: Copy; readonly title: Title; readonly loan?: Loan }
  | { readonly kind: 'title'; readonly title: Title }
  | { readonly kind: 'ambiguous'; readonly titles: readonly Title[] }
  | { readonly kind: 'not-found'; readonly code: string };
export interface Snapshot {
  readonly readers: readonly Reader[];
  readonly titles: readonly Title[];
  readonly copies: readonly Copy[];
  readonly loans: readonly Loan[];
  readonly reservations: readonly Reservation[];
  readonly legacyStock: Readonly<Record<string, number>>;
  readonly connection: Readonly<{ server: boolean; internet: boolean }>;
  readonly persistence: 'sqlcipher';
}
