<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { ArrowLeft, BookOpen, Search } from '@lucide/vue'
import type { Snapshot, Title } from '../domain/terminalTypes.ts'
import TouchInput from '../components/TouchInput.vue'

const props = defineProps<{ modelValue: string; snapshot: Snapshot }>()
const emit = defineEmits<{ 'update:modelValue': [value: string]; back: []; keyboard: [] }>()
const searched = ref(false)

type SearchResult = { title: Title; available: number; total: number; dueSoon: number }

const normalizedQuery = computed(() => props.modelValue.trim().toLocaleLowerCase('ru-RU'))
const results = computed<SearchResult[]>(() => {
  if (!searched.value || !normalizedQuery.value) return []
  const query = normalizedQuery.value
  return props.snapshot.titles
    .filter((title) => [title.name, title.author, title.isbn].some((value) => value.toLocaleLowerCase('ru-RU').includes(query)))
    .map((title) => {
      const copies = props.snapshot.copies.filter((copy) => copy.titleId === title.id)
      const copyLoans = props.snapshot.loans.filter((loan) => loan.titleId === title.id && loan.mode === 'COPY')
      const legacyLoans = props.snapshot.loans.filter((loan) => loan.titleId === title.id && loan.mode === 'LEGACY_TITLE')
      const legacyTotal = props.snapshot.legacyStock[title.id] ?? 0
      const total = copies.length + legacyTotal
      const available = Math.max(0, copies.length - copyLoans.length) + Math.max(0, legacyTotal - legacyLoans.reduce((sum, loan) => sum + loan.quantity, 0))
      return { title, available, total, dueSoon: copyLoans.length + legacyLoans.reduce((sum, loan) => sum + loan.quantity, 0) }
    })
    .sort((a, b) => a.title.name.localeCompare(b.title.name, 'ru-RU'))
})

watch(() => props.modelValue, () => { searched.value = false })
function submit() { searched.value = !!normalizedQuery.value }
</script>

<template>
  <section class="library-search-view" aria-labelledby="library-search-heading">
    <div class="library-search-topline">
      <button class="btn ghost library-search-back" type="button" @click="emit('back')"><ArrowLeft />Назад</button>
      <button class="btn ghost library-search-home" type="button" @click="emit('back')">На главную</button>
    </div>
    <div class="library-search-heading">
      <div><h1 id="library-search-heading">Найти книгу</h1><p>Проверьте наличие в фонде и информацию об издании.</p></div>
    </div>
    <form class="library-search-form" @submit.prevent="submit">
      <TouchInput :model-value="modelValue" label="Название, автор или ISBN" placeholder="Например, Алгебра или 9786010123456" @update:model-value="emit('update:modelValue', $event)" @keyboard="emit('keyboard')" @enter="submit" />
      <button class="btn primary library-search-submit" type="submit" :disabled="!normalizedQuery"><Search />Найти</button>
    </form>
    <div class="library-search-results" aria-live="polite">
      <div v-if="!searched" class="library-search-empty"><Search :stroke-width="1.6" /><h2>Найдите нужное издание</h2><p>Введите название, автора или ISBN. Здесь появятся наличие и книги, которые сейчас на руках.</p></div>
      <div v-else-if="!results.length" class="library-search-empty"><BookOpen :stroke-width="1.6" /><h2>Ничего не найдено</h2><p>Проверьте написание названия, автора или ISBN.</p></div>
      <div v-else class="library-result-list">
        <article v-for="result in results" :key="result.title.id" class="library-result">
          <span class="library-result-icon"><BookOpen :stroke-width="1.8" /></span>
          <div class="library-result-copy"><h2>{{ result.title.name }}</h2><p>{{ result.title.author }}<span v-if="result.title.year"> · {{ result.title.year }}</span></p><small v-if="result.title.isbn">ISBN {{ result.title.isbn }}</small></div>
          <div class="library-result-availability" :class="{ unavailable: result.available === 0 }"><span>Доступно</span><strong>{{ result.available }} из {{ result.total }}</strong><small v-if="result.dueSoon">На руках: {{ result.dueSoon }}</small><small v-else>Все книги в библиотеке</small></div>
        </article>
      </div>
    </div>
  </section>
</template>

<style scoped>
.library-search-view { width:min(1180px,100%); height:100%; margin:0 auto; display:flex; flex-direction:column; gap:24px; }
.library-search-topline { display:flex; justify-content:space-between; align-items:center; min-height:56px; }.library-search-back { min-width:112px; justify-content:flex-start; padding-left:0; }.library-search-home { color:var(--edus-blue); }
.library-search-heading h1 { font-size:40px; }.library-search-heading p { margin-top:8px; color:var(--muted); font-size:22px; }
.library-search-form { display:grid; grid-template-columns:minmax(0,1fr) 208px; align-items:end; gap:20px; padding:24px; border:1px solid var(--line); border-radius:18px; background:var(--surface); }.library-search-submit { min-height:72px; font-size:21px; }
.library-search-results { min-height:0; flex:1; overflow:auto; }.library-search-empty { min-height:280px; padding:32px; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:14px; border:1px dashed #B9D8F8; border-radius:18px; color:var(--muted); text-align:center; }.library-search-empty svg { width:52px; height:52px; color:var(--edus-blue); }.library-search-empty h2 { color:var(--edus-navy); }.library-search-empty p { max-width:540px; font-size:19px; line-height:1.45; }
.library-result-list { display:flex; flex-direction:column; gap:12px; }.library-result { min-height:112px; display:grid; grid-template-columns:64px minmax(0,1fr) auto; align-items:center; gap:18px; padding:18px 22px; border:1px solid var(--line); border-radius:16px; background:var(--surface); }.library-result-icon { width:56px; height:56px; display:grid; place-items:center; border-radius:50%; color:var(--edus-blue); background:var(--blue-soft); }.library-result-icon svg { width:32px; height:32px; }.library-result-copy h2 { font-size:23px; }.library-result-copy p { margin-top:4px; color:var(--muted); font-size:18px; }.library-result-copy small { display:block; margin-top:4px; color:var(--muted); font-size:16px; }.library-result-availability { min-width:180px; padding-left:22px; border-left:1px solid var(--line); display:flex; flex-direction:column; align-items:flex-start; gap:2px; color:var(--success); }.library-result-availability>span { color:var(--muted); font-size:16px; }.library-result-availability strong { font-size:24px; line-height:1.2; }.library-result-availability small { color:var(--muted); font-size:15px; }.library-result-availability.unavailable { color:var(--danger); }
@media (max-height:820px) and (min-width:900px) { .library-search-view { gap:16px; }.library-search-heading h1 { font-size:36px; }.library-search-heading p { font-size:20px; }.library-search-form { padding:18px; }.library-result { min-height:92px; padding:14px 18px; }.library-search-empty { min-height:220px; } }
@media (max-width:760px) { .library-search-form { grid-template-columns:1fr; }.library-result { grid-template-columns:52px minmax(0,1fr); gap:12px; }.library-result-availability { grid-column:1/-1; min-width:0; padding:12px 0 0; border-left:0; border-top:1px solid var(--line); }.library-search-heading h1 { font-size:32px; }.library-search-heading p { font-size:20px; }.library-search-home { display:none; } }
</style>
