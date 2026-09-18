<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { ArrowLeft, ArrowRight, Barcode, BookOpen, Check, ChevronRight, Layers3, LoaderCircle, Plus, Search, Trash2, X } from '@lucide/vue'
import { DomainError, type AccountingMode, type Title } from '../domain/terminalTypes'
import { adapter, refresh, screen, result, operationId, busy, message, errorText, server, sessionExpired, home } from '../composables/useTerminal'
import ActionBar from '../components/ActionBar.vue'
import TouchInput from '../components/TouchInput.vue'
import TouchKeyboard from '../components/TouchKeyboard.vue'
import QuantityControl from '../components/QuantityControl.vue'
import StateMessage from '../components/StateMessage.vue'

type Stage = 'search' | 'details' | 'metadata' | 'mode' | 'copies' | 'review'
type FormField = 'name' | 'author' | 'isbn' | 'publisher' | 'year' | 'language' | 'subject' | 'grade'
type KeyboardField = FormField | 'search' | 'code'
const stage = ref<Stage>('search')
const searchQuery = ref('')
const searchResults = ref<Title[]>([])
const searching = ref(false)
const searched = ref(false)
const selected = ref<Title | null>(null)
const form = reactive<Record<FormField, string>>({ name: '', author: '', isbn: '', publisher: '', year: '', language: '', subject: '', grade: '' })
const errors = reactive<Partial<Record<FormField, string>>>({})
const mode = ref<AccountingMode | null>(null)
const codes = ref<string[]>([])
const codeValue = ref('')
const quantity = ref(1)
const keyboard = ref<KeyboardField | null>(null)
const leaveOpen = ref(false)
const leaveDialog = ref<HTMLElement | null>(null)
const notice = ref('')
const titleName = computed(() => selected.value?.name || form.name)
const draftTitle = computed<Omit<Title, 'id'>>(() => ({ ...form, year: Number(form.year) }))
const total = computed(() => mode.value === 'COPY' ? codes.value.length : quantity.value)
const hasDraft = computed(() => !!selected.value || Object.values(form).some(Boolean) || codes.value.length > 0)
const locked = computed(() => busy.value || sessionExpired.value)
const fields = computed<FormField[]>(() => stage.value === 'metadata' ? ['year', 'language', 'subject', 'grade'] : ['name', 'author', 'isbn', 'publisher'])
const labels: Record<KeyboardField, string> = { name: 'Название книги', author: 'Автор', isbn: 'ISBN · если есть', publisher: 'Издательство', year: 'Год издания', language: 'Язык', subject: 'Предмет · если есть', grade: 'Класс · если есть', search: 'ISBN, название или автор', code: 'Существующий инвентарный номер' }
const placeholders: Record<FormField, string> = { name: 'Полное название издания', author: 'Фамилия и инициалы автора', isbn: '10 или 13 знаков', publisher: 'Название издательства', year: 'Например, 2024', language: 'Например, Қазақша', subject: 'Например, математика', grade: 'Например, 7 или 10–11' }
const headings = computed(() => ({ search: 'Найдите издание', details: selected.value ? 'Проверьте карточку издания' : 'Новое издание · основные сведения', metadata: 'Новое издание · об издании', mode: 'Как учитывать эти книги?', copies: mode.value === 'COPY' ? 'Введите инвентарные номера' : 'Укажите количество книг', review: 'Всё готово к добавлению?' })[stage.value])
const instructions = computed(() => ({ search: 'Сканируйте ISBN или найдите книгу по названию и автору.', details: selected.value ? 'Добавим книги к этой карточке в библиотечном фонде.' : 'Шаг 1 из 2. Название и автор обязательны; ISBN можно пропустить.', metadata: 'Шаг 2 из 2. Укажите год и язык. Предмет и класс — при наличии.', mode: 'Выберите способ, который уже используется в вашем фонде.', copies: mode.value === 'COPY' ? 'Сканируйте или вводите номера с книг. Новые наклейки не нужны.' : 'Для одинаковых книг без индивидуальных номеров.', review: 'Проверьте издание, способ учёта и количество.' })[stage.value])
const keyboardValue = computed({
  get: () => keyboard.value === 'search' ? searchQuery.value : keyboard.value === 'code' ? codeValue.value : keyboard.value ? form[keyboard.value] : '',
  set: (value: string) => { if (keyboard.value === 'search') searchQuery.value = value; else if (keyboard.value === 'code') codeValue.value = value; else if (keyboard.value) { form[keyboard.value] = value.slice(0, 300); delete errors[keyboard.value] } },
})
const canContinue = computed(() => !locked.value && (stage.value === 'details' || stage.value === 'metadata' || (stage.value === 'mode' && !!mode.value) || (stage.value === 'copies' && total.value > 0) || stage.value === 'review'))
const continueLabel = computed(() => stage.value === 'review' ? `Добавить в фонд · ${total.value} шт.` : stage.value === 'copies' ? 'Проверить и добавить' : stage.value === 'details' && selected.value ? 'Добавить книги этого издания' : 'Продолжить')
let searchTimer: ReturnType<typeof setTimeout> | undefined

