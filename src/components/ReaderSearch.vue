<script setup lang="ts">
import { ArrowRight, LoaderCircle, Search, UserRound } from '@lucide/vue'
import TouchInput from './TouchInput.vue'
type ReaderResult = { id: string; name: string; group: string; card: string }
withDefaults(defineProps<{ modelValue: string; loading?: boolean; results: ReaderResult[]; searched?: boolean }>(), { loading: false, searched: false })
defineEmits<{ 'update:modelValue': [value: string]; select: [reader: ReaderResult]; keyboard: [] }>()
</script>

<template>
  <section class="reader-search">
    <TouchInput :model-value="modelValue" label="Найдите читателя" placeholder="Фамилия, имя или номер карты" @update:model-value="$emit('update:modelValue', $event)" @keyboard="$emit('keyboard')" />
    <div class="search-results" aria-live="polite" :aria-busy="loading">
      <div v-if="loading" class="search-state"><LoaderCircle class="spinner" :size="32" :stroke-width="1.8" /><span>Ищем читателя…</span></div>
      <template v-else-if="results.length"><p class="results-label">Выберите читателя · найдено {{ results.length }}</p><button v-for="reader in results" :key="reader.id" class="reader-result" type="button" @click="$emit('select', reader)"><span class="result-symbol" aria-hidden="true"><UserRound :size="25" :stroke-width="1.8" /></span><span class="result-copy"><strong>{{ reader.name }}</strong><span>{{ reader.group }}<span class="result-divider">·</span>Карта {{ reader.card }}</span></span><ArrowRight :size="23" :stroke-width="1.8" /></button></template>
      <div v-else-if="searched" class="search-state"><Search :size="34" :stroke-width="1.8" /><strong>Читатель не найден</strong><span>Проверьте написание или введите номер карты.</span></div>
      <div v-else class="search-state initial"><Search :size="32" :stroke-width="1.8" /><span>Введите фамилию или номер карты</span></div>
    </div>
  </section>
</template>

<style scoped>
.reader-search { display: flex; flex-direction: column; gap: 20px; min-height: 0; }.search-results { min-height: 0; overflow-y: auto; scrollbar-width: thin; }.results-label { margin: 0 0 12px; color: var(--muted, #5E6B82); font-size: 17px; }
.reader-result { display: flex; align-items: center; text-align: left; gap: 16px; padding: 20px; min-height: 100px; width: 100%; border: 1px solid var(--line, #DCE6F0); border-radius: 12px; margin-bottom: 10px; background: var(--surface, #fff); color: var(--edus-navy, #0B1A4D); cursor: pointer; font: inherit; }.reader-result > svg { color: var(--edus-blue, #046BC8); flex: 0 0 auto; margin-left: auto; }.reader-result:hover { background: #f2f8fe; border-color: var(--edus-blue, #046BC8); }.reader-result:active { background: #e1effc; }.reader-result:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: -3px; }.result-symbol { flex: 0 0 48px; height: 48px; border-radius: 50%; background: var(--blue-soft, #EAF3FD); color: var(--edus-blue, #046BC8); display: grid; place-items: center; }.result-copy { display: flex; flex-direction: column; min-width: 0; gap: 8px; }.result-copy strong { font-size: 22px; font-weight: 600; line-height: 1.3; overflow-wrap: anywhere; }.result-copy > span { font-size: 18px; color: var(--muted, #5E6B82); line-height: 1.35; }.result-divider { display: inline-block; margin: 0 12px; }
.search-state { min-height: 190px; padding: 28px 20px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; text-align: center; color: var(--muted, #5E6B82); font-size: 19px; line-height: 1.45; }.search-state > svg { color: var(--edus-blue, #046BC8); }.search-state strong { color: var(--edus-navy, #0B1A4D); font-size: 23px; font-weight: 600; }.spinner { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 600px) { .reader-result { padding: 16px; gap: 12px; }.result-symbol { display: none; }.result-copy > span { font-size: 17px; } }
@media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
