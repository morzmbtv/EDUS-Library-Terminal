/**
 * Reproducible UI-only EDUS acceptance and screenshot suite.
 * Uses a fresh isolated Chromium context for every viewport. It never imports
 * application code, reads storage, injects state, or connects to a user browser.
 * Run: node scripts/qa-browser.mjs [--viewport=1600x900] [--quick]
 * Current-interface evidence is written under .qa/regression2 and
 * screenshots/regression2; original baseline captures are preserved.
 */
import { createRequire } from 'node:module';
import { resolve, join } from 'node:path';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';

const dependencyRoot = process.env.CODEX_NODE_MODULES
  ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(resolve(dependencyRoot, 'package.json'));
const { chromium } = require('playwright');
const sharp = require('sharp');
const reportRoot = process.env.EDUS_QA_REPORT_ROOT ?? join('.qa', 'regression2');
const screenshotRoot = process.env.EDUS_QA_SCREENSHOT_ROOT ?? join('screenshots', 'regression2');
async function composeOverview() {
  // Assemble current full-size captures; no application state is modified.
  const main = ['01-home', '02-identify', '09-scan-two', '10-confirmation'];
  const mainDir = join(screenshotRoot, '1600x900');
  if (main.every(name => existsSync(join(mainDir, `${name}.png`)))) {
    const cells = await Promise.all(main.map((name, i) => sharp(join(mainDir, `${name}.png`)).resize(800, 450).png().toBuffer().then(input => ({ input, left: (i % 2) * 824 + 24, top: Math.floor(i / 2) * 474 + 24 }))));
    await sharp({ create: { width: 1672, height: 972, channels: 3, background: '#dce6f0' } }).composite(cells).png().toFile(join(screenshotRoot, 'overview.png'));
  }
}
if (process.argv.includes('--compose-only')) { await composeOverview(); process.exit(0); }
const baseURL = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173';
const requested = process.argv.find(arg => arg.startsWith('--viewport='))?.split('=')[1];
const quick = process.argv.includes('--quick');
const targetSizes = requested ? [requested] : ['1920x1080', '1600x900', '1366x768', '390x844'];
mkdirSync(reportRoot, { recursive: true });
mkdirSync(screenshotRoot, { recursive: true });
const report = {
  timestamp: new Date().toISOString(), baseURL,
  browser: 'Bundled Playwright Chromium, fresh isolated contexts',
  reason: 'User requested full PNG exports and permits Playwright. Primary built-in browser inspection was performed separately; its documented APIs provide inline screenshots but no local PNG artifact export.',
  method: 'Visible UI interactions only; DOM measurements are read-only; no app imports or storage mutation.',
  runs: [],
};
const browser = await chromium.launch({ headless: true });

function check(condition, message) { if (!condition) throw new Error(message); }