function findTitles() {
  clearTimeout(searchTimer)
  if (!searchQuery.value.trim()) { searchResults.value = []; searching.value = false; searched.value = false; return }
  if (!server.value) { searching.value = false; message.value = 'Нет связи с локальным сервером. Поиск продолжится после восстановления соединения.'; return }
  searchResults.value = adapter.searchTitles(searchQuery.value)
  searched.value = true
  searching.value = false
}
watch(searchQuery, () => {
  clearTimeout(searchTimer)
  searching.value = !!searchQuery.value.trim()
  searched.value = false
  message.value = ''
  searchTimer = setTimeout(findTitles, 250)
})
watch(server, (connected) => { if (connected && stage.value === 'search' && searchQuery.value.trim()) findTitles() })
onBeforeUnmount(() => clearTimeout(searchTimer))
watch(leaveOpen, async (open) => { if (open) { await nextTick(); leaveDialog.value?.querySelector<HTMLElement>('.primary')?.focus() } })

function go(next: Stage) { stage.value = next; keyboard.value = null; message.value = ''; notice.value = '' }
function choose(title: Title) { selected.value = title; go('details') }
function createTitle() {
  selected.value = null
  if (!form.name && !form.isbn) {
    if (/^[\dXx\s-]{10,20}$/.test(searchQuery.value.trim())) form.isbn = searchQuery.value.trim()
    else form.name = searchQuery.value.trim()
  }
  go('details')
}
function openKeyboard(field: KeyboardField) { keyboard.value = field; void nextTick(() => document.querySelector<HTMLInputElement>('.registration-work .touch-field input')?.focus()) }
function fieldVisible(field: FormField) { return !keyboard.value || keyboard.value === field }
function adjacentField(direction: number) {
  const index = fields.value.indexOf(keyboard.value as FormField)
  const next = fields.value[index + direction]
  if (next) openKeyboard(next)
}
function validate(stageToCheck: 'details' | 'metadata'): boolean {
  for (const key of Object.keys(errors) as FormField[]) delete errors[key]
  if (stageToCheck === 'details') {
    if (!form.name.trim()) errors.name = 'Введите название книги.'
    if (!form.author.trim()) errors.author = 'Введите автора.'
    const isbn = form.isbn.replace(/[\s-]/g, '').toUpperCase()
    if (isbn && !/^(\d{13}|\d{9}[\dX])$/.test(isbn)) errors.isbn = 'В ISBN должно быть 10 или 13 знаков.'
    if (isbn && adapter.searchTitles(isbn).some((title) => title.isbn.replace(/[\s-]/g, '').toUpperCase() === isbn)) errors.isbn = 'Издание с этим ISBN уже есть. Вернитесь к поиску и выберите его.'
  } else {
    const year = Number(form.year)
    if (!/^\d{4}$/.test(form.year) || !Number.isInteger(year) || year < 1400 || year > new Date().getFullYear() + 1) errors.year = 'Укажите корректный год издания.'
    if (!form.language.trim()) errors.language = 'Укажите язык издания.'
  }
  const invalid = (Object.keys(errors) as FormField[])[0]
  if (invalid) { if (keyboard.value) openKeyboard(invalid); return false }
  return true
}
function addCode(raw = codeValue.value) {
  if (locked.value) return
  const value = raw.trim()
  message.value = ''; notice.value = ''
  if (!value || value.length > 80 || [...value].some((character) => character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127)) { message.value = 'Введите инвентарный номер длиной от 1 до 80 знаков.'; return }
  if (codes.value.some((code) => code.toLocaleUpperCase('ru') === value.toLocaleUpperCase('ru'))) { message.value = 'Этот номер уже добавлен. Повторный скан не создаёт ещё один экземпляр.'; return }
  const existing = adapter.resolveCode(value)
  if (existing.kind === 'copy' && existing.title.id !== selected.value?.id) { message.value = 'Этот номер принадлежит другому изданию. Проверьте книгу и номер.'; return }
  if (codes.value.length >= 999) { message.value = 'За одну операцию можно добавить не более 999 экземпляров.'; return }
  codes.value.push(value)
  codeValue.value = ''
  notice.value = existing.kind === 'copy' ? `Номер ${value} уже есть в фонде — используем существующий экземпляр.` : `Номер ${value} добавлен в список.`
}
function removeCode(index: number) { codes.value.splice(index, 1); message.value = ''; notice.value = '' }
function scan(code: string) {
  if (locked.value) return
  if (stage.value === 'search') { searchQuery.value = code; void nextTick(findTitles) }
  else if (stage.value === 'copies' && mode.value === 'COPY') addCode(code)
}
function back() {
  if (locked.value) return
  if (keyboard.value) { keyboard.value = null; return }
  if (stage.value === 'search') requestLeave()
  else if (stage.value === 'details') go('search')
  else if (stage.value === 'metadata') go('details')
  else if (stage.value === 'mode') go(selected.value ? 'details' : 'metadata')
  else if (stage.value === 'copies') go('mode')
  else go('copies')
}
function requestLeave() { if (!locked.value) { if (hasDraft.value) leaveOpen.value = true; else home() } }
function leave() { leaveOpen.value = false; home() }
function trapLeaveFocus(event: KeyboardEvent) {
  if (event.key !== 'Tab') return
  const controls = leaveDialog.value?.querySelectorAll<HTMLElement>('button:not([disabled])')
  if (!controls?.length) return
  const first = controls[0], last = controls[controls.length - 1]
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus() }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus() }
}
function keyboardEnter() {
  if (keyboard.value === 'search') { findTitles(); keyboard.value = null }
  else if (keyboard.value === 'code') addCode()
  else {
    const index = fields.value.indexOf(keyboard.value as FormField)
    if (index < fields.value.length - 1) adjacentField(1)
    else { keyboard.value = null; advance() }
  }
}
async function advance() {
  if (!canContinue.value) return
  if (stage.value === 'details') { if (selected.value) go('mode'); else if (validate('details')) go('metadata') }
  else if (stage.value === 'metadata') { if (validate('metadata')) go('mode') }
  else if (stage.value === 'mode') go('copies')
  else if (stage.value === 'copies') { operationId.value = crypto.randomUUID(); go('review') }
  else if (stage.value === 'review') {
    if (!server.value) { message.value = 'Нет связи с локальным сервером. Список сохранён — повторите после восстановления связи.'; return }
    if (!mode.value) return
    busy.value = true; message.value = ''
    try {
      result.value = await adapter.register(selected.value?.id ?? draftTitle.value, mode.value, mode.value === 'COPY' ? [...codes.value] : [], total.value, operationId.value)
      refresh(); screen.value = 'success'
    } catch (error) {
      if (error instanceof DomainError && error.code === 'UNKNOWN') { operationId.value = error.operationId ?? operationId.value; screen.value = 'unknown' }
      else message.value = errorText(error)
    } finally { busy.value = false }
  }
}
defineExpose({ back, requestLeave, hasDraft, scan })
</script>

