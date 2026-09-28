import fs from 'node:fs';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(
  process.env.EDUS_PLAYWRIGHT_PACKAGE ??
    'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/package.json',
);
const { chromium } = require('playwright');
const dir = process.env.EDUS_E2E_OUTPUT ?? 'E:/Codex/artifacts/edus-split-2.0-frontend-e2e';
fs.mkdirSync(dir, { recursive: true });
const browser = await chromium.launch({ channel: 'msedge', headless: false });
const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
const page = await context.newPage();
const errors = [];
const checks = [];
let faults = false;
page.on('pageerror', (e) => errors.push(e.message));
page.on('response', (r) => {
  if (!faults && r.status() >= 400) errors.push(r.status() + ' ' + r.url());
});
const shot = async (name) => {
  await page.screenshot({ path: dir + '/' + name + '.png' });
  const bounds = await page.evaluate(() => ({
    width: innerWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  assert.equal(bounds.width, 1280);
  assert.ok(bounds.scroll <= 1280);
};
const card = async () => {
  await page.locator('.identify-content').waitFor();
  await page.keyboard.type('00001009', { delay: 10 });
  await page.keyboard.press('Enter');
  await page.locator('.screen-scan').waitFor();
};
try {
  await page.goto('http://127.0.0.1:43180');
  await page.locator('.home-action').first().waitFor();
  await page.locator('.system-header button').last().click();
  await page.getByRole('button', { name: 'Русский', exact: true }).click();
  await page.getByRole('button', { name: 'Закрыть настройки', exact: true }).click();
  await shot('home-ru');
  checks.push('Edge home1280x800');
  const snap = await page.evaluate(async () => {
    const r = await fetch('/api/local/v1/snapshot');
    return r.json();
  });
  const copy = snap.copies.find(
    (c) => c.status === 'AVAILABLE' && !snap.loans.some((l) => l.copyId === c.id),
  );
  assert.ok(copy);
  const title = snap.titles.find((t) => t.id === copy.titleId);
  assert.ok(title);
  fs.writeFileSync(
    dir + '/fixture.json',
    JSON.stringify({ copyId: copy.id, titleId: title.id, code: copy.code }, null, 2),
  );
  await page.locator('.home-action.issue').click();
  await page.getByRole('button', { name: 'По лицу', exact: true }).click();
  await page.locator('.face-identification').waitFor();
  await shot('face-ru');
  await page.getByRole('button', { name: 'Назад', exact: true }).click();
  await shot('card-ru');
  await card();
  await page.keyboard.type(copy.code, { delay: 10 });
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Перейти к подтверждению', exact: true }).click();
  await shot('issue-confirm');
  await page.getByRole('button', { name: /^Выдать 1 книгу/ }).click();
  await page.getByRole('heading', { name: 'Книги выданы', exact: true }).waitFor();
  checks.push('reader,faceback,issue committed');
  await page.goto('http://127.0.0.1:43180');
  await page.locator('.home-action.search').click();
  const input = page.locator('.library-search-form input');
  await input.click();
  await page.locator('.touch-keyboard').waitFor();
  await input.fill(title.name);
  await page.locator('.library-search-submit').click();
  await page.locator('.catalogue-row').first().waitFor();
  await shot('search-keyboard');
  checks.push('book search via HTTP, keyboard');
  await page.goto('http://127.0.0.1:43180');
  await page.locator('.home-action.return').click();
  await card();
  await page.locator('.return-loan-list').waitFor();
  await shot('return-unmarked');
  await page.keyboard.type(copy.code, { delay: 10 });
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: /Перейти к подтверждению/ }).click();
  await page.getByRole('button', { name: /^Принять 1 книгу/ }).click();
  await page.getByRole('heading', { name: 'Книги приняты', exact: true }).waitFor();
  checks.push('return committed after browser restart');
  await page.goto('http://127.0.0.1:43180');
  const unavailable = snap.titles.find((t) => t.id === 'title-history');
  assert.ok(unavailable);
  await page.locator('.home-action.search').click();
  await page.locator('.library-search-form input').fill(unavailable.name);
  await page.locator('.library-search-submit').click();
  await page.getByRole('button', { name: 'Поставить в очередь', exact: true }).click();
  await page.locator('.identify-content').waitFor();
  await page.keyboard.type('00001009', { delay: 10 });
  await page.keyboard.press('Enter');
  await page.locator('.reservation-confirmation').waitFor();
  await page.getByRole('button', { name: 'Поставить в очередь', exact: true }).click();
  await page.getByRole('heading', { name: 'Вы записаны в очередь', exact: true }).waitFor();
  await shot('reservation-confirmed');
  checks.push('reservation committed via Edge after identification');

  await page.goto('http://127.0.0.1:43180');
  await page.getByRole('button', { name: 'Настройки', exact: true }).click();
  await page.getByRole('button', { name: 'Қазақша', exact: true }).click();
  await page.waitForTimeout(350);
  await page.getByRole('button', { name: 'Баптауларды жабу', exact: true }).click();
  await shot('home-kk');
  const second = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const another = await second.newPage();
  await another.goto('http://127.0.0.1:43180');
  await another.getByRole('heading', { name: 'Кітапхана терминалы', exact: true }).waitFor();
  assert.equal(await another.evaluate(() => localStorage.length), 0);
  await another.screenshot({ path: dir + '/new-private-session-kk.png' });
  await second.close();
  checks.push('language survives new browser session; no operational storage');
  await page.getByRole('button', { name: 'Көмек', exact: true }).click();
  await shot('help-kk');
  await page.keyboard.press('Escape');
  faults = true;
  await page.route('**/health', (route) => route.abort());
  await page.goto('http://127.0.0.1:43180');
  await page.getByRole('button', { name: /Повторить проверку|Тексеруді қайталау/ }).waitFor();
  await shot('service-unavailable');
  await page.unroute('**/health');
  checks.push('service unavailable controlled state');
  await page.route('**/api/local/v1/version', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        backend_version: '99',
        local_api_version: '99',
        minimum_frontend_version: '99',
        maximum_frontend_version: '99',
      }),
    }),
  );
  await page.goto('http://127.0.0.1:43180');
  await page
    .getByText('Версии компонентов EDUS несовместимы. Требуется обновление.', { exact: true })
    .waitFor();
  await shot('version-incompatible');
  await page.unroute('**/api/local/v1/version');
  checks.push('version mismatch fails closed');
  assert.deepEqual(errors, []);
  fs.writeFileSync(
    dir + '/result.json',
    JSON.stringify(
      {
        checks,
        errors,
        status: 'PASS',
        scope: 'actual Edge + console backend; no SCM/Assigned Access/hardware',
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify({ checks, errors, status: 'PASS' }));
} catch (error) {
  await page.screenshot({ path: dir + '/failure.png' });
  fs.writeFileSync(dir + '/failure.txt', String(error) + '\n' + (await page.locator('body').innerText()));
  throw error;
} finally {
  await browser.close();
}