for (const size of targetSizes) {
  const [width, height] = size.split('x').map(Number);
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, locale: 'ru-RU', timezoneId: 'Asia/Qyzylorda' });
  const page = await context.newPage();
  page.setDefaultTimeout(6000);
  const dir = join(screenshotRoot, size);
  mkdirSync(dir, { recursive: true });
  const run = { timestamp: new Date().toISOString(), viewport: { width, height, dpr: 1 }, checks: [], screenshots: [], console: [], pageErrors: [], requestFailures: [], badResponses: [], externalRequests: [], failures: [] };
  report.runs.push(run);
  page.on('console', msg => { if (['warning', 'error'].includes(msg.type())) run.console.push({ type: msg.type(), text: msg.text() }); });
  page.on('pageerror', error => run.pageErrors.push(error.message));
  page.on('requestfailed', req => run.requestFailures.push({ url: req.url(), error: req.failure()?.errorText }));
  page.on('response', response => { if (response.status() >= 400) run.badResponses.push({ url: response.url(), status: response.status() }); });
  page.on('request', req => { if (/^https?:/.test(req.url()) && new URL(req.url()).origin !== new URL(baseURL).origin) run.externalRequests.push(req.url()); });
  let currentStage = 'load';
  const button = name => page.getByRole('button', { name, exact: typeof name === 'string' });
  const click = async name => { await button(name).click(); };
  const heading = async text => { await page.getByRole('heading', { name: text, exact: true }).first().waitFor({ state: 'visible' }); };
  const text = async (locator) => (await locator.textContent()) ?? '';

  async function capture(name, { fullPage = false, source = page, strictNoScroll = false } = {}) {
    currentStage = name;
    await source.waitForTimeout(60);
    await source.evaluate(() => document.fonts.ready);
    const measurement = await source.evaluate(() => {
      const viewport = { width: innerWidth, height: innerHeight };
      const visible = e => { const r = e.getBoundingClientRect(); const s = getComputedStyle(e); return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none'; };
      const controls = [...document.querySelectorAll('button, a.btn, input, select')].filter(visible).map(e => {
        const r = e.getBoundingClientRect();
        return { text: e.getAttribute('aria-label') || e.textContent?.trim().slice(0, 100) || e.getAttribute('placeholder') || e.tagName, tag: e.tagName, width: Math.round(r.width * 10) / 10, height: Math.round(r.height * 10) / 10, top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right), disabled: e.hasAttribute('disabled'), inViewport: r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth, keyboard: !!e.closest('.touch-keyboard') };
      });
      const clipped = [...document.querySelectorAll('h1,h2,h3,p,strong,label,button')].filter(visible).filter(e => e.clientWidth > 0 && e.scrollWidth > e.clientWidth + 2 && getComputedStyle(e).overflowX !== 'auto').map(e => ({ tag: e.tagName, text: e.textContent?.trim().slice(0, 110), clientWidth: e.clientWidth, scrollWidth: e.scrollWidth }));
      const tinyText = [...document.querySelectorAll('h1,h2,h3,p,span,strong,label,button,small')].filter(visible).filter(e => e.children.length === 0 && e.textContent?.trim()).map(e => ({ text: e.textContent.trim().slice(0, 80), size: parseFloat(getComputedStyle(e).fontSize) })).filter(x => x.size < 16);
      return { viewport, document: { width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight }, horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1, verticalOverflow: document.documentElement.scrollHeight > innerHeight + 1, controls, smallTouch: controls.filter(c => c.inViewport && (c.width < 55.5 || c.height < 55.5)), clipped, tinyText, title: document.title, h1: document.querySelector('h1')?.textContent, overlay: !!document.querySelector('vite-error-overlay'), brokenImages: [...document.images].filter(i => !i.complete || !i.naturalWidth).map(i => i.src) };
    });
    const path = join(dir, `${name}.png`);
    await source.screenshot({ path, fullPage, animations: 'allow' });
    const meta = await sharp(path).metadata();
    const item = { name, path, fullPage, width: meta.width, height: meta.height, ...measurement };
    run.screenshots.push(item);
    check(meta.width === width && (fullPage || meta.height === height), `PNG dimensions mismatch for ${name}`);
    if (measurement.horizontalOverflow) run.failures.push({ stage: name, kind: 'layout', message: `Horizontal overflow ${measurement.document.width}/${width}` });
    if (strictNoScroll && width >= 900 && measurement.verticalOverflow) run.failures.push({ stage: name, kind: 'layout', message: `Unexpected page scroll ${measurement.document.height}/${height}` });
    if (measurement.overlay || measurement.brokenImages.length) run.failures.push({ stage: name, kind: 'render', message: 'Framework overlay or broken image', evidence: measurement });
    console.log(`[${size}] ${name}`);
    return item;
  }

  async function scenario(name, fn) {
    try { await fn(); run.checks.push({ name, passed: true }); }
    catch (error) {
      run.checks.push({ name, passed: false, error: error.message });
      run.failures.push({ stage: currentStage, kind: 'interaction', scenario: name, message: error.message });
      try { await capture(`FAIL-${name.replace(/[^a-z0-9-]/gi, '-')}`); writeFileSync(join(reportRoot, `${size}-${name}-failure-dom.txt`), await page.locator('body').innerText()); } catch { /* Preserve the original interaction failure. */ }
      console.error(`[${size}] FAILED ${name}: ${error.message}`);
      return false;
    }
    return true;
  }

  async function manual(code, beforeSubmitName) {
    await click('Ввести код вручную');
    await heading('Введите код книги');
    await page.getByLabel('Код книги', { exact: true }).fill(code);
    if (beforeSubmitName) await capture(beforeSubmitName);
    await click('Найти книгу');
  }
  async function selectReader(searchTerm = 'Айша', capturePrefix = '') {
    await click('Найти вручную');
    await heading('Найдите читателя');
    const input = page.getByPlaceholder('Фамилия, имя или номер карты');
    if (capturePrefix) {
      await capture(`${capturePrefix}-search-empty`);
      await input.fill('Несуществующий читатель');
      await page.getByText('Читатель не найден', { exact: true }).waitFor();
      await capture(`${capturePrefix}-search-none`);
    }
    await input.fill(searchTerm);
    const result = page.getByRole('button', { name: /Сәрсенова Айша Ерланқызы/ });
    await result.waitFor();
    if (capturePrefix) await capture(`${capturePrefix}-search-found`);
    await result.click();
  }
  async function settings() { await click('Настройки и диагностика'); await heading('Настройки и диагностика'); }
  async function demoTools() { await settings(); const control = button('Панель сценариев'); if (await control.getAttribute('aria-expanded') !== 'true') await control.click(); }
  async function homeFromSuccess() { await click('На главную'); await heading('Что будем делать?'); }
  async function finalSubmit(operation, quantity) {
    await click('Продолжить'); await heading('Всё верно?');
    await click(new RegExp(`^${operation} ${quantity} книг`));
    await heading(operation === 'Выдать' ? 'Книги выданы' : 'Книги приняты');
  }
  async function goHomeWithDiscard() {
    const home = page.getByRole('button', { name: 'На главную', exact: true });
    await home.first().click();
    const leave = page.getByRole('dialog', { name: 'Завершить без сохранения?' });
    if (await leave.isVisible()) await leave.getByRole('button', { name: 'На главную', exact: true }).click();
    await heading('Что будем делать?');
  }

  try {
    await page.goto(baseURL, { waitUntil: 'networkidle' });
    check((await page.title()).includes('EDUS'), 'Page title lacks EDUS');
    await heading('Что будем делать?');
    await capture('01-home', { strictNoScroll: true });
    let display;
    if (size === '1600x900' && !quick) {
      display = await context.newPage();
      await display.goto(`${baseURL}/#/display`, { waitUntil: 'networkidle' });
      await capture('display-idle', { source: display });
    }
    const issuePassed = await scenario('issue-and-scan-errors', async () => {
      await click(/^Выдать книги/);
      await heading('Приложите карту ученика');
      await capture('02-identify', { strictNoScroll: true });
      if (size === '1600x900' && !quick) {
        const idleBounds = await page.locator('.nfc-animation .edus-card').boundingBox();
        for (const [name, delay] of [['card-idle-early', 0], ['card-idle-700ms', 700], ['card-idle-1700ms', 1000], ['card-idle-3700ms', 2000]]) {
          await page.waitForTimeout(delay); await capture(name);
          check(await page.locator('.state-mark.success').count() === 0, 'Idle animation invented NFC success');
          check(JSON.stringify(await page.locator('.nfc-animation .edus-card').boundingBox()) === JSON.stringify(idleBounds), 'Idle card position changed');
        }
      }
      await selectReader('Айша', '03');
      await heading('Сканируйте книги');
      await capture('04-scan-empty');
      await manual('000124', '05-manual-input');
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'First copy not in basket');
      await capture('06-scan-one');
      await manual('000124');
      await page.getByText(/Эта книга уже в списке/).first().waitFor();
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'Duplicate scan changed count');
      await capture('07-duplicate-error');
      await manual('000123');
      await page.getByText(/Эта книга уже выдана/).first().waitFor();
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'Already-issued scan changed count');
      await capture('07b-already-issued-error'); await click('Закрыть');
      await manual('DEMO-MULTI');
      await heading('Найдено издание'); await capture('07c-ambiguous-editions');
      await page.getByRole('dialog').getByRole('button', { name: /Қазақ тілі/ }).click();
      await page.getByText('ISBN обозначает издание, а не отдельную книгу.', { exact: true }).waitFor();
      await capture('07d-isbn-edition'); await click('Закрыть');
      await click('Изменить'); await heading('Выбрать другого читателя?');
      await capture('07e-change-reader-decision'); await click('Продолжить работу');
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'Canceling reader change lost basket');
      await manual('BOOK-NOT-FOUND');
      await page.getByText(/Код не найден/).first().waitFor();
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'Unknown scan lost basket');
      await capture('08-unknown-code');
      if (await button('Закрыть').isVisible()) await click('Закрыть');
      await manual('000125');
      check((await text(page.locator('.action-count'))).includes('2 к выдаче'), 'Second copy not in basket');
      await capture('09-scan-two');
      if (display) { await display.getByRole('heading', { name: 'Выдаём книги', exact: true }).waitFor(); await capture('display-active', { source: display }); }
      await click('Продолжить'); await heading('Всё верно?');
      check((await page.locator('.confirm-layout').innerText()).includes('Сәрсенова Айша Ерланқызы'), 'Confirmation reader changed');
      await capture('10-confirmation');
      await click('Выдать 2 книги'); await heading('Книги выданы');
      await capture('11-issue-success', { strictNoScroll: true });
      if (display) {
        await display.getByRole('heading', { name: 'Книги выданы', exact: true }).waitFor(); await capture('display-success', { source: display });
        await display.getByRole('heading', { name: /Добро пожаловать/ }).waitFor({ timeout: 18000 });
        check(!(await display.locator('body').innerText()).includes('Сәрсенова'), 'Second display did not clear personal data after success timeout');
        await capture('display-success-expired', { source: display });
      }
    });
    if (!issuePassed) continue;

    await scenario('unique-return', async () => {
      await homeFromSuccess(); await click(/^Принять книги/); await heading('Приложите карту ученика');
      check(await page.locator('.scanner-panel').count() === 0, 'Return exposed book scanning before reader identification');
      await capture('12-return-identify', { strictNoScroll: true });
      await demoTools(); await click('Карта Айши'); await heading('Какие книги принимаем?');
      check((await page.locator('.reader-summary').innerText()).includes('Сәрсенова Айша Ерланқызы'), 'Return card selected wrong reader');
      await capture('12b-return-reader-selected'); await manual('000124');
      check((await text(page.locator('.action-count'))).includes('1 к приёму'), 'Return candidate missing');
      await capture('13-return-copy'); await click('Продолжить'); await capture('14-return-confirm');
      await click('Принять 1 книгу'); await heading('Книги приняты');
      await capture('15-return-success', { strictNoScroll: true });
      await homeFromSuccess(); await click(/^Принять книги/); await heading('Приложите карту ученика'); await selectReader(); await heading('Какие книги принимаем?'); await manual('000124');
      await page.getByText(/У этой книги нет активной выдачи/).first().waitFor();
      await capture('16-repeat-return-error'); await click('Закрыть'); await goHomeWithDiscard();
    });

    await scenario('legacy-return-reader-context', async () => {
      await click(/^Принять книги/); await heading('Приложите карту ученика');
      check(await page.locator('.reader-summary').count() === 0, 'Return assigned a reader before identification');
      check(await page.locator('.scanner-panel').count() === 0, 'Legacy return allowed scanning without reader context');
      await capture('17-legacy-needs-reader', { strictNoScroll: true }); await selectReader();
      await heading('Какие книги принимаем?'); await manual('9786010123456');
      await heading('Найдено издание'); await capture('18-legacy-matched-loan');
      check((await page.locator('.reader-summary').innerText()).includes('Сәрсенова Айша Ерланқызы'), 'ISBN changed the selected reader');
      await click('Добавить к приёму'); await capture('19-legacy-basket');
      await click('Продолжить'); await capture('20-legacy-confirm');
      await click('Принять 1 книгу'); await heading('Книги приняты');
      await capture('21-legacy-success', { strictNoScroll: true }); await homeFromSuccess();
    });

    await scenario('new-edition-and-copy-registration', async () => {
      await click(/^Добавить книги/); await heading('Найдите издание'); await capture('22-registration-search');
      await click('Создать новое издание'); await heading('Новое издание · основные сведения');
      await click('Продолжить'); await page.getByText('Введите название книги.', { exact: true }).waitFor();
      await capture('23-registration-validation');
      await page.getByLabel('Название книги *', { exact: true }).fill('Қазақ әдебиеті. Әңгімелер мен шығармалар жинағы');
      await page.getByLabel('Автор *', { exact: true }).fill('Мұхтар Әуезов');
      await page.getByLabel('Издательство', { exact: true }).fill('Атамұра');
      await capture('24-registration-details'); await click('Продолжить');
      await heading('Новое издание · об издании');
      await page.getByLabel('Год издания *', { exact: true }).fill('2024');
      await page.getByLabel('Язык *', { exact: true }).fill('Қазақша');
      await page.getByLabel('Предмет · если есть', { exact: true }).fill('Литература');
      await page.getByLabel('Класс · если есть', { exact: true }).fill('8–9');
      await capture('25-registration-metadata'); await click('Продолжить');
      await heading('Как учитывать эти книги?'); await capture('26-registration-mode');
      await click(/^По инвентарным номерам/); await click('Продолжить'); await heading('Введите инвентарные номера');
      await page.getByLabel('Существующий инвентарный номер', { exact: true }).fill('QA-000077'); await click('В список');
      await capture('27-registration-copies'); await click('Проверить и добавить');
      await heading('Всё готово к добавлению?'); await capture('28-registration-confirm');
      await click('Добавить в фонд · 1 шт.'); await heading('Книги добавлены'); await capture('29-registration-success', { strictNoScroll: true });
      await homeFromSuccess();
      await click(/^Добавить книги/); await page.getByLabel('ISBN, название или автор', { exact: true }).fill('Әңгімелер');
      await page.getByRole('button', { name: /^Қазақ әдебиеті/ }).click();
      await heading('Проверьте карточку издания'); await capture('30-registered-edition-found');
      await click('Добавить книги этого издания'); await click(/^Старый фонд — количеством/); await click('Продолжить');
      await heading('Укажите количество книг'); await capture('31-registration-quantity');
      await click('Проверить и добавить'); await click('Добавить в фонд · 1 шт.'); await heading('Книги добавлены');
      await homeFromSuccess();
    });

    await scenario('connection-loss-and-unknown-recovery', async () => {
      await click(/^Выдать книги/); await selectReader(); await manual('000124');
      await settings(); await capture('32-settings');
      await page.locator('.setting-row').filter({ hasText: 'Локальный сервер' }).getByRole('button').click();
      await click('Закрыть настройки');
      check(await button('Продолжить').isDisabled(), 'Offline server allowed submit');
      check((await text(page.locator('.action-count'))).includes('1 к выдаче'), 'Server loss lost basket');
      await capture('33-server-offline'); await settings();
      await page.locator('.setting-row').filter({ hasText: 'Локальный сервер' }).getByRole('button').click();
      await page.locator('.setting-row').filter({ hasText: 'Не нужен для локальных операций' }).getByRole('button').click();
      await click('Закрыть настройки');
      check(await button('Продолжить').isEnabled(), 'Internet loss prevented local operation');
      await capture('34-internet-offline-local-ready');
      await demoTools(); await click('Задержать ответ операции');
      await click('Продолжить'); await click('Выдать 1 книгу'); await heading('Проверяем результат операции');
      await capture('35-unknown-operation');
      check(await button('На главную').isDisabled(), 'Unknown result allowed unsafe reset');
      await page.reload({ waitUntil: 'networkidle' }); await heading('Проверяем результат операции');
      await capture('36-unknown-after-reload'); await click('Проверить результат'); await heading('Книги выданы');
      await capture('37-unknown-recovered-success', { strictNoScroll: true }); await homeFromSuccess();
    });

    if (!quick) {
      await scenario('nfc-adapter-identification', async () => {
        await click(/^Выдать книги/); await heading('Приложите карту ученика');
        await demoTools();
        // Observe the short real adapter states without delaying the transition
        // with PNG encoding. This only reads visible DOM class changes.
        const observedStates = page.evaluate(() => new Promise(resolve => {
          const states = [];
          const record = () => {
            const element = document.querySelector('.nfc-animation');
            if (element && states.at(-1) !== element.className) states.push(element.className);
            if (!element && states.length) { observer.disconnect(); clearTimeout(timer); resolve(states); }
          };
          const observer = new MutationObserver(record);
          observer.observe(document.body, { attributes: true, childList: true, subtree: true });
          const timer = setTimeout(() => { observer.disconnect(); resolve(states); }, 4000);
          record();
        }));
        await click('Карта Айши');
        await heading('Сканируйте книги');
        const observed = await observedStates;
        run.nfcAdapterStates = observed;
        check(observed.some(s => s.includes('state-reading')), 'NFC adapter reading state was not observed');
        check(observed.some(s => s.includes('state-success')), 'NFC adapter success state was not observed');
        check((await page.locator('.reader-summary').innerText()).includes('Сәрсенова Айша Ерланқызы'), 'NFC adapter selected wrong reader');
        await capture('38c-nfc-reader-selected'); await goHomeWithDiscard();
      });
      await scenario('touch-keyboard-and-session', async () => {
        await click(/^Выдать книги/); await click('Найти вручную');
        await click('Открыть клавиатуру: Найдите читателя');
        await click('Изменить язык клавиатуры'); await click('ә');
        if (width < 600) await capture('38-touch-keyboard-page-one');
        for (let attempt = 0; !(await button('ғ').isVisible()) && attempt < 3; attempt++) {
          await click(/^Следующая страница клавиатуры/);
        }
        await click('ғ');
        check(await page.getByPlaceholder('Фамилия, имя или номер карты').inputValue() === 'әғ', 'Kazakh keyboard input failed');
        await capture('38-touch-keyboard'); await click('Скрыть');
        await demoTools(); await click('Истекла сессия'); await heading('Сессия приостановлена');
        await capture('39-session-expired'); await click('Продолжить сессию');
        await goHomeWithDiscard();
      });
      await scenario('gallery-nfc-states', async () => {
        await page.goto(`${baseURL}/#/components`, { waitUntil: 'networkidle' });
        await heading('Один язык интерфейса'); await capture('40-component-gallery', { fullPage: true });
        const controls = page.locator('.state-switch');
        for (const state of ['Чтение', 'Успех', 'Ошибка', 'Ожидание']) {
          await controls.getByRole('button', { name: state, exact: true }).click();
          await page.locator('.nfc-demo').scrollIntoViewIfNeeded();
          await capture(`41-card-${{ Чтение: 'reading', Успех: 'success', Ошибка: 'error', Ожидание: 'idle' }[state]}`);
        }
        await page.emulateMedia({ reducedMotion: 'reduce' });
        check(await page.locator('.nfc-animation .state-mark.success').count() === 0, 'Reduced-motion idle invented success');
        const animation = await page.locator('.nfc-animation .edus-card').evaluate(e => getComputedStyle(e).animationName);
        check(animation === 'none', 'Reduced motion did not suppress NFC animation');
        await capture('42-card-reduced-motion');
      });
      if (display) {
        await scenario('second-display-privacy', async () => {
          await page.goto(baseURL, { waitUntil: 'networkidle' });
          await display.getByRole('heading', { name: /Добро пожаловать/ }).waitFor();
          check(!(await display.locator('body').innerText()).includes('Ерланқызы'), 'Patronymic leaked on second display');
          check(await display.locator('button,input').count() === 0, 'Second display is not read-only');
          await capture('display-cleared', { source: display });
        });
      }
    }
  } catch (error) {
    run.failures.push({ stage: currentStage, kind: 'uncaught', message: error.message });
    console.error(`[${size}] run error: ${error.message}`);
  } finally {
    run.externalRequests = [...new Set(run.externalRequests)];
    writeFileSync(join(reportRoot, `browser-${size}.json`), JSON.stringify(run, null, 2));
    await context.close();
  }
}
await browser.close();

