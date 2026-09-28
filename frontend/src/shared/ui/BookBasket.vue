<script setup lang="ts">
import { t } from '../i18n/index.ts';
withDefaults(defineProps<{ title?: string; count: number }>(), { title: 'Книги в списке' });
</script>

<template>
  <section class="book-basket" :aria-label="title">
    <div class="basket-heading">
      <h2>{{ title }}</h2>
      <span class="basket-count" :aria-label="t('Количество книг')">{{ count }}</span
      ><slot name="action" />
    </div>
    <div class="basket-scroll" tabindex="0" :aria-label="t('Список книг')">
      <slot v-if="count > 0" />
      <div v-else class="basket-empty">
        <slot name="empty"
          ><p>{{ t('Добавленные книги появятся здесь') }}</p></slot
        >
      </div>
    </div>
  </section>
</template>

<style scoped>
.book-basket {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border: 1px solid var(--line, #dce6f0);
  border-radius: 18px;
  background: var(--surface, #fff);
}
.basket-heading {
  padding: 20px 24px;
  display: flex;
  align-items: center;
  gap: 12px;
  border-bottom: 1px solid var(--line, #dce6f0);
  min-height: 76px;
  flex: 0 0 auto;
}
.basket-heading h2 {
  font-size: 21px;
  font-weight: 600;
  line-height: 1.3;
  margin: 0;
}
.basket-count {
  min-width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  padding: 0 7px;
  border-radius: 8px;
  background: var(--blue-soft, #eaf3fd);
  color: var(--edus-blue, #046bc8);
  font-size: 17px;
  font-weight: 600;
}
.basket-scroll {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: #bdccdc transparent;
  border-radius: 0 0 17px 17px;
}
.basket-scroll:focus-visible {
  outline: 3px solid var(--edus-blue, #046bc8);
  outline-offset: -3px;
}
.basket-empty {
  min-height: 140px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  color: var(--muted, #5e6b82);
  text-align: center;
  font-size: 19px;
  line-height: 1.5;
}
.basket-empty p {
  margin: 0;
}
@media (max-width: 700px) {
  .basket-heading {
    padding: 16px;
  }
}
</style>
