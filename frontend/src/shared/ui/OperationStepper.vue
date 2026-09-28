<script setup lang="ts">
import { t } from '../i18n/index.ts';
import { Check } from '@lucide/vue';
defineProps<{ steps: string[]; active: number }>();
</script>

<template>
  <ol class="stepper" :aria-label="t('Этапы операции')">
    <li
      v-for="(step, index) in steps"
      :key="step"
      :class="{ current: index === active, complete: index < active }"
      :aria-current="index === active ? 'step' : undefined"
    >
      <span class="step-marker"
        ><Check v-if="index < active" :size="19" :stroke-width="2" /><span v-else>{{ index + 1 }}</span></span
      ><span class="step-name">{{ t(step) }}</span
      ><span v-if="index < steps.length - 1" class="step-line" aria-hidden="true" />
    </li>
  </ol>
</template>

<style scoped>
.stepper {
  display: flex;
  align-items: center;
  list-style: none;
  padding: 0;
  margin: 0;
  gap: 16px;
}
.stepper li {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--muted, #5e6b82);
  font-size: 17px;
  white-space: nowrap;
}
.step-marker {
  width: 32px;
  height: 32px;
  border: 1px solid var(--line, #dce6f0);
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 16px;
  line-height: 1;
  background: var(--surface, #fff);
}
.current {
  color: var(--edus-blue, #046bc8) !important;
  font-weight: 600;
}
.current .step-marker {
  background: var(--edus-blue, #046bc8);
  color: #fff;
  border-color: var(--edus-blue, #046bc8);
}
.complete .step-marker {
  color: var(--edus-blue, #046bc8);
  background: var(--blue-soft, #eaf3fd);
  border-color: transparent;
}
.step-line {
  width: 32px;
  height: 1px;
  background: var(--line, #dce6f0);
  margin-left: 6px;
}
@media (max-width: 700px) {
  .stepper {
    gap: 12px;
    flex-wrap: wrap;
  }
  .stepper li {
    font-size: 16px;
    gap: 8px;
  }
  .step-line {
    display: none;
  }
}
</style>
