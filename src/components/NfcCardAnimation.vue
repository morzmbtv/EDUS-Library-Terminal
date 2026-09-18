<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue'

type CardState = 'idle' | 'reading' | 'success' | 'error'

const props = withDefaults(defineProps<{ state?: CardState }>(), { state: 'idle' })
const uid = useId().replace(/[^a-zA-Z0-9_-]/g, '')
const clipId = `edus-card-clip-${uid}`
const trailId = `edus-card-trail-${uid}`
const edgeId = `edus-card-edge-${uid}`
const reducedMotion = ref(false)
const readingTime = ref(0)
const resultVisible = ref(false)
let frame = 0
let startedAt = 0
let resultTimer: ReturnType<typeof setTimeout> | undefined
let mediaQuery: ReturnType<typeof window.matchMedia> | undefined

const smooth = (value: number) => {
  const clamped = Math.min(1, Math.max(0, value))
  return clamped * clamped * (3 - 2 * clamped)
}
const scanProgress = () => reducedMotion.value ? .5 : smooth((readingTime.value % 2100) / 2100)
const scanOpacity = () => {
  if (props.state !== 'reading') return 0
  if (reducedMotion.value) return .65
  const phase = (readingTime.value % 2100) / 2100
  return Math.min(smooth(phase / .09), smooth((1 - phase) / .09)) * .85
}
const scanTransform = () => `translate(0 ${65 + 339 * scanProgress()})`
const waveOpacity = (index: number) => {
  if (props.state !== 'reading') return 0
  if (reducedMotion.value) return .3
  const phase = ((readingTime.value - index * 140) % 1300 + 1300) % 1300 / 1300
  return Math.pow(Math.sin(phase * Math.PI), 4) * .52
}

function stopReading() {
  window.cancelAnimationFrame(frame)
  frame = 0
  startedAt = 0
  readingTime.value = 0
}
function draw(timestamp: number) {
  if (!startedAt) startedAt = timestamp
  readingTime.value = timestamp - startedAt
  frame = window.requestAnimationFrame(draw)
}
function updateState() {
  stopReading()
  clearTimeout(resultTimer)
  resultVisible.value = false
  if (props.state === 'reading' && !reducedMotion.value) frame = window.requestAnimationFrame(draw)
  if (props.state === 'success' || props.state === 'error') {
    // Source result appearance, without a permanent frame loop after it becomes static.
    resultTimer = setTimeout(() => { resultVisible.value = true }, reducedMotion.value ? 0 : 16)
  }
}
function onMotionChange(event: { matches: boolean }) {
  reducedMotion.value = event.matches
  updateState()
}

watch(() => props.state, updateState, { flush: 'sync' })
onMounted(() => {
  mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  reducedMotion.value = mediaQuery.matches
  nextTick(updateState)
  mediaQuery.addEventListener('change', onMotionChange)
})
onBeforeUnmount(() => {
  stopReading()
  clearTimeout(resultTimer)
  mediaQuery?.removeEventListener('change', onMotionChange)
})
</script>

