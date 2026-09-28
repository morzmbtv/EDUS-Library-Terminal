<script setup lang="ts">
import { CircleHelp, Settings } from '@lucide/vue';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { localeTag, t } from '../i18n/index.ts';

const props = withDefaults(
  defineProps<{ time?: string; date?: string; terminalTest?: boolean; schoolName?: string }>(),
  {
    time: '',
    date: '',
    terminalTest: false,
    schoolName: 'EDUS Library',
  },
);
defineEmits<{ settings: []; help: [] }>();
const now = ref(new Date());
let timer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  timer = setInterval(() => {
    now.value = new Date();
  }, 1000);
});
onUnmounted(() => clearInterval(timer));
const shownTime = computed(
  () => props.time || now.value.toLocaleTimeString(localeTag.value, { hour: '2-digit', minute: '2-digit' }),
);
const shownDate = computed(
  () =>
    props.date ||
    now.value.toLocaleDateString(localeTag.value, { day: 'numeric', month: 'long', year: 'numeric' }),
);
const academicYear = computed(() => {
  const year = now.value.getFullYear() - (now.value.getMonth() < 8 ? 1 : 0);
  return `${year}–${year + 1}`;
});
</script>

<template>
  <div class="system-header">
    <div class="identity">
      <img class="brand" src="/assets/edus-logo.png" alt="EDUS" width="148" height="34" />
      <span class="brand-divider" aria-hidden="true" />
      <div class="product">
        <strong>{{ t('Библиотечный терминал') }}</strong
        ><span :title="schoolName">{{ schoolName }}</span>
      </div>
      <span v-if="terminalTest" class="test-mode-badge">{{ t('Тестовый режим') }}</span>
    </div>
    <div class="system-tools">
      <div class="operator">
        <span
          ><span class="operator-label">{{ t('Учебный год') }}</span
          >{{ academicYear }}</span
        >
      </div>
      <div class="header-clock">
        <time>{{ shownTime }}</time
        ><span class="header-date">{{ shownDate }}</span>
      </div>
      <button
        class="icon-button help"
        type="button"
        :aria-label="t('Помощь')"
        :title="t('Помощь')"
        @click="$emit('help')"
      >
        <CircleHelp :size="26" :stroke-width="1.8" />
      </button>
      <button
        class="icon-button"
        type="button"
        :aria-label="t('Настройки')"
        :title="t('Настройки')"
        @click="$emit('settings')"
      >
        <Settings :size="26" :stroke-width="1.8" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.system-header {
  min-height: 100px;
  padding: 16px 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  background: var(--surface, #fff);
  border-bottom: 1px solid var(--line, #dce6f0);
}
.identity,
.system-tools,
.operator {
  display: flex;
  align-items: center;
}
.identity {
  gap: 24px;
  min-width: 0;
}
.brand {
  width: 148px;
  height: auto;
  flex: 0 0 auto;
}
.brand-divider {
  height: 40px;
  width: 1px;
  background: var(--line, #dce6f0);
}
.product {
  min-width: 0;
  max-width: 340px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  white-space: nowrap;
}
.product strong {
  font-size: 23px;
  font-weight: 650;
  line-height: 1.25;
}
.product span {
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--muted, #5e6b82);
  font-size: 16px;
  line-height: 1.4;
}
.test-mode-badge {
  flex: 0 0 auto;
  border: 1px solid #d79a17;
  color: #7b4d00;
  background: #fff6e3;
  border-radius: 999px;
  padding: 7px 11px;
  font-size: 14px;
  font-weight: 700;
  white-space: nowrap;
}
.system-tools {
  gap: 16px;
  flex: 0 0 auto;
}
.operator {
  gap: 10px;
  font-size: 16px;
  line-height: 1.35;
  color: var(--muted, #5e6b82);
  padding-right: 16px;
  border-right: 1px solid var(--line, #dce6f0);
}
.operator svg {
  color: var(--edus-blue, #046bc8);
}
.operator-label {
  display: block;
  color: var(--edus-navy, #0b1a4d);
  font-size: 14px;
}
.header-clock {
  min-width: 126px;
  padding-right: 16px;
  border-right: 1px solid var(--line, #dce6f0);
}
.header-clock time {
  display: block;
  margin: 0;
  color: var(--edus-navy, #0b1a4d);
  font-size: 22px;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
  line-height: 1.15;
}
.header-date {
  display: block;
  font-size: 14px;
  line-height: 1.35;
  white-space: nowrap;
  color: var(--muted, #5e6b82);
}
.icon-button {
  width: 56px;
  height: 56px;
  border: 1px solid var(--line, #dce6f0);
  border-radius: 12px;
  background: var(--surface, #fff);
  color: var(--edus-navy, #0b1a4d);
  display: grid;
  place-items: center;
  cursor: pointer;
}
.icon-button:hover {
  background: var(--blue-soft, #eaf3fd);
  border-color: var(--edus-blue, #046bc8);
}
.icon-button:active {
  background: #d7e9fb;
}
.icon-button:focus-visible {
  outline: 3px solid var(--edus-blue, #046bc8);
  outline-offset: 3px;
}
@media (max-width: 1400px) {
  .system-header {
    min-height: 88px;
    padding: 12px 32px;
  }
  .identity {
    gap: 20px;
  }
  .brand {
    width: 132px;
  }
  .system-tools {
    gap: 12px;
  }
  .operator {
    padding-right: 12px;
  }
  .header-clock {
    padding-right: 12px;
  }
}
@media (max-width: 1100px) {
  .operator {
    display: none;
  }
}
@media (max-width: 900px) {
  .system-header {
    padding-inline: 24px;
  }
  .test-mode-badge {
    font-size: 12px;
    padding: 6px 8px;
  }
}
@media (max-width: 650px) {
  .system-header {
    padding: 12px;
    gap: 8px;
  }
  .identity {
    gap: 10px;
  }
  .brand {
    width: 78px;
  }
  .brand-divider {
    display: none;
  }
  .product strong {
    font-size: 18px;
  }
  .product span,
  .header-clock {
    display: none;
  }
  .system-tools {
    gap: 4px;
  }
}
@media (min-width: 901px) and (max-width: 1300px) and (max-height: 820px) {
  .system-header {
    min-height: 80px;
    padding: 8px 24px;
  }
  .brand {
    width: 132px;
  }
  .identity {
    gap: 16px;
  }
  .product strong {
    font-size: 22px;
  }
  .operator {
    padding-right: 10px;
  }
  .header-clock {
    min-width: 118px;
    padding-right: 10px;
  }
  .icon-button {
    width: 56px;
    height: 56px;
  }
}
</style>
