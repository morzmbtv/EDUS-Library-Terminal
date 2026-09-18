<script setup lang="ts">
import { CheckCircle2, CircleAlert, Info, TriangleAlert } from '@lucide/vue'
withDefaults(defineProps<{ kind?: 'success' | 'error' | 'warning' | 'info'; title: string; text?: string }>(), { kind: 'info', text: '' })
</script>

<template>
  <div class="state-message" :class="kind" :role="kind === 'error' ? 'alert' : 'status'" :aria-live="kind === 'error' ? 'assertive' : 'polite'">
    <CheckCircle2 v-if="kind === 'success'" :size="26" :stroke-width="1.8" /><CircleAlert v-else-if="kind === 'error'" :size="26" :stroke-width="1.8" /><TriangleAlert v-else-if="kind === 'warning'" :size="26" :stroke-width="1.8" /><Info v-else :size="26" :stroke-width="1.8" />
    <div><strong>{{ title }}</strong><p v-if="text">{{ text }}</p><slot /></div>
  </div>
</template>

<style scoped>
.state-message { display: flex; gap: 14px; align-items: flex-start; padding: 20px 24px; border: 1px solid; border-radius: 14px; font-size: 18px; line-height: 1.45; }
.state-message > svg { flex: 0 0 auto; margin-top: 1px; }.state-message strong { font-weight: 600; font-size: 20px; }.state-message p { margin: 5px 0 0; }
.info { background: var(--blue-soft, #EAF3FD); border-color: #d0e4f8; color: #164d80; }.success { background: #edf8f3; border-color: #c5e6d8; color: #106448; }.error { background: #fff2f2; border-color: #f1c8cc; color: #a82731; }.warning { background: #fff7e7; border-color: #efd8a6; color: #805410; }
</style>
