<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef, type Component } from 'vue';
import SystemHeader from '../shared/ui/SystemHeader.vue';
import { api, compatibleVersion } from '../shared/api/LocalServiceApiClient.ts';
import { hydrateLocale, t } from '../shared/i18n/index.ts';
const app = shallowRef<Component | null>(null),
  loading = ref(false),
  error = ref(''),
  incompatible = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined,
  stopped = false;
async function initialize() {
  if (loading.value || stopped) return;
  loading.value = true;
  error.value = '';
  try {
    const response = await fetch('/health', {
      credentials: 'same-origin',
      cache: 'no-store',
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) throw new Error();
    const status = (await response.json()) as { ready: boolean; state: string; application: string };
    if (status.application !== 'EDUSLibraryService') throw new Error();
    const version = await api.getVersion();
    if (!compatibleVersion(version)) {
      incompatible.value = true;
      error.value = t('Версии компонентов EDUS несовместимы. Требуется обновление.');
      return;
    }
    if (!status.ready) {
      error.value =
        status.state === 'SETUP_REQUIRED'
          ? t('Рабочее пространство не настроено. Откройте EDUS Terminal Configurator.')
          : t('Служба запускается. Повторяем проверку…');
      return;
    }
    await hydrateLocale();
    app.value = (await import('./App.vue')).default;
  } catch {
    error.value = t('Локальная служба EDUS недоступна');
  } finally {
    loading.value = false;
    if (!app.value && !incompatible.value && !stopped) timer = setTimeout(initialize, 5000);
  }
}
onMounted(initialize);
onUnmounted(() => {
  stopped = true;
  clearTimeout(timer);
});
</script>
<template>
  <component :is="app" v-if="app" />
  <div v-else class="boot-shell">
    <SystemHeader
      @help="error = t('Проверьте установку Local Backend. Данные не сбрасываются автоматически.')"
    />
    <main>
      <h1>{{ t('Библиотечный терминал') }}</h1>
      <p v-if="loading" role="status">{{ t('Открываем приложение…') }}</p>
      <p v-else role="alert">{{ error }}</p>
      <button class="btn secondary" :disabled="loading" @click="initialize">
        {{ t('Повторить проверку') }}
      </button>
    </main>
  </div>
</template>
<style scoped>
.boot-shell {
  min-height: 100dvh;
  background: var(--background);
  color: var(--edus-navy);
}
main {
  max-width: 1000px;
  margin: auto;
  padding: 64px 32px;
}
h1 {
  font-size: 36px;
}
p {
  font-size: 22px;
  line-height: 1.5;
  max-width: 800px;
}
.btn {
  min-height: 64px;
}
</style>
