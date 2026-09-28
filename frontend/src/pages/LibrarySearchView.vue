<script setup lang="ts">
import { t } from '../shared/i18n/index.ts';
import { computed, nextTick, ref, watch } from 'vue';
import {
  ArrowRight,
  BookOpen,
  CalendarDays,
  CheckCircle2,
  CircleAlert,
  MapPin,
  Search,
  X,
} from '@lucide/vue';
import type { CatalogSearchResult, CatalogSort } from '../shared/types/catalogSearch.ts';
import { formatInventoryStatus } from '../shared/types/catalogSearch.ts';
import type { Snapshot, Title } from '../shared/types/terminalTypes.ts';
import { api } from '../shared/api/LocalServiceApiClient.ts';
import { formatDate } from '../shared/i18n/index.ts';
import TouchInput from '../shared/ui/TouchInput.vue';
import TerminalBackButton from '../shared/ui/TerminalBackButton.vue';

const props = defineProps<{ modelValue: string; snapshot: Snapshot; serverAvailable: boolean }>();
const emit = defineEmits<{
  'update:modelValue': [value: string];
  back: [];
  keyboard: [];
  issue: [intent: { titleId: string; copyId?: string }];
  reserve: [titleId: string];
}>();
const input = ref<InstanceType<typeof TouchInput> | null>(null);
const status = ref<'idle' | 'searching' | 'results' | 'empty' | 'error'>('idle');
const sort = ref<CatalogSort>('relevance');
const found = ref<CatalogSearchResult[]>([]);
const selectedId = ref<string | null>(null);
const related = ref(false);
const visibleCount = ref(60);
const visibleResults = computed(() => found.value.slice(0, visibleCount.value));
const query = computed(() => props.modelValue.trim());
const selected = computed(() => found.value.find((result) => result.title.id === selectedId.value) ?? null);
const isAvailable = (result: CatalogSearchResult) => result.availableCopies + result.legacyAvailable > 0;
const countLabel = computed(
  () =>
    found.value.length +
    ' ' +
    (found.value.length === 1 ? 'издание' : found.value.length < 5 ? 'издания' : 'изданий'),
);
let searchGeneration = 0;
async function runSearch() {
  const generation = ++searchGeneration;
  related.value = false;
  visibleCount.value = 60;
  if (!query.value) {
    status.value = 'idle';
    found.value = [];
    selectedId.value = null;
    return;
  }
  status.value = 'searching';
  try {
    const results = await api.searchBooks(query.value, sort.value);
    if (generation !== searchGeneration) return;
    found.value = results;
    selectedId.value = results[0]?.title.id ?? null;
    status.value = results.length ? 'results' : 'empty';
  } catch {
    if (generation === searchGeneration) {
      status.value = 'error';
      found.value = [];
      selectedId.value = null;
    }
  }
}
function backToResults() {
  if (selectedId.value) {
    selectedId.value = null;
    return;
  }
  emit('back');
}
function clearSearch() {
  emit('update:modelValue', '');
  found.value = [];
  selectedId.value = null;
  related.value = false;
  status.value = 'idle';
  void nextTick(() => input.value?.focus());
}
function changeSort(event: Event) {
  const value = (event.target as { value?: unknown }).value;
  if (typeof value === 'string') sort.value = value as CatalogSort;
  if (query.value) runSearch();
}
async function relatedEditions() {
  if (!selected.value) return;
  const generation = ++searchGeneration;
  const author = selected.value.title.author;
  related.value = true;
  status.value = 'searching';
  try {
    const results = await api.searchBooks(author, 'title');
    if (generation !== searchGeneration) return;
    found.value = results.filter((item) => item.title.author === author);
    selectedId.value = found.value[0]?.title.id ?? null;
    status.value = found.value.length ? 'results' : 'empty';
  } catch {
    if (generation === searchGeneration) status.value = 'error';
  }
}
function formattedDate(date: string) {
  const parsed = new Date(date.slice(0, 10) + 'T00:00:00');
  return Number.isNaN(parsed.valueOf())
    ? t('Дата не указана')
    : formatDate(parsed, { day: 'numeric', month: 'long' });
}
watch(
  () => props.snapshot,
  async () => {
    if (status.value === 'results') {
      const id = selectedId.value;
      await runSearch();
      if (found.value.some((row) => row.title.id === id)) selectedId.value = id;
    }
  },
);
function metadata(title: Title) {
  return [title.subject, title.language, title.year].filter(Boolean).join(' · ');
}
defineExpose({ submit: runSearch, focus: () => input.value?.focus() });
</script>

