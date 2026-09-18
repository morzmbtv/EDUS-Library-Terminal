<script setup lang="ts">
import { BookOpen, Check, Trash2 } from '@lucide/vue'
const props = withDefaults(defineProps<{ title: string; author?: string; code?: string; mode?: string; quantity?: number; removable?: boolean; selected?: boolean }>(), { author: '', code: '', mode: '', quantity: 1, removable: true, selected: undefined })
const emit = defineEmits<{ remove: []; select: [] }>()
function select() { if (props.selected !== undefined) emit('select') }
const modeLabel = (mode: string) => mode === 'COPY' ? 'Индивидуальный экземпляр' : mode === 'LEGACY_TITLE' ? 'Учёт количеством' : mode
</script>

<template>
  <div class="book-row" :class="{ selectable: selected !== undefined, selected }">
    <button v-if="selected !== undefined" type="button" class="book-selection" :aria-label="`${selected ? 'Убрать из выбора' : 'Выбрать'}: ${title}`" :aria-pressed="selected" @click="select"><span class="check-box"><Check v-if="selected" :size="21" :stroke-width="2" /></span></button>
    <span v-else class="book-symbol" aria-hidden="true"><BookOpen :size="26" :stroke-width="1.8" /></span>
    <div class="book-copy" :class="{ 'can-select': selected !== undefined }" @click="select"><strong>{{ title }}</strong><span v-if="author" class="book-author">{{ author }}</span><div class="book-meta"><span v-if="code">№ {{ code }}</span><span v-if="mode">{{ modeLabel(mode) }}</span><span v-if="quantity > 1" class="quantity">{{ quantity }} шт.</span></div></div>
    <button v-if="removable" type="button" class="remove-book" :aria-label="`Убрать из списка: ${title}`" title="Убрать из списка" @click="emit('remove')"><Trash2 :size="22" :stroke-width="1.8" /></button>
  </div>
</template>

<style scoped>
.book-row { min-height: 104px; padding: 20px 24px; display: flex; align-items: center; gap: 16px; border-bottom: 1px solid var(--line, #DCE6F0); background: var(--surface, #fff); }.book-row:last-child { border-bottom: 0; }.book-row.selected { background: #f1f7fe; }
.book-symbol { display: grid; place-items: center; color: var(--edus-blue, #046BC8); flex: 0 0 44px; height: 52px; }.book-copy { min-width: 0; flex: 1; }.book-copy strong { display: block; font-size: 22px; font-weight: 600; line-height: 1.3; overflow-wrap: anywhere; }.book-author { display: block; margin-top: 5px; color: var(--muted, #5E6B82); font-size: 18px; line-height: 1.3; }.book-meta { display: flex; align-items: center; gap: 10px 20px; margin-top: 7px; flex-wrap: wrap; color: var(--muted, #5E6B82); font-size: 16px; line-height: 1.35; }.quantity { color: var(--edus-navy, #0B1A4D); font-weight: 600; }
.remove-book, .book-selection { flex: 0 0 56px; height: 56px; display: grid; place-items: center; border: 0; border-radius: 12px; background: transparent; color: var(--muted, #5E6B82); cursor: pointer; }.remove-book:hover { background: #fff0f1; color: #ae2b34; }.remove-book:active { background: #f6dadd; }.remove-book:focus-visible, .book-selection:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 2px; }.book-selection:hover { background: #e1effc; }.check-box { width: 27px; height: 27px; display: grid; place-items: center; border: 1px solid #9cacc0; border-radius: 7px; color: #fff; }.selected .check-box { background: var(--edus-blue, #046BC8); border-color: var(--edus-blue, #046BC8); }.can-select { cursor: pointer; }
@media (max-width: 700px) { .book-row { padding: 16px; gap: 10px; }.book-symbol { display: none; }.book-copy strong { font-size: 21px; }.book-meta { gap: 6px 12px; } }
</style>
