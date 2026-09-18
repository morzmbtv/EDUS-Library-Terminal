<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Check, ChevronDown, Delete, Space } from '@lucide/vue'
const props = withDefaults(defineProps<{ modelValue: string; label?: string; inline?: boolean }>(), { label: 'Ввод текста', inline: false })
const emit = defineEmits<{ 'update:modelValue': [value: string]; close: []; enter: [] }>()
const language = ref<'ru' | 'kk' | 'en' | 'number'>('ru')
const shifted = ref(false)
const mobilePage = ref(0)
const layouts: Record<typeof language.value, string[]> = {
  ru: ['йцукенгшщзхъ', 'фывапролджэ', 'ячсмитьбюё.-'],
  kk: ['йцукенгшщзхъәі', 'фывапролджэңғү', 'ячсмитьбюөқұһё'],
  en: ['qwertyuiop', 'asdfghjkl', 'zxcvbnm.-'],
  number: ['1234567890', 'АБВГДЕЖЗИК', '-/.,()№+'],
}
const rows = computed(() => layouts[language.value].map(row => [...(shifted.value ? row.toUpperCase() : row)]))
const mobileCharacters = computed(() => rows.value.flat())
const mobilePageCount = computed(() => Math.ceil(mobileCharacters.value.length / 18))
const mobileRows = computed(() => Array.from({ length: 3 }, (_, row) => Array.from({ length: 6 }, (_, column) => mobileCharacters.value[mobilePage.value * 18 + row * 6 + column] ?? '')))
watch(language, () => { mobilePage.value = 0 })
function nextMobilePage() { mobilePage.value = (mobilePage.value + 1) % mobilePageCount.value }
function type(char: string) { emit('update:modelValue', props.modelValue + char) }
function backspace() { emit('update:modelValue', [...props.modelValue].slice(0, -1).join('')) }
function cycleLanguage() { const languages = ['ru', 'kk', 'en'] as const; language.value = languages[(languages.indexOf(language.value as 'ru' | 'kk' | 'en') + 1) % languages.length]! }
function confirm() { emit('enter'); emit('close') }
</script>

<template>
  <section class="touch-keyboard" :class="{ inline }" aria-label="Экранная клавиатура" @pointerdown.prevent>
    <div class="keyboard-heading"><span class="keyboard-field-label">{{ label }}</span><button type="button" class="mobile-page-button" :aria-label="`Следующая страница клавиатуры. Страница ${mobilePage + 1} из ${mobilePageCount}`" @click="nextMobilePage">{{ language === 'number' ? 'Знаки' : 'Буквы' }} {{ mobilePage + 1 }}/{{ mobilePageCount }}<span aria-hidden="true">→</span></button><button type="button" class="close-keyboard" @click="emit('close')"><ChevronDown :size="20" :stroke-width="1.8" />Скрыть</button></div>
    <div class="keyboard-layout">
      <div v-for="(row, index) in rows" :key="`desktop-${index}`" class="key-row desktop-letter-row"><button v-for="char in row" :key="char" type="button" class="key" @click="type(char)">{{ char }}</button></div>
      <div v-for="(row, index) in mobileRows" :key="`mobile-${index}`" class="key-row mobile-letter-row"><template v-for="(char, column) in row" :key="column"><button v-if="char" type="button" class="key" @click="type(char)">{{ char }}</button><span v-else class="key-spacer" aria-hidden="true" /></template></div>
      <div class="key-row utilities"><button type="button" class="key language" aria-label="Изменить язык клавиатуры" @click="cycleLanguage">{{ language === 'kk' ? 'ҚАЗ' : language === 'en' ? 'ENG' : 'РУС' }}</button><button type="button" class="key mode" :aria-label="language === 'number' ? 'Переключить на буквы' : 'Переключить на цифры и знаки'" :aria-pressed="language === 'number'" @click="language = language === 'number' ? 'ru' : 'number'">{{ language === 'number' ? 'АБВ' : '123' }}</button><button type="button" class="key shift" :aria-pressed="shifted" aria-label="Заглавные буквы" @click="shifted = !shifted">Аа</button><button type="button" class="key space" aria-label="Пробел" @click="type(' ')"><Space :size="23" :stroke-width="1.8" /><span>Пробел</span></button><button type="button" class="key backspace" aria-label="Удалить последний символ" @click="backspace"><Delete :size="26" :stroke-width="1.8" /></button><button type="button" class="key done" aria-label="Готово" @click="confirm"><Check :size="22" :stroke-width="1.8" /><span>Готово</span></button></div>
    </div>
  </section>
