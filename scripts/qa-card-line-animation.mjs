/** Focused visual and interaction check for the supplied EDUS card animation integration. */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules'
const require = createRequire(resolve(runtime, 'package.json'))
const { chromium } = require('playwright')
const baseUrl = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173'
const sourceUrl = 'file:///C:/Users/User/Desktop/EDUS_card_line_animation.html?autoplay=0'
const output = 'screenshots/card-line-animation'
mkdirSync(output, { recursive: true })

const report = { baseUrl, sourceUrl, startedAt: new Date().toISOString(), runs: [], failures: [] }
const assert = (value, message) => { if (!value) throw new Error(message) }
let browser = await chromium.launch({ headless: true })

async function openApp(context, operation) {
  const page = await context.newPage()
  const consoleErrors = []
  const failedRequests = []
  page.on('console', event => { if (event.type() === 'error') consoleErrors.push(event.text()) })
  page.on('pageerror', error => consoleErrors.push(error.message))
  page.on('requestfailed', request => failedRequests.push(request.url()))
  await page.goto(baseUrl, { waitUntil: 'networkidle' })
  await page.getByRole('button', { name: operation === 'issue' ? /^Выдать книги/ : /^Принять книги/ }).click()
  await page.getByRole('heading', { name: 'Приложите карту ученика', exact: true }).waitFor()
  return { page, consoleErrors, failedRequests }
}
async function scan(page, value) {
  await page.locator('h1').click()
  await page.keyboard.type(value, { delay: 3 })
  await page.keyboard.press('Enter')
}
async function capture(page, path) {
  await page.screenshot({ path: join(output, path), animations: 'allow' })
}

