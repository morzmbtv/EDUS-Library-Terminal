/** Targeted rendered verification of the supplied EDUS identification reference. */
import { createRequire } from 'node:module';
import { resolve, join } from 'node:path';
import { mkdirSync, readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs';

const runtime = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(resolve(runtime, 'package.json'));
const { chromium } = require('playwright');
const sharp = require('sharp');
const baseURL = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173';
const chosen = process.argv.find(arg => arg.startsWith('--viewport='))?.split('=')[1];
const viewports = chosen ? [chosen] : ['1536x1024', '1920x1080', '1600x900', '1366x768', '840x900'];
const assert = (value, message) => { if (!value) throw new Error(message); };
const shotRoot = 'screenshots/identify-reference';
mkdirSync('.qa', { recursive: true });
mkdirSync(shotRoot, { recursive: true });
const referenceSource = 'C:/Users/User/AppData/Local/Temp/codex-clipboard-ae6a482a-904b-4386-b019-52cc2281f139.png';
if (existsSync(referenceSource)) copyFileSync(referenceSource, join(shotRoot, 'reference.png'));
const report = {
  timestamp: new Date().toISOString(), baseURL,
  method: 'Fresh isolated Chromium context for each scenario. Real UI controls and scanner-like keyboard events. Read-only DOM geometry/animation observations; no state injection, application imports, storage writes or changes to the user browser.',
  reference: { source: referenceSource, localCopy: join(shotRoot, 'reference.png'), suppliedSize: [1536, 1024] },
  asset: {}, runs: [],
};
const asset = readFileSync('public/assets/edus-card-schematic.svg', 'utf8');
report.asset = {
  path: 'public/assets/edus-card-schematic.svg', viewBox: asset.match(/viewBox="([^"]+)"/)?.[1],
  externalReferences: [...asset.matchAll(/(?:href|src)="(https?:[^\"]+)"/g)].map(m => m[1]),
  embeddedRaster: /<image\b|data:image/i.test(asset),
  filtersOrGradients: /<(?:filter|linearGradient|radialGradient)\b/i.test(asset),
};
const browser = await chromium.launch({ headless: true });
report.browser = browser.version();

