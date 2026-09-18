/** Terminal-test-only smoke test: panel, explicit capture, persistence and no external requests. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:43180'
const output = 'screenshots/terminal-test'
mkdirSync(output, { recursive: true })
const report = { baseUrl, viewport: { width: 1280, height: 800 }, checks: [], consoleErrors: [], externalRequests: [], failures: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }
async function typeScan(page, value) { await page.locator('main').click({ position: { x: 12, y: 12 } }); await page.keyboard.type(value, { delay: 3 }); await page.keyboard.press('Enter') }
const browser = await chromium.launch({ headless: true })
try {
  const context = await browser.newContext({ viewport: report.viewport, locale: 'ru-RU' })
  const page = await context.newPage()
  page.on('console', event => { if (event.type() === 'error') report.consoleErrors.push(event.text()) })
  page.on('request', request => { const url = new URL(request.url()); if (url.hostname !== '127.0.0.1' && url.hostname !== 'localhost') report.externalRequests.push(request.url()) })
  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  assert(await page.getByText('Тестовый режим', { exact: true }).count() === 1, 'Terminal test badge is missing')
  assert(!(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)), 'Home has horizontal overflow at 1280×800')
  await page.screenshot({ path: `${output}/01-home.png`, animations: 'disabled' })
  await page.getByRole('button', { name: 'Настройки и диагностика', exact: true }).click()
  await page.getByRole('button', { name: 'Панель тестирования', exact: true }).click()
  await page.getByRole('heading', { name: 'Панель тестирования терминала', exact: true }).waitFor()
  await page.screenshot({ path: `${output}/02-test-panel.png`, animations: 'disabled' })
  await page.getByRole('button', { name: 'Ожидать карту', exact: true }).click()
  await page.keyboard.type('PHYSICAL-CARD-77', { delay: 3 }); await page.keyboard.press('Enter')
  assert(await page.getByText('Получено:', { exact: false }).count() >= 1, 'Explicit card capture did not show received value')
  await page.getByRole('button', { name: 'Привязать карту локально', exact: true }).click()
  await page.getByRole('button', { name: 'Закрыть настройки', exact: true }).click()
  await page.getByRole('button', { name: /^Выдать книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await typeScan(page, 'PHYSICAL-CARD-77')
  await page.locator('.issue-scan-layout').waitFor()
  assert(await page.getByText('Сәрсенова Айша Ерланқызы', { exact: true }).count() >= 1, 'Bound card did not identify selected reader')
  await page.screenshot({ path: `${output}/03-bound-card-issue.png`, animations: 'disabled' })
  await page.getByRole('button', { name: 'Настройки и диагностика', exact: true }).click()
  await page.getByRole('heading', { name: 'Панель тестирования терминала', exact: true }).waitFor()
  const scannerTest = page.locator('.terminal-test-section').filter({ hasText: 'Сканер книг' })
  await scannerTest.locator('select').nth(1).selectOption('copy-2')
  await scannerTest.getByRole('button', { name: 'Ожидать сканер', exact: true }).click()
  await page.keyboard.type('PHYSICAL-COPY-77', { delay: 3 }); await page.keyboard.press('Enter')
  await scannerTest.getByRole('button', { name: 'Привязать код локально', exact: true }).click()
  await page.getByRole('button', { name: 'Закрыть настройки', exact: true }).click()
  await typeScan(page, 'PHYSICAL-COPY-77')
  assert(await page.getByText('Алгебра. 7 класс', { exact: true }).count() >= 1, 'Bound scanner code did not resolve a local copy')
  report.checks.push('explicit scanner capture maps a raw code to one selected local copy')
  await page.reload({ waitUntil: 'networkidle' })
  await page.getByRole('button', { name: /^Выдать книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await typeScan(page, 'PHYSICAL-CARD-77')
  await page.locator('.issue-scan-layout').waitFor()
  report.checks.push('test badge, explicit capture, local card mapping, and browser-refresh persistence')
  assert(report.externalRequests.length === 0, `External requests detected: ${report.externalRequests.join(', ')}`)
  assert(report.consoleErrors.length === 0, `Console errors detected: ${report.consoleErrors.join(', ')}`)
  report.checks.push('no external or school API requests, no console errors')
  report.passed = true
} catch (error) { report.failures.push(error instanceof Error ? error.message : String(error)); report.passed = false } finally { await browser.close(); report.finishedAt = new Date().toISOString(); writeFileSync(`${output}/report.json`, JSON.stringify(report, null, 2)); console.log(JSON.stringify(report, null, 2)); if (!report.passed) process.exitCode = 1 }