<template>
  <div class="registration-view" :class="{ 'keyboard-open': keyboard }">
    <div class="registration-work">
      <header class="registration-heading">
        <div><h2>{{ headings }}</h2><p v-if="!keyboard">{{ instructions }}</p></div>
        <span v-if="(stage === 'details' && !selected) || stage === 'metadata'" class="form-step">{{ stage === 'metadata' ? '2' : '1' }} / 2</span>
      </header>

      <div class="registration-content">
        <template v-if="stage === 'search'">
          <div class="search-line">
            <TouchInput v-model="searchQuery" :label="labels.search" placeholder="Сканируйте ISBN или начните вводить" :disabled="locked" @keyboard="openKeyboard('search')" @enter="findTitles" />
            <button class="btn primary search-button" :disabled="!searchQuery.trim() || locked" @click="findTitles"><Search :size="25" />Найти</button>
          </div>
          <div v-if="!keyboard" class="search-results" aria-live="polite">
            <div v-if="searching" class="empty-search"><LoaderCircle class="spin" :size="34" /><span>Ищем издание…</span></div>
            <template v-else-if="searched && searchResults.length">
              <p class="result-caption">Найдено изданий: {{ searchResults.length }}. Выберите нужное.</p>
              <button v-for="title in searchResults" :key="title.id" class="title-result" :disabled="locked" @click="choose(title)"><BookOpen :size="30" /><span><strong>{{ title.name }}</strong><span>{{ title.author }}</span><small>{{ title.publisher }} · {{ title.year }}<template v-if="title.isbn"> · ISBN {{ title.isbn }}</template></small></span><ChevronRight :size="26" /></button>
            </template>
            <div v-else-if="searched" class="empty-search"><Search :size="38" /><strong>Издание не найдено</strong><span>Проверьте написание или создайте новую карточку.</span></div>
            <div v-else class="empty-search"><Barcode :size="44" /><strong>Начните с ISBN на обложке</strong><span>Если ISBN нет, найдите издание по названию.</span></div>
          </div>
        </template>

        <template v-else-if="stage === 'details' && selected">
          <article class="edition-card panel"><div class="edition-icon"><BookOpen :size="40" /></div><div class="edition-info"><span class="eyebrow">Издание уже в фонде</span><h3>{{ selected.name }}</h3><p class="author">{{ selected.author }}</p><dl class="edition-facts"><div><dt>ISBN</dt><dd>{{ selected.isbn || 'Не указан' }}</dd></div><div><dt>Издательство / год</dt><dd>{{ selected.publisher || 'Не указано' }} · {{ selected.year }}</dd></div><div><dt>Язык</dt><dd>{{ selected.language }}</dd></div><div><dt>Предмет / класс</dt><dd>{{ selected.subject || 'Не указан' }}<template v-if="selected.grade"> · {{ selected.grade }} класс</template></dd></div></dl></div></article>
        </template>

        <template v-else-if="stage === 'details' || stage === 'metadata'">
          <div class="registration-fields" :class="{ 'one-field': keyboard }">
            <TouchInput v-for="field in fields.filter(fieldVisible)" :key="field" v-model="form[field]" :label="labels[field] + (['name', 'author', 'year', 'language'].includes(field) ? ' *' : '')" :placeholder="placeholders[field]" :error="errors[field]" :disabled="locked" :maxlength="field === 'year' ? 4 : 300" :inputmode="field === 'year' ? 'numeric' : 'text'" @keyboard="openKeyboard(field)" @enter="advance" @update:model-value="delete errors[field]" />
            <div v-if="keyboard" class="field-switch"><button class="btn secondary" :disabled="fields.indexOf(keyboard as FormField) === 0" aria-label="Предыдущее поле" @click="adjacentField(-1)"><ArrowLeft :size="24" /></button><span>{{ fields.indexOf(keyboard as FormField) + 1 }} / {{ fields.length }}</span><button class="btn secondary" :disabled="fields.indexOf(keyboard as FormField) === fields.length - 1" aria-label="Следующее поле" @click="adjacentField(1)"><ArrowRight :size="24" /></button></div>
          </div>
          <p v-if="!keyboard" class="required-note">* Обязательные поля</p>
        </template>

        <template v-else-if="stage === 'mode'">
          <div class="selected-edition"><BookOpen :size="27" /><strong>{{ titleName }}</strong></div>
          <div class="mode-grid" role="group" aria-label="Способ регистрации книг">
            <button class="mode-choice" :class="{ selected: mode === 'COPY' }" :aria-pressed="mode === 'COPY'" :disabled="locked" @click="mode = 'COPY'"><div class="mode-icon"><Barcode :size="38" /></div><strong>По инвентарным номерам</strong><span>У каждой книги есть свой номер.<br />Используем существующие коды.</span><span class="mode-check"><Check v-if="mode === 'COPY'" :size="23" /></span></button>
            <button class="mode-choice" :class="{ selected: mode === 'LEGACY_TITLE' }" :aria-pressed="mode === 'LEGACY_TITLE'" :disabled="locked" @click="mode = 'LEGACY_TITLE'"><div class="mode-icon"><Layers3 :size="38" /></div><strong>Старый фонд — количеством</strong><span>Одинаковые книги без индивидуальных<br class="desktop-break" /> номеров. Укажем общее количество.</span><span class="mode-check"><Check v-if="mode === 'LEGACY_TITLE'" :size="23" /></span></button>
          </div>
        </template>

        <template v-else-if="stage === 'copies' && mode === 'COPY'">
          <div v-if="!keyboard" class="selected-edition"><BookOpen :size="27" /><strong>{{ titleName }}</strong><span>{{ codes.length }} шт.</span></div>
          <div class="search-line"><TouchInput v-model="codeValue" :label="labels.code" placeholder="Например, 000124 или КЗ-007" :disabled="locked" maxlength="80" @keyboard="openKeyboard('code')" @enter="addCode()" /><button class="btn primary search-button" :disabled="!codeValue.trim() || locked" @click="addCode()"><Plus :size="25" />В список</button></div>
          <p v-if="notice" class="code-notice" role="status">{{ notice }}</p>
          <div v-if="!keyboard" class="code-list" aria-label="Инвентарные номера к регистрации"><div v-for="(code, index) in codes" :key="code" class="code-row"><span class="code-index">{{ index + 1 }}</span><Barcode :size="25" /><strong>{{ code }}</strong><button class="remove-code" :disabled="locked" :aria-label="`Убрать номер ${code} из списка`" @click="removeCode(index)"><Trash2 :size="24" /></button></div><p v-if="!codes.length" class="empty-codes">Добавленные номера появятся здесь. Буквы и ведущие нули сохраняются.</p></div>
        </template>

        <template v-else-if="stage === 'copies'">
          <div class="selected-edition"><BookOpen :size="27" /><strong>{{ titleName }}</strong></div>
          <div class="quantity-panel panel"><Layers3 :size="46" /><div><h3>Сколько книг добавляем?</h3><p>Укажите фактическое количество одинаковых книг.<br />В фонд будет добавлено именно это количество.</p></div><QuantityControl v-model="quantity" :min="1" :max="999" :disabled="locked" /></div>
          <StateMessage kind="info" title="Учёт без индивидуальных номеров" text="При выдаче и приёме такие книги учитываются по читателю, изданию и количеству." />
        </template>

        <template v-else-if="stage === 'review'">
          <div class="review-panel panel"><div class="review-title"><BookOpen :size="34" /><div><span class="eyebrow">{{ selected ? 'Существующее издание' : 'Новое издание' }}</span><h3>{{ titleName }}</h3><p>{{ selected?.author ?? form.author }}</p></div></div><div class="review-bottom"><div><span>Способ учёта</span><strong>{{ mode === 'COPY' ? 'По инвентарным номерам' : 'Старый фонд — количеством' }}</strong></div><div class="review-count"><span>К добавлению</span><strong>{{ total }} <small>шт.</small></strong></div></div><div v-if="mode === 'COPY'" class="review-codes"><span v-for="code in codes" :key="code">{{ code }}</span></div></div>
          <p class="review-note">{{ mode === 'COPY' ? 'Уже зарегистрированные номера будут переиспользованы без дублирования.' : 'Количество старого фонда увеличится после успешного подтверждения.' }}</p>
        </template>
      </div>
      <StateMessage v-if="message" class="registration-error" kind="error" title="Проверьте данные" :text="message" />
    </div>

    <div v-if="keyboard" class="registration-keyboard"><TouchKeyboard v-model="keyboardValue" :label="labels[keyboard]" inline @close="keyboard = null" @enter="keyboardEnter" /></div>

    <ActionBar class="registration-actions"><button class="btn secondary" :disabled="locked" @click="back"><ArrowLeft :size="24" />{{ stage === 'search' ? 'На главную' : 'Назад' }}</button><template #right><button v-if="stage === 'search'" class="btn primary" :disabled="locked" @click="createTitle"><Plus :size="25" />Создать новое издание</button><template v-else><span v-if="stage === 'review'" class="confirmation-hint">{{ busy ? 'Сохраняем в фонд…' : 'Проверьте количество перед подтверждением' }}</span><button class="btn primary continue-button" :disabled="!canContinue || (stage === 'review' && !server)" @click="advance"><LoaderCircle v-if="busy" class="spin" :size="25" />{{ continueLabel }}<ArrowRight v-if="stage !== 'review'" :size="24" /><Check v-else-if="!busy" :size="24" /></button></template></template></ActionBar>

    <div v-if="leaveOpen" class="leave-backdrop" @keydown.esc="leaveOpen = false" @click.self="leaveOpen = false"><section ref="leaveDialog" class="leave-dialog" role="dialog" aria-modal="true" aria-labelledby="registration-leave-title" @keydown="trapLeaveFocus"><button class="dialog-close" aria-label="Продолжить добавление" @click="leaveOpen = false"><X :size="27" /></button><h2 id="registration-leave-title">Выйти из добавления?</h2><p>Введённые данные ещё не сохранены в фонд. При выходе они будут очищены.</p><div><button class="btn secondary" @click="leave">Выйти на главную</button><button class="btn primary" @click="leaveOpen = false">Продолжить добавление</button></div></section></div>
  </div>
