<script setup lang="ts">
import { CircleAlert, Search, X } from '@lucide/vue'
import NfcCardAnimation from './NfcCardAnimation.vue'

defineProps<{
  state: 'idle' | 'reading' | 'success' | 'error'
  busy: boolean
  message: string
}>()
defineEmits<{ search: []; dismiss: [] }>()
</script>

<template>
  <section class="identify-content" aria-labelledby="identify-heading">
    <h1 id="identify-heading">Приложите карту ученика</h1>
    <p class="identify-subtitle">Дождитесь, пока появится имя читателя</p>
    <div class="identify-illustration"><NfcCardAnimation :state="state" /></div>
    <div class="identify-alternative">
      <div class="identify-divider"><span />или<span /></div>
      <button class="btn secondary identify-search" :disabled="busy" @click="$emit('search')"><Search :stroke-width="1.8" />Найти вручную</button>
    </div>
    <div v-if="message" class="identify-error" role="alert"><CircleAlert /><span>{{ message }}</span><button aria-label="Закрыть сообщение" @click="$emit('dismiss')"><X /></button></div>
    <span class="sr-only" role="status">{{ state === 'reading' ? 'Определяем карту' : state === 'success' ? 'Читатель найден' : '' }}</span>
  </section>
</template>

<style scoped>
.identify-content { --card-height: 370px; display: flex; flex-direction: column; align-items: center; flex: 1; width: 100%; min-height: 0; text-align: center; }
.identify-content h1 { font-size: 46px; line-height: 58px; letter-spacing: -1.4px; font-weight: 650; }
.identify-subtitle { margin-top: 12px; font-size: 27px; line-height: 42px; color: var(--muted); }
.identify-illustration { margin-top: 48px; flex-shrink: 0; }
.identify-alternative { margin-top: 60px; width: 410px; max-width: 100%; }
.identify-divider { display: flex; align-items: center; gap: 22px; color: var(--muted); font-size: 18px; line-height: 30px; }
.identify-divider span { flex: 1; height: 1px; background: var(--line); }
.identify-search { width: 396px; max-width: 100%; min-height: 94px; margin-top: 20px; border-radius: 16px; border-color: color-mix(in srgb, var(--edus-blue) 45%, white); background: var(--surface); font-size: 28px; font-weight: 550; gap: 22px; }
.identify-search svg { width: 40px; height: 40px; }
.identify-error { display: flex; align-items: center; gap: 12px; width: min(680px, 100%); margin-top: 16px; padding: 8px 12px 8px 20px; text-align: left; font-size: 18px; line-height: 1.4; border-radius: 12px; color: var(--danger); background: var(--danger-soft); }
.identify-error > svg { flex-shrink: 0; }
.identify-error button { display: grid; place-items: center; flex: 0 0 56px; min-height: 56px; margin-left: auto; border: 0; background: transparent; color: inherit; border-radius: 8px; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
@media (max-height: 950px) {
  .identify-content { --card-height: 310px; }
  .identify-content h1 { font-size: 42px; line-height: 52px; }
  .identify-subtitle { font-size: 25px; line-height: 36px; margin-top: 8px; }
  .identify-illustration { margin-top: 24px; }
  .identify-alternative { margin-top: 26px; }
  .identify-search { min-height: 80px; margin-top: 14px; font-size: 26px; }
}
@media (max-height: 820px) {
  .identify-content { --card-height: 260px; }
  .identify-content h1 { font-size: 38px; line-height: 48px; }
  .identify-subtitle { font-size: 23px; line-height: 34px; }
  .identify-illustration { margin-top: 18px; }
  .identify-alternative { margin-top: 18px; }
  .identify-search { min-height: 72px; margin-top: 10px; font-size: 24px; }
  .identify-search svg { width: 32px; height: 32px; }
}
@media (max-width: 760px) {
  .identify-content { --card-height: 280px; padding: 8px 0 24px; }
  .identify-content h1 { font-size: 32px; line-height: 1.25; letter-spacing: -.7px; }
  .identify-subtitle { font-size: 20px; line-height: 1.45; }
  .identify-illustration { margin-top: 24px; }
  .identify-alternative { margin-top: 24px; }
  .identify-search { font-size: 23px; padding-inline: 16px; gap: 12px; }
}
</style>
