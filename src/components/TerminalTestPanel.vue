<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { CreditCard, ScanLine, RotateCcw, AlertTriangle, Timer, CheckCircle2 } from '@lucide/vue'
import type { Copy, Reader, Title } from '../domain/terminalTypes.ts'

const props = defineProps<{
  screen: string
  readers: readonly Reader[]
  titles: readonly Title[]
  copies: readonly Copy[]
  captureKind: '' | 'card' | 'book'
  capturedValue: string
  capturedSuffix: string
  persistence: 'memory' | 'local-storage' | 'none'
}>()
const emit = defineEmits<{
  capture: [kind: 'card' | 'book']
  cancelCapture: []
  bindCard: [readerId: string, rawCode: string]
  bindCode: [kind: 'copy' | 'title', id: string, rawCode: string]
  card: [code: string]
  scan: [code: string]
  outcome: [outcome: 'unknown' | 'error']
  delay: [delayMs: number]
  reset: []
}>()

const readerId = ref('')
const cardCode = ref('')
const bindingKind = ref<'copy' | 'title'>('copy')
const bindingId = ref('')
const bookCode = ref('')
const resetConfirm = ref(false)
watch(() => props.readers, (readers) => { if (!readers.some((reader) => reader.id === readerId.value)) readerId.value = readers[0]?.id ?? '' }, { immediate: true })
const choices = computed(() => bindingKind.value === 'copy' ? props.copies.map((copy) => ({ id: copy.id, label: `Экземпляр · ${copy.code}` })) : props.titles.map((title) => ({ id: title.id, label: `Издание · ${title.name}${title.isbn ? ` · ISBN ${title.isbn}` : ''}` })))
watch(choices, (items) => { if (!items.some((item) => item.id === bindingId.value)) bindingId.value = items[0]?.id ?? '' }, { immediate: true })
const cardValue = computed(() => props.capturedValue || cardCode.value.trim())
const bookValue = computed(() => props.capturedValue || bookCode.value.trim())
function bindCard() { if (readerId.value && cardValue.value) { emit('bindCard', readerId.value, cardValue.value); cardCode.value = '' } }
function bindBook() { if (bindingId.value && bookValue.value) { emit('bindCode', bindingKind.value, bindingId.value, bookValue.value); bookCode.value = '' } }
function reset() { if (resetConfirm.value) { emit('reset'); resetConfirm.value = false } else resetConfirm.value = true }
</script>

