<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { BookOpen, CheckCircle2, ChevronRight, CircleAlert, LoaderCircle, RefreshCw } from '@lucide/vue'

export type ReturnLoanListItem = {
  id: string
  title: string
  author?: string
  code?: string
  mode: 'COPY' | 'LEGACY_TITLE'
  quantity: number
  selectedQuantity: number
}

const props = withDefaults(defineProps<{
  items: ReturnLoanListItem[]
  state?: 'loading' | 'ready' | 'error'
}>(), { state: 'ready' })
const emit = defineEmits<{ details: [item: ReturnLoanListItem]; retry: [] }>()
const list = ref<HTMLElement | null>(null)
const selectedTotal = computed(() => props.items.reduce((sum, item) => sum + item.selectedQuantity, 0))
const onHandTotal = computed(() => props.items.reduce((sum, item) => sum + item.quantity, 0))
function plural(value: number, one: string, few: string, many: string) {
  const mod100 = value % 100
  const mod10 = value % 10
  if (mod100 > 10 && mod100 < 20) return many
  if (mod10 === 1) return one
  if (mod10 > 1 && mod10 < 5) return few
  return many
}

watch(() => props.items.map(item => `${item.id}:${item.selectedQuantity}`).join('|'), async (_after, before) => {
  const changed = props.items.find(item => item.selectedQuantity > 0 && !before.includes(`${item.id}:${item.selectedQuantity}`))
  if (!changed) return
  await nextTick()
  list.value?.querySelector<HTMLElement>(`[data-loan-id="${changed.id}"]`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
})
</script>

<template>
  <section class="return-loan-list" aria-label="Книги у читателя">
    <header class="return-loan-heading">
      <div><h2>Книги у читателя</h2><p v-if="state==='ready' && items.length">{{ items.length }} {{ plural(items.length, 'название', 'названия', 'названий') }} · {{ onHandTotal }} {{ plural(onHandTotal, 'книга', 'книги', 'книг') }}</p></div>
      <span v-if="state==='ready'" class="return-loan-count" aria-label="Книг к приёму">{{ selectedTotal }}</span>
    </header>
    <div ref="list" class="return-loan-scroll" tabindex="0" aria-label="Активные выдачи читателя">
      <div v-if="state==='loading'" class="return-list-state"><LoaderCircle class="spin" /><p>Загружаем книги читателя…</p></div>
      <div v-else-if="state==='error'" class="return-list-state return-list-error"><CircleAlert /><p>Не удалось загрузить книги читателя.</p><button class="btn secondary" type="button" @click="emit('retry')"><RefreshCw />Повторить</button></div>
      <div v-else-if="!items.length" class="return-list-state"><BookOpen :stroke-width="1.5" /><p>У читателя нет книг на руках.</p></div>
      <article v-for="item in items" v-else :key="item.id" :data-loan-id="item.id" class="return-loan-row" :class="{ selected: item.selectedQuantity > 0 }">
        <BookOpen class="return-loan-icon" :size="29" :stroke-width="1.75" aria-hidden="true" />
        <div class="return-loan-copy"><strong>{{ item.title }}</strong><span>{{ item.code ? `Инв. № ${item.code}` : 'Учёт количеством' }}</span></div>
        <div class="return-loan-status"><span class="return-on-hand">На руках: {{ item.quantity }}</span><span v-if="item.selectedQuantity" class="return-selected"><CheckCircle2 :size="18" />К приёму<span v-if="item.selectedQuantity !== item.quantity">: {{ item.selectedQuantity }} из {{ item.quantity }}</span></span></div>
        <button v-if="item.selectedQuantity" class="return-item-action" type="button" :aria-label="`Действия для книги «${item.title}»`" @click="emit('details', item)"><ChevronRight :size="25" /></button>
      </article>
    </div>
  </section>
</template>

<style scoped>
.return-loan-list{display:flex;flex-direction:column;min-height:0;border:1px solid var(--line);border-radius:16px;background:var(--surface);overflow:hidden}
.return-loan-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:12px;padding:14px 16px 12px;border-bottom:1px solid var(--line);background:#f7fbff;flex:0 0 auto}.return-loan-heading h2{margin:0;color:var(--edus-navy);font-size:21px;line-height:1.2;font-weight:650}.return-loan-heading p{margin:5px 0 0;color:var(--muted);font-size:16px;line-height:1.3}.return-loan-count{display:grid;place-items:center;min-width:32px;height:32px;padding:0 8px;border-radius:10px;background:var(--blue-soft);color:var(--edus-blue);font-size:18px;font-weight:700}
.return-loan-scroll{min-height:0;flex:1;overflow-y:auto;scroll-behavior:smooth;padding:8px;scrollbar-width:thin;scrollbar-color:#bdccdc transparent}.return-loan-scroll:focus-visible{outline:3px solid var(--edus-blue);outline-offset:-3px}
.return-loan-row{display:grid;grid-template-columns:32px minmax(0,1fr) auto;align-items:center;gap:10px;min-height:82px;padding:10px 12px;margin-bottom:7px;border:1px solid transparent;border-radius:12px;background:#f2f4f7;color:#556278;transition:background-color .18s ease,border-color .18s ease}.return-loan-row:last-child{margin-bottom:0}.return-loan-row.selected{grid-template-columns:32px minmax(0,1fr) auto 44px;background:var(--blue-soft);border-color:#c8e0f8;color:var(--edus-blue)}
.return-loan-icon{color:#7d8999}.selected .return-loan-icon{color:var(--edus-blue)}.return-loan-copy{min-width:0;display:flex;flex-direction:column;gap:3px}.return-loan-copy strong{overflow:hidden;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;font-size:18px;line-height:1.2;font-weight:650;color:#4f5d70}.selected .return-loan-copy strong{color:var(--edus-blue)}.return-loan-copy span{font-size:15px;line-height:1.2;color:#718096}.return-loan-status{display:flex;flex-direction:column;align-items:flex-end;gap:4px;white-space:nowrap}.return-on-hand{font-size:16px;font-weight:650;color:#58677a}.return-selected{display:flex;align-items:center;justify-content:flex-end;gap:4px;font-size:15px;font-weight:650;color:#168b58}.return-item-action{width:44px;height:44px;display:grid;place-items:center;border:0;border-radius:10px;background:transparent;color:var(--edus-blue)}.return-item-action:focus-visible{outline:3px solid var(--edus-blue);outline-offset:2px}
.return-list-state{min-height:180px;height:100%;padding:24px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:14px;text-align:center;color:var(--muted);font-size:18px;line-height:1.4}.return-list-state svg{color:#9ab0c7;width:44px;height:44px}.return-list-state p{margin:0}.return-list-error{color:var(--edus-navy)}.return-list-error svg{color:var(--danger)}.return-list-error .btn{min-height:56px}.spin{animation:spin .9s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
@media(prefers-reduced-motion:reduce){.return-loan-row{transition:none}.spin{animation:none}}
</style>
