<script setup lang="ts">
import { CircleHelp, Settings, UserRound } from '@lucide/vue'
withDefaults(defineProps<{ time?: string; date?: string; identification?: boolean; terminalTest?: boolean }>(), { time: '10:24', date: '', identification: false, terminalTest: false })
defineEmits<{ settings: []; help: [] }>()
</script>

<template>
  <div class="system-header" :class="{ 'identification-header': identification }">
    <div class="identity">
      <img class="brand" src="/assets/edus-logo.png" alt="EDUS" width="148" height="34" />
      <span class="brand-divider" aria-hidden="true" />
      <div class="product"><strong>Библиотека</strong><span>Школа-лицей № 23</span></div>
      <span v-if="terminalTest" class="test-mode-badge">Тестовый режим</span>
    </div>
    <div class="system-tools">
      <div class="operator"><UserRound :size="24" :stroke-width="1.8" /><span><span v-if="identification" class="operator-label">Оператор</span>Алия Садыкова</span></div>
      <div class="header-clock"><time>{{ time }}</time><span v-if="identification" class="header-date">{{ date }}</span></div>
      <button class="icon-button help" type="button" aria-label="Помощь" title="Помощь" @click="$emit('help')"><CircleHelp :size="26" :stroke-width="1.8" /></button>
      <button class="icon-button" type="button" aria-label="Настройки и диагностика" title="Настройки и диагностика" @click="$emit('settings')"><Settings :size="26" :stroke-width="1.8" /></button>
    </div>
  </div>
</template>

<style scoped>
.system-header { min-height: 100px; padding: 16px 48px; display: flex; align-items: center; justify-content: space-between; gap: 24px; background: var(--surface, #fff); border-bottom: 1px solid var(--line, #DCE6F0); }
.identity, .system-tools, .operator { display: flex; align-items: center; }
.identity { gap: 24px; min-width: 0; }
.brand { width: 148px; height: auto; flex: 0 0 auto; }
.brand-divider { height: 40px; width: 1px; background: var(--line, #DCE6F0); }
.product { display: flex; flex-direction: column; gap: 3px; white-space: nowrap; }
.product strong { font-size: 23px; font-weight: 650; line-height: 1.25; }
.product span { color: var(--muted, #5E6B82); font-size: 16px; line-height: 1.4; }
.test-mode-badge { flex:0 0 auto; border:1px solid #d79a17; color:#7b4d00; background:#fff6e3; border-radius:999px; padding:7px 11px; font-size:14px; font-weight:700; white-space:nowrap; }
.system-tools { gap: 16px; flex: 0 0 auto; }
.operator { gap: 10px; font-size: 18px; margin-right: 12px; color: var(--muted, #5E6B82); }
.operator svg { color: var(--edus-blue, #046BC8); }
time { font-size: 22px; font-weight: 600; font-variant-numeric: tabular-nums; margin: 0 8px; }
.icon-button { width: 56px; height: 56px; border: 1px solid var(--line, #DCE6F0); border-radius: 12px; background: var(--surface, #fff); color: var(--edus-navy, #0B1A4D); display: grid; place-items: center; cursor: pointer; }
.icon-button:hover { background: var(--blue-soft, #EAF3FD); border-color: var(--edus-blue, #046BC8); }
.icon-button:active { background: #d7e9fb; }
.icon-button:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 3px; }
.identification-header { padding-inline: 64px; border-bottom-color: transparent; background: var(--background); }
.identification-header .identity { gap: 34px; }
.identification-header .brand { width: 170px; }
.identification-header .brand-divider { height: 48px; }
.identification-header .product strong { font-size: 24px; }
.identification-header .product span { font-size: 18px; }
.identification-header .operator { font-size: 16px; line-height: 1.45; gap: 18px; padding-right: 24px; margin-right: 0; border-right: 1px solid var(--line); }
.identification-header .operator svg { width: 32px; height: 36px; color: var(--edus-navy); }
.operator-label { display: block; color: var(--edus-navy); }
.identification-header .header-clock { min-width: 138px; padding-inline: 20px; border-right: 1px solid var(--line); }
.identification-header time { display: block; margin: 0; font-size: 24px; line-height: 1.2; }
.header-date { display: block; font-size: 16px; line-height: 1.5; white-space: nowrap; color: var(--muted); }
.identification-header .icon-button { border: 0; background: transparent; color: var(--edus-blue); }
.identification-header .help svg { width: 50px; height: 50px; stroke-width: 1; }
.identification-header .icon-button:hover { background: var(--blue-soft); }
@media (max-width: 1400px) { .system-header { min-height: 88px; padding: 12px 32px; } .identity { gap: 20px; } .brand { width: 132px; } .operator { margin-right: 0; } .system-tools { gap: 12px; } }
@media (max-width: 900px) { .operator { display: none; } .system-header { padding-inline: 24px; } .test-mode-badge { font-size:12px; padding:6px 8px; } }
@media (max-width: 650px) { .system-header { padding: 12px; gap: 8px; } .identity { gap: 10px; } .brand { width: 78px; } .brand-divider { display: none; } .product strong { font-size: 18px; } .product span { display:none; } .system-tools { gap: 4px; } time { display: none; } }
@media (max-width: 1400px) { .identification-header { padding-inline: 40px; } .identification-header .identity { gap: 24px; } .identification-header .brand { width: 148px; } .identification-header .operator { gap: 12px; padding-right: 16px; } .identification-header .header-clock { padding-inline: 12px; } }
@media (max-width: 1100px) { .identification-header .operator { display: none; } }
@media (max-width: 760px) { .identification-header { padding: 12px 20px; } .identification-header .brand { width: 108px; } .identification-header .identity { gap: 16px; } .identification-header .product strong { font-size: 20px; } .identification-header .product span, .identification-header .header-clock { display: none; } }
@media (max-width: 480px) { .identification-header { padding-inline: 12px; gap: 8px; } .identification-header .identity { gap: 12px; } .identification-header .brand { width: 86px; } .identification-header .product strong { font-size: 18px; } }
@media (min-width: 901px) and (max-width: 1300px) and (max-height: 820px) { .system-header { min-height: 80px; padding: 8px 24px; } .identification-header { padding-inline: 24px; } .identification-header .brand { width: 132px; } .identification-header .brand-divider { height: 40px; } .identification-header .product strong { font-size: 22px; } .identification-header .product span { font-size: 16px; } .identification-header .operator { padding-right: 12px; gap: 10px; } .identification-header .operator svg { width: 28px; height: 30px; } .identification-header .header-clock { min-width: 126px; padding-inline: 12px; } .identification-header time { font-size: 22px; } }
</style>
