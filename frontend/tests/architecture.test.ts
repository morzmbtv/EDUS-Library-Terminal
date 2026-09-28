import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
const root = new URL('../', import.meta.url);
const walk = (dir: string): string[] =>
  fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]));
test('frontend runtime has no native, cloud, mock engine, persistent operational storage or cross-repository access', () => {
  for (const file of walk(new URL('src', root).pathname.replace(/^\/(\w:)/, '$1'))) {
    const source = fs.readFileSync(file, 'utf8');
    assert.doesNotMatch(
      source,
      /@tauri-apps|tauriAdapter|createMockAdapter|EDUS_DEMO_RUNTIME|localStorage|indexedDB|edus-library-backend\/|edus-library-terminal\/|ExecuteSql|NamedPipe/,
    );
  }
  const pkg = JSON.parse(fs.readFileSync(new URL('package.json', root), 'utf8'));
  assert.deepEqual(Object.keys(pkg.dependencies).sort(), ['@lucide/vue', 'vue']);
});
test('safe settings have language but no UAT admin controls; agreed navigation/card/search preserved', () => {
  const app = fs.readFileSync(new URL('src/app/App.vue', root), 'utf8');
  assert.match(app, /setLocale\('ru'\)/);
  assert.match(app, /setLocale\('kk'\)/);
  assert.doesNotMatch(
    app,
    /TestReadersPanel|ServiceDiagnostics|request_admin|Создать администратора|На главную|RegistrationView/,
  );
  const card = fs.readFileSync(new URL('src/shared/ui/ReaderIdentification.vue', root), 'utf8');
  assert.match(card, /По лицу/);
  assert.match(card, /или/);
  assert.doesNotMatch(card, /ReaderSearch/);
  const search = fs.readFileSync(new URL('src/pages/LibrarySearchView.vue', root), 'utf8');
  assert.match(search, /TerminalBackButton/);
  assert.match(search, /activate-on-focus/);
  assert.doesNotMatch(search, /catalogueAvailability|searchCatalogue/);
});