// Preserve the latest evidence for other sizes during a targeted rerun.
report.runs = ['1920x1080', '1600x900', '1366x768', '390x844'].map(size => {
  const path = join(reportRoot, `browser-${size}.json`);
  return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null;
}).filter(Boolean);

await composeOverview();
writeFileSync(join(reportRoot, 'browser-report.json'), JSON.stringify(report, null, 2));
const summary = report.runs.map(run => ({ viewport: run.viewport, scenarios: run.checks.length, passed: run.checks.filter(c => c.passed).length, screenshots: run.screenshots.length, consoleErrors: run.console.filter(c => c.type === 'error').length, pageErrors: run.pageErrors.length, requestFailures: run.requestFailures.length, badResponses: run.badResponses.length, externalRequests: run.externalRequests, failures: run.failures, smallTouchStates: run.screenshots.filter(s => s.smallTouch.length).map(s => ({ name: s.name, controls: s.smallTouch })) }));
writeFileSync(join(reportRoot, 'browser-summary.json'), JSON.stringify(summary, null, 2));
writeFileSync(join(reportRoot, 'README.md'), [
  '# Browser verification evidence',
  '',
  `Generated: ${new Date().toISOString()}`,
  '',
  'The suite uses fresh isolated Playwright Chromium contexts and visible UI actions. It does not import application modules, mutate hidden state or storage, or connect to the user browser. DOM measurements and the short NFC state observer are read-only.',
  '',
  '| Viewport | Scenario checks | PNGs | Failures | Runtime errors | External requests |',
  '| --- | --- | --- | --- | --- | --- |',
  ...summary.map(r => `| ${r.viewport.width}×${r.viewport.height} @1× | ${r.passed}/${r.scenarios} | ${r.screenshots} | ${r.failures.length} | ${r.pageErrors + r.consoleErrors} | ${r.externalRequests.length} |`),
  '',
  'Run from the repository root: `node scripts/qa-browser.mjs`. For a targeted rerun use `--viewport=390x844`; aggregate reports retain the latest evidence for all four sizes. The script uses the existing bundled Playwright and Sharp dependencies; it installs nothing.',
  '',
  `\`browser-report.json\` contains complete measurements and browser events. \`browser-summary.json\` gives the concise results. \`browser-<width>x<height>.json\` records each isolated viewport run. Reports are stored in \`${reportRoot}\` and full-size PNGs in \`${screenshotRoot}/<viewport>\`. The current main captures are \`01-home.png\`, \`02-identify.png\`, \`09-scan-two.png\` and \`10-confirmation.png\`; \`${screenshotRoot}/overview.png\` assembles these four screens. Earlier baseline captures in other folders are preserved. Rebuild this contact sheet with \`node scripts/qa-browser.mjs --compose-only\`, using the same EDUS_QA_REPORT_ROOT and EDUS_QA_SCREENSHOT_ROOT environment overrides.`,
  '',
  'The gallery captures all four school-card states (idle, reading, success, error) and reduced-motion behavior. Static idle is sampled over time and may never invent success. Real mock-adapter identification records idle → reading → success before showing the selected reader. Both return modes require reader identification before books. At 1600×900 the second display is checked in idle, active, success, and after its real success timeout clears personal information. No physical NFC/scanner device, Ubuntu kiosk session, or real backend is covered by these browser tests.',
  '',
].join('\n'));
console.log(JSON.stringify(summary.map(({ smallTouchStates, ...rest }) => ({ ...rest, statesWithSmallTouch: smallTouchStates.length })), null, 2));
process.exitCode = summary.some(run => run.failures.length || run.pageErrors || run.consoleErrors || run.badResponses) ? 1 : 0;