try {
  const sourceContext = await browser.newContext({ viewport: { width: 840, height: 700 }, deviceScaleFactor: 1 })
  const source = await sourceContext.newPage()
  await source.goto(sourceUrl)
  await source.locator('.card-scene').waitFor()
  const sourceCard = await source.locator('rect[x="212"][y="50"]').first().evaluate(element => ({ width: Number(element.getAttribute('width')), height: Number(element.getAttribute('height')) }))
  assert(sourceCard.width === 216 && sourceCard.height === 370, 'Unexpected source card dimensions')
  await capture(source, '00-source-idle.png')
  report.source = { card: sourceCard, screenshot: join(output, '00-source-idle.png') }
  await sourceContext.close()
  await browser.close()

  for (const [name, viewport] of [['1920x1080', { width: 1920, height: 1080 }], ['1366x768', { width: 1366, height: 768 }]]) {
    // The mock adapter is browser-memory based. A fresh browser makes each
    // viewport scenario independent, as it is in an actual kiosk session.
    browser = await chromium.launch({ headless: true })
    const context = await browser.newContext({ viewport, deviceScaleFactor: 1, locale: 'ru-RU' })
    const run = { viewport, name, checks: [], consoleErrors: [], failedRequests: [] }
    report.runs.push(run)
    for (const operation of ['issue', 'accept']) {
      const { page, consoleErrors, failedRequests } = await openApp(context, operation)
      const nfc = page.locator('.nfc-animation')
      const scene = page.locator('.card-scene')
      const card = page.locator('.edus-card-outline')
      const manual = page.getByRole('button', { name: 'Найти вручную', exact: true })
      assert(await nfc.evaluate(element => element.classList.contains('state-idle')), `${operation}: idle state missing`)
      const cardBox = await card.boundingBox()
      // getBoundingClientRect includes the original source's 1.7px outer stroke.
      // The compact 1366×768 layout deliberately scales the complete composition;
      // its ratio must still match the transferred source geometry.
      assert(cardBox && Math.abs(cardBox.width / cardBox.height - 217.7 / 371.7) < 0.01, `${operation}: card no longer preserves the source geometry (${cardBox?.width}×${cardBox?.height})`)
      if (viewport.height >= 900) {
        assert(Math.abs(cardBox.width - 217.7) < 1 && Math.abs(cardBox.height - 371.7) < 1, `${operation}: large-screen card is not rendered at source size (${cardBox.width}×${cardBox.height})`)
      }
      assert(await manual.isVisible(), `${operation}: manual search unavailable`)
      const manualBox = await manual.boundingBox()
      assert(manualBox && manualBox.width >= 56 && manualBox.height >= 56, `${operation}: manual touch target too small`)
      await capture(page, `${name}-${operation}-idle.png`)
      run.checks.push(`${operation}: idle, proportions, and manual search`)

      await scan(page, 'EDUS-NOT-FOUND')
      await page.locator('.nfc-animation.state-error .state-mark.error').waitFor()
      assert(await page.locator('.state-mark.success').count() === 0, `${operation}: error showed success`)
      await capture(page, `${name}-${operation}-error.png`)
      run.consoleErrors.push(...consoleErrors)
      run.failedRequests.push(...failedRequests)
      await page.close()

      // Capture the short reading state in a separate flow. Screenshot capture
      // can outlast the deliberately brief adapter delay on compact hardware.
      const readingFlow = await openApp(context, operation)
      const readingPage = readingFlow.page
      await scan(readingPage, operation === 'issue' ? 'EDUS-1001' : 'EDUS-1002')
      await readingPage.locator('.nfc-animation.state-reading .reading-line').waitFor()
      const readingStart = await readingPage.locator('.reading-line').getAttribute('transform')
      await capture(readingPage, `${name}-${operation}-reading.png`)
      await readingPage.waitForTimeout(120)
      const readingEnd = await readingPage.locator('.reading-line').getAttribute('transform')
      assert(readingStart !== readingEnd, `${operation}: source scan line did not move`)
      run.consoleErrors.push(...readingFlow.consoleErrors)
      run.failedRequests.push(...readingFlow.failedRequests)
      await readingPage.close()

      // A fresh identification flow verifies that success comes from the mock
      // adapter rather than from a state change following the error demo.
      const successFlow = await openApp(context, operation)
      const successPage = successFlow.page
      const selected = successPage.locator('.reader-summary')
      await scan(successPage, operation === 'issue' ? 'EDUS-1001' : 'EDUS-1002')
      // The workflow intentionally moves to the selected reader shortly after
      // the confirmed result. Wait for its reactive state first, then capture
      // the source's short result treatment while it is still on screen.
      await successPage.locator('.nfc-animation.state-success').waitFor()
      await successPage.waitForTimeout(32)
      assert(await successPage.locator('.state-mark.success').count() === 1, `${operation}: confirmed result has no success marker`)
      await capture(successPage, `${name}-${operation}-success.png`)
      await selected.waitFor()
      run.checks.push(`${operation}: error, reading line, success marker, and reader transition`)
      run.consoleErrors.push(...successFlow.consoleErrors)
      run.failedRequests.push(...successFlow.failedRequests)
      await successPage.close()
    }
    assert(!run.consoleErrors.length && !run.failedRequests.length, `${name}: browser console or request errors`)
    await context.close()
    await browser.close()
  }

  browser = await chromium.launch({ headless: true })
  const reducedContext = await browser.newContext({ viewport: { width: 1366, height: 768 }, reducedMotion: 'reduce', deviceScaleFactor: 1 })
  const { page: reduced, consoleErrors, failedRequests } = await openApp(reducedContext, 'issue')
  await scan(reduced, 'EDUS-1001')
  await reduced.locator('.nfc-animation.state-reading .reading-line').waitFor()
  const firstTransform = await reduced.locator('.reading-line').getAttribute('transform')
  await reduced.waitForTimeout(160)
  const secondTransform = await reduced.locator('.reading-line').getAttribute('transform')
  assert(firstTransform === secondTransform, 'Reduced motion scan line moved')
  assert(await reduced.locator('.state-mark.success').count() === 0, 'Reduced motion invented a successful read')
  await capture(reduced, '1366x768-issue-reading-reduced-motion.png')
  report.reducedMotion = { passed: true, screenshot: join(output, '1366x768-issue-reading-reduced-motion.png'), consoleErrors, failedRequests }
  await reducedContext.close()
  await browser.close()
} catch (error) {
  report.failures.push(error instanceof Error ? error.message : String(error))
} finally {
  await browser.close()
}
report.finishedAt = new Date().toISOString()
report.passed = report.failures.length === 0 && report.runs.every(run => !run.consoleErrors.length && !run.failedRequests.length) && report.reducedMotion?.passed === true
writeFileSync('.qa/card-line-animation-report.json', JSON.stringify(report, null, 2))
console.log(JSON.stringify({ passed: report.passed, source: report.source, runs: report.runs.map(run => ({ name: run.name, checks: run.checks, consoleErrors: run.consoleErrors, failedRequests: run.failedRequests })), reducedMotion: report.reducedMotion, failures: report.failures }, null, 2))
process.exitCode = report.passed ? 0 : 1
