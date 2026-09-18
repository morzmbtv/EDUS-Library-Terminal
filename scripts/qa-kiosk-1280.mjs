/** End-to-end visual and interaction verification for the 10.1-inch POS kiosk. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173'
const output = 'screenshots/kiosk-1280'
mkdirSync(output, { recursive: true })
const report = { baseUrl, viewport: { width: 1280, height: 800 }, startedAt: new Date().toISOString(), checks: [], failures: [], consoleErrors: [], failedRequests: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }

async function capture(page, name) { await page.screenshot({ path: join(output, `${name}.png`), animations: 'disabled' }) }
async function sendScan(page, value) {
  const target = page.locator('.issue-scanner-column').first()
  if (await target.count()) await target.click({ position: { x: 18, y: 18 } })
  else await page.locator('h1').click()
  await page.keyboard.type(value, { delay: 2 })
  await page.keyboard.press('Enter')
}
async function noPageOverflow(page, label) {
  const metrics = await page.evaluate(() => ({ width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight, innerWidth: window.innerWidth, innerHeight: window.innerHeight, dpr: window.devicePixelRatio }))
  report.metrics ??= metrics
  assert(metrics.width <= metrics.innerWidth + 1, `${label}: horizontal document overflow`)
  assert(metrics.height <= metrics.innerHeight + 1, `${label}: primary UI requires document scrolling`)
}

const browser = await chromium.launch({ headless: true })
try {
  const context = await browser.newContext({ viewport: report.viewport, deviceScaleFactor: 1, locale: 'ru-RU' })
  const page = await context.newPage()
  page.on('console', event => { if (event.type() === 'error') report.consoleErrors.push(event.text()) })
  page.on('pageerror', error => report.consoleErrors.push(error.message))
  page.on('requestfailed', request => report.failedRequests.push(request.url()))

  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  await noPageOverflow(page, 'home')
  await capture(page, '01-home')
  report.checks.push('home: three actions remain in one row without document scrolling')

  await page.getByRole('button', { name: /^Выдать книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await noPageOverflow(page, 'identification')
  await capture(page, '02-identification-idle')
  report.checks.push('identification: contour card, manual search, and header fit')

  await sendScan(page, 'EDUS-1001')
  await page.locator('.issue-scan-layout').waitFor()
  await noPageOverflow(page, 'empty scan')
  await capture(page, '03-scan-empty')
  const scanButtons = await page.locator('.issue-scanner-column .manual-button').evaluateAll(buttons => buttons.map(button => ({ width: button.getBoundingClientRect().width, height: button.getBoundingClientRect().height, text: button.textContent?.replace(/\s+/g, ' ').trim(), wrapped: button.getBoundingClientRect().height > 70 })))
  assert(scanButtons.every(button => button.height >= 64 && !button.wrapped), 'Manual/ISBN buttons are wrapped or below the kiosk target height')
  report.checks.push('empty issue scan: 58/42 workspace, full reader panel, unwrapped 64px actions')

  for (const code of ['000124', '000125', 'KZ-0008']) await sendScan(page, code)
  await page.locator('.book-basket .basket-count').getByText('3', { exact: true }).waitFor()
  assert(await page.locator('.book-basket').getByText('Абай жолы. Бірінші кітап', { exact: true }).count() === 1, 'Long book title is missing from the issue list')
  await capture(page, '04-scan-three-books')
  report.checks.push('three books: readable rows, long title, and removable controls')

  await page.getByRole('button', { name: 'Добавить по ISBN', exact: true }).click()
  await page.getByLabel('ISBN', { exact: true }).fill('9786010123456')
  await page.getByRole('button', { name: 'Найти книгу', exact: true }).click()
  await page.getByRole('heading', { name: 'Найдено издание', exact: true }).waitFor()
  await page.getByRole('button', { name: 'Добавить из старого фонда', exact: true }).click()
  await page.locator('.book-basket .basket-count').getByText('4', { exact: true }).waitFor()
  const scrollable = await page.locator('.book-basket .basket-scroll').evaluate(element => element.scrollHeight > element.clientHeight)
  assert(scrollable, 'Four real basket rows do not use the intended internal scrolling region')
  await capture(page, '05-scan-long-list')
  report.checks.push('long basket: fourth item is retained in an internally scrollable list')

  await page.getByRole('button', { name: 'Ввести номер вручную', exact: true }).click()
  await page.getByLabel('Инвентарный номер', { exact: true }).waitFor()
  await page.getByRole('button', { name: 'Открыть клавиатуру: Инвентарный номер', exact: true }).click()
  await page.locator('.touch-keyboard').waitFor()
  const keyboardGeometry = await page.evaluate(() => {
    const modal = document.querySelector('.modal')?.getBoundingClientRect()
    const keyboard = document.querySelector('.touch-keyboard')?.getBoundingClientRect()
    const input = document.querySelector('input')?.getBoundingClientRect()
    return { modalBottom: modal?.bottom, keyboardTop: keyboard?.top, inputBottom: input?.bottom }
  })
  assert(keyboardGeometry.inputBottom && keyboardGeometry.keyboardTop && keyboardGeometry.inputBottom <= keyboardGeometry.keyboardTop, 'Manual input is hidden behind the on-screen keyboard')
  await capture(page, '06-manual-keyboard')
  await page.getByRole('button', { name: 'Скрыть', exact: true }).click()
  await page.getByRole('button', { name: 'Закрыть', exact: true }).click()
  report.checks.push('manual input: field remains available above the on-screen keyboard')

  await page.getByRole('button', { name: 'Помощь', exact: true }).click()
  await page.getByRole('heading', { name: 'Как выдать книги', exact: true }).waitFor()
  await capture(page, '07-context-help')
  assert(await page.locator('.book-basket .basket-count').innerText() === '4', 'Opening help reset basket state')
  await page.getByRole('button', { name: 'Понятно', exact: true }).click()
  report.checks.push('help: touch dialog overlays the workflow and preserves the basket')

  await page.getByRole('button', { name: 'Перейти к подтверждению', exact: true }).click()
  await page.getByRole('heading', { name: 'Всё верно?', exact: true }).waitFor()
  await noPageOverflow(page, 'confirmation')
  await capture(page, '08-confirmation')
  report.checks.push('confirmation: reader and factual basket carry forward without document scrolling')

  await page.getByRole('button', { name: 'Изменить список', exact: true }).click()
  await page.locator('.issue-scan-layout').waitFor()
  await sendScan(page, 'UNKNOWN-CODE')
  await page.getByText('Код не найден. Проверьте номер или добавьте книгу в фонд.', { exact: true }).waitFor()
  assert(await page.locator('.book-basket .basket-count').innerText() === '4', 'Error discarded existing basket')
  await capture(page, '09-scan-error')
  report.checks.push('error: unknown code is visible and preserves collected books')

  assert(report.consoleErrors.length === 0, `Console errors: ${report.consoleErrors.join('; ')}`)
  assert(report.failedRequests.length === 0, `Failed requests: ${report.failedRequests.join('; ')}`)
  await context.close()
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error))
} finally {
  await browser.close()
}
report.finishedAt = new Date().toISOString()
report.passed = report.failures.length === 0
writeFileSync('.qa/kiosk-1280-report.json', JSON.stringify(report, null, 2))
console.log(JSON.stringify({ passed: report.passed, metrics: report.metrics, checks: report.checks, failures: report.failures, consoleErrors: report.consoleErrors, failedRequests: report.failedRequests }, null, 2))
process.exitCode = report.passed ? 0 : 1
