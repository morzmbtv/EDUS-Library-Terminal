/** Visual and flow verification for the terminal home redesign. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173'
const output = 'screenshots/home-redesign'
mkdirSync(output, { recursive: true })

const report = { baseUrl, checks: [], failures: [], consoleErrors: [], failedRequests: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }

async function verifyNoPageOverflow(page, label) {
  const size = await page.evaluate(() => ({ width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight, viewportWidth: innerWidth, viewportHeight: innerHeight }))
  assert(size.width <= size.viewportWidth + 1, `${label}: horizontal page overflow`)
  assert(size.height <= size.viewportHeight + 1, `${label}: vertical page overflow`)
  return size
}

async function capture(page, file) {
  await page.screenshot({ path: join(output, file), animations: 'disabled' })
}

const browser = await chromium.launch({ headless: true })
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, locale: 'ru-RU' })
  const page = await context.newPage()
  page.on('console', event => { if (event.type() === 'error') report.consoleErrors.push(event.text()) })
  page.on('pageerror', error => report.consoleErrors.push(error.message))
  page.on('requestfailed', request => report.failedRequests.push(request.url()))

  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  report.home1280 = await verifyNoPageOverflow(page, '1280×800 home')
  const actions = page.locator('.home-action')
  assert(await actions.count() === 3, 'Home must show exactly three actions')
  await page.getByRole('button', { name: /Выдать книги/ }).waitFor()
  await page.getByRole('button', { name: /Принять книги/ }).waitFor()
  await page.getByRole('button', { name: /Найти книгу/ }).waitFor()
  await capture(page, 'home-1280x800.png')
  report.checks.push('1280×800: three equal home actions, no page overflow')

  await page.getByRole('button', { name: 'Инструкция', exact: true }).click()
  await page.getByRole('heading', { name: 'Как работать с терминалом', exact: true }).waitFor()
  await capture(page, 'home-help-1280x800.png')
  await page.getByRole('button', { name: 'Понятно', exact: true }).click()
  report.checks.push('Home instruction opens context help without changing page state')

  await page.getByRole('button', { name: /Выдать книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await page.getByText('Выдача книг', { exact: true }).waitFor()
  report.checks.push('Issue action enters existing reader identification flow')

  await page.getByRole('button', { name: 'На главную', exact: true }).click()
  await page.getByRole('heading', { name: 'Библиотечный терминал', exact: true }).waitFor()
  await page.getByRole('button', { name: /Принять книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await page.getByText('Приём книг', { exact: true }).waitFor()
  report.checks.push('Return action enters existing reader-first identification flow')

  await page.getByRole('button', { name: 'На главную', exact: true }).click()
  await page.getByRole('heading', { name: 'Библиотечный терминал', exact: true }).waitFor()
  await page.getByRole('button', { name: /Найти книгу/ }).click()
  await page.getByRole('heading', { name: 'Найти книгу', exact: true }).waitFor()
  assert(new URL(page.url()).hash === '#/library-search', 'Search action did not use the catalogue route')
  await page.getByLabel('Название, автор или ISBN', { exact: true }).fill('Алгебра')
  await page.getByRole('button', { name: 'Найти', exact: true }).click()
  await page.getByText('Алгебра. 7 класс', { exact: true }).waitFor()
  await capture(page, 'catalogue-search-1280x800.png')
  report.search1280 = await verifyNoPageOverflow(page, '1280×800 catalogue search')
  report.checks.push('Catalogue search displays local title availability without an operation')

  await page.setViewportSize({ width: 1366, height: 768 })
  await page.goto(`${baseUrl}#/`, { waitUntil: 'networkidle' })
  await page.evaluate(() => { (document.activeElement instanceof HTMLElement ? document.activeElement : null)?.blur() })
  report.home1366 = await verifyNoPageOverflow(page, '1366×768 home')
  await capture(page, 'home-1366x768.png')
  report.checks.push('1366×768: home remains a single visible kiosk composition')

  assert(report.consoleErrors.length === 0, `Console errors: ${report.consoleErrors.join('; ')}`)
  assert(report.failedRequests.length === 0, `Failed requests: ${report.failedRequests.join('; ')}`)
  await context.close()
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error))
} finally {
  await browser.close()
}

report.passed = report.failures.length === 0
writeFileSync('.qa/home-redesign-report.json', JSON.stringify(report, null, 2))
console.log(JSON.stringify(report, null, 2))
process.exitCode = report.passed ? 0 : 1
