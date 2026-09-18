<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Keyboard, X } from '@lucide/vue'
import type { Operation, Screen } from '../composables/useTerminal'

const props = defineProps<{ screen: Screen; operation: Operation }>()
const emit = defineEmits<{ close: [] }>()
const closeButton = ref<HTMLElement | null>(null)
const content = computed(() => {
  if (props.screen === 'home') return {
    title: 'Как работать с терминалом',
    steps: [
      { label: 'Выдать книги', text: 'Определите ученика, отсканируйте книги и подтвердите выдачу.' },
      { label: 'Принять книги', text: 'Сначала определите ученика, затем отсканируйте его книги и подтвердите приём.' },
      { label: 'Добавить книги', text: 'Найдите или создайте карточку издания и зарегистрируйте поступление.' },
    ],
  }
  if (props.screen === 'identify') return {
    title: 'Как это работает?',
    steps: [
      { text: 'Приложите школьную карту к считывателю' },
      { text: 'Дождитесь появления имени ученика' },
      { text: 'Проверьте, что выбран нужный человек' },
    ],
    note: 'Нет карты? Нажмите «Найти вручную».',
  }
  if (props.screen === 'search') return {
    title: 'Как найти читателя',
    steps: [
      { text: 'Введите фамилию, имя или номер карты.' },
      { text: 'Выберите нужного человека в списке.' },
      { text: 'Проверьте имя и класс перед сканированием книг.' },
    ],
    note: 'Кнопка клавиатуры рядом с полем открывает экранную клавиатуру.',
  }
  if (props.screen === 'unknown') return {
    title: 'Как проверить результат',
    steps: [
      { text: 'Оставьте текущий список открытым: операция могла выполниться.' },
      { text: 'Дождитесь восстановления связи.' },
      { text: 'Нажмите «Проверить результат».' },
    ],
    note: 'Повторная отправка заблокирована, чтобы не оформить книги дважды.',
  }
  if (props.screen === 'success') return {
    title: 'Операция завершена',
    steps: [
      { text: 'Проверьте количество обработанных книг.' },
      { text: 'Для новой операции нажмите «На главную».' },
      { text: 'Чтобы повторить этот вид операции, используйте соседнюю кнопку.' },
    ],
  }
  if (props.operation === 'register') return {
    title: 'Как добавить книги',
    steps: [
      { text: 'Отсканируйте ISBN или найдите книгу вручную.' },
      { text: 'Выберите существующее издание либо создайте новое.' },
      { text: 'Укажите номера экземпляров или количество книг старого фонда.' },
      { text: 'Проверьте список и подтвердите добавление.' },
    ],
  }
  const accepting = props.operation === 'accept'
  if (props.screen === 'confirm') return {
    title: accepting ? 'Перед приёмом книг' : 'Перед выдачей книг',
    steps: [
      { text: 'Проверьте имя выбранного ученика.' },
      { text: 'Сверьте книги и количество. Для исправления нажмите «Изменить список».' },
      { text: accepting ? 'Нажмите «Принять» и дождитесь результата.' : 'Нажмите «Выдать» и дождитесь результата.' },
    ],
  }
  return {
    title: accepting ? 'Как принять книги' : 'Как выдать книги',
    steps: [
      { text: accepting ? 'После выбора читателя справа видны его книги.' : 'Проверьте имя выбранного ученика.' },
      { text: accepting ? 'Сканируйте возвращаемые книги — они отметятся к приёму.' : 'По очереди сканируйте книги.' },
      { text: accepting ? 'Проверьте количество и перейдите к подтверждению.' : 'Проверьте список и подтвердите выдачу.' },
    ],
    note: accepting ? 'Код не читается? Введите номер вручную или найдите книгу в списке на руках.' : undefined,
  }
})
function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); emit('close'); return }
  if (event.key !== 'Tab') return
  const buttons = (event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('button')
  const first = buttons[0], last = buttons[buttons.length - 1]
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus() }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus() }
}
onMounted(() => closeButton.value?.focus())
</script>