<template>
  <section class="library-search-view" aria-labelledby="library-search-heading">
    <div class="library-search-topline"><TerminalBackButton @click="backToResults" /></div>
    <header class="library-search-heading">
      <Search :size="42" aria-hidden="true" />
      <div>
        <h1 id="library-search-heading">{{ t('Найти книгу') }}</h1>
        <p>{{ t('Введите название, автора или ISBN') }}</p>
      </div>
    </header>
    <form class="library-search-form" @submit.prevent="runSearch">
      <TouchInput
        ref="input"
        :model-value="modelValue"
        :label="t('Поиск по каталогу')"
        :placeholder="t('Название, автор или ISBN')"
        inputmode="none"
        autocomplete="off"
        :keyboard="false"
        activate-on-focus
        @update:model-value="emit('update:modelValue', $event)"
        @keyboard="emit('keyboard')"
        @enter="runSearch"
      />
      <button
        v-if="query"
        class="clear-search"
        type="button"
        :aria-label="t('Очистить поиск')"
        @click="clearSearch"
      >
        <X />
      </button>
      <button class="btn primary library-search-submit" type="submit" :disabled="!query">
        <Search />{{ t('Найти') }}
      </button>
    </form>
    <div v-if="status === 'idle'" class="library-search-empty" aria-live="polite">
      <Search :stroke-width="1.6" />
      <h2>{{ t('Найдите нужное издание') }}</h2>
      <p>
        {{ t('Введите название, автора или ISBN. Терминал покажет доступность книг и ожидаемые возвраты.') }}
      </p>
    </div>
    <div v-else-if="status === 'searching'" class="library-search-empty" aria-live="polite">
      <BookOpen :stroke-width="1.6" />
      <h2>{{ t('Ищем в каталоге…') }}</h2>
    </div>
    <div v-else-if="status === 'error'" class="library-search-empty error-state" role="alert">
      <CircleAlert />
      <h2>{{ t('Каталог сейчас недоступен') }}</h2>
      <p>{{ t('Нет связи с локальным сервером. Восстановите соединение и повторите поиск.') }}</p>
      <button class="btn primary" type="button" @click="runSearch">{{ t('Повторить') }}</button>
    </div>
    <div v-else-if="status === 'empty'" class="library-search-empty" aria-live="polite">
      <BookOpen :stroke-width="1.6" />
      <h2>{{ t('Ничего не найдено') }}</h2>
      <p>
        {{
          related
            ? t('Для выбранного автора других изданий пока нет.')
            : t('Проверьте написание названия, автора или ISBN.')
        }}
      </p>
      <button class="btn secondary" type="button" @click="clearSearch">{{ t('Очистить поиск') }}</button>
    </div>
    <div v-else class="catalogue-workspace">
      <section class="catalogue-results" :aria-label="t('Результаты поиска')">
        <div class="results-toolbar">
          <strong>{{ related ? t('Другие издания автора') : t('Найдено: ') + countLabel }}</strong
          ><label
            >{{ t('Сортировка:')
            }}<select :value="sort" @change="changeSort">
              <option value="relevance">{{ t('По релевантности') }}</option>
              <option value="title">{{ t('По названию') }}</option>
              <option value="available">{{ t('По доступности') }}</option>
            </select></label
          >
        </div>
        <div class="result-scroll" role="listbox" :aria-label="t('Издания')">
          <button
            v-for="result in visibleResults"
            :key="result.title.id"
            class="catalogue-row"
            :class="{ selected: selectedId === result.title.id }"
            type="button"
            role="option"
            :aria-selected="selectedId === result.title.id"
            @click="selectedId = result.title.id"
          >
            <span class="book-placeholder"><BookOpen :stroke-width="1.7" /></span
            ><span class="catalogue-row-copy"
              ><strong>{{ result.title.name }}</strong
              ><span>{{ result.title.author }}</span
              ><small>{{ metadata(result.title) }}</small></span
            >
            <span class="catalogue-row-availability" :class="{ unavailable: !isAvailable(result) }"
              ><strong
                >{{ result.totalCopies + result.legacyTotal }}
                {{ result.totalCopies + result.legacyTotal === 1 ? t('экземпляр') : t('экземпляра') }}</strong
              ><span v-if="isAvailable(result)"
                >{{ t('Доступно:') }} {{ result.availableCopies + result.legacyAvailable }}</span
              ><span v-else>{{ t('Нет в наличии') }}</span></span
            ><ArrowRight class="row-arrow" aria-hidden="true" />
          </button>
          <button
            v-if="visibleCount < found.length"
            class="btn secondary"
            type="button"
            @click="visibleCount += 60"
          >
            {{ t('Показать ещё 60 изданий') }}
          </button>
        </div>
      </section>
      <aside v-if="selected" class="catalogue-detail" aria-live="polite">
        <div class="detail-title">
          <span class="book-placeholder large"><BookOpen :stroke-width="1.6" /></span>
          <div>
            <h2>{{ selected.title.name }}</h2>
            <p>{{ selected.title.author }}</p>
          </div>
        </div>
        <dl class="detail-metadata">
          <div>
            <dt>{{ t('Жанр') }}</dt>
            <dd>{{ selected.title.subject }}</dd>
          </div>
          <div>
            <dt>{{ t('Язык') }}</dt>
            <dd>{{ selected.title.language }}</dd>
          </div>
          <div>
            <dt>{{ t('Год издания') }}</dt>
            <dd>{{ selected.title.year }}</dd>
          </div>
          <div>
            <dt>ISBN</dt>
            <dd>{{ selected.title.isbn || t('ISBN не указан') }}</dd>
          </div>
        </dl>
        <div class="availability-panel" :class="{ unavailable: !isAvailable(selected) }">
          <template v-if="isAvailable(selected)"
            ><CheckCircle2 />
            <div>
              <strong>{{ t('Есть свободные экземпляры') }}</strong
              ><span
                >{{ t('Доступно сейчас:') }} {{ selected.availableCopies + selected.legacyAvailable }}</span
              >
            </div></template
          ><template v-else
            ><CircleAlert />
            <div>
              <strong>{{ t('Свободных экземпляров нет') }}</strong
              ><span>{{ t('Проверьте состояния экземпляров в списке ниже.') }}</span>
            </div></template
          >
        </div>
        <div class="detail-actions">
          <button
            v-if="isAvailable(selected)"
            class="btn primary"
            type="button"
            @click="emit('issue', { titleId: selected.title.id, copyId: selected.availableCopyIds[0] })"
          >
            {{ t('Выдать книгу') }}<ArrowRight /></button
          ><button v-else class="btn primary" type="button" @click="emit('reserve', selected.title.id)">
            {{ t('Поставить в очередь') }}<ArrowRight /></button
          ><button class="btn secondary" type="button" @click="relatedEditions">
            {{ t('Другие издания') }}
          </button>
        </div>
        <div v-if="selected.legacyTotal" class="legacy-note">
          <strong>{{ t('По учёту доступно:') }} {{ selected.legacyAvailable }}</strong
          ><span>{{ t('Книги без индивидуальных номеров: проверьте наличие на полке.') }}</span>
        </div>
        <div v-if="selected.nearestDueDate || selected.hasOverdue" class="return-note">
          <CalendarDays />
          <div>
            <strong v-if="selected.nearestDueDate"
              >{{ t('Ближайший ожидаемый возврат:') }} {{ formattedDate(selected.nearestDueDate) }}</strong
            ><strong v-else>{{ t('Дата возврата не указана') }}</strong
            ><span
              >{{ t('Дата может измениться')
              }}<span v-if="selected.hasOverdue"> {{ t('· есть просроченные выдачи') }}</span></span
            >
          </div>
        </div>
        <div v-if="selected.queueCount" class="queue-count">
          {{ t('В очереди:') }} {{ selected.queueCount }}
          {{ selected.queueCount === 1 ? t('человек') : t('человека') }}
        </div>
        <section class="copy-section">
          <h3>{{ t('Экземпляры в фонде') }}</h3>
          <div class="copy-list">
            <div v-for="copy in selected.copies" :key="copy.id" class="copy-row">
              <div>
                <strong>{{ copy.code }}</strong
                ><span v-if="copy.status === 'ON_LOAN' && copy.dueDate"
                  >{{ t('Ожидаемый возврат:') }} {{ formattedDate(copy.dueDate) }}</span
                ><span v-else-if="copy.location"><MapPin :size="15" />{{ copy.location }}</span
                ><span v-else-if="copy.status === 'ON_LOAN'">{{ t('Дата возврата не указана') }}</span>
              </div>
              <span class="copy-status" :class="copy.status.toLowerCase()">{{
                formatInventoryStatus(copy.status)
              }}</span>
            </div>
            <p v-if="!selected.copies.length && selected.legacyTotal" class="muted">
              {{ t('Индивидуальные номера не ведутся.') }}
            </p>
          </div>
        </section>
      </aside>
    </div>
  </section>
