<script setup lang="ts">
import { t } from '../i18n/index.ts';
import { ref, useId } from 'vue';
import { Keyboard } from '@lucide/vue';
defineOptions({ inheritAttrs: false });
const props = withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    placeholder?: string;
    error?: string;
    type?: string;
    disabled?: boolean;
    keyboard?: boolean;
    activateOnFocus?: boolean;
  }>(),
  { placeholder: '', error: '', type: 'text', disabled: false, keyboard: true, activateOnFocus: false },
);
const emit = defineEmits<{
  'update:modelValue': [value: string];
  keyboard: [];
  enter: [];
  focus: [event: FocusEvent];
}>();
const inputRef = ref<HTMLInputElement | null>(null);
const id = useId();
defineExpose({ focus: () => inputRef.value?.focus(), input: inputRef });
function openKeyboard() {
  inputRef.value?.focus();
  emit('keyboard');
}
function onFocus(event: FocusEvent) {
  emit('focus', event);
  if (props.activateOnFocus) emit('keyboard');
}
function reopenKeyboardOnPointerDown() {
  // A focused kiosk field may have had its keyboard dismissed. A further tap on
  // the field must reopen the existing keyboard without requiring an icon.
  if (props.activateOnFocus && document.activeElement === inputRef.value) emit('keyboard');
}
</script>

<template>
  <div class="touch-field" :class="{ 'has-error': error, disabled }">
    <label :for="id">{{ label }}</label>
    <div class="input-wrap">
      <input
        v-bind="$attrs"
        :id="id"
        ref="inputRef"
        :value="modelValue"
        :type="type"
        :placeholder="placeholder"
        :disabled="disabled"
        :aria-invalid="!!error"
        :aria-describedby="error ? `${id}-error` : undefined"
        autocomplete="off"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        @keydown.enter.prevent="emit('enter')"
        @focus="onFocus"
        @pointerdown="reopenKeyboardOnPointerDown"
      />
      <button
        v-if="keyboard"
        type="button"
        class="keyboard-button"
        :disabled="disabled"
        :aria-label="`Открыть клавиатуру: ${label}`"
        :title="t('Экранная клавиатура')"
        @pointerdown.prevent
        @click="openKeyboard"
      >
        <Keyboard :size="25" :stroke-width="1.8" />
      </button>
    </div>
    <p v-if="error" :id="`${id}-error`" class="field-error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.touch-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}
label {
  font-size: 18px;
  font-weight: 500;
  color: var(--edus-navy, #0b1a4d);
  line-height: 1.4;
}
.input-wrap {
  display: flex;
  align-items: center;
  min-height: 72px;
  border: 1px solid #b9c8d9;
  border-radius: 12px;
  background: var(--surface, #fff);
  transition:
    border-color 120ms,
    box-shadow 120ms;
}
.input-wrap:focus-within {
  border-color: var(--edus-blue, #046bc8);
  box-shadow: 0 0 0 3px rgb(4 107 200 / 12%);
}
input {
  min-width: 0;
  width: 100%;
  min-height: 70px;
  padding: 16px 20px;
  border: 0;
  border-radius: 12px;
  outline: none;
  background: transparent;
  color: var(--edus-navy, #0b1a4d);
  font: inherit;
  font-size: 22px;
  line-height: 1.3;
}
input::placeholder {
  color: var(--muted, #5e6b82);
  opacity: 1;
}
.keyboard-button {
  flex: 0 0 56px;
  min-height: 56px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--edus-blue, #046bc8);
  margin-right: 7px;
  cursor: pointer;
}
.keyboard-button:hover {
  background: var(--blue-soft, #eaf3fd);
}
.keyboard-button:focus-visible {
  outline: 3px solid var(--edus-blue, #046bc8);
}
.keyboard-button:active {
  background: #d7e9fb;
}
.has-error .input-wrap {
  border-color: var(--danger, #c7353d);
}
.field-error {
  color: var(--danger, #c7353d);
  font-size: 17px;
  margin: 0;
  line-height: 1.4;
}
.disabled .input-wrap {
  background: #f1f4f8;
  opacity: 0.65;
}
.disabled input,
.keyboard-button:disabled {
  cursor: not-allowed;
}
</style>
