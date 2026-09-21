<script setup lang="ts">
import { ArrowRight, BookOpen, CornerDownLeft, Plus, Search } from '@lucide/vue'

const props = defineProps<{
  title: string
  description: string
  icon: 'issue' | 'return' | 'search' | 'add'
  action?: string
  /** Kept for the component gallery while the terminal home uses semantic actions. */
  accent?: boolean
}>()

defineEmits<{ click: [] }>()
</script>

<template>
  <button type="button" class="home-action" :class="props.icon" @click="$emit('click')">
    <span class="action-icon" aria-hidden="true">
      <BookOpen v-if="icon === 'issue'" :stroke-width="1.8" />
      <CornerDownLeft v-else-if="icon === 'return'" :stroke-width="1.8" />
      <Search v-else-if="icon === 'search'" :stroke-width="1.8" />
      <Plus v-else :stroke-width="1.8" />
    </span>
    <span class="action-copy">
      <span class="action-title">{{ title }}</span>
      <span class="action-description">{{ description }}</span>
    </span>
    <span class="action-button"><ArrowRight :stroke-width="2" />{{ action ?? (icon === 'add' ? 'Добавить' : icon === 'search' ? 'Поиск' : 'Начать') }}</span>
  </button>
</template>

<style scoped>
.home-action { width:100%; min-height:390px; padding:34px 28px 28px; border:1px solid var(--line,#DCE6F0); border-radius:var(--radius-card,24px); background:var(--surface,#fff); color:var(--edus-navy,#0B1A4D); display:flex; flex-direction:column; align-items:center; text-align:center; cursor:pointer; box-shadow:var(--shadow); transition:border-color var(--motion),background var(--motion),box-shadow var(--motion),transform var(--motion); }
.action-icon { width:108px; height:108px; display:grid; place-items:center; border-radius:50%; flex:0 0 auto; }
.action-icon svg { width:62px; height:62px; }.issue .action-icon { color:var(--edus-blue); background:var(--blue-soft); }.return .action-icon { color:var(--success); background:var(--success-soft); }.search .action-icon { color:#D36A12; background:#FFF0E7; }.add .action-icon { color:var(--edus-blue); background:var(--blue-soft); }
.action-copy { display:flex; flex-direction:column; align-items:center; gap:12px; margin-top:22px; }.action-title { font-size:30px; line-height:1.18; font-weight:700; letter-spacing:-.6px; }.action-description { max-width:320px; min-height:56px; color:var(--muted); font-size:20px; line-height:1.4; }
.action-button { width:100%; min-height:68px; margin-top:auto; padding:14px 20px; border-radius:var(--radius-control,12px); display:inline-flex; align-items:center; justify-content:center; gap:14px; color:#fff; font-size:21px; font-weight:650; }.issue .action-button { background:var(--edus-blue); }.return .action-button { background:var(--success); }.search .action-button { background:#D96D12; }.add .action-button { background:var(--edus-blue); }
.home-action:hover:not(:disabled) { border-color:#B8D6F2; background:#FBFDFF; box-shadow:0 8px 24px #0B1A4D0D; }.home-action:active:not(:disabled) { transform:translateY(2px); box-shadow:0 2px 10px #0B1A4D12; }.home-action:focus-visible { outline:4px solid var(--edus-blue); outline-offset:4px; }.home-action:disabled { opacity:.5; cursor:not-allowed; }
@media (max-height:820px) and (min-width:900px) { .home-action { min-height:370px; padding:28px 24px 24px; }.action-icon { width:96px; height:96px; }.action-icon svg { width:56px; height:56px; }.action-copy { margin-top:18px; gap:10px; }.action-title { font-size:28px; }.action-description { min-height:50px; font-size:18px; }.action-button { min-height:64px; font-size:20px; } }
@media (max-width:760px) { .home-action { min-height:280px; }.action-icon { width:88px; height:88px; }.action-icon svg { width:50px; height:50px; }.action-title { font-size:28px; } }
@media (prefers-reduced-motion:reduce) { .home-action { transition:none; } }
</style>
