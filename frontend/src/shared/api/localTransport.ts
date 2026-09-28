import { DomainError } from '../types/terminalTypes.ts';

let csrf = '';
let renewing: Promise<void> | undefined;
export function localError(value: unknown): DomainError {
  const body =
    value && typeof value === 'object'
      ? (value as { code?: string; message?: string; operationId?: string })
      : {};
  return new DomainError(
    (body.code ?? 'UNKNOWN') as ConstructorParameters<typeof DomainError>[0],
    body.message ?? 'Локальная служба недоступна. Проверьте сохранённый результат операции.',
    body.operationId,
  );
}
async function renew(): Promise<void> {
  if (!renewing)
    renewing = (async () => {
      const response = await fetch('/api/local/v1/session/renew', {
        method: 'POST',
        credentials: 'same-origin',
        cache: 'no-store',
        headers: { 'Content-Type': 'application/json' },
        body: '{}',
        signal: AbortSignal.timeout(20000),
      });
      if (!response.ok) throw localError(await response.json().catch(() => undefined));
      csrf = ((await response.json()) as { csrfToken: string }).csrfToken;
    })().finally(() => {
      renewing = undefined;
    });
  await renewing;
}
export async function localRequest<T>(path: string, options: RequestInit = {}, mutation = false): Promise<T> {
  if (!csrf) await renew();
  for (let attempt = 0; attempt < 2; attempt++) {
    const headers = new Headers(options.headers);
    headers.set('Accept', 'application/json');
    if (mutation) {
      headers.set('Content-Type', 'application/json');
      headers.set('X-EDUS-CSRF', csrf);
    }
    // No retry on timeout/disconnect: the server might already have committed.
    let response: Response;
    try {
      response = await fetch('/api/local/v1' + path, {
        ...options,
        headers,
        credentials: 'same-origin',
        cache: 'no-store',
        signal: options.signal ?? AbortSignal.timeout(20000),
      });
    } catch {
      let operationId: string | undefined;
      if (typeof options.body === 'string') {
        try {
          const body: unknown = JSON.parse(options.body);
          if (
            body &&
            typeof body === 'object' &&
            'operationId' in body &&
            typeof body.operationId === 'string'
          )
            operationId = body.operationId;
        } catch {
          /* No request payload is logged. */
        }
      }
      throw new DomainError(
        mutation && operationId ? 'UNKNOWN' : 'SERVER_OFFLINE',
        mutation && operationId
          ? 'Ответ ещё не получен. Проверьте результат операции перед повторной отправкой.'
          : 'Локальная служба EDUS недоступна',
        operationId,
      );
    }
    if (response.ok) return response.json() as Promise<T>;
    const body: unknown = await response.json().catch(() => undefined);
    const error = localError(body);
    // These rejections happen before domain dispatch, so replay is safe.
    if (
      attempt === 0 &&
      ((error.code as string) === 'LOCAL_SESSION_REQUIRED' || (error.code as string) === 'CSRF_REJECTED')
    ) {
      await renew();
      continue;
    }
    throw error;
  }
  throw localError(undefined);
}