for (const size of viewports) {
  const [width, height] = size.split('x').map(Number);
  const dir = join(shotRoot, size);
  mkdirSync(dir, { recursive: true });
  const run = { viewport: { width, height, dpr: 1 }, checks: [], screenshots: [], observations: [], failures: [], console: [], pageErrors: [], requestFailures: [], badResponses: [], externalRequests: [] };
  report.runs.push(run);
  let context, page;
  const button = name => page.getByRole('button', { name, exact: typeof name === 'string' });
  const help = () => page.getByRole('dialog', { name: 'Как это работает?', exact: true });
  const box = selector => page.locator(selector).first().evaluate(element => { const r = element.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height, centerX: r.x + r.width / 2, bottom: r.bottom }; });
  const cardBox = () => box('.card-illustration');
  const sameBox = (a, b) => ['x', 'y', 'width', 'height'].every(key => Math.abs(a[key] - b[key]) < .5);
  const heading = text => page.getByRole('heading', { name: text, exact: true }).first().waitFor();
  async function open(operation) {
    await button(operation === 'issue' ? /^Выдать книги/ : /^Принять книги/).click();
    await heading('Приложите карту ученика');
    assert(await page.locator('.nfc-animation.state-idle').isVisible(), 'Identification did not start idle');
    assert(await page.locator('.scanner-panel').count() === 0, 'Book scanning exposed before selecting reader');
  }
  async function scan(code, focus = true) {
    if (focus) await page.locator('h1').first().click();
    await page.keyboard.type(code, { delay: 3 }); await page.keyboard.press('Enter');
  }
  async function selected(expected, operation) {
    await page.locator('.reader-summary').waitFor();
    const name = (await page.locator('.reader-summary .reader-identity strong').innerText()).trim();
    assert(name === expected, `Wrong reader selected: ${name}`);
    await heading(operation === 'issue' ? 'Сканируйте книги' : 'Какие книги принимаем?');
    return name;
  }
  async function screenshot(name, { noScroll = true, measure = true } = {}) {
    const path = join(dir, `${name}.png`);
    // Capture transient reading/success states immediately; inspect the static PNG afterwards.
    await page.screenshot({ path, animations: 'allow' });
    const dimensions = await sharp(path).metadata();
    assert(dimensions.width === width && dimensions.height === height, `Incorrect PNG dimensions for ${name}`);
    let layout = {};
    if (measure) layout = await page.evaluate(() => {
      const visible = e => { const r = e.getBoundingClientRect(), s = getComputedStyle(e); return r.width > 0 && r.height > 0 && s.display !== 'none' && s.visibility !== 'hidden'; };
      const bounds = selector => { const e = document.querySelector(selector); if (!e) return null; const r = e.getBoundingClientRect(), style = getComputedStyle(e); return { x: r.x, y: r.y, width: r.width, height: r.height, bottom: r.bottom, centerX: r.x + r.width / 2, fontSize: style.fontSize, lineHeight: style.lineHeight }; };
      const controls = [...document.querySelectorAll('button')].filter(e => visible(e) && !e.closest('[inert]')).map(e => {
        const r = e.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        return { name: e.getAttribute('aria-label') || e.textContent.trim(), width: r.width, height: r.height, x: r.x, y: r.y, bottom: r.bottom, disabled: e.disabled, inViewport: r.top >= 0 && r.left >= 0 && r.bottom <= innerHeight && r.right <= innerWidth, unobscured: e.contains(hit) };
      });
      const clippedHelpText = [...document.querySelectorAll('.context-help h2,.context-help p')].filter(visible).filter(e => e.scrollWidth > e.clientWidth + 1).map(e => ({ text: e.textContent.trim(), width: e.clientWidth, scrollWidth: e.scrollWidth }));
      const helpTitle = document.querySelector('.context-help h2'), helpClose = document.querySelector('.context-help [aria-label="Закрыть помощь"]');
      let helpHeadingCloseCollision = false;
      if (helpTitle && helpClose) { const a = helpTitle.getBoundingClientRect(), b = helpClose.getBoundingClientRect(); helpHeadingCloseCollision = a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top; }
      return {
        horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1, verticalOverflow: document.documentElement.scrollHeight > innerHeight + 1,
        controls, card: bounds('.card-illustration'), animation: bounds('.nfc-animation'), heading: bounds('.identify-content h1'), subtitle: bounds('.identify-content > p'), manual: bounds('.identify-alternative button'), help: bounds('.context-help'),
        clippedHelpText, helpHeadingCloseCollision, brokenImages: [...document.images].filter(i => !i.complete || !i.naturalWidth).map(i => i.src), frameworkOverlay: !!document.querySelector('vite-error-overlay'),
      };
    });
    run.screenshots.push({ name, path, width, height, ...layout });
    if (measure) {
      assert(!layout.horizontalOverflow && (!noScroll || !layout.verticalOverflow), `Overflow in ${name}`);
      assert(!layout.frameworkOverlay && !layout.brokenImages.length, `Broken rendering in ${name}`);
      assert(!layout.clippedHelpText.length && !layout.helpHeadingCloseCollision, `Help text clipped or collides with close button in ${name}: ${JSON.stringify(layout.clippedHelpText)}`);
      const small = layout.controls.filter(c => c.width < 55.5 || c.height < 55.5);
      assert(!small.length, `Touch target below 56px in ${name}: ${JSON.stringify(small)}`);
      const clipped = layout.controls.filter(c => !c.disabled && (!c.inViewport || !c.unobscured));
      assert(!clipped.length, `Clipped/obscured control in ${name}: ${JSON.stringify(clipped)}`);
    }
    console.log(`[identify-reference ${size}] ${name}`);
    return layout;
  }
  async function scenario(name, action) {
    context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, locale: 'ru-RU', timezoneId: 'Asia/Qyzylorda' });
    page = await context.newPage(); page.setDefaultTimeout(6500);
    page.on('console', e => { if (['warning', 'error'].includes(e.type())) run.console.push({ scenario: name, type: e.type(), text: e.text() }); });
    page.on('pageerror', e => run.pageErrors.push({ scenario: name, message: e.message }));
    page.on('requestfailed', r => run.requestFailures.push({ scenario: name, url: r.url(), error: r.failure()?.errorText }));
    page.on('response', r => { if (r.status() >= 400) run.badResponses.push({ scenario: name, url: r.url(), status: r.status() }); });
    page.on('request', r => { if (/^https?:/.test(r.url()) && new URL(r.url()).origin !== new URL(baseURL).origin) run.externalRequests.push(r.url()); });
    try {
      await page.goto(baseURL, { waitUntil: 'networkidle' }); await heading('Что будем делать?');
      await page.evaluate(() => document.fonts.ready);
      await action(); run.checks.push({ name, passed: true });
    } catch (e) {
      run.checks.push({ name, passed: false, message: e.message }); run.failures.push({ name, message: e.message });
      console.error(`[identify-reference ${size}] FAIL ${name}: ${e.message}`);
      try { await page.screenshot({ path: join(dir, `FAIL-${name}.png`) }); writeFileSync(join('.qa', `identify-reference-${size}-${name}.txt`), await page.locator('body').innerText()); } catch { /* Preserve the first error. */ }
    } finally { await context.close(); }
  }

  await scenario('reference-open-closed-help-and-geometry', async () => {
    await open('issue');
    const closed = { card: await cardBox(), heading: await box('.identify-content h1'), manual: await button('Найти вручную').boundingBox() };
    await button('Помощь').click(); await help().waitFor();
    assert(await help().locator('li').count() === 3, 'Identification help should have three steps');
    const opened = await screenshot('01-help-open');
    assert(JSON.stringify(await cardBox()) === JSON.stringify(closed.card), 'Help moved the card');
    assert(JSON.stringify(await box('.identify-content h1')) === JSON.stringify(closed.heading), 'Help moved the heading');
    assert(JSON.stringify(await button('Найти вручную').boundingBox()) === JSON.stringify(closed.manual), 'Help moved manual search');
    assert(Math.abs(closed.card.centerX - width / 2) <= 2, 'Card is not centered in viewport');
    if (size === '1536x1024') {
      assert(Math.abs(closed.card.width - 216) <= 8 && Math.abs(closed.card.height - 370) <= 8, 'Card differs substantially from reference dimensions');
      assert(Math.abs(closed.card.y - 345) <= 25, 'Card top differs substantially from reference');
      assert(Math.abs(opened.help.width - 350) <= 16, 'Help width differs substantially from reference');
      assert(opened.help.x > closed.card.x + closed.card.width, 'Wide-screen help is not on the right');
    }
    await scan('EDUS-1001', false); await page.waitForTimeout(1050);
    assert(await help().isVisible() && await page.locator('.nfc-animation.state-idle').count() === 1, 'Scan executed behind help');
    await page.keyboard.press('Escape'); await help().waitFor({ state: 'hidden' });
    assert(await button('Помощь').evaluate(e => e === document.activeElement), 'Escape did not restore help opener focus');
    await screenshot('02-help-closed');
    await button('Помощь').click(); await help().waitFor(); await button('Закрыть помощь').click();
    assert(await button('Помощь').evaluate(e => e === document.activeElement), 'Close button did not restore help opener focus');
    await button('Найти вручную').click(); await heading('Найдите читателя');
    run.observations.push({ type: 'reference-geometry', closed, help: opened.help, helpDoesNotMoveComposition: true, newScanBlockedWhileHelpOpen: true });
  });

  if (width < 1000) {
    writeFileSync(join('.qa', `identify-reference-${size}.json`), JSON.stringify(run, null, 2));
    continue;
  }

  await scenario('idle-stability-both-operations', async () => {
    for (const op of ['issue', 'accept']) {
      if (op === 'accept') { await button('На главную').click(); await heading('Что будем делать?'); }
      await open(op); const initial = await cardBox();
      await screenshot(`03-${op}-idle`);
      await page.waitForTimeout(2200);
      assert(await page.locator('.nfc-animation.state-idle').count() === 1 && await page.locator('.state-mark.success').count() === 0, 'Idle generated false success');
      assert(JSON.stringify(await cardBox()) === JSON.stringify(initial), 'Idle moved the card');
    }
  });

  await scenario('live-error-retry-reading-success', async () => {
    await open('accept'); const baseline = await cardBox();
    await scan('EDUS-NOT-FOUND'); await page.locator('.nfc-animation.state-error').waitFor();
    assert(await page.locator('.state-mark.success').count() === 0 && await page.getByRole('alert').isVisible(), 'Unknown card lacked error or showed success');
    await screenshot('04-error');
    // Observe short real adapter states before sending input; screenshot work must not postpone state observation.
    const observation = page.evaluate(() => new Promise(resolve => {
      const states = [], readingPositions = [];
      let lastState = '', frame = 0;
      const finish = () => { cancelAnimationFrame(frame); clearTimeout(timeout); resolve({ states, readingPositions }); };
      const record = () => {
        const animation = document.querySelector('.nfc-animation'), card = document.querySelector('.card-illustration');
        if (!animation || !card) { if (states.some(s => s.state.includes('state-reading'))) return finish(); }
        else {
          const state = animation.className;
          if (state !== lastState) { const r = card.getBoundingClientRect(); states.push({ state, card: { x: r.x, y: r.y, width: r.width, height: r.height }, successMark: !!animation.querySelector('.state-mark.success') }); lastState = state; }
          const line = animation.querySelector('.reading-line'); if (line) readingPositions.push(getComputedStyle(line).top);
        }
        frame = requestAnimationFrame(record);
      };
      const timeout = setTimeout(finish, 5000); record();
    }));
    const readingCapture = page.locator('.nfc-animation.state-reading').waitFor().then(() => screenshot('05-reading', { measure: false }));
    const successCapture = page.locator('.nfc-animation.state-success').waitFor().then(() => screenshot('06-success', { measure: false }));
    await scan('EDUS-1001');
    await Promise.all([readingCapture, successCapture, selected('Сәрсенова Айша Ерланқызы', 'accept')]);
    const observed = await observation;
    assert(new Set(observed.readingPositions).size > 1, 'Reading line did not move');
    assert(observed.states.some(s => s.state.includes('state-reading')) && observed.states.some(s => s.state.includes('state-success') && s.successMark), 'Real adapter did not show reading and successful check');
    for (const state of observed.states.filter(s => /state-(reading|success)/.test(s.state))) assert(sameBox(state.card, baseline), `State moved the card: ${JSON.stringify({ baseline, state })}`);
    await screenshot('07-accept-reader-selected');
    run.observations.push({ type: 'real-adapter-states', ...observed, stationaryCard: baseline });
  });

  await scenario('pending-identification-kept-behind-help', async () => {
    await open('issue'); await scan('EDUS-1002'); await page.locator('.nfc-animation.state-reading').waitFor();
    await button('Помощь').click(); await help().waitFor();
    await page.waitForTimeout(1150);
    assert(await help().isVisible(), 'Completed request unexpectedly closed help');
    assert(await page.locator('.reader-summary').count() === 0, 'Completed request navigated behind help');
    assert(await page.locator('.nfc-animation.state-success').count() === 1, 'Pending successful response was not retained');
    await screenshot('08-pending-success-help-open');
    await button('Закрыть помощь').click();
    await selected('Нұрланұлы Әлихан', 'issue');
    run.observations.push({ type: 'pending-identification', heldWhileHelpOpen: true, readerAfterClose: 'Нұрланұлы Әлихан' });
  });

  await scenario('cancel-inflight-does-not-navigate-late', async () => {
    await open('issue'); await scan('EDUS-1001'); await page.locator('.nfc-animation.state-reading').waitFor();
    assert(await button('Назад').isEnabled(), 'Cancel navigation disabled during identification');
    await button('Назад').click(); await heading('Что будем делать?');
    await page.waitForTimeout(1200); await heading('Что будем делать?');
    assert(await page.locator('.reader-summary').count() === 0 && await page.locator('.nfc-animation').count() === 0, 'Late response returned to cancelled operation');
    await open('accept');
    assert(await page.locator('.nfc-animation.state-idle').isVisible(), 'New operation inherited old identification state');
    await scan('EDUS-1002'); await selected('Нұрланұлы Әлихан', 'accept');
  });

  await scenario('operation-route-cancels-inflight-identification', async () => {
    await open('issue');
    const documentOrigin = await page.evaluate(() => performance.timeOrigin);
    await scan('EDUS-1001'); await page.locator('.nfc-animation.state-reading').waitFor();
    await page.goto(`${baseURL}/#/accept`);
    assert(await page.evaluate(() => performance.timeOrigin) === documentOrigin, 'Route check reloaded instead of performing same-document navigation');
    await heading('Приложите карту ученика');
    assert((await page.locator('.topline').innerText()).includes('Приём книг'), 'Route did not switch operation context');
    await page.waitForTimeout(1200);
    assert(await page.locator('.nfc-animation.state-idle').isVisible() && await page.locator('.reader-summary').count() === 0, 'Old issue response contaminated accept context');
    await scan('EDUS-1002'); await selected('Нұрланұлы Әлихан', 'accept');
    run.observations.push({ type: 'same-document-route-cancellation', from: 'issue', to: '#/accept', previousCard: 'EDUS-1001', selectedAfterNewScan: 'Нұрланұлы Әлихан' });
  });

  await scenario('manual-search-distinct-readers-preserves-context', async () => {
    for (const [operation, query, name] of [['issue', 'Айша', 'Сәрсенова Айша Ерланқызы'], ['accept', 'Әлихан', 'Нұрланұлы Әлихан']]) {
      if (operation === 'accept') { await button('На главную').click(); await heading('Что будем делать?'); }
      await open(operation); await button('Найти вручную').click(); await heading('Найдите читателя');
      const input = page.getByPlaceholder('Фамилия, имя или номер карты');
      await input.click(); await page.keyboard.type(query, { delay: 45 });
      await page.getByRole('button', { name: new RegExp(name) }).waitFor();
      assert(await input.inputValue() === query && await input.evaluate(e => e === document.activeElement), 'Scanner stole manual search input');
      await button('Помощь').click(); await page.getByRole('dialog', { name: 'Как найти читателя' }).waitFor();
      await page.keyboard.press('Escape');
      assert(await input.inputValue() === query && await input.evaluate(e => e === document.activeElement), 'Search help lost query/focus');
      await page.getByRole('button', { name: new RegExp(name) }).click(); await selected(name, operation);
      await screenshot(`09-manual-${operation}-selected`);
      run.observations.push({ type: 'manual-reader', operation, query, selected: name });
    }
  });

  await scenario('reduced-motion-real-states', async () => {
    await page.emulateMedia({ reducedMotion: 'reduce' }); await open('accept');
    await scan('EDUS-NOT-FOUND'); await page.locator('.nfc-animation.state-reading').waitFor();
    assert(await page.locator('.reading-line').isVisible(), 'Reduced motion hid reading feedback');
    const first = await page.locator('.reading-line').evaluate(e => getComputedStyle(e).top);
    const running = await page.locator('.nfc-animation').evaluate(e => e.getAnimations({ subtree: true }).filter(a => a.playState === 'running').length);
    await screenshot('10-reduced-motion-reading', { measure: false });
    const second = await page.locator('.reading-line').evaluate(e => getComputedStyle(e).top);
    assert(running === 0 && first === second, 'Reduced motion still animates');
    await page.locator('.nfc-animation.state-error').waitFor(); await screenshot('11-reduced-motion-error');
    await scan('EDUS-1001'); await page.locator('.nfc-animation.state-success').waitFor();
    await screenshot('12-reduced-motion-success', { measure: false });
    await selected('Сәрсенова Айша Ерланқызы', 'accept');
    run.observations.push({ type: 'reduced-motion', runningAnimations: running, visibleStaticReadingLine: true, errorAndSuccessFromAdapter: true });
  });
  run.externalRequests = [...new Set(run.externalRequests)];
  writeFileSync(join('.qa', `identify-reference-${size}.json`), JSON.stringify(run, null, 2));
}
await browser.close();

