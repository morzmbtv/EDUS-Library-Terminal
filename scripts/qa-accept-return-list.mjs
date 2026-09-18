/** Browser QA for the reader-first return screen with active-loan rows. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173'
const output = 'screenshots/accept-return-list'
mkdirSync(output, { recursive: true })
const report = { baseUrl, viewport: { width: 1280, height: 800 }, checks: [], failures: [], consoleErrors: [], failedRequests: [] }
const assert = (value, text) => { if (!value) throw new Error(text) }

async function scan(page, code, surface = 'h1') {
  await page.locator(surface).first().click({ position: { x: 20, y: 20 } })
  await page.keyboard.type(code, { delay: 2 })
  await page.keyboard.press('Enter')
}
async function openReader(page) {
  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  await page.getByRole('button', { name: /^Принять книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await scan(page, 'EDUS-1001')
  await page.locator('.accept-scan-layout').waitFor()
}

const browser = await chromium.launch({ headless: true })
try {
  const context = await browser.newContext({ viewport: report.viewport, locale: 'ru-RU' })
  const page = await context.newPage()
  page.on('console', event => { if (event.type() === 'error') report.consoleErrors.push(event.text()) })
  page.on('pageerror', error => report.consoleErrors.push(error.message))
  page.on('requestfailed', request => report.failedRequests.push(request.url()))

  await openReader(page)
  assert(await page.locator('.return-loan-row').count() === 2, 'Active loans do not appear immediately after selecting the reader')
  assert(await page.locator('.return-loan-row.selected').count() === 0, 'A loan is selected before a scan')
  assert(await page.getByText('2 названия · 3 книги', { exact: true }).count() === 1, 'Title and book counters are not derived from active loans')
  assert(await page.getByRole('button', { name: 'Сбросить отметки', exact: true }).isDisabled(), 'Reset is enabled with no return draft')
  assert(await page.getByRole('button', { name: /Перейти к подтверждению/ }).isDisabled(), 'Confirmation is enabled with no return draft')
  assert(!(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)), 'Return view has horizontal overflow at 1280×800')
  assert(!(await page.evaluate(() => document.documentElement.scrollHeight > innerHeight + 1)), 'Return view requires document scrolling at 1280×800')
  await page.screenshot({ path: join(output, '1280x800-all-grey.png'), animations: 'disabled' })
  report.checks.push('reader card immediately shows two grey active-loan rows and correct 2-title / 3-book counters')

  await scan(page, '000123', '.accept-scanner-column')
  await page.locator('.return-loan-row.selected').waitFor()
  assert(await page.locator('.return-loan-row.selected').count() === 1, 'A copy scan selected more than one row')
  assert(await page.getByText('Книг на руках', { exact: true }).count() === 1 && await page.locator('.scan-reader-summary').getByText('3', { exact: true }).count() === 1, 'On-hand count changed before confirmation')
  assert(!(await page.getByRole('button', { name: /Перейти к подтверждению/ }).isDisabled()), 'Confirmation remains disabled after a valid scan')
  await page.screenshot({ path: join(output, '1280x800-one-selected.png'), animations: 'disabled' })
  await page.getByRole('button', { name: /Действия для книги/ }).click()
  assert(await page.getByRole('button', { name: 'Убрать из приёма', exact: true }).isVisible(), 'Selected row has no remove action')
  await page.screenshot({ path: join(output, '1280x800-selected-actions.png'), animations: 'disabled' })
  await page.getByRole('button', { name: 'Убрать из приёма', exact: true }).click()
  assert(await page.locator('.return-loan-row.selected').count() === 0, 'Removing an item does not return its active-loan row to grey')
  report.checks.push('single scanned copy is selected once, exposes actions, and removal preserves the active list')

  await page.getByRole('button', { name: 'Найти книгу на руках', exact: true }).click()
  await page.getByRole('button', { name: /Қазақстан тарихы/ }).click()
  await page.getByRole('button', { name: 'Отметить к приёму', exact: true }).click()
  assert(await page.getByText('К приёму: 1 из 2', { exact: true }).count() === 1, 'Legacy title cannot be partially selected')
  await page.screenshot({ path: join(output, '1280x800-partial-one-of-two.png'), animations: 'disabled' })
  report.checks.push('manual active-loan search requires explicit marking and supports 1 of 2 legacy copies')

  await page.getByRole('button', { name: 'Помощь', exact: true }).click()
  await page.getByRole('heading', { name: 'Как принять книги', exact: true }).waitFor()
  assert(await page.getByText('Код не читается? Введите номер вручную или найдите книгу в списке на руках.', { exact: true }).count() === 1, 'Return help lacks the manual fallback')
  await page.getByRole('button', { name: 'Понятно', exact: true }).click()
  assert(await page.getByText('К приёму: 1 из 2', { exact: true }).count() === 1, 'Help reset the return draft')
  report.checks.push('context help preserves the return draft')

  await page.getByRole('button', { name: 'Сбросить отметки', exact: true }).click()
  await page.getByRole('heading', { name: 'Сбросить отметки?', exact: true }).waitFor()
  await page.locator('.modal .modal-actions .btn.primary').click()
  assert(await page.locator('.return-loan-row').count() === 2 && await page.locator('.return-loan-row.selected').count() === 0, 'Reset cleared active loans instead of only the draft')
  report.checks.push('reset confirmation keeps active rows and reader context')

  await scan(page, 'KZ-0007', '.accept-scanner-column')
  await page.getByRole('heading', { name: 'Книга другого читателя', exact: true }).waitFor()
  assert(await page.locator('.return-loan-row.selected').count() === 0, 'Another reader’s copy entered the current return draft')
  await page.getByRole('button', { name: 'Не добавлять книгу', exact: true }).click()
  report.checks.push('another reader’s copy is rejected without altering the current list')

  await scan(page, '000123', '.accept-scanner-column')
  await page.getByRole('button', { name: /Перейти к подтверждению/ }).click()
  assert(await page.locator('.review-list .book-row').count() === 1, 'Confirmation received active loans that were not marked for return')
  await page.getByRole('button', { name: /Принять 1 книгу/ }).click()
  await page.getByRole('heading', { name: 'Книги приняты', exact: true }).waitFor()
  await page.getByRole('button', { name: 'Следующий возврат', exact: true }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await scan(page, 'EDUS-1001')
  await page.locator('.accept-scan-layout').waitFor()
  assert(await page.locator('.return-loan-row').count() === 1, 'Completed return remained in active-loan list')
  assert(await page.getByText('1 название · 2 книги', { exact: true }).count() === 1, 'On-hand total was not refreshed after confirmation')
  report.checks.push('confirmation receives only marked loans and refreshes active-loan quantities after success')

  assert(!report.consoleErrors.length && !report.failedRequests.length, 'Browser emitted console or request errors')
  await context.close()
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error))
} finally {
  await browser.close()
}
report.finishedAt = new Date().toISOString()
report.passed = report.failures.length === 0 && report.consoleErrors.length === 0 && report.failedRequests.length === 0
writeFileSync('.qa/accept-return-list-report.json', JSON.stringify(report, null, 2))
console.log(JSON.stringify(report, null, 2))
process.exitCode = report.passed ? 0 : 1
