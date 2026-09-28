<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Camera, ScanFace, ShieldAlert } from '@lucide/vue';
import { createCameraSession } from '../lib/cameraSession.ts';
import { t } from '../i18n/index.ts';
const props = withDefaults(defineProps<{ cameraActive: boolean; terminalTest?: boolean }>(), {
  terminalTest: false,
});
defineEmits<{ card: [] }>();
const video = ref<HTMLVideoElement | null>(null),
  cameraStatus = ref(t('Камера выключена')),
  streaming = ref(false);
const session = createCameraSession(() =>
  navigator.mediaDevices.getUserMedia({
    video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 } },
    audio: false,
  }),
);
let mounted = false;
async function updateCamera() {
  session.stop();
  streaming.value = false;
  if (video.value) video.value.srcObject = null;
  if (!mounted || !props.cameraActive || document.hidden) {
    cameraStatus.value = t('Камера выключена');
    return;
  }
  if (!props.terminalTest) {
    cameraStatus.value = t('Провайдер распознавания не подключён');
    return;
  }
  cameraStatus.value = t('Открываем камеру…');
  try {
    const stream = await session.start();
    if (!stream || !mounted) return;
    if (video.value) {
      video.value.srcObject = stream;
      await video.value.play();
    }
    streaming.value = true;
    cameraStatus.value = 'UAT: камера работает. Распознавание не выполняется.';
  } catch {
    session.stop();
    cameraStatus.value = t('Камера недоступна. Проверьте подключение и разрешение Windows.');
  }
}
watch(
  () => [props.cameraActive, props.terminalTest],
  () => {
    if (mounted) void updateCamera();
  },
);
onMounted(() => {
  mounted = true;
  document.addEventListener('visibilitychange', updateCamera);
  void updateCamera();
});
onBeforeUnmount(() => {
  mounted = false;
  session.stop();
  document.removeEventListener('visibilitychange', updateCamera);
  if (video.value) video.value.srcObject = null;
});
</script>

<template>
  <section class="face-identification" aria-labelledby="face-identification-heading">
    <div class="face-copy">
      <h1 id="face-identification-heading">{{ t('Посмотрите в камеру') }}</h1>
      <p>
        {{
          terminalTest
            ? t('Тест камеры: изображение не сохраняется и не отправляется.')
            : `${t('Распознавание лица пока недоступно')}. ${t('Используйте карту.')}`
        }}
      </p>
    </div>
    <div class="face-frame" :class="{ active: cameraActive }" :aria-label="cameraStatus">
      <span class="face-corner top-left" /><span class="face-corner top-right" /><span
        class="face-corner bottom-left"
      /><span class="face-corner bottom-right" />
      <video
        v-if="terminalTest"
        v-show="streaming"
        ref="video"
        muted
        autoplay
        playsinline
        :aria-label="t('Предпросмотр камеры UAT')"
      />
      <ScanFace v-if="!streaming" :size="96" :stroke-width="1.4" />
    </div>
    <div class="face-status" role="status">
      <Camera :size="23" /><span>{{ cameraStatus }}</span>
    </div>
    <div class="face-unavailable" role="status">
      <ShieldAlert :size="26" /><span
        ><strong>{{ t('Распознавание лица пока недоступно') }}</strong
        >{{ t('Приложите карту ученика, чтобы продолжить операцию.') }}</span
      >
    </div>
    <button type="button" class="btn secondary face-card" @click="$emit('card')">{{ t('По карте') }}</button>
  </section>
</template>

<style scoped>
.face-frame video {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 24px;
}
.face-frame:has(video:not([style*='display: none']))::after {
  display: none;
}
.face-identification {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  min-height: 0;
  padding: clamp(16px, 3vh, 34px) 24px 28px;
  overflow-y: auto;
  text-align: center;
}
.face-copy {
  margin-top: 12px;
}
.face-copy h1 {
  margin: 0;
  color: var(--edus-navy);
  font-size: clamp(34px, 3vw, 46px);
  font-weight: 650;
  letter-spacing: -1.1px;
  line-height: 1.2;
}
.face-copy p {
  margin: 12px 0 0;
  max-width: 680px;
  color: var(--muted);
  font-size: 22px;
  line-height: 1.4;
}
.face-frame {
  position: relative;
  width: min(260px, 52vw);
  aspect-ratio: 1 / 1.18;
  display: grid;
  place-items: center;
  margin: clamp(20px, 4vh, 40px) 0 12px;
  color: var(--edus-blue);
  background: color-mix(in srgb, var(--blue-soft) 55%, white);
  border: 1px solid color-mix(in srgb, var(--edus-blue) 22%, white);
  border-radius: 24px;
}
.face-frame.active {
  border-color: color-mix(in srgb, var(--edus-blue) 50%, white);
}
.face-frame::after {
  content: '';
  position: absolute;
  width: 45%;
  height: 1px;
  background: color-mix(in srgb, var(--edus-blue) 32%, transparent);
}
.face-corner {
  position: absolute;
  width: 38px;
  height: 38px;
  border-color: var(--edus-blue);
  border-style: solid;
}
.top-left {
  top: 20px;
  left: 20px;
  border-width: 2px 0 0 2px;
  border-radius: 8px 0 0;
}
.top-right {
  top: 20px;
  right: 20px;
  border-width: 2px 2px 0 0;
  border-radius: 0 8px 0 0;
}
.bottom-left {
  bottom: 20px;
  left: 20px;
  border-width: 0 0 2px 2px;
  border-radius: 0 0 0 8px;
}
.bottom-right {
  right: 20px;
  bottom: 20px;
  border-width: 0 2px 2px 0;
  border-radius: 0 0 8px 0;
}
.face-status {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 40px;
  color: var(--muted);
  font-size: 17px;
}
.face-status svg {
  color: var(--edus-blue);
}
.face-unavailable {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  width: min(560px, 100%);
  margin-top: 8px;
  padding: 15px 18px;
  border: 1px solid #e8bf71;
  border-radius: 14px;
  background: #fff8eb;
  color: #704500;
  text-align: left;
}
.face-unavailable > svg {
  flex: 0 0 auto;
  margin-top: 2px;
}
.face-unavailable span {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 17px;
  line-height: 1.4;
}
.face-unavailable strong {
  font-size: 18px;
}
.face-card {
  width: min(560px, 100%);
  min-height: 68px;
  margin-top: 20px;
  font-size: 20px;
}
@media (max-height: 820px) {
  .face-identification {
    padding-top: 10px;
  }
  .face-copy {
    margin-top: 4px;
  }
  .face-copy h1 {
    font-size: 34px;
  }
  .face-copy p {
    margin-top: 8px;
    font-size: 19px;
  }
  .face-frame {
    width: 190px;
    margin: 14px 0 8px;
    border-radius: 20px;
  }
  .face-frame > svg {
    width: 76px;
    height: 76px;
  }
  .face-unavailable {
    padding-block: 10px;
  }
  .face-card {
    min-height: 60px;
    margin-top: 12px;
  }
}
@media (max-width: 600px) {
  .face-identification {
    padding-inline: 16px;
  }
  .face-copy p {
    font-size: 19px;
  }
  .face-frame {
    width: 220px;
  }
}
</style>
