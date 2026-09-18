<script setup lang="ts">
import { Minus, Plus } from '@lucide/vue'
const props = withDefaults(defineProps<{ modelValue: number; min?: number; max?: number; disabled?: boolean; label?: string }>(), { min: 1, max: 99, disabled: false, label: 'Количество книг' })
const emit = defineEmits<{ 'update:modelValue': [value: number] }>()
function update(value: number) { emit('update:modelValue', Math.min(props.max, Math.max(props.min, value))) }
function onInput(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.value.trim() === '') return
  const value = Number(input.value)
  if (Number.isFinite(value)) update(Math.trunc(value))
}
function onBlur(event: FocusEvent) { (event.target as HTMLInputElement).value = String(props.modelValue) }
</script>

<template>
  <div class="quantity-control" role="group" :aria-label="label">
    <button type="button" :disabled="disabled || modelValue <= min" aria-label="Уменьшить количество" @click="update(modelValue - 1)"><Minus :size="24" :stroke-width="1.8" /></button>
    <input type="number" inputmode="numeric" :value="modelValue" :min="min" :max="max" :disabled="disabled" :aria-label="label" @input="onInput" @blur="onBlur" />
    <button type="button" :disabled="disabled || modelValue >= max" aria-label="Увеличить количество" @click="update(modelValue + 1)"><Plus :size="24" :stroke-width="1.8" /></button>
  </div>
</template>

<style scoped>
.quantity-control { display: inline-flex; align-items: stretch; height: 64px; border: 1px solid var(--line, #DCE6F0); border-radius: 12px; background: var(--surface, #fff); }
button { width: 64px; min-width: 56px; border: 0; border-radius: 11px; display: grid; place-items: center; color: var(--edus-blue, #046BC8); background: transparent; cursor: pointer; }
input { width: 76px; min-width: 0; border: solid var(--line, #DCE6F0); border-width: 0 1px; border-radius: 0; color: var(--edus-navy, #0B1A4D); background: transparent; text-align: center; font: inherit; font-size: 24px; font-weight: 600; appearance: textfield; -moz-appearance: textfield; }
input::-webkit-inner-spin-button, input::-webkit-outer-spin-button { appearance: none; margin: 0; }
button:hover:not(:disabled) { background: var(--blue-soft, #EAF3FD); }button:active:not(:disabled) { background: #d7e9fb; }button:disabled { color: #a8b3c2; cursor: not-allowed; }button:focus-visible, input:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 2px; z-index: 1; }
</style>
