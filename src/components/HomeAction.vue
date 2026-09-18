<script setup lang="ts">
import { ArrowDownToLine, ArrowUpRight, BookOpen, CornerDownLeft, LoaderCircle, Plus } from '@lucide/vue'
withDefaults(defineProps<{ title: string; description: string; icon: 'issue' | 'return' | 'add'; accent?: boolean; disabled?: boolean; loading?: boolean }>(), { accent: false, disabled: false, loading: false })
defineEmits<{ click: [] }>()
</script>

<template>
  <button type="button" class="home-action" :class="[icon, { accent, loading }]" :disabled="disabled || loading" :aria-busy="loading" @click="$emit('click')">
    <div class="action-top"><span class="action-icon"><LoaderCircle v-if="loading" class="spinner" :size="42" :stroke-width="1.8" /><CornerDownLeft v-else-if="icon === 'return'" :size="44" :stroke-width="1.8" /><BookOpen v-else :size="44" :stroke-width="1.8" /><span v-if="icon === 'add' && !loading" class="add-mark" aria-hidden="true"><Plus :size="20" :stroke-width="2" /></span></span><ArrowUpRight class="direction" :size="28" :stroke-width="1.8" /></div>
    <div class="action-copy"><h2>{{ title }}</h2><p>{{ description }}</p></div>
    <span class="action-bottom" aria-hidden="true"><span>{{ icon === 'add' ? 'Пополнить библиотеку' : 'По карте читателя' }}</span><ArrowDownToLine v-if="icon === 'add'" :size="22" :stroke-width="1.8" /></span>
  </button>
</template>

<style scoped>
.home-action { width: 100%; min-height: 330px; padding: 32px; border: 1px solid var(--line, #DCE6F0); border-radius: 24px; background: var(--surface, #fff); color: var(--edus-navy, #0B1A4D); display: flex; flex-direction: column; text-align: left; cursor: pointer; transition: border-color 150ms, background 150ms, transform 150ms; }
.home-action.issue { background: var(--edus-blue, #046BC8); border-color: var(--edus-blue, #046BC8); color: #fff; }
.home-action.return { background: var(--gold-soft, #FFF5DF); border-color: #eed7a6; }
.home-action.add { border-color: #c3d7ed; }
.action-top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 32px; }
.action-icon { position: relative; display: grid; place-items: center; width: 80px; height: 80px; border-radius: 18px; background: var(--blue-soft, #EAF3FD); color: var(--edus-blue, #046BC8); }
.issue .action-icon { background: rgb(255 255 255 / 14%); color: #fff; }
.return .action-icon { background: var(--edus-gold, #E39300); color: var(--edus-navy, #0B1A4D); }
.add-mark { position: absolute; bottom: 9px; right: 8px; width: 26px; height: 26px; border-radius: 7px; display: grid; place-items: center; color: var(--edus-navy, #0B1A4D); background: var(--edus-gold, #E39300); border: 2px solid var(--blue-soft, #EAF3FD); }
.direction { color: var(--muted, #5E6B82); opacity: .8; }
.issue .direction { color: #fff; }
.return .direction { color: #956100; }
.add .direction { color: var(--edus-blue, #046BC8); }
.action-copy h2 { margin: 0; font-size: clamp(28px, 2.1vw, 34px); line-height: 1.22; font-weight: 650; letter-spacing: -.025em; }
.action-copy p { max-width: 280px; margin: 14px 0 28px; font-size: 20px; line-height: 1.5; color: var(--muted, #5E6B82); }
.issue .action-copy p { color: #e3effb; }
.return .action-copy p { color: #695936; }
.action-bottom { border-top: 1px solid var(--line, #DCE6F0); padding-top: 20px; margin-top: auto; display: flex; justify-content: space-between; gap: 12px; align-items: center; font-size: 17px; font-weight: 500; }
.issue .action-bottom { border-color: rgb(255 255 255 / 22%); color: #fff; }
.return .action-bottom { border-color: #e8ce97; color: #644f25; }
.add .action-bottom { color: var(--edus-blue, #046BC8); }
.add .action-bottom > svg { color: #a66c00; }
.home-action:hover:not(:disabled) { border-color: var(--edus-blue, #046BC8); background: #f3f8fe; }
.home-action.issue:hover:not(:disabled) { background: #055dab; }
.home-action.return:hover:not(:disabled) { background: #ffefd0; border-color: var(--edus-gold, #E39300); }
.home-action:active:not(:disabled) { transform: translateY(2px); }
.home-action:focus-visible { outline: 4px solid var(--edus-gold, #E39300); outline-offset: 4px; }
.home-action:disabled { opacity: .5; cursor: not-allowed; }
.spinner { animation: rotate 1s linear infinite; }
@keyframes rotate { to { transform: rotate(360deg); } }
@media (max-width: 1400px) { .home-action { min-height: 310px; padding: 28px; } .action-top { margin-bottom: 24px; } .action-icon { width: 72px; height: 72px; } .action-copy p { font-size: 19px; } }
@media (max-width: 800px) { .home-action { min-height: 250px; } .action-top { margin-bottom: 16px; } }
@media (prefers-reduced-motion: reduce) { .home-action { transition: none; } .spinner { animation: none; } }
</style>
