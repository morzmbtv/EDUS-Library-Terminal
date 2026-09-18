<script setup lang="ts">
import { DEMO_CODES } from '../domain/index.ts'

defineProps<{ screen: string }>()
const emit = defineEmits<{
  card: [code: string]
  scan: [code: string]
  outcome: [outcome: 'unknown' | 'error']
  expire: []
}>()
</script>

<template>
  <div class="modal-section">
    <h3>Демонстрационные события</h3>
    <p class="small">Выберите операцию на главной, затем отправьте событие соответствующего устройства.</p>
    <div class="demo-tools" style="margin-top:16px">
      <button class="btn" :disabled="screen!=='identify'" @click="emit('card', DEMO_CODES.readerCard)">Карта Айши</button>
      <button class="btn" :disabled="screen!=='identify'" @click="emit('card', DEMO_CODES.secondReaderCard)">Карта Әлихана</button>
      <button class="btn" :disabled="!['scan','registration'].includes(screen)" @click="emit('scan', DEMO_CODES.availableCopy)">Книга 000124</button>
      <button class="btn" :disabled="!['scan','registration'].includes(screen)" @click="emit('scan', DEMO_CODES.secondAvailableCopy)">Книга 000125</button>
      <button class="btn" :disabled="screen!=='scan'" @click="emit('scan', DEMO_CODES.issuedCopy)">Выданная 000123</button>
      <button class="btn" :disabled="!['scan','registration'].includes(screen)" @click="emit('scan', DEMO_CODES.legacyIsbn)">ISBN старого фонда</button>
      <button class="btn" :disabled="screen!=='scan'" @click="emit('scan', DEMO_CODES.unknown)">Неизвестный код</button>
      <button class="btn" :disabled="screen!=='scan'" @click="emit('scan', DEMO_CODES.ambiguous)">Неоднозначный код</button>
    </div>
    <div class="demo-tools" style="margin-top:16px">
      <button class="btn" @click="emit('outcome', 'unknown')">Задержать ответ операции</button>
      <button class="btn" @click="emit('outcome', 'error')">Ошибка следующей операции</button>
      <button class="btn" @click="emit('expire')">Истекла сессия</button>
    </div>
  </div>
</template>