<template>
  <div class="nfc-animation" :class="`state-${state}`" role="img" :aria-label="state === 'success' ? 'Карта успешно прочитана' : state === 'error' ? 'Карта не найдена. Попробуйте ещё раз.' : state === 'reading' ? 'Определяем читателя' : 'Ожидание карты'">
    <svg class="card-scene" viewBox="54 50 532 370" fill="none" aria-hidden="true">
      <defs>
        <clipPath :id="clipId"><rect x="219" y="57" width="202" height="356" rx="19" /></clipPath>
        <linearGradient :id="trailId" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#046BC8" stop-opacity="0" /><stop offset="1" stop-color="#046BC8" stop-opacity=".085" /></linearGradient>
        <linearGradient :id="edgeId" gradientUnits="userSpaceOnUse" x1="226" y1="0" x2="414" y2="0"><stop stop-color="#046BC8" stop-opacity="0" /><stop offset=".15" stop-color="#046BC8" stop-opacity=".82" /><stop offset=".85" stop-color="#046BC8" stop-opacity=".82" /><stop offset="1" stop-color="#046BC8" stop-opacity="0" /></linearGradient>
        <path :id="`ornament-half-${uid}`" d="M0 10 C-6 -2 -28 -3 -29 13 C-31 26 -16 34 -10 24 C-6 18 -11 10 -17 13 C-22 15 -19 22 -15 20 M-24 29 C-35 33 -32 49 -20 50 C-10 51 -7 40 -14 37 C-20 34 -24 42 -18 44 M-25 49 C-26 60 -17 66 -12 75 L-26 97 C-33 110 -28 123 -16 123 C-6 123 -4 113 -10 109 C-15 106 -21 109 -18 114 M-12 76 L-3 60 M-8 93 C-4 90 0 91 0 95 L0 110 M0 109 C-3 101 -10 101 -13 104" />
        <g :id="`ornament-${uid}`" fill="none" stroke="#D2E3F7" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"><use :href="`#ornament-half-${uid}`" /><use :href="`#ornament-half-${uid}`" transform="scale(-1 1)" /><path d="M0 38 L4 46 L0 56 L-4 46 Z" stroke-width="3" /></g>
        <path :id="`wave-small-${uid}`" d="M181 190 Q154 235 181 280" /><path :id="`wave-medium-${uid}`" d="M158 164 Q112 235 158 306" /><path :id="`wave-large-${uid}`" d="M134 130 Q54 235 134 340" />
      </defs>

      <g fill="none" stroke="#C4DFFA" stroke-width="4.4" stroke-linecap="round">
        <use :href="`#wave-small-${uid}`" /><use :href="`#wave-medium-${uid}`" /><use :href="`#wave-large-${uid}`" />
        <g transform="translate(640 0) scale(-1 1)"><use :href="`#wave-small-${uid}`" /><use :href="`#wave-medium-${uid}`" /><use :href="`#wave-large-${uid}`" /></g>
      </g>
      <g fill="none" stroke="#046BC8" stroke-width="4.4" stroke-linecap="round">
        <g :opacity="waveOpacity(0)"><use :href="`#wave-small-${uid}`" /><use :href="`#wave-small-${uid}`" transform="translate(640 0) scale(-1 1)" /></g>
        <g :opacity="waveOpacity(1)"><use :href="`#wave-medium-${uid}`" /><use :href="`#wave-medium-${uid}`" transform="translate(640 0) scale(-1 1)" /></g>
        <g :opacity="waveOpacity(2)"><use :href="`#wave-large-${uid}`" /><use :href="`#wave-large-${uid}`" transform="translate(640 0) scale(-1 1)" /></g>
      </g>

      <rect class="edus-card-outline" x="212" y="50" width="216" height="370" rx="26" fill="#FFFFFF" stroke="#046BC8" stroke-width="1.7" />
      <rect x="218" y="56" width="204" height="358" rx="20" fill="none" stroke="#046BC8" stroke-width=".85" />
      <g :clip-path="`url(#${clipId})`">
        <use :href="`#ornament-${uid}`" transform="translate(320 32)" /><use :href="`#ornament-${uid}`" transform="translate(320 300)" />
        <rect x="280" y="168" width="80" height="80" rx="15" fill="#E39300" />
        <path d="M316 183 H298 V230 H316 M321 191 H341 V238 H321" fill="none" stroke="#FFFFFF" stroke-width="6" stroke-linecap="square" stroke-linejoin="miter" />
        <image x="257" y="263" width="126" height="28.87" href="/assets/edus-logo.png" preserveAspectRatio="xMidYMid meet" />
        <g class="reading-line" :opacity="scanOpacity()" :transform="scanTransform()"><rect x="220" y="-28" width="200" height="28" :fill="`url(#${trailId})`" /><path d="M226 0 H414" :stroke="`url(#${edgeId})`" stroke-width="1.8" /></g>
      </g>

      <rect x="212" y="50" width="216" height="370" rx="26" fill="none" :stroke="state === 'success' ? '#188B59' : '#BD4343'" stroke-width="1.7" :opacity="(state === 'success' || state === 'error') && resultVisible ? .8 : 0" class="result-outline" />
      <g v-if="state === 'success'" class="result-mark state-mark success" :class="{ visible: resultVisible }"><circle cx="432" cy="64" r="28" fill="#F8FBFE" /><circle cx="432" cy="64" r="23" fill="#188B59" /><path d="M421 64 L429 72 L444 55" fill="none" stroke="#FFFFFF" stroke-width="3.1" stroke-linecap="round" stroke-linejoin="round" pathLength="1" /></g>
      <g v-else-if="state === 'error'" class="result-mark state-mark error" :class="{ visible: resultVisible }"><circle cx="432" cy="64" r="28" fill="#F8FBFE" /><circle cx="432" cy="64" r="23" fill="#BD4343" /><path d="M432 53 V65" fill="none" stroke="#FFFFFF" stroke-width="3.1" stroke-linecap="round" /><circle cx="432" cy="74" r="1.8" fill="#FFFFFF" /></g>
    </svg>
  </div>
</template>

<style scoped>
.nfc-animation { width: calc(var(--card-height, 253px) * 532 / 370); height: var(--card-height, 253px); position: relative; flex: 0 0 auto; margin-inline: auto; isolation: isolate; }
.card-scene { display: block; width: 100%; height: 100%; overflow: visible; }
.result-outline { transition: opacity 200ms ease-out; }
.result-mark { opacity: 0; transition: opacity 200ms ease-out; }
.result-mark.visible { opacity: 1; }
.result-mark path { stroke-dasharray: 1; stroke-dashoffset: 1; transition: stroke-dashoffset 260ms 80ms ease-out; }
.result-mark.visible path { stroke-dashoffset: 0; }
@media (prefers-reduced-motion: reduce) { .result-outline, .result-mark, .result-mark path { transition: none; } }
</style>
