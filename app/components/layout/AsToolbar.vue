<template>
  <div class="toolbar">
    <slot />
    <div v-if="$slots.end" class="toolbar__end"><slot name="end" /></div>
  </div>
</template>

<script setup lang="ts">
/** The bar above a page's panes: controls on the left, `end` slot pushed right. */
defineSlots<{ default?: () => unknown; end?: () => unknown }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ak-space-2) var(--ak-space-3);
  margin-bottom: var(--ak-space-4);
  padding: var(--ak-space-3) var(--ak-space-4);
  background: var(--ak-bg-surface);
  border: 1px solid var(--ak-border);

  &__end {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ak-space-2);
    margin-left: auto;
  }

  :slotted(:where(.ak-search)) {
    width: min(100%, 320px);
  }
}
</style>