</template>

<style scoped>
.library-search-view {
  width: min(1460px, 100%);
  height: 100%;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}
.library-search-topline {
  height: 56px;
  display: flex;
  align-items: center;
}
.library-search-heading {
  display: flex;
  align-items: center;
  gap: 16px;
}
.library-search-heading svg {
  color: var(--edus-blue);
}
.library-search-heading h1 {
  font-size: 40px;
  line-height: 1.1;
}
.library-search-heading p {
  margin-top: 6px;
  color: var(--muted);
  font-size: 21px;
}
.library-search-form {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 196px;
  gap: 14px;
  align-items: end;
  position: relative;
}
.library-search-form :deep(.touch-field label) {
  position: absolute;
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
  width: 1px;
  height: 1px;
  overflow: hidden;
  white-space: nowrap;
}
.library-search-submit {
  min-height: 72px;
  font-size: 21px;
}
.clear-search {
  position: absolute;
  right: 218px;
  bottom: 12px;
  width: 56px;
  height: 56px;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: var(--muted);
  display: grid;
  place-items: center;
  z-index: 2;
}
.clear-search:hover {
  background: var(--blue-soft);
  color: var(--edus-blue);
}
.library-search-empty {
  min-height: 0;
  flex: 1;
  border: 1px dashed #b9d8f8;
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  text-align: center;
  gap: 14px;
  padding: 36px;
  color: var(--muted);
}
.library-search-empty svg {
  width: 56px;
  height: 56px;
  color: var(--edus-blue);
}
.library-search-empty h2 {
  color: var(--edus-navy);
  font-size: 26px;
}
.library-search-empty p {
  max-width: 600px;
  font-size: 20px;
  line-height: 1.45;
}
.error-state {
  border-color: #f2bbc0;
}
.error-state svg {
  color: var(--danger);
}
.catalogue-workspace {
  min-height: 0;
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 60%) minmax(390px, 40%);
  gap: 16px;
}
.catalogue-results,
.catalogue-detail {
  min-height: 0;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
}
.catalogue-results {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.results-toolbar {
  min-height: 62px;
  padding: 10px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--line);
  font-size: 18px;
}
.results-toolbar strong {
  color: var(--muted);
  font-weight: 550;
}
.results-toolbar label {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--muted);
  font-size: 17px;
}
.results-toolbar select {
  height: 56px;
  padding: 0 28px 0 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface);
  color: var(--edus-navy);
  font: inherit;
}
.result-scroll {
  overflow: auto;
  min-height: 0;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.catalogue-row {
  width: 100%;
  min-height: 100px;
  display: grid;
  grid-template-columns: 56px minmax(0, 1fr) minmax(120px, auto) 24px;
  gap: 14px;
  align-items: center;
  text-align: left;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  padding: 12px 14px;
  color: var(--edus-navy);
  cursor: pointer;
}
.catalogue-row:hover,
.catalogue-row:focus-visible {
  border-color: var(--edus-blue);
  outline: none;
  background: #f8fbff;
}
.catalogue-row.selected {
  border-color: var(--edus-blue);
  background: var(--blue-soft);
  box-shadow: inset 0 0 0 1px var(--edus-blue);
}
.book-placeholder {
  width: 52px;
  height: 64px;
  border: 1px solid #c9d7e6;
  border-radius: 8px;
  background: #f4f8fc;
  color: var(--edus-blue);
  display: grid;
  place-items: center;
}
.book-placeholder svg {
  width: 30px;
  height: 30px;
}
.catalogue-row-copy {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.catalogue-row-copy strong {
  font-size: 20px;
  line-height: 1.2;
}
.catalogue-row-copy span {
  font-size: 17px;
  color: var(--muted);
}
.catalogue-row-copy small {
  font-size: 18px;
  color: var(--muted);
}
.catalogue-row-availability {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--success);
  font-size: 17px;
}
.catalogue-row-availability strong {
  color: var(--edus-navy);
  font-size: 17px;
}
.catalogue-row-availability.unavailable span {
  color: var(--danger);
}
.row-arrow {
  color: #587096;
}
.catalogue-detail {
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.detail-title {
  display: flex;
  gap: 16px;
  align-items: center;
}
.book-placeholder.large {
  width: 90px;
  height: 112px;
  border-radius: 10px;
}
.book-placeholder.large svg {
  width: 40px;
  height: 40px;
}
.detail-title h2 {
  font-size: 27px;
  line-height: 1.15;
}
.detail-title p {
  font-size: 19px;
  color: var(--muted);
  margin-top: 7px;
}
.detail-metadata {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 16px;
  margin: 0;
}
.detail-metadata div {
  display: flex;
  gap: 8px;
  font-size: 17px;
}
.detail-metadata dt {
  color: var(--muted);
}
.detail-metadata dd {
  margin: 0;
  word-break: break-word;
}
.availability-panel,
.return-note,
.legacy-note {
  padding: 14px;
  border-radius: 12px;
  display: flex;
  gap: 12px;
  align-items: flex-start;
  background: #e9f8f0;
  color: #087c51;
}
.availability-panel.unavailable {
  background: #fff0f0;
  color: var(--danger);
}
.availability-panel svg,
.return-note svg {
  flex: 0 0 28px;
  width: 28px;
  height: 28px;
}
.availability-panel div,
.return-note div,
.legacy-note {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.availability-panel strong,
.return-note strong,
.legacy-note strong {
  font-size: 19px;
}
.availability-panel span,
.return-note span,
.legacy-note span {
  font-size: 18px;
  line-height: 1.35;
}
.legacy-note {
  background: var(--gold-soft);
  color: var(--edus-navy);
}
.return-note {
  background: var(--blue-soft);
  color: var(--edus-navy);
}
.queue-count {
  font-size: 18px;
  font-weight: 600;
  color: var(--edus-navy);
}
.copy-section {
  border-top: 1px solid var(--line);
  padding-top: 14px;
}
.copy-section h3 {
  font-size: 19px;
}
.copy-list {
  margin-top: 8px;
}
.copy-row {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 0;
  border-bottom: 1px solid var(--line);
}
.copy-row > div {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.copy-row strong {
  font-size: 17px;
}
.copy-row div > span {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--muted);
  font-size: 18px;
}
.copy-status {
  align-self: center;
  padding: 5px 9px;
  border-radius: 99px;
  background: var(--blue-soft);
  color: var(--edus-blue);
  font-size: 14px;
  white-space: nowrap;
}
.copy-status.available {
  background: #e9f8f0;
  color: var(--success);
}
.copy-status.on_loan {
  background: #eef3f9;
  color: #48637f;
}
.copy-status.repair,
.copy-status.lost,
.copy-status.written_off {
  background: #fff0f0;
  color: var(--danger);
}
.copy-status.verifying,
.copy-status.reserved {
  background: var(--gold-soft);
  color: #7a5600;
}
.detail-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.detail-actions .primary {
  grid-column: 1/-1;
}
.detail-actions .btn {
  min-height: 64px;
  font-size: 18px;
}
.muted {
  color: var(--muted);
  font-size: 18px;
}
@media (max-height: 820px) and (min-width: 900px) {
  .library-search-view {
    gap: 10px;
  }
  .library-search-heading h1 {
    font-size: 34px;
  }
  .library-search-heading p {
    font-size: 19px;
  }
  .catalogue-row {
    min-height: 84px;
    padding: 9px 12px;
  }
  .book-placeholder {
    width: 46px;
    height: 56px;
  }
  .catalogue-row-copy strong {
    font-size: 20px;
  }
  .catalogue-row-copy span {
    font-size: 18px;
  }
  .catalogue-detail {
    padding: 15px;
    gap: 11px;
  }
  .detail-title h2 {
    font-size: 23px;
  }
  .detail-title p {
    font-size: 17px;
  }
  .book-placeholder.large {
    width: 72px;
    height: 86px;
  }
  .availability-panel,
  .return-note,
  .legacy-note {
    padding: 11px;
  }
  .copy-row {
    padding: 7px 0;
  }
  .detail-actions .btn {
    min-height: 58px;
  }
  .results-toolbar {
    min-height: 52px;
  }
  .library-search-form :deep(.input-wrap) {
    min-height: 64px;
  }
  .library-search-form :deep(input) {
    min-height: 62px;
  }
  .library-search-submit {
    min-height: 64px;
  }
  .clear-search {
    bottom: 8px;
  }
}
@media (max-width: 900px) {
  .catalogue-workspace {
    grid-template-columns: 1fr;
  }
  .catalogue-detail {
    min-height: 540px;
  }
  .library-search-view {
    height: auto;
  }
  .catalogue-results {
    min-height: 450px;
  }
  .library-search-empty {
    min-height: 280px;
  }
  .clear-search {
    right: 214px;
  }
}
@media (max-width: 620px) {
  .library-search-heading h1 {
    font-size: 31px;
  }
  .library-search-form {
    grid-template-columns: 1fr;
  }
  .library-search-submit {
    min-height: 64px;
  }
  .clear-search {
    right: 12px;
    bottom: 82px;
  }
  .catalogue-row {
    grid-template-columns: 46px minmax(0, 1fr) 20px;
  }
  .catalogue-row-availability {
    grid-column: 2/3;
  }
  .row-arrow {
    grid-column: 3;
    grid-row: 1/3;
  }
  .book-placeholder {
    width: 42px;
    height: 52px;
  }
  .results-toolbar {
    align-items: flex-start;
    flex-direction: column;
  }
  .detail-actions {
    grid-template-columns: 1fr;
  }
}
</style>