<template>
  <section class="terminal-test-panel" aria-labelledby="terminal-test-heading">
    <div class="terminal-test-heading"><div><h3 id="terminal-test-heading">Панель тестирования терминала</h3><p>Только локальные тестовые данные. Запросы к школьным системам не выполняются.</p></div><span class="terminal-test-storage" :class="{ warning: persistence !== 'local-storage' }">{{ persistence === 'local-storage' ? 'Данные сохраняются локально' : 'Хранилище недоступно' }}</span></div>

    <div class="terminal-test-grid">
      <section class="terminal-test-section"><h4><CreditCard />Карта читателя</h4><label>Читатель<select v-model="readerId"><option v-for="reader in readers" :key="reader.id" :value="reader.id">{{ reader.name }} · {{ reader.group }}</option></select></label><div class="terminal-test-actions"><button class="btn secondary" :class="{ active: captureKind === 'card' }" @click="captureKind === 'card' ? emit('cancelCapture') : emit('capture', 'card')">{{ captureKind === 'card' ? 'Отменить ожидание' : 'Ожидать карту' }}</button><button class="btn" :disabled="screen!=='identify'" @click="emit('card', readers.find((reader) => reader.id === readerId)?.card ?? '')">Симулировать карту</button></div><label>Код карты<input v-model="cardCode" inputmode="text" autocomplete="off" placeholder="Сканируйте только после «Ожидать карту»" /></label><p v-if="captureKind === 'card'" class="capture-state"><ScanLine />Ожидание ввода карты. Завершите скан Enter.</p><p v-else-if="capturedValue && capturedSuffix" class="capture-state"><CheckCircle2 />Получено: <strong>{{ capturedValue }}</strong> · {{ capturedSuffix }}</p><button class="btn primary full" :disabled="!readerId || !cardValue" @click="bindCard">Привязать карту локально</button></section>

      <section class="terminal-test-section"><h4><ScanLine />Сканер книг</h4><label>Тип<select v-model="bindingKind"><option value="copy">Конкретный экземпляр</option><option value="title">Издание по ISBN</option></select></label><label>Привязка<select v-model="bindingId"><option v-for="choice in choices" :key="choice.id" :value="choice.id">{{ choice.label }}</option></select></label><div class="terminal-test-actions"><button class="btn secondary" :class="{ active: captureKind === 'book' }" @click="captureKind === 'book' ? emit('cancelCapture') : emit('capture', 'book')">{{ captureKind === 'book' ? 'Отменить ожидание' : 'Ожидать сканер' }}</button><button class="btn" :disabled="!['scan','registration'].includes(screen)" @click="emit('scan', copies[0]?.code ?? '')">Симулировать скан</button></div><label>Код сканера<input v-model="bookCode" inputmode="text" autocomplete="off" placeholder="Штрихкод или ISBN" /></label><p v-if="captureKind === 'book'" class="capture-state"><ScanLine />Ожидание ввода сканера. Завершите скан Enter.</p><p v-else-if="capturedValue && capturedSuffix" class="capture-state"><CheckCircle2 />Получено: <strong>{{ capturedValue }}</strong> · {{ capturedSuffix }}</p><p class="small muted">ISBN привязывается к изданию, а не к случайному экземпляру.</p><button class="btn primary full" :disabled="!bindingId || !bookValue" @click="bindBook">Привязать код локально</button></section>
    </div>

    <div class="terminal-test-shortcuts"><button class="btn" :disabled="screen!=='scan'" @click="emit('scan', 'BOOK-NOT-FOUND')"><AlertTriangle />Неизвестный код</button><button class="btn" @click="emit('outcome', 'error')"><AlertTriangle />Ошибка следующей операции</button><button class="btn" @click="emit('outcome', 'unknown')"><Timer />Неизвестный результат</button><button class="btn" @click="emit('delay', 2500)"><Timer />Задержка 2,5 с</button><button class="btn secondary danger" @click="reset"><RotateCcw />{{ resetConfirm ? 'Подтвердить сброс' : 'Сбросить тестовые данные' }}</button></div>
    <p v-if="resetConfirm" class="terminal-test-confirm">Будут сброшены только данные режима «Тестовый режим» в этом браузерном профиле. Production-данные и настройки не затрагиваются.</p>
  </section>
</template>

<style scoped>
.terminal-test-panel{margin-top:20px;padding-top:20px;border-top:1px solid var(--line)}.terminal-test-heading{display:flex;justify-content:space-between;align-items:flex-start;gap:16px}.terminal-test-heading h3{margin:0}.terminal-test-heading p{margin:6px 0 0}.terminal-test-storage{flex:0 0 auto;background:#e7f7ef;color:#126b43;padding:6px 10px;border-radius:999px;font-size:14px;font-weight:650}.terminal-test-storage.warning{background:#fff1dc;color:#8a5700}.terminal-test-grid{display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-top:16px}.terminal-test-section{padding:16px;border:1px solid var(--line);border-radius:14px;background:var(--surface)}.terminal-test-section h4{display:flex;align-items:center;gap:8px;margin:0 0 12px;color:var(--edus-navy)}.terminal-test-section label{display:grid;gap:6px;margin-top:10px;font-size:15px;font-weight:600;color:var(--edus-navy)}.terminal-test-section select,.terminal-test-section input{width:100%;height:44px;border:1px solid var(--line);border-radius:9px;background:#fff;padding:0 10px;color:var(--edus-navy);font:inherit}.terminal-test-actions{display:flex;gap:8px;margin-top:12px}.terminal-test-actions .btn{min-height:48px;flex:1;font-size:15px}.terminal-test-actions .active{border-color:var(--edus-blue);background:var(--blue-soft)}.capture-state{display:flex;align-items:center;gap:7px;min-height:25px;margin:10px 0;color:var(--edus-blue);font-size:14px}.terminal-test-shortcuts{display:flex;flex-wrap:wrap;gap:10px;margin-top:16px}.terminal-test-shortcuts .btn{min-height:48px;font-size:15px}.danger{color:#9b3030}.terminal-test-confirm{margin:10px 0 0;color:#8a5700;font-size:14px}.full{width:100%;margin-top:8px}@media(max-width:850px){.terminal-test-grid{grid-template-columns:1fr}.terminal-test-heading{flex-direction:column}.terminal-test-storage{align-self:flex-start}}
</style>
