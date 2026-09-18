<script setup lang="ts">
import { Server, Wifi, WifiOff } from '@lucide/vue'
defineProps<{ server: boolean; internet: boolean }>()
</script>

<template>
  <div class="connection-status" aria-label="Моделируемое состояние соединения">
    <span class="status-item" :class="server ? 'online' : 'offline'"><Server :size="17" :stroke-width="1.8" /><span>Сервер: {{ server ? 'доступен' : 'нет связи' }}</span><i aria-hidden="true" /></span>
    <span class="status-item" :class="internet ? 'online' : 'limited'"><Wifi v-if="internet" :size="18" :stroke-width="1.8" /><WifiOff v-else :size="18" :stroke-width="1.8" /><span>Интернет: {{ internet ? 'доступен' : 'нет связи' }}</span><i aria-hidden="true" /></span>
    <span class="simulation">Демонстрационный режим · соединение моделируется</span>
  </div>
</template>

<style scoped>
.connection-status { display: flex; align-items: center; gap: 28px; flex-wrap: wrap; padding: 18px 48px; min-height: 56px; font-size: 16px; line-height: 1.35; color: var(--muted, #5E6B82); background: var(--surface, #fff); border-top: 1px solid var(--line, #DCE6F0); }
.status-item { display: flex; align-items: center; gap: 8px; }.status-item i { display: block; width: 6px; height: 6px; border-radius: 50%; background: var(--success, #12805C); margin-left: 3px; }.offline { color: #ac2934; }.offline i { background: #ac2934; }.limited { color: #805410; }.limited i { background: #ad7400; }.simulation { margin-left: auto; }
@media (max-width: 1400px) { .connection-status { padding-inline: 32px; gap: 24px; } }
@media (max-width: 900px) { .connection-status { gap: 10px 24px; padding: 12px 24px; }.simulation { margin-left: 0; flex-basis: 100%; } }
@media (max-width: 600px) { .connection-status { padding-inline: 16px; } }
</style>
