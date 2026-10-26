<template>
  <div class="page-header">
    <h1 class="page-header__title">
      {{ title }}<span class="ak-en page-header__en">{{ en }}</span>
    </h1>
    <div v-if="$slots.default" class="page-header__tools"><slot /></div>
  </div>
</template>

<script setup lang="ts">
import { watchEffect } from "vue";

/** Page title with the accent bar, English subtitle, and PageTool actions on the right. */
const props = defineProps<{
  title: string;
  /** English subtitle under the title */
  en: string;
  /** Browser tab title; defaults to the page title */
  documentTitle?: string;
}>();
defineSlots<{ default?: () => unknown }>();

watchEffect(() => {
  document.title = `${props.documentTitle ?? props.title} · Arknights Asset Storage`;
});
</script>

<style scoped lang="scss">
.page-header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--ak-space-3) var(--ak-space-4);
  margin-bottom: var(--ak-space-4);

  &__title {
    flex: 9999 1 auto;
    min-width: 0;
    margin: 0;
    padding: 0 0 0 var(--ak-space-4);
    border-left: var(--ak-bar-w-lg) solid var(--ak-accent);
    font-family: var(--ak-font-body);
    font-size: var(--ak-fs-h1);
    font-weight: 800;
    line-height: 1.2;
    letter-spacing: 0.01em;
    overflow-wrap: anywhere;
  }

  &__en {
    display: block;
    margin-top: 2px;
    font-size: 0.5em;
    color: var(--ak-fg-muted);
    letter-spacing: var(--ak-tracking-caps);
  }

  &__tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    margin-bottom: 2px;
  }
}
</style>