</template>

<style scoped>
.touch-keyboard { position: fixed; bottom: 0; left: 0; right: 0; z-index: 100; height: 300px; background: #eaf0f7; border-top: 1px solid #becedf; padding: 0 24px 4px; box-shadow: 0 -4px 20px rgb(11 26 77 / 6%); box-sizing: border-box; }
.touch-keyboard.inline { position: relative; inset: auto; flex: 0 0 300px; width: 100%; box-shadow: none; }
.keyboard-heading { height: 56px; max-width: 1280px; margin: 0 auto; display: flex; align-items: center; justify-content: space-between; gap: 16px; font-size: 16px; font-weight: 500; color: var(--muted, #5E6B82); }
.close-keyboard { min-height: 56px; border: 0; background: transparent; color: var(--edus-navy, #0B1A4D); display: inline-flex; align-items: center; gap: 6px; padding: 0 12px; font: inherit; cursor: pointer; border-radius: 8px; }
.mobile-page-button, .key-row.mobile-letter-row { display: none; }
.keyboard-layout { display: flex; flex-direction: column; gap: 5px; max-width: 1280px; margin: 0 auto; }
.key-row { display: flex; justify-content: center; gap: 7px; }.key { flex: 1; min-width: 0; height: 56px; border: 1px solid #c8d4e1; border-bottom-width: 2px; border-radius: 9px; background: var(--surface, #fff); color: var(--edus-navy, #0B1A4D); font: inherit; font-size: 23px; line-height: 1; display: inline-flex; align-items: center; justify-content: center; gap: 10px; cursor: pointer; }
.key:hover { background: #f7fbff; }.key:active { background: #d2e5f8; border-color: var(--edus-blue, #046BC8); }.key:focus-visible, .close-keyboard:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: 1px; }.key[aria-pressed='true'] { border-color: var(--edus-blue, #046BC8); color: var(--edus-blue, #046BC8); background: #e1effc; }
.utilities .key { font-size: 18px; }.utilities .space { flex: 4; }.utilities .done { flex: 2; background: var(--edus-blue, #046BC8); color: #fff; border-color: var(--edus-blue, #046BC8); }.utilities .done:active { background: #055dab; }
@media (max-width: 900px) { .touch-keyboard { padding-inline: 12px; }.keyboard-layout, .key-row { gap: 5px; }.key { font-size: 21px; }.utilities .space span { display: none; } }
@media (max-width: 600px) { .touch-keyboard { padding-inline: 8px; }.keyboard-field-label, .desktop-letter-row { display: none; }.key-row.mobile-letter-row { display: flex; }.mobile-page-button { display: inline-flex; min-height: 56px; gap: 12px; align-items: center; border: 0; border-radius: 8px; padding: 0 12px; font: inherit; color: var(--edus-blue, #046BC8); background: transparent; cursor: pointer; }.mobile-page-button:focus-visible { outline: 3px solid var(--edus-blue, #046BC8); outline-offset: -3px; }.mobile-page-button:active { background: #d2e5f8; }.key { min-width: 56px; font-size: 23px; }.key-spacer { flex: 1; min-width: 56px; height: 56px; }.utilities .key { flex: 1; font-size: 16px; gap: 0; padding-inline: 0; }.utilities .space span, .utilities .done span { display: none; } }
</style>