const targetScreenshot = join(shotRoot, '1536x1024/01-help-open.png');
if (existsSync(targetScreenshot) && existsSync(join(shotRoot, 'reference.png'))) {
  await sharp({ create: { width: 3072, height: 1024, channels: 3, background: '#eef2f7' } }).composite([
    { input: await sharp(join(shotRoot, 'reference.png')).resize(1536, 1024, { fit: 'contain', background: '#fff' }).png().toBuffer(), left: 0, top: 0 },
    { input: targetScreenshot, left: 1536, top: 0 },
  ]).png().toFile(join(shotRoot, 'comparison-1536x1024.png'));
  report.comparison = { path: join(shotRoot, 'comparison-1536x1024.png'), width: 3072, height: 1024, method: 'Side-by-side at 1:1 size: source reference on the left, unchanged full viewport browser capture on the right.' };
}
report.passed = report.asset.externalReferences.length === 0 && !report.asset.filtersOrGradients && report.runs.every(r => !r.failures.length && !r.pageErrors.length && !r.console.some(e => e.type === 'error') && !r.requestFailures.length && !r.badResponses.length && !r.externalRequests.length);
writeFileSync('.qa/identify-reference-report.json', JSON.stringify(report, null, 2));
console.log(JSON.stringify({ passed: report.passed, asset: report.asset, runs: report.runs.map(r => ({ viewport: r.viewport, passed: r.checks.filter(c => c.passed).length, total: r.checks.length, screenshots: r.screenshots.length, failures: r.failures, console: r.console, pageErrors: r.pageErrors, networkErrors: r.requestFailures.length + r.badResponses.length, externalRequests: r.externalRequests })) }, null, 2));
process.exitCode = report.passed ? 0 : 1;
