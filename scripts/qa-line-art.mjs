/** Targeted UI verification of the EDUS monoline card illustration. */
import { createRequire } from 'node:module';
import { resolve, join } from 'node:path';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(resolve(runtime, 'package.json'));
const { chromium } = require('playwright');
const sharp = require('sharp');
const baseURL = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173';
const viewports = ['1920x1080', '1600x900', '1366x768'];
const assert = (condition, message) => { if (!condition) throw new Error(message); };
mkdirSync('.qa', { recursive: true });
mkdirSync('screenshots/line-art', { recursive: true });

const asset = readFileSync('public/assets/edus-card-schematic.svg', 'utf8');
const report = {
  timestamp: new Date().toISOString(), baseURL,
  method: 'Fresh isolated Chromium contexts. UI controls and scanner-like keyboard events only; read-only DOM measurement and mutation observation. Source SVG inspected as an asset, without importing application state.',
  asset: {
    path: 'public/assets/edus-card-schematic.svg',
    viewBox: asset.match(/viewBox="([^"]+)"/)?.[1],
    fills: [...new Set([...asset.matchAll(/\bfill="([^"]+)"/g)].map(match => match[1]))],
    strokes: [...new Set([...asset.matchAll(/\bstroke="([^"]+)"/g)].map(match => match[1]))],
    strokeWidths: [...new Set([...asset.matchAll(/\bstroke-width="([^"]+)"/g)].map(match => Number(match[1])))],
    rasterOrEffects: /<(?:image|filter|linearGradient|radialGradient)\b|data:image|box-shadow|drop-shadow/i.test(asset),
    goldFill: /fill="#(?:e39300|f0a000|e69a00|f59e0b)"/i.test(asset),
  },
  runs: [],
};

const browser = await chromium.launch({ headless: true });
report.browser = browser.version();
for (const size of viewports) {
  const [width, height] = size.split('x').map(Number);
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, locale: 'ru-RU', timezoneId: 'Asia/Qyzylorda' });
  const page = await context.newPage();
  page.setDefaultTimeout(6000);
  const dir = join('screenshots/line-art', size);
  mkdirSync(dir, { recursive: true });
  const run = { viewport: { width, height, dpr: 1 }, checks: [], screenshots: [], observations: [], failures: [], console: [], pageErrors: [], badResponses: [], requestFailures: [], externalRequests: [] };
  report.runs.push(run);
  page.on('console', event => { if (['warning', 'error'].includes(event.type())) run.console.push({ type: event.type(), text: event.text() }); });
  page.on('pageerror', error => run.pageErrors.push(error.message));
  page.on('response', response => { if (response.status() >= 400) run.badResponses.push({ status: response.status(), url: response.url() }); });
  page.on('requestfailed', request => run.requestFailures.push({ url: request.url(), error: request.failure()?.errorText }));
  page.on('request', request => { if (/^https?:/.test(request.url()) && new URL(request.url()).origin !== new URL(baseURL).origin) run.externalRequests.push(request.url()); });
  const button = name => page.getByRole('button', { name, exact: typeof name === 'string' });
  const heading = name => page.getByRole('heading', { name, exact: true }).first().waitFor();

  async function capture(name, noScroll = false) {
    await page.evaluate(() => document.fonts.ready);
    const measurements = await page.evaluate(() => {
      const box = selector => { const element = document.querySelector(selector); if (!element) return null; const r = element.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height, bottom: r.bottom }; };
      const controls = [...document.querySelectorAll('button')].filter(element => { const r = element.getBoundingClientRect(); return r.width && r.height && !element.closest('[inert]') && r.top >= 0 && r.bottom <= innerHeight; }).map(element => { const r = element.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return { name: element.getAttribute('aria-label') || element.textContent.trim(), width: r.width, height: r.height, disabled: element.disabled, unobscured: element.contains(hit) }; });
      const image = document.querySelector('.nfc-animation .edus-card');
      return { horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1, verticalOverflow: document.documentElement.scrollHeight > innerHeight + 1, card: box('.card-illustration'), animation: box('.nfc-animation'), controls, assetLoaded: !image || (image.complete && image.naturalWidth > 0), brokenImages: [...document.images].filter(image => !image.complete || !image.naturalWidth).map(image => image.src), frameworkOverlay: !!document.querySelector('vite-error-overlay') };
    });
    const path = join(dir, `${name}.png`);
    await page.screenshot({ path, animations: 'allow' });
    const metadata = await sharp(path).metadata();
    assert(metadata.width === width && metadata.height === height, `Wrong PNG dimensions for ${name}`);
    run.screenshots.push({ name, path, width, height, ...measurements });
    assert(!measurements.horizontalOverflow && (!noScroll || !measurements.verticalOverflow), `Overflow on ${name}`);
    assert(measurements.assetLoaded && !measurements.brokenImages.length && !measurements.frameworkOverlay, `Broken rendering on ${name}`);
    console.log(`[line-art ${size}] ${name}`);
  }

  async function scenario(name, action) {
    try { await action(); run.checks.push({ name, passed: true }); }
    catch (error) { run.failures.push({ name, message: error.message }); run.checks.push({ name, passed: false }); await page.screenshot({ path: join(dir, `FAIL-${name}.png`) }); console.error(`[line-art ${size}] FAIL ${name}: ${error.message}`); }
  }

  async function start(operation) {
    await page.goto(baseURL, { waitUntil: 'networkidle' }); await heading('Что будем делать?');
    await button(operation === 'issue' ? /^Выдать книги/ : /^Принять книги/).click();
    await heading(operation === 'issue' ? 'Кому выдаём книги?' : 'Кто возвращает книги?');
    await page.getByText('Приложите карту ученика', { exact: true }).waitFor();
  }
  async function scan(code) { await page.locator('h1').click(); await page.keyboard.type(code, { delay: 3 }); await page.keyboard.press('Enter'); }
  const cardBox = () => page.locator('.card-illustration').evaluate(element => { const r = element.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; });

  await scenario('issue-and-accept-identify', async () => {
    for (const operation of ['issue', 'accept']) {
      await start(operation);
      const manual = button('Найти вручную');
      const touch = await manual.evaluate(element => { const r = element.getBoundingClientRect(); return { width: r.width, height: r.height, unobscured: element.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)) }; });
      assert(touch.width >= 56 && touch.height >= 56 && touch.unobscured, `${operation}: manual search is small or obscured`);
      await capture(`01-${operation}-identify`, true);
      const initial = await cardBox();
      await page.waitForTimeout(2400);
      assert(await page.locator('.nfc-animation.state-idle').count() === 1 && await page.locator('.state-mark.success').count() === 0, 'Idle simulated successful card reading');
      assert(JSON.stringify(await cardBox()) === JSON.stringify(initial), 'Idle card moved');
      await manual.click(); await heading('Найдите читателя');
      assert(await page.getByPlaceholder('Фамилия, имя или номер карты').isVisible(), 'Manual reader lookup did not open');
      run.observations.push({ type: 'identify', operation, manual: touch, card: initial, idleStableMilliseconds: 2400 });
    }
  });

  await scenario('live-adapter-states', async () => {
    await start('accept');
    await scan('EDUS-NOT-FOUND'); await page.locator('.nfc-animation.state-error').waitFor();
    await capture('02-live-card-error', true);
    const observation = page.evaluate(() => new Promise(resolve => {
      const states = [];
      const record = () => { const element = document.querySelector('.nfc-animation'); if (element && element.className !== states.at(-1)) states.push(element.className); if (!element && states.length) { observer.disconnect(); clearTimeout(timer); resolve(states); } };
      const observer = new MutationObserver(record); observer.observe(document.body, { attributes: true, childList: true, subtree: true });
      const timer = setTimeout(() => { observer.disconnect(); resolve(states); }, 5000); record();
    }));
    await scan('EDUS-1001'); await page.locator('.reader-summary').waitFor();
    const states = await observation;
    assert(states.some(state => state.includes('state-reading')) && states.some(state => state.includes('state-success')), 'Live adapter did not produce reading then success');
    assert((await page.locator('.reader-summary').innerText()).includes('Сәрсенова Айша'), 'Live card did not select its reader');
    run.observations.push({ type: 'live-adapter', states });
    await capture('03-live-reader-selected', true);
  });

  await scenario('gallery-states-and-reduced-motion', async () => {
    await page.goto(`${baseURL}/#/components`, { waitUntil: 'networkidle' }); await heading('Один язык интерфейса');
    const switcher = page.locator('.state-switch');
    let baseline;
    for (const [label, state] of [['Ожидание', 'idle'], ['Чтение', 'reading'], ['Успех', 'success'], ['Ошибка', 'error']]) {
      await switcher.getByRole('button', { name: label, exact: true }).click(); await page.locator('.nfc-demo').scrollIntoViewIfNeeded();
      const bounds = await cardBox(); if (!baseline) baseline = bounds;
      assert(JSON.stringify(bounds) === JSON.stringify(baseline), `Card geometry changed for ${state}`);
      assert(await page.locator(`.nfc-animation.state-${state}`).isVisible(), `${state} not rendered`);
      if (state === 'success' || state === 'error') assert(await page.locator(`.state-mark.${state}`).isVisible(), `${state} marker missing`);
      else assert(await page.locator('.state-mark').count() === 0, `False result marker during ${state}`);
      if (state === 'reading') {
        const line = page.locator('.reading-line');
        const before = await line.evaluate(element => getComputedStyle(element).transform);
        await page.waitForTimeout(220);
        const after = await line.evaluate(element => getComputedStyle(element).transform);
        assert(before !== after, 'Reading line is not animated in normal motion');
      }
      await capture(`04-gallery-${state}`);
      if (size === '1600x900') { const path = join(dir, `card-${state}-detail.png`); await page.locator('.nfc-animation').screenshot({ path, animations: 'allow' }); run.observations.push({ type: 'detail-capture', state, path }); }
    }
    await page.emulateMedia({ reducedMotion: 'reduce' }); await switcher.getByRole('button', { name: 'Чтение', exact: true }).click();
    await page.waitForTimeout(100);
    assert(await page.locator('.reading-line').isVisible(), 'Reduced motion hid reading feedback');
    const animations = await page.locator('.nfc-animation').evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === 'running').length);
    assert(animations === 0 && await page.locator('.state-mark.success').count() === 0, 'Reduced motion is animated or falsely successful');
    await capture('05-reading-reduced-motion');
    run.observations.push({ type: 'gallery-geometry', stableBounds: baseline, reducedMotionRunningAnimations: animations });
  });
  run.externalRequests = [...new Set(run.externalRequests)];
  await context.close();
}
await browser.close();
report.asset.passed = !report.asset.rasterOrEffects && !report.asset.goldFill && report.asset.strokeWidths.every(width => width <= 3) && report.asset.fills.every(fill => fill === 'none');
report.passed = report.asset.passed && report.runs.every(run => !run.failures.length && !run.pageErrors.length && !run.console.some(event => event.type === 'error') && !run.badResponses.length && !run.requestFailures.length && !run.externalRequests.length);
writeFileSync('.qa/line-art-report.json', JSON.stringify(report, null, 2));
console.log(JSON.stringify({ passed: report.passed, asset: report.asset, runs: report.runs.map(run => ({ viewport: run.viewport, checks: run.checks, screenshots: run.screenshots.length, failures: run.failures, console: run.console, pageErrors: run.pageErrors, networkErrors: run.badResponses.length + run.requestFailures.length, externalRequests: run.externalRequests })) }, null, 2));
process.exitCode = report.passed ? 0 : 1;
