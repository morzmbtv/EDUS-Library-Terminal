import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:43173'
const output = 'screenshots/production-boundary'
mkdirSync(output, { recursive: true })
const report = { baseUrl, viewport: { width: 1280, height: 800 }, checks: [], consoleErrors: [], failedRequests: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }
const browser = await chromium.launch({ headless: true })
try {
  const context = await browser.newContext({ viewport: report.viewport, locale: 'ru-RU' })
  const page = await context.newPage()
  page.on('console', event => { if (event.type() === 'error') report.consoleErrors.push(event.text()) })
  page.on('pageerror', error => report.consoleErrors.push(error.message))
  page.on('requestfailed', request => report.failedRequests.push(request.url()))
  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  await page.screenshot({ path: `${output}/01-production-home.png`, animations: 'disabled' })
  assert(await page.getByRole('button', { name: /^Принять книги/ }).count() === 1, 'Production home is not rendered')
  assert(!(await page.locator('text=Демонстрационные события').count()), 'Production build exposes demo scenario tools')
  await page.getByRole('button', { name: /^Принять книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await page.screenshot({ path: `${output}/02-production-identify.png`, animations: 'disabled' })
  await page.locator('h1').click()
  await page.keyboard.type('EDUS-1001')
  await page.keyboard.press('Enter')
  await page.getByText('Библиотечный API не настроен. Укажите адрес API, школу и устройство в runtime-config.js.', { exact: true }).waitFor()
  await page.screenshot({ path: `${output}/03-production-api-blocked.png`, animations: 'disabled' })
  assert(!(await page.locator('.accept-scan-layout').count()), 'Production build entered return scanning with no real reader/API')
  assert(!report.consoleErrors.length && !report.failedRequests.length, 'Production browser emitted console or request errors')
  report.checks.push('production bundle renders the approved UI, has no demo controls, and blocks reader identification until library API configuration/adapter is confirmed')
  await context.close()
  writeFileSync('.qa/production-boundary-report.json', `${JSON.stringify(report, null, 2)}\n`)
} finally { await browser.close() }
