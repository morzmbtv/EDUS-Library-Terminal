<script setup lang="ts">
import { t } from '../i18n/index.ts';
import { ArrowLeft } from '@lucide/vue';

defineProps<{ disabled?: boolean }>();
const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <button
    class="btn ghost back-button"
    type="button"
    :disabled="disabled"
    :aria-label="t('Назад')"
    @click="emit('click')"
  >
    <ArrowLeft :size="24" />{{ t('Назад') }}
  </button>
</template>
