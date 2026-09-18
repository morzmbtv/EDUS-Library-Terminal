/** Browser QA for the reader-first issue scanning screen. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173'
const output = 'screenshots/issue-scan-reference'
mkdirSync(output, { recursive: true })
const report = { baseUrl, startedAt: new Date().toISOString(), runs: [], failures: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }

async function scan(page, code) {
  const scanSurface = page.locator('.issue-scanner-column').first()
  if (await scanSurface.count()) await scanSurface.click({ position: { x: 24, y: 24 } })
  else await page.locator('h1').click()
  await page.keyboard.type(code, { delay: 3 })
  await page.keyboard.press('Enter')
}

async function identifyIssue(page) {
  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  await page.getByRole('button', { name: /^Выдать книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  await scan(page, 'EDUS-1001')
  await page.locator('.issue-scan-layout').waitFor()
}

const browser = await chromium.launch({ headless: true })
try {
  for (const viewport of [{ width: 1280, height: 800 }, { width: 1536, height: 1024 }, { width: 1920, height: 1080 }, { width: 1366, height: 768 }]) {
    const context = await browser.newContext({ viewport, locale: 'ru-RU' })
    const page = await context.newPage()
    const run = { viewport, checks: [], consoleErrors: [], failedRequests: [] }
    report.runs.push(run)
    page.on('console', event => { if (event.type() === 'error') run.consoleErrors.push(event.text()) })
    page.on('pageerror', error => run.consoleErrors.push(error.message))
    page.on('requestfailed', request => run.failedRequests.push(request.url()))

    await identifyIssue(page)
    await page.getByRole('button', { name: 'Сканирование книг', exact: true }).count().catch(() => 0)
    assert(await page.locator('.issue-scan-navigation .stepper .complete').count() === 1, 'Reader step is not complete')
    assert(await page.locator('.issue-scan-navigation .stepper .current').count() === 1, 'Books step is not current')
    assert(await page.locator('.scan-reader-summary').getByText('Карта № EDUS-1001', { exact: true }).count() === 1, 'Reader card is not derived from selected reader')
    assert(await page.getByRole('button', { name: 'Ввести номер вручную', exact: true }).isVisible(), 'Manual inventory action is missing')
    assert(await page.getByRole('button', { name: 'Добавить по ISBN', exact: true }).isVisible(), 'ISBN action is missing')
    assert(await page.getByRole('button', { name: 'Очистить список', exact: true }).isDisabled(), 'Empty basket clear is not disabled')
    assert(await page.getByRole('button', { name: 'Перейти к подтверждению', exact: true }).isDisabled(), 'Empty basket confirmation is not disabled')
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)
    assert(!overflow, 'Issue scan page has horizontal overflow')
    if (viewport.width === 1280 || viewport.width === 1536) {
      const verticalOverflow = await page.evaluate(() => document.documentElement.scrollHeight > window.innerHeight + 1)
      assert(!verticalOverflow, `${viewport.width}×${viewport.height} empty scan page should fit without document scrolling`)
    }
    await page.screenshot({ path: join(output, `${viewport.width}x${viewport.height}-empty.png`), animations: 'disabled' })
    run.checks.push('reader panel, active step, empty basket, controls, and width')

    await page.getByRole('button', { name: 'Ввести номер вручную', exact: true }).click()
    await page.getByLabel('Инвентарный номер', { exact: true }).waitFor()
    await page.getByRole('button', { name: 'Закрыть', exact: true }).click()
    await page.getByRole('button', { name: 'Добавить по ISBN', exact: true }).click()
    await page.getByLabel('ISBN', { exact: true }).fill('9786010123456')
    await page.getByRole('button', { name: 'Найти книгу', exact: true }).click()
    await page.getByRole('heading', { name: 'Найдено издание', exact: true }).waitFor()
    assert(await page.getByText('ISBN обозначает издание, а не отдельную книгу.', { exact: true }).count() === 1, 'ISBN flow no longer explains edition-only matching')
    await page.getByRole('button', { name: 'Закрыть', exact: true }).click()
    run.checks.push('manual inventory and ISBN dialog flows')

    await scan(page, '000124')
    await page.locator('.book-basket').getByText('Алгебра. 7 класс', { exact: true }).waitFor()
    assert(!(await page.getByRole('button', { name: 'Очистить список', exact: true }).isDisabled()), 'Filled basket clear remains disabled')
    assert(!(await page.getByRole('button', { name: 'Перейти к подтверждению', exact: true }).isDisabled()), 'Filled basket confirmation remains disabled')
    await page.screenshot({ path: join(output, `${viewport.width}x${viewport.height}-filled.png`), animations: 'disabled' })
    run.checks.push('real scanner adapter adds an eligible copy')

    await page.getByRole('button', { name: 'Помощь', exact: true }).click()
    await page.getByRole('heading', { name: 'Как выдать книги', exact: true }).waitFor()
    await page.keyboard.type('000125')
    await page.keyboard.press('Enter')
    assert(await page.locator('.book-basket .basket-count').innerText() === '1', 'Help allowed a hidden scanner event to change the basket')
    await page.getByRole('button', { name: 'Понятно', exact: true }).click()
    assert(await page.locator('.book-basket .basket-count').innerText() === '1', 'Closing help reset the basket')
    run.checks.push('context help preserves basket and blocks hidden scans')

    await page.getByRole('button', { name: 'Сменить читателя', exact: true }).click()
    await page.getByRole('heading', { name: 'Выбрать другого читателя?', exact: true }).waitFor()
    assert(await page.locator('.book-basket .basket-count').innerText() === '1', 'Reader-change prompt reassigned or cleared books before consent')
    await page.getByRole('button', { name: 'Продолжить работу', exact: true }).click()
    run.checks.push('reader change requires explicit basket-clear consent')

    await page.getByRole('button', { name: 'Очистить список', exact: true }).click()
    await page.getByRole('heading', { name: 'Очистить список?', exact: true }).waitFor()
    await page.locator('.modal .modal-actions .btn.primary').click()
    assert(await page.locator('.book-basket .basket-count').innerText() === '0', 'Basket was not cleared')
    assert(await page.locator('.scan-reader-summary').getByText('Карта № EDUS-1001', { exact: true }).count() === 1, 'Clearing basket lost the reader')
    run.checks.push('clear confirmation preserves selected reader')

    assert(!run.consoleErrors.length && !run.failedRequests.length, 'Browser emitted console or request errors')
    await context.close()
  }
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error))
} finally {
  await browser.close()
}
report.finishedAt = new Date().toISOString()
report.passed = report.failures.length === 0 && report.runs.every(run => !run.consoleErrors.length && !run.failedRequests.length)
writeFileSync('.qa/issue-scan-reference-report.json', JSON.stringify(report, null, 2))
console.log(JSON.stringify({ passed: report.passed, runs: report.runs, failures: report.failures }, null, 2))
process.exitCode = report.passed ? 0 : 1