<template>
  <div class="modal-backdrop help-backdrop" :class="{ 'identify-help-backdrop': screen === 'identify' }" @click.self="emit('close')">
    <section class="modal context-help" :class="{ 'identify-help': screen === 'identify' }" role="dialog" aria-modal="true" aria-labelledby="help-heading" @keydown="onKey">
      <div class="modal-header"><span v-if="screen === 'identify'" class="help-info" aria-hidden="true">i</span><h2 id="help-heading">{{ content.title }}</h2><button ref="closeButton" class="btn" aria-label="Закрыть помощь" @click="emit('close')"><X /></button></div>
      <ol class="help-steps">
        <li v-for="(step, index) in content.steps" :key="index"><span class="help-number" aria-hidden="true">{{ index + 1 }}</span><p><strong v-if="'label' in step">{{ step.label }}</strong>{{ step.text }}</p></li>
      </ol>
      <div v-if="screen === 'identify'" class="identify-help-note"><Keyboard :stroke-width="1.6" /><p>Нет карты?<br />Нажмите «Найти вручную».</p></div>
      <p v-else-if="content.note" class="help-note">{{ content.note }}</p>
      <button v-if="screen !== 'identify'" class="btn primary help-done" @click="emit('close')">Понятно</button>
    </section>
  </div>
</template>

<style scoped>
.help-backdrop{z-index:200}
.context-help{width:min(800px,100%);gap:24px}
.help-steps{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:22px}
.help-steps li{display:flex;align-items:flex-start;gap:18px}
.help-number{display:grid;place-items:center;flex:0 0 40px;height:40px;border-radius:12px;background:var(--blue-soft);color:var(--edus-blue);font-size:20px;font-weight:650}
.help-steps p{font-size:22px;line-height:1.5;color:var(--edus-navy)}
.help-steps strong{display:block;font-size:22px;margin-bottom:4px}
.help-note{font-size:18px;line-height:1.5;background:var(--blue-soft);padding:16px 20px;border-radius:12px}
.help-done{align-self:stretch;flex-shrink:0}
@media(max-height:820px){.context-help{gap:20px}.help-steps{gap:18px}}
@media(max-width:600px){.context-help{padding:20px}.help-steps p,.help-steps strong{font-size:20px}.help-steps li{gap:12px}.help-number{flex-basis:32px;height:32px}.help-note{padding:14px}.help-steps{gap:16px}}
.identify-help-backdrop { background: transparent; align-items: flex-start; justify-content: flex-end; padding: 234px 32px 24px; }
.identify-help { width: 350px; padding: 22px; gap: 22px; border: 1px solid var(--line); border-radius: 16px; background: var(--background); box-shadow: none; }
.identify-help .modal-header { min-height: 38px; gap: 14px; padding-right: 20px; justify-content: flex-start; }
.identify-help .modal-header h2 { font-size: 22px; line-height: 1.3; font-weight: 550; letter-spacing: -.5px; color: var(--edus-blue); white-space: nowrap; }
.identify-help .modal-header button { position: absolute; right: 4px; top: 4px; border: 0; color: var(--muted); background: transparent; }
.help-info { width: 38px; height: 38px; flex: 0 0 38px; display: grid; place-items: center; font-size: 24px; line-height: 1; border-radius: 50%; background: var(--edus-blue); color: white; }
.identify-help .help-steps { gap: 26px; }
.identify-help .help-steps li { gap: 22px; }
.identify-help .help-number { flex-basis: 40px; height: 40px; border-radius: 50%; border: 1px solid var(--line); font-size: 22px; font-weight: 400; }
.identify-help .help-steps p { font-size: 20px; line-height: 1.4; color: var(--muted); }
.identify-help-note { display: flex; align-items: flex-start; gap: 20px; padding-top: 20px; border-top: 1px solid var(--line); margin-top: 6px; }
.identify-help-note svg { width: 44px; height: 34px; flex: 0 0 44px; color: var(--muted); }
.identify-help-note p { font-size: 18px; line-height: 1.45; letter-spacing: -.4px; white-space: nowrap; }
@media (max-height: 950px) and (min-width: 1250px) { .identify-help-backdrop { padding-top: 210px; } }
@media (max-height: 820px) and (min-width: 1250px) { .identify-help-backdrop { padding-top: 188px; } .identify-help { gap: 20px; } .identify-help .help-steps { gap: 22px; } }
@media (max-width: 1249px) { .identify-help-backdrop { background: #0B1A4D45; align-items: center; justify-content: center; padding: 24px; } .identify-help { width: min(440px, 100%); max-height: calc(100dvh - 48px); } }
@media (max-width: 420px) { .identify-help-backdrop { padding: 12px; } .identify-help { padding: 22px 18px; } .identify-help .modal-header { gap: 10px; } .identify-help .modal-header h2 { font-size: 21px; } .identify-help .help-steps li { gap: 16px; } .identify-help-note p { white-space: normal; } }
</style>
