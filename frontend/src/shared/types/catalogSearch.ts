import type { CopyStatus, Title } from './terminalTypes.ts';

export type CatalogSort = 'relevance' | 'title' | 'available';
export type CatalogCopyStatus = CopyStatus | 'ON_LOAN';

export interface CatalogCopy {
  readonly id: string;
  readonly code: string;
  readonly status: CatalogCopyStatus;
  readonly location?: string | null;
  readonly dueDate?: string | null;
}

export interface CatalogAvailability {
  readonly title: Title;
  readonly totalCopies: number;
  readonly availableCopies: number;
  readonly onLoanCopies: number;
  readonly legacyTotal: number;
  readonly legacyOnLoan: number;
  readonly legacyAvailable: number;
  readonly queueCount: number;
  readonly copies: readonly CatalogCopy[];
  readonly availableCopyIds: readonly string[];
  readonly nearestDueDate?: string | null;
  readonly hasOverdue: boolean;
}

export interface CatalogSearchResult extends CatalogAvailability {
  readonly relevance: number;
}

export function formatInventoryStatus(status: CatalogCopyStatus): string {
  return {
    AVAILABLE: 'Доступен',
    ON_LOAN: 'На руках',
    RESERVED: 'Забронирован',
    REPAIR: 'В ремонте',
    LOST: 'Утрачен',
    WRITTEN_OFF: 'Списан',
    VERIFYING: 'Требует проверки',
  }[status];
}
