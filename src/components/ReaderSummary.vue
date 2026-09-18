<script setup lang="ts">
import { ArrowRight, BookOpen, LibraryBig, UserRound } from '@lucide/vue'
withDefaults(defineProps<{ name: string; group: string; count: number; card?: string; changeable?: boolean; variant?: 'default'|'scan' }>(), { card: '', changeable: true, variant: 'default' })
defineEmits<{ change: []; loans: [] }>()
const bookWord = (n: number) => n % 10 === 1 && n % 100 !== 11 ? 'книга' : [2, 3, 4].includes(n % 10) && ![12, 13, 14].includes(n % 100) ? 'книги' : 'книг'
</script>

<template>
  <section class="reader-summary" :class="{ 'scan-reader-summary': variant === 'scan' }" aria-label="Выбранный читатель">
    <span class="reader-symbol" aria-hidden="true"><UserRound :size="27" :stroke-width="1.8" /></span>
    <div class="reader-identity"><strong>{{ name }}</strong><span>{{ group }}</span><span v-if="variant === 'scan' && card" class="reader-card">Карта № {{ card }}</span></div>
    <template v-if="variant === 'scan'">
      <div class="reader-loans"><span class="loan-symbol" aria-hidden="true"><LibraryBig :size="42" :stroke-width="1.65" /></span><div><span>Книг на руках</span><strong>{{ count }}</strong><button type="button" @click="$emit('loans')">Показать список <ArrowRight :size="21" /></button></div></div>
    </template>
    <template v-else><div class="on-loan"><BookOpen :size="22" :stroke-width="1.8" /><span>{{ count }} {{ bookWord(count) }} на руках</span></div><button v-if="changeable" type="button" @click="$emit('change')">Изменить</button></template>
  </section>
</template>

<style scoped>
.reader-summary { display: flex; align-items: center; gap: 16px; min-height: 104px; padding: 20px 24px; border: 1px solid var(--line, #DCE6F0); border-radius: 18px; background: var(--surface, #fff); }
.reader-symbol { flex: 0 0 52px; height: 52px; color: var(--edus-blue, #046BC8); background: var(--blue-soft, #EAF3FD); border-radius: 50%; display: grid; place-items: center; }
.reader-identity { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
.reader-identity strong { font-size: 23px; line-height: 1.3; font-weight: 650; overflow-wrap: anywhere; }
.reader-identity > span { font-size: 18px; color: var(--muted, #5E6B82); }
.on-loan { display: flex; align-items: center; gap: 10px; margin-left: auto; font-size: 18px; white-space: nowrap; color: var(--muted, #5E6B82); }
button { flex: 0 0 auto; min-height: 56px; padding: 12px 18px; color: var(--edus-blue, #046BC8); background: transparent; border: 1px solid var(--line, #DCE6F0); border-radius: 12px; font: inherit; font-size: 18px; cursor: pointer; margin-left: 8px; }
button:hover { background: var(--blue-soft, #EAF3FD); } button:active { background: #d7e9fb; } button:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 3px; }
.scan-reader-summary { min-height: 180px; padding: 28px 72px; gap: 36px; }.scan-reader-summary .reader-symbol { flex-basis: 120px; height: 120px; }.scan-reader-summary .reader-symbol svg { width: 56px; height: 56px; }.scan-reader-summary .reader-identity strong { font-size: 36px; line-height: 1.22; }.scan-reader-summary .reader-identity > span { font-size: 24px; }.scan-reader-summary .reader-card { margin-top: 2px; }.reader-loans { display: flex; align-items: center; gap: 28px; min-width: 390px; min-height: 120px; margin-left: auto; padding-left: 80px; border-left: 1px solid var(--line, #DCE6F0); }.loan-symbol { display: grid; place-items: center; flex: 0 0 94px; height: 94px; border-radius: 50%; color: var(--edus-blue, #046BC8); background: var(--blue-soft, #EAF3FD); }.reader-loans > div { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; }.reader-loans > div > span { font-size: 22px; color: var(--muted, #5E6B82); }.reader-loans strong { font-size: 42px; line-height: 1.1; color: var(--edus-blue, #046BC8); }.reader-loans button { display: inline-flex; align-items: center; min-height: 42px; padding: 0; margin: 0; border: 0; color: var(--edus-blue, #046BC8); font-size: 19px; }.reader-loans button svg { margin-left: 8px; }
.scan-reader-summary .reader-identity { flex: 1 1 0; }
@media (max-width: 900px) { .reader-summary { flex-wrap: wrap; } .on-loan { margin-left: 68px; } button { margin-left: auto; } }
@media (max-width: 1100px) { .scan-reader-summary { padding: 24px 36px; }.reader-loans { min-width: 330px; padding-left: 36px; } }
@media (max-width: 600px) { .reader-summary { padding: 16px; gap: 12px; } .reader-symbol { display: none; } .reader-identity { flex: 1 1 100%; } .on-loan { margin-left: 0; white-space: normal; } }
</style>