</template>

<style scoped>
.registration-view{display:flex;flex-direction:column;flex:1;min-height:0;min-width:0}.registration-work{display:flex;flex-direction:column;flex:1;min-height:0;padding:0 48px 24px;gap:22px}.registration-heading{display:flex;align-items:center;justify-content:space-between;gap:20px;flex:0 0 auto}.registration-heading h2{font-size:32px;line-height:1.25;letter-spacing:-.025em;margin:0;font-weight:650}.registration-heading p{margin:9px 0 0;color:var(--muted);font-size:21px;line-height:1.4}.form-step{color:var(--edus-blue);background:var(--blue-soft);padding:12px 20px;border-radius:12px;font-size:22px;font-weight:600;white-space:nowrap}.registration-content{min-height:0;overflow-y:auto;display:flex;flex-direction:column;gap:20px;flex:1;padding:3px}.search-line{display:grid;grid-template-columns:minmax(0,1fr) auto;align-items:end;gap:16px;flex:0 0 auto}.search-button{min-width:164px}.search-results{min-height:0;overflow-y:auto;display:flex;flex-direction:column;gap:10px}.empty-search{display:flex;align-items:center;justify-content:center;flex-direction:column;gap:14px;min-height:200px;text-align:center;color:var(--muted);font-size:20px}.empty-search svg{color:var(--edus-blue)}.empty-search strong{font-size:24px;color:var(--edus-navy);font-weight:600}.result-caption{margin:0 0 4px;color:var(--muted);font-size:18px}.title-result{display:flex;align-items:center;gap:22px;width:100%;padding:20px 24px;border:1px solid var(--line);border-radius:16px;background:var(--surface);color:var(--edus-navy);text-align:left;cursor:pointer;min-height:104px}.title-result>svg:first-child{color:var(--edus-blue);flex-shrink:0}.title-result>span{display:flex;flex-direction:column;gap:5px;flex:1;min-width:0}.title-result strong{font-size:23px;font-weight:600}.title-result span span{font-size:19px}.title-result small{font-size:17px;color:var(--muted)}.title-result:hover{background:var(--blue-soft);border-color:var(--edus-blue)}.title-result:active{background:#dcecfb}.title-result:focus-visible,.mode-choice:focus-visible,.remove-code:focus-visible{outline:3px solid var(--edus-blue);outline-offset:3px}.edition-card{display:flex;gap:30px;padding:32px}.edition-icon{height:84px;width:84px;background:var(--blue-soft);color:var(--edus-blue);display:grid;place-items:center;border-radius:18px;flex-shrink:0}.edition-info{min-width:0;flex:1}.eyebrow{font-size:17px;font-weight:500;color:var(--muted)}.edition-info h3,.review-title h3{font-size:29px;font-weight:650;line-height:1.3;margin:8px 0}.author{font-size:23px;margin:0;color:var(--muted)}.edition-facts{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:22px 32px;margin:28px 0 0}.edition-facts dt{font-size:17px;color:var(--muted);margin-bottom:6px}.edition-facts dd{font-size:21px;margin:0;overflow-wrap:anywhere}.registration-fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px 32px}.required-note{font-size:17px;color:var(--muted);margin:0}.selected-edition{display:flex;align-items:center;gap:14px;color:var(--edus-navy);padding:18px 22px;background:var(--blue-soft);border-radius:12px;flex:0 0 auto}.selected-edition>svg{color:var(--edus-blue);flex:0 0 auto}.selected-edition>strong{font-size:22px;font-weight:600;flex:1}.selected-edition>span{font-size:21px;font-weight:600}.mode-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px}.mode-choice{position:relative;text-align:left;display:flex;flex-direction:column;gap:16px;min-height:248px;padding:28px;border:2px solid var(--line);border-radius:18px;background:var(--surface);color:var(--edus-navy);cursor:pointer}.mode-choice.selected{border-color:var(--edus-blue);background:var(--blue-soft)}.mode-choice strong{font-size:25px;font-weight:650}.mode-choice>span:not(.mode-check){font-size:20px;line-height:1.5;color:var(--muted)}.mode-icon{color:var(--edus-blue);margin-bottom:4px}.mode-check{position:absolute;right:22px;top:24px;display:grid;place-items:center;border:2px solid #a9bfd6;border-radius:50%;height:32px;width:32px}.selected .mode-check{background:var(--edus-blue);color:white;border-color:var(--edus-blue)}.code-list{display:flex;flex-direction:column;min-height:0;overflow-y:auto;gap:8px}.code-row{display:flex;align-items:center;min-height:72px;gap:16px;padding:5px 8px 5px 18px;background:var(--surface);border:1px solid var(--line);border-radius:12px}.code-index{width:30px;color:var(--muted);font-size:17px}.code-row>svg{color:var(--edus-blue);flex-shrink:0}.code-row strong{font-size:23px;font-weight:550;letter-spacing:.02em;overflow-wrap:anywhere}.remove-code{display:grid;place-items:center;min-width:56px;min-height:56px;background:transparent;color:var(--muted);border:none;border-radius:10px;margin-left:auto;cursor:pointer}.remove-code:hover{color:var(--danger);background:var(--danger-soft)}.code-notice{font-size:18px;color:var(--success);margin:0}.empty-codes{margin:12px 0;color:var(--muted);font-size:19px}.quantity-panel{display:flex;align-items:center;gap:28px;padding:36px}.quantity-panel>svg{color:var(--edus-blue);flex-shrink:0}.quantity-panel>div{flex:1}.quantity-panel h3{font-size:26px;margin:0 0 10px;font-weight:600}.quantity-panel p{font-size:20px;color:var(--muted);line-height:1.5;margin:0}.review-panel{padding:30px;display:flex;flex-direction:column;gap:24px}.review-title{display:flex;align-items:flex-start;gap:22px}.review-title>svg{color:var(--edus-blue);margin-top:16px;flex-shrink:0}.review-title p{margin:0;color:var(--muted);font-size:21px}.review-bottom{display:flex;justify-content:space-between;align-items:center;padding-top:22px;border-top:1px solid var(--line);gap:24px}.review-bottom>div{display:flex;flex-direction:column;gap:9px}.review-bottom span{color:var(--muted);font-size:18px}.review-bottom strong{font-size:23px;font-weight:600}.review-count{text-align:right}.review-count strong{font-size:34px}.review-count small{font-size:22px;font-weight:500}.review-codes{display:flex;gap:10px;flex-wrap:wrap;max-height:120px;overflow-y:auto}.review-codes span{padding:10px 14px;border:1px solid var(--line);border-radius:10px;font-size:19px}.review-note{font-size:19px;line-height:1.5;color:var(--muted);margin:0}.registration-error{flex-shrink:0}.confirmation-hint{font-size:17px;max-width:230px;color:var(--muted)}.continue-button{min-width:260px}.keyboard-open .registration-work{flex:0 0 auto;gap:12px;padding-bottom:14px}.keyboard-open .registration-heading h2{font-size:27px}.keyboard-open .registration-content{overflow:visible;flex:0 0 auto;gap:12px}.keyboard-open .registration-fields{grid-template-columns:minmax(0,1fr) auto;gap:24px}.field-switch{display:flex;align-items:center;gap:14px;align-self:end;height:72px}.field-switch .btn{min-width:64px;padding:0 18px}.field-switch span{font-size:18px;color:var(--muted)}.registration-keyboard{min-height:0;flex:1;display:flex;flex-direction:column}.registration-keyboard :deep(.touch-keyboard){position:static;flex:1;width:100%;box-shadow:none}.registration-actions{margin-top:auto}.leave-backdrop{position:fixed;inset:0;background:#0b1a4d59;display:grid;place-items:center;padding:24px;z-index:100}.leave-dialog{position:relative;background:var(--surface);width:min(720px,100%);padding:40px;border-radius:24px;box-shadow:0 16px 60px #0b1a4d24}.leave-dialog h2{font-size:30px;line-height:1.3;margin:0 45px 18px 0}.leave-dialog p{font-size:22px;line-height:1.5;color:var(--muted);margin:0 0 30px}.leave-dialog>div{display:flex;gap:16px;justify-content:flex-end;flex-wrap:wrap}.dialog-close{position:absolute;right:20px;top:16px;background:transparent;border:none;min-height:56px;min-width:56px;display:grid;place-items:center;color:var(--muted);cursor:pointer;border-radius:12px}.spin{animation:spin 1s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
@media(max-width:1400px){.registration-work{padding:0 32px 20px;gap:18px}.registration-heading h2{font-size:29px}.registration-heading p{font-size:20px}.edition-card{padding:24px}.edition-facts{gap:18px 26px;margin-top:22px}.mode-choice{min-height:220px;padding:24px;gap:12px}.mode-choice strong{font-size:23px}.mode-choice>span:not(.mode-check){font-size:19px}.registration-fields{gap:20px 28px}.quantity-panel{padding:28px}.review-panel{padding:24px}.confirmation-hint{display:none}.keyboard-open .registration-work{padding-bottom:10px;gap:6px}.keyboard-open .registration-heading h2{font-size:24px}.keyboard-open .registration-actions{padding-top:12px;padding-bottom:12px}.keyboard-open .registration-actions :deep(.btn){min-height:62px}}
@media(max-width:800px){.registration-work{padding:0 18px 20px}.registration-heading h2{font-size:26px}.registration-heading p{font-size:19px}.search-line{grid-template-columns:minmax(0,1fr)}.search-button{min-width:0;justify-self:stretch}.mode-grid,.registration-fields{grid-template-columns:1fr}.mode-choice{min-height:208px}.edition-card{gap:16px}.edition-icon{display:none}.edition-facts{grid-template-columns:1fr}.quantity-panel{flex-wrap:wrap}.quantity-panel>div{flex-basis:calc(100% - 90px)}.quantity-panel :deep(.quantity-control){flex:0 0 auto;margin-left:74px}.review-title h3,.edition-info h3{font-size:25px}.review-bottom strong{font-size:21px}.review-count strong{font-size:30px}.continue-button{min-width:0}.registration-actions :deep(.action-right){min-width:0}.registration-actions .btn{font-size:18px;padding-inline:18px}.keyboard-open .search-line{grid-template-columns:minmax(0,1fr) auto}.keyboard-open .search-button{font-size:0;min-width:64px;padding:0 16px}.keyboard-open .registration-fields{grid-template-columns:minmax(0,1fr)}.field-switch{display:none}.leave-dialog{padding:28px}.leave-dialog h2{font-size:27px}.leave-dialog p{font-size:20px}.leave-dialog>div>.btn{flex:1;min-width:220px}}
@media(prefers-reduced-motion:reduce){.spin{animation:none}}
.registration-work{padding-left:0;padding-right:0}.quantity-panel :deep(.quantity-control){flex:0 0 auto}.registration-actions{padding-left:0;padding-right:0;background:transparent}
@media(max-height:820px) and (min-width:900px){.keyboard-open .registration-heading{display:none}.keyboard-open .registration-work{padding-bottom:0;gap:0}.keyboard-open .registration-content{padding:0}.keyboard-open .registration-actions{padding-top:10px;padding-bottom:10px}.keyboard-open .registration-error{padding:10px 16px}}
</style>

\n