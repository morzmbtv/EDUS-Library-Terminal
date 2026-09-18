/** Iteration 2: UI-only EDUS regression. No application imports or hidden state writes. */
import { createRequire } from 'node:module';
import { resolve, join } from 'node:path';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';

const dependencies = process.env.CODEX_NODE_MODULES ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(resolve(dependencies, 'package.json'));
const { chromium } = require('playwright');
const sharp = require('sharp');
const baseURL = process.env.EDUS_QA_URL ?? 'http://127.0.0.1:5173';
const selectedViewport = process.argv.find(arg => arg.startsWith('--viewport='))?.split('=')[1];
const motionOnly = process.argv.includes('--motion-only');
const sizes = selectedViewport ? [selectedViewport] : ['1920x1080', '1600x900', '1366x768', '390x844'];
mkdirSync('.qa', { recursive: true });
mkdirSync('screenshots/iteration2', { recursive: true });
async function composeOverview() {
  const items = [['01-home', 'Главная'], ['02-issue-identify', 'Выдача · выбор читателя'], ['05-accept-identify', 'Приём · выбор читателя'], ['08-accept-scan', 'Приём книг'], ['09-wrong-reader-book', 'Книга другого читателя'], ['10-help-with-return-basket', 'Контекстная помощь']];
  if (!items.every(([name]) => existsSync(`screenshots/iteration2/1600x900/${name}.png`))) return;
  const gap = 24, tileWidth = 800, tileHeight = 490, width = 1672, height = 1614;
  const textSvg = (text, w, h, fontSize = 22) => Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><rect width="100%" height="100%" fill="#fff"/><text x="16" y="${h / 2 + fontSize / 3}" font-family="Noto Sans, sans-serif" font-size="${fontSize}" font-weight="bold" fill="#0b1a4d">${text}</text></svg>`);
  const composite = [{ input: textSvg('EDUS · Интерфейс библиотечного терминала', width - gap * 2, 48, 28), left: gap, top: 16 }];
  for (let index = 0; index < items.length; index++) {
    const [name, label] = items[index], left = gap + (index % 2) * (tileWidth + gap), top = 72 + Math.floor(index / 2) * (tileHeight + gap);
    composite.push({ input: textSvg(label, tileWidth, 40), left, top });
    composite.push({ input: await sharp(`screenshots/iteration2/1600x900/${name}.png`).resize(tileWidth, 450).png().toBuffer(), left, top: top + 40 });
  }
  await sharp({ create: { width, height, channels: 3, background: '#eef2f7' } }).composite(composite).png().toFile('screenshots/iteration2/overview.png');
  writeFileSync('.qa/iteration2-overview.json', JSON.stringify({ path: 'screenshots/iteration2/overview.png', width, height, method: 'Contact sheet assembled from unaltered full-viewport PNG captures; proportional resizing for overview only.', sources: items.map(([name, label]) => ({ path: `screenshots/iteration2/1600x900/${name}.png`, label })) }, null, 2));
}
if (process.argv.includes('--compose-only')) { await composeOverview(); process.exit(0); }
const browser = await chromium.launch({ headless: true });
const report = { timestamp: new Date().toISOString(), baseURL, browser: browser.version(), method: 'Fresh isolated Chromium contexts. Visible UI controls and scanner-like keyboard events only. DOM measurements/observers are read-only. No application imports, localStorage access, or application state injection.', runs: [] };
const assert = (condition, message) => { if (!condition) throw new Error(message); };

for (const size of sizes) {
  const [width, height] = size.split('x').map(Number);
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, locale: 'ru-RU', timezoneId: 'Asia/Qyzylorda' });
  const page = await context.newPage();
  page.setDefaultTimeout(6500);
  const dir = join('screenshots', 'iteration2', size);
  mkdirSync(dir, { recursive: true });
  const run = motionOnly && existsSync(join('.qa', `iteration2-${size}.json`)) ? JSON.parse(readFileSync(join('.qa', `iteration2-${size}.json`), 'utf8')) : { timestamp: new Date().toISOString(), viewport: { width, height, dpr: 1 }, checks: [], screenshots: [], failures: [], console: [], pageErrors: [], requestFailures: [], badResponses: [], externalRequests: [], observations: [] };
  if (motionOnly) { run.motionRefreshedAt = new Date().toISOString(); run.checks = run.checks.filter(check => check.name !== 'nfc-gallery-and-reduced-motion'); run.screenshots = run.screenshots.filter(shot => !/^(26|27)-?/.test(shot.name)); run.failures = run.failures.filter(failure => failure.scenario !== 'nfc-gallery-and-reduced-motion'); }
  report.runs.push(run);
  page.on('console', event => { if (['warning', 'error'].includes(event.type())) run.console.push({ type: event.type(), text: event.text() }); });
  page.on('pageerror', error => run.pageErrors.push(error.message));
  page.on('requestfailed', request => run.requestFailures.push({ url: request.url(), error: request.failure()?.errorText }));
  page.on('response', response => { if (response.status() >= 400) run.badResponses.push({ url: response.url(), status: response.status() }); });
  page.on('request', request => { if (/^https?:/.test(request.url()) && new URL(request.url()).origin !== new URL(baseURL).origin) run.externalRequests.push(request.url()); });
  let stage = 'loading';
  const button = name => page.getByRole('button', { name, exact: typeof name === 'string' });
  const click = async name => button(name).click();
  const heading = async name => page.getByRole('heading', { name, exact: typeof name === 'string' }).first().waitFor({ state: 'visible' });
  const content = async locator => (await locator.innerText()).trim();
  const readerName = async () => content(page.locator('.reader-summary .reader-identity strong'));
  const basketText = async () => content(page.locator('.book-basket'));
  const countText = async () => content(page.locator('.action-count'));

  async function capture(name, { noScroll = false, fullPage = false } = {}) {
    stage = name;
    await page.waitForTimeout(50);
    await page.evaluate(() => document.fonts.ready);
    const measurements = await page.evaluate(() => {
      const visible = element => { const rect = element.getBoundingClientRect(); const style = getComputedStyle(element); return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden'; };
      const controls = [...document.querySelectorAll('button,a.btn,input,select')].filter(visible).map(element => {
        const r = element.getBoundingClientRect();
        const x = r.left + r.width / 2, y = r.top + r.height / 2;
        let centerVisible = x > 0 && x < innerWidth && y > 0 && y < innerHeight && !element.closest('[inert]');
        for (let parent = element.parentElement; parent && centerVisible; parent = parent.parentElement) {
          const style = getComputedStyle(parent), bounds = parent.getBoundingClientRect();
          if (/(auto|scroll|hidden|clip)/.test(style.overflowY) && (y < bounds.top || y > bounds.bottom)) centerVisible = false;
          if (/(auto|scroll|hidden|clip)/.test(style.overflowX) && (x < bounds.left || x > bounds.right)) centerVisible = false;
        }
        const hit = centerVisible ? document.elementFromPoint(x, y) : null;
        return { name: element.getAttribute('aria-label') || element.textContent.trim().slice(0, 90) || element.getAttribute('placeholder'), width: +r.width.toFixed(1), height: +r.height.toFixed(1), top: +r.top.toFixed(1), bottom: +r.bottom.toFixed(1), disabled: element.hasAttribute('disabled'), inViewport: r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth, occluded: !!centerVisible && !element.contains(hit), occludedBy: centerVisible && !element.contains(hit) ? hit?.className : undefined };
      });
      const clipped = [...document.querySelectorAll('h1,h2,h3,p,strong,label,button')].filter(visible).filter(element => element.clientWidth && element.scrollWidth > element.clientWidth + 2 && getComputedStyle(element).overflowX !== 'auto').map(element => ({ text: element.textContent.trim().slice(0, 100), width: element.clientWidth, scrollWidth: element.scrollWidth }));
      const tiny = [...document.querySelectorAll('h1,h2,h3,p,span,strong,label,button,small')].filter(visible).filter(element => !element.children.length && element.textContent.trim()).map(element => ({ text: element.textContent.trim().slice(0, 100), fontSize: parseFloat(getComputedStyle(element).fontSize) })).filter(item => item.fontSize < 16);
      return { title: document.title, horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1, verticalOverflow: document.documentElement.scrollHeight > innerHeight + 1, documentHeight: document.documentElement.scrollHeight, controls, occluded: controls.filter(item => item.occluded && !item.disabled), smallTouch: controls.filter(item => item.inViewport && (item.width < 55.5 || item.height < 55.5)), clipped, tiny, frameworkOverlay: !!document.querySelector('vite-error-overlay'), brokenImages: [...document.images].filter(image => !image.complete || !image.naturalWidth).map(image => image.src), visibleTechnicalStatus: /Сервер: доступен|Интернет: доступен|соединение моделируется/.test(document.body.innerText) };
    });
    const path = join(dir, `${name}.png`);
    await page.screenshot({ path, fullPage, animations: 'allow' });
    const png = await sharp(path).metadata();
    assert(png.width === width && (fullPage || png.height === height), `Incorrect PNG dimensions: ${name}`);
    run.screenshots.push({ name, path, width: png.width, height: png.height, fullPage, ...measurements });
    if (measurements.horizontalOverflow || (noScroll && width >= 900 && measurements.verticalOverflow)) run.failures.push({ stage: name, type: 'overflow', ...measurements });
    if (measurements.frameworkOverlay || measurements.brokenImages.length) run.failures.push({ stage: name, type: 'render', ...measurements });
    if (measurements.occluded.length) run.failures.push({ stage: name, type: 'occluded-controls', controls: measurements.occluded });
    if (measurements.smallTouch.length || measurements.clipped.length || measurements.tiny.length) run.failures.push({ stage: name, type: 'touch-and-text', smallTouch: measurements.smallTouch, clipped: measurements.clipped, tiny: measurements.tiny });
    console.log(`[iteration2 ${size}] ${name}`);
  }
  async function scenario(name, action) {
    if (motionOnly && name !== 'nfc-gallery-and-reduced-motion') return;
    try {
      await page.goto(baseURL, { waitUntil: 'networkidle' });
      await heading('Что будем делать?');
      await action();
      run.checks.push({ name, passed: true });
    } catch (error) {
      run.checks.push({ name, passed: false, message: error.message });
      run.failures.push({ stage, type: 'interaction', scenario: name, message: error.message });
      try { await capture(`FAIL-${name}`); writeFileSync(join('.qa', `iteration2-${size}-${name}-failure.txt`), await page.locator('body').innerText()); } catch { /* Keep the first failure. */ }
      console.error(`[iteration2 ${size}] FAIL ${name}: ${error.message}`);
    }
  }
  async function identifyScreen(operation) {
    await click(operation === 'issue' ? /^Выдать книги/ : /^Принять книги/);
    await heading(operation === 'issue' ? 'Кому выдаём книги?' : 'Кто возвращает книги?');
    await page.getByText('Приложите карту ученика', { exact: true }).waitFor();
    assert(await button('Найти вручную').isVisible(), 'Manual reader selection is missing');
    assert(await page.locator('.scanner-panel').count() === 0, 'Return exposed book scanning before reader selection');
  }
  async function manualReader(name = 'Айша') {
    await click('Найти вручную'); await heading('Найдите читателя');
    await page.getByPlaceholder('Фамилия, имя или номер карты').fill(name);
    await page.getByRole('button', { name: name === 'Айша' ? /Сәрсенова Айша Ерланқызы/ : /Нұрланұлы Әлихан/ }).click();
    await page.locator('.reader-summary').waitFor();
  }
  async function manualBook(code) {
    await click('Ввести код вручную'); await heading('Введите код книги');
    await page.getByLabel('Код книги', { exact: true }).fill(code);
    await click('Найти книгу');
  }
  async function closeBookDialog() { const modal = page.getByRole('dialog', { name: 'Введите код книги', exact: true }); if (await modal.isVisible()) await modal.getByRole('button', { name: 'Закрыть', exact: true }).click(); }
  async function hardwareKeys(code) {
    await page.locator('h1').first().click();
    await page.keyboard.type(code, { delay: 3 }); await page.keyboard.press('Enter');
  }
  async function observeNfc() {
    return page.evaluate(() => new Promise(resolve => {
      const states = [];
      const record = () => { const element = document.querySelector('.nfc-animation'); const value = element?.className; if (value && value !== states.at(-1)) states.push(value); if (!element && states.length) { observer.disconnect(); clearTimeout(timer); resolve(states); } };
      const observer = new MutationObserver(record); observer.observe(document.body, { attributes: true, subtree: true, childList: true });
      const timer = setTimeout(() => { observer.disconnect(); resolve(states); }, 5000); record();
    }));
  }
  async function settings() { await click('Настройки и диагностика'); await heading('Настройки и диагностика'); }
  async function panel() { await settings(); const toggle = button('Панель сценариев'); if (await toggle.getAttribute('aria-expanded') !== 'true') await toggle.click(); }
  async function openHelp() { await click('Помощь'); await page.getByRole('button', { name: 'Понятно', exact: true }).waitFor(); }
  async function closeHelp() { if (await button('Понятно').isVisible()) await click('Понятно'); }
  async function verifyHelpSteps(contextName) {
    const help = page.getByRole('dialog').filter({ has: page.getByRole('button', { name: 'Понятно', exact: true }) });
    const text = await help.innerText();
    assert(text.length > 100, `Help has no usable content for ${contextName}`);
    const steps = await help.locator('li').count();
    run.observations.push({ type: 'help', context: contextName, steps, text });
    if (steps) assert(steps >= 3 && steps <= 4, `Help should contain 3–4 steps, found ${steps}`);
    return help;
  }
  async function submitAccept(name = 'Принять 1 книгу') { await click('Продолжить'); await heading('Всё верно?'); await click(name); await heading('Книги приняты'); }

  await scenario('home-and-help-search-focus', async () => {
    assert(await page.locator('.home-action').count() === 3, 'Home must have exactly three operation cards');
    const colors = await page.locator('.home-action').evaluateAll(elements => elements.map(element => ({ label: element.textContent.trim(), background: getComputedStyle(element).backgroundColor, color: getComputedStyle(element).color })));
    run.observations.push({ type: 'home-colors', colors });
    assert(colors[0].background !== colors[1].background && colors[1].background !== colors[2].background, 'Home operation colors are not distinct blue / warm gold / white');
    assert(await page.locator('.system-footer').count() === 0, 'Technical footer is still present');
    await capture('01-home', { noScroll: true });
    await identifyScreen('issue'); await capture('02-issue-identify', { noScroll: true });
    assert(await page.locator('.nfc-zone').count() === 0, 'Old NFC circle/zone remains');
    assert(await page.locator('.nfc-animation img[src*="edus-card.svg"]').count() === 0, 'Historic blue card remains in the instruction illustration');
    await openHelp(); await verifyHelpSteps('issue-identify'); await capture('03-issue-help'); await closeHelp();
    await click('Найти вручную');
    const field = page.getByPlaceholder('Фамилия, имя или номер карты');
    await field.fill('Айша'); await field.click();
    const fieldId = await field.getAttribute('id');
    await openHelp(); await verifyHelpSteps('reader-search');
    await page.keyboard.type('EDUS-1002', { delay: 3 }); await page.keyboard.press('Enter');
    assert(await button('Понятно').isVisible(), 'Scanner Enter unexpectedly dismissed contextual help');
    await closeHelp();
    assert(await field.inputValue() === 'Айша', 'Help/scanner changed manual search input');
    assert(await page.evaluate(() => document.activeElement?.id) === fieldId, 'Help did not restore manual search focus');
    await capture('04-search-focus-restored');
  });

  await scenario('accept-card-wrong-owner-help-and-success', async () => {
    await identifyScreen('accept'); await capture('05-accept-identify', { noScroll: true });
    await page.waitForTimeout(2200);
    assert(await page.locator('.nfc-animation.state-success').count() === 0, 'Idle invented a successful identification');
    await hardwareKeys('EDUS-NOT-FOUND');
    await page.locator('.nfc-animation.state-error').waitFor(); await capture('06-card-error');
    const observed = observeNfc();
    await hardwareKeys('EDUS-1001');
    await page.locator('.reader-summary').waitFor();
    const states = await observed; run.observations.push({ type: 'adapter-nfc-states', states });
    assert(states.some(value => value.includes('state-reading')) && states.some(value => value.includes('state-success')), 'Adapter reading/success states were not observed');
    assert((await readerName()).includes('Айша'), 'Card selected the wrong reader');
    assert((await content(page.locator('.reader-summary'))).includes('7 «А»'), 'Reader class is missing');
    if (width >= 900) {
      const geometry = await page.evaluate(() => {
        const symbol = document.querySelector('.scanner-symbol').getBoundingClientRect(), title = document.querySelector('.operation-heading').getBoundingClientRect(), manual = document.querySelector('.manual-button').getBoundingClientRect(), loans = document.querySelector('.loan-options').getBoundingClientRect(), hit = document.elementFromPoint(manual.x + manual.width / 2, manual.y + manual.height / 2);
        return { symbolTop: symbol.top, titleBottom: title.bottom, manualBottom: manual.bottom, loansTop: loans.top, manualUnobscured: document.querySelector('.manual-button').contains(hit) };
      });
      run.observations.push({ type: 'return-scanner-geometry', ...geometry });
      assert(geometry.symbolTop >= geometry.titleBottom, 'Return scanner overlaps operation heading');
      assert(geometry.manualBottom <= geometry.loansTop + 1 && geometry.manualUnobscured, 'Return loan list overlaps manual code button');
    }
    await capture('07-accept-reader-selected');
    await manualBook('000123'); assert((await countText()).startsWith('1 к приёму'), 'Own copy not selected for return');
    await capture('08-accept-scan');
    await manualBook('KZ-0007'); await heading('Книга другого читателя');
    assert((await countText()).startsWith('1 к приёму'), 'Wrong owner changed return basket');
    assert((await readerName()).includes('Айша'), 'Wrong owner silently changed reader');
    await capture('09-wrong-reader-book'); await click('Не добавлять книгу'); await closeBookDialog();
    await manualBook('KZ-0007'); await heading('Книга другого читателя'); await click('Сменить читателя');
    await heading('Выбрать другого читателя?'); await capture('09b-wrong-reader-change-consent'); await click('Продолжить работу');
    assert((await readerName()).includes('Айша') && (await countText()).startsWith('1 к приёму'), 'Canceling explicit wrong-owner switch lost context');
    const previous = { reader: await readerName(), basket: await basketText(), count: await countText() };
    await openHelp(); await verifyHelpSteps('accept-scan'); await capture('10-help-with-return-basket');
    await page.keyboard.type('000123', { delay: 3 }); await page.keyboard.press('Enter');
    assert(await button('Понятно').isVisible(), 'Scanner Enter dismissed help instead of staying blocked');
    await page.keyboard.type('9786010123456', { delay: 3 }); await page.keyboard.press('Enter');
    assert(await button('Понятно').isVisible(), 'Repeated scanner events dismissed help');
    await closeHelp();
    assert(await readerName() === previous.reader && await basketText() === previous.basket && await countText() === previous.count, 'Help allowed scanner events to mutate reader/basket');
    assert(await page.getByRole('dialog', { name: 'Найдено издание', exact: true }).count() === 0, 'Scanner opened a book dialog beneath help');
    await click('Продолжить'); await heading('Всё верно?'); await capture('11-accept-confirmation');
    await click('Принять 1 книгу'); await heading('Книги приняты'); await capture('12-accept-success', { noScroll: true });
    assert(/принято/i.test(await page.locator('main').innerText()), 'Completed return lacks accepted status');
    await click(/^Следующий (возврат|читатель)/); await heading('Кто возвращает книги?');
    assert(await page.locator('.reader-summary').count() === 0 && await page.locator('.book-row').count() === 0, 'New return session retained reader/books');
    await capture('13-new-return-session', { noScroll: true });
  });

  await scenario('accept-manual-isbn-and-change-reader', async () => {
    await identifyScreen('accept'); await manualReader('Айша');
    await manualBook('9786010123456'); await heading('Найдено издание');
    assert((await content(page.getByRole('dialog'))).includes('доступно 2'), 'ISBN did not use the selected reader loan quantity');
    await capture('14-isbn-selected-reader'); await click('Добавить к приёму');
    assert((await countText()).startsWith('1 к приёму'), 'Legacy return basket quantity incorrect');
    await click('Изменить'); await heading('Выбрать другого читателя?'); await capture('15-change-reader-decision');
    await click('Продолжить работу'); assert((await readerName()).includes('Айша') && (await countText()).startsWith('1 к приёму'), 'Canceling reader change lost context');
    await click('Изменить'); await click('Очистить и выбрать'); await heading('Кто возвращает книги?');
    await manualReader('Әлихан'); assert((await countText()).startsWith('0 к приёму'), 'Reader change transferred the previous basket');
    await manualBook('9786010123456'); await heading('Найдено издание');
    assert((await content(page.getByRole('dialog'))).includes('доступно 1'), 'ISBN did not switch to the new reader loan');
    await capture('16-isbn-new-reader'); await click('Добавить к приёму'); await capture('17-manual-reader-return-basket');
    await submitAccept(); await capture('18-manual-legacy-return-success', { noScroll: true });
  });

  await scenario('issue-regression-server-and-internet', async () => {
    await identifyScreen('issue'); await manualReader('Айша'); await manualBook('000124');
    await manualBook('000124'); await page.getByText(/уже в списке/).first().waitFor();
    assert((await countText()).startsWith('1 к выдаче'), 'Repeated copy duplicated the issue');
    await capture('19-issue-repeat-protected');
    await settings(); await page.locator('.setting-row').filter({ hasText: 'Локальный сервер' }).getByRole('button').click(); await click('Закрыть настройки');
    assert(await button('Продолжить').isDisabled(), 'No-server operation is not blocked');
    await page.getByText(/Нет связи с локальным сервером/).first().waitFor();
    assert((await countText()).startsWith('1 к выдаче'), 'Server loss cleared basket'); await capture('20-server-error-blocks');
    await settings(); await page.locator('.setting-row').filter({ hasText: 'Локальный сервер' }).getByRole('button').click();
    await page.locator('.setting-row').filter({ hasText: 'Не нужен для локальных операций' }).getByRole('button').click(); await click('Закрыть настройки');
    assert(await button('Продолжить').isEnabled(), 'Internet loss blocked a local operation');
    assert(await page.locator('.system-footer').count() === 0, 'Status footer reappeared'); await capture('21-internet-offline-can-continue');
    await panel(); await click('Задержать ответ операции'); await click('Продолжить'); await click('Выдать 1 книгу');
    await heading('Проверяем результат операции'); await capture('22-unknown-operation');
    assert(await button('На главную').isDisabled(), 'Unknown result allows unsafe navigation');
    await page.reload({ waitUntil: 'networkidle' }); await heading('Проверяем результат операции');
    await click('Проверить результат'); await heading('Книги выданы'); await capture('23-issue-recovered-success', { noScroll: true });
    await click('Следующий читатель'); await heading('Кому выдаём книги?');
    assert(await page.locator('.reader-summary').count() === 0 && await page.locator('.book-row').count() === 0, 'Next issue session retained reader/books');
  });

  await scenario('registration-help-preserves-form', async () => {
    await click(/^Добавить книги/); await heading('Найдите издание'); await click('Создать новое издание');
    const name = page.getByLabel('Название книги *', { exact: true });
    await name.fill('Қазақ әдебиеті. Шығармалар жинағы');
    await page.getByLabel('Автор *', { exact: true }).fill('Мұхтар Әуезов'); await name.click();
    await click('Открыть клавиатуру: Название книги *');
    await page.locator('.registration-keyboard .touch-keyboard').waitFor();
    await capture('24a-registration-keyboard');
    await openHelp(); await verifyHelpSteps('registration'); await capture('24-registration-help');
    await page.keyboard.type('000123', { delay: 3 }); await page.keyboard.press('Enter');
    assert(await button('Понятно').isVisible(), 'Registration scanner unexpectedly dismissed help');
    await closeHelp();
    assert(await page.locator('.registration-keyboard .touch-keyboard').isVisible(), 'Help discarded registration keyboard');
    await page.locator('.registration-keyboard').getByRole('button', { name: 'Пробел', exact: true }).click();
    await page.locator('.registration-keyboard').getByRole('button', { name: 'Удалить последний символ', exact: true }).click();
    await click('Скрыть');
    assert(await name.inputValue() === 'Қазақ әдебиеті. Шығармалар жинағы', 'Help reset new-edition title');
    assert(await page.getByLabel('Автор *', { exact: true }).inputValue() === 'Мұхтар Әуезов', 'Help reset new-edition author');
    run.observations.push({ type: 'registration-help-focus', focus: await page.evaluate(() => ({ tag: document.activeElement?.tagName, id: document.activeElement?.id, label: document.activeElement?.getAttribute('aria-label') })) });
    await capture('25-registration-form-preserved');
    await click('Продолжить'); await heading('Новое издание · об издании');
    await page.getByLabel('Год издания *', { exact: true }).fill('2024'); await page.getByLabel('Язык *', { exact: true }).fill('Қазақша');
    await click('Продолжить'); await heading('Как учитывать эти книги?'); await click(/^По инвентарным номерам/); await click('Продолжить');
    await page.getByLabel('Существующий инвентарный номер', { exact: true }).fill(`QA-I2-${size}`); await click('В список');
    await click('Проверить и добавить'); await heading('Всё готово к добавлению?'); await capture('25b-registration-review');
    await click('Добавить в фонд · 1 шт.'); await heading('Книги добавлены'); await capture('25c-registration-success', { noScroll: true });
  });

  await scenario('nfc-gallery-and-reduced-motion', async () => {
    await page.goto(`${baseURL}/#/components`, { waitUntil: 'networkidle' }); await heading('Один язык интерфейса');
    const controls = page.locator('.state-switch');
    for (const [label, name] of [['Ожидание', 'idle'], ['Чтение', 'reading'], ['Успех', 'success'], ['Ошибка', 'error']]) {
      await controls.getByRole('button', { name: label, exact: true }).click(); await page.locator('.nfc-demo').scrollIntoViewIfNeeded();
      await capture(`26-animation-${name}`);
    }
    await controls.getByRole('button', { name: 'Ожидание', exact: true }).click(); await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForTimeout(100);
    assert(await page.locator('.nfc-animation.state-success').count() === 0, 'Reduced motion invented success');
    const animations = await page.locator('.nfc-animation').evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === 'running').length);
    assert(animations === 0, 'NFC motion remains enabled with reduced motion'); await capture('27-animation-reduced-motion');
    for (const [label, name, marker] of [['Чтение', 'reading', '.reading-line'], ['Успех', 'success', '.state-mark.success'], ['Ошибка', 'error', '.state-mark.error']]) {
      await controls.getByRole('button', { name: label, exact: true }).click(); await page.waitForTimeout(100);
      assert(await page.locator(`.nfc-animation ${marker}`).isVisible(), `Reduced ${name} lost its state marker`);
      const running = await page.locator('.nfc-animation').evaluate(element => element.getAnimations({ subtree: true }).filter(animation => animation.playState === 'running').length);
      assert(running === 0, `Reduced ${name} still animates`);
      await capture(`27-${name}-reduced-motion`);
      run.observations.push({ type: 'reduced-motion-state', state: name, markerVisible: true, runningAnimations: running });
    }
  });

  await scenario('workflow-deep-links-require-reader', async () => {
    for (const route of ['#/accept/scan', '#/accept/confirm', '#/issue/scan', '#/issue/confirm']) {
      await page.goto(`${baseURL}/${route}`, { waitUntil: 'networkidle' });
      await heading(route.includes('accept') ? 'Кто возвращает книги?' : 'Кому выдаём книги?');
      assert(await page.locator('.reader-summary,.scanner-panel,.confirm-layout').count() === 0, `Direct ${route} bypassed reader selection`);
      await capture(`28-deep-link-${route.replace('#/', '').replace('/', '-')}`, { noScroll: true });
    }
  });

  run.externalRequests = [...new Set(run.externalRequests)];
  writeFileSync(join('.qa', `iteration2-${size}.json`), JSON.stringify(run, null, 2));
  await context.close();
}
await browser.close();
report.runs = ['1920x1080', '1600x900', '1366x768', '390x844'].map(size => { const path = join('.qa', `iteration2-${size}.json`); return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null; }).filter(Boolean);
writeFileSync('.qa/iteration2-report.json', JSON.stringify(report, null, 2));
const summary = report.runs.map(run => ({ viewport: run.viewport, checks: run.checks, screenshots: run.screenshots.length, failures: run.failures, pageErrors: run.pageErrors, console: run.console, badResponses: run.badResponses, requestFailures: run.requestFailures, externalRequests: run.externalRequests, smallTouch: run.screenshots.filter(item => item.smallTouch.length).map(item => ({ stage: item.name, items: item.smallTouch })), clipped: run.screenshots.filter(item => item.clipped.length).map(item => ({ stage: item.name, items: item.clipped })), tiny: run.screenshots.filter(item => item.tiny.length).map(item => ({ stage: item.name, items: item.tiny })) }));
writeFileSync('.qa/iteration2-summary.json', JSON.stringify(summary, null, 2));
await composeOverview();
console.log(JSON.stringify(summary.map(({ smallTouch, clipped, tiny, ...result }) => ({ ...result, smallTouchStates: smallTouch.length, clippedStates: clipped.length, tinyStates: tiny.length })), null, 2));
process.exitCode = summary.some(run => run.failures.length || run.pageErrors.length || run.console.some(event => event.type === 'error') || run.badResponses.length || run.requestFailures.length || run.externalRequests.length) ? 1 : 0;
