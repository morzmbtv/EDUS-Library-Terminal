<script setup lang="ts">
import { Barcode, Keyboard, Search } from '@lucide/vue'
withDefaults(defineProps<{ title?: string; description?: string; compact?: boolean; issue?: boolean; accept?: boolean }>(), { title: 'Сканируйте книги', description: 'Поднесите штрихкод книги к сканеру', compact: false, issue: false, accept: false })
defineEmits<{ manual: []; isbn: []; onHandSearch: [] }>()
</script>

<template>
  <section class="scanner-panel" :class="{ compact }">
    <div class="scanner-symbol" aria-hidden="true"><span class="corner tl" /><span class="corner tr" /><Barcode :size="76" :stroke-width="1.6" /><span class="corner bl" /><span class="corner br" /></div>
    <div class="scanner-copy"><h2>{{ title }}</h2><p>{{ description }}</p></div>
    <div class="scanner-actions">
      <button type="button" class="manual-button" @click="$emit('manual')"><Keyboard :size="24" :stroke-width="1.8" /><span>{{ issue || accept ? 'Ввести номер вручную' : 'Ввести код вручную' }}</span></button>
      <button v-if="issue" type="button" class="manual-button isbn-button" @click="$emit('isbn')"><Barcode :size="25" :stroke-width="1.8" /><span>Добавить по ISBN</span></button>
      <button v-if="accept" type="button" class="manual-button isbn-button" @click="$emit('onHandSearch')"><Search :size="25" :stroke-width="1.8" /><span>Найти книгу на руках</span></button>
    </div>
  </section>
</template>

<style scoped>
.scanner-panel { display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 40px 24px; min-height: 330px; }
.scanner-symbol { position: relative; width: 126px; height: 104px; display: grid; place-items: center; color: var(--edus-navy, #0B1A4D); margin-bottom: 26px; }
.corner { position: absolute; width: 20px; height: 20px; border-color: var(--edus-blue, #046BC8); border-style: solid; }
.tl { left: 0; top: 0; border-width: 3px 0 0 3px; border-top-left-radius: 5px; }.tr { right: 0; top: 0; border-width: 3px 3px 0 0; border-top-right-radius: 5px; }.bl { left: 0; bottom: 0; border-width: 0 0 3px 3px; border-bottom-left-radius: 5px; }.br { right: 0; bottom: 0; border-width: 0 3px 3px 0; border-bottom-right-radius: 5px; }
h2 { font-size: 28px; line-height: 1.3; margin: 0 0 12px; font-weight: 650; letter-spacing: -.015em; }
p { font-size: 20px; line-height: 1.5; margin: 0; max-width: 430px; color: var(--muted, #5E6B82); }
.scanner-actions { display: flex; align-items: center; justify-content: center; flex-wrap: wrap; gap: 24px; margin-top: 28px; }
.manual-button { min-height: 56px; min-width: 260px; background: transparent; color: var(--edus-blue, #046BC8); border: 1px solid var(--line, #DCE6F0); border-radius: 12px; padding: 12px 20px; display: inline-flex; gap: 12px; align-items: center; justify-content: center; font: inherit; font-size: 18px; cursor: pointer; }
.manual-button:hover { background: var(--blue-soft, #EAF3FD); }.manual-button:active { background: #d7e9fb; }.manual-button:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 3px; }
.compact { padding: 24px 20px; min-height: 250px; }.compact .scanner-symbol { width: 108px; height: 88px; margin-bottom: 20px; }.compact h2 { font-size: 24px; }.compact p { font-size: 18px; }.compact .scanner-actions { margin-top: 24px; }
@media (max-height: 800px) { .scanner-panel { padding-block: 24px; min-height: 280px; } .scanner-symbol { margin-bottom: 20px; } .scanner-actions { margin-top: 20px; } }
</style>
