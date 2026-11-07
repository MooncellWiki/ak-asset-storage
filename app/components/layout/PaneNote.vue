<template>
  <div class="pane-note"><slot /></div>
</template>

<script setup lang="ts">
/** Padded content inside a pane: skeletons and one-line notes. */
defineSlots<{ default?: () => unknown }>();
</script>

<style scoped lang="scss">
.pane-note {
  padding: var(--ak-space-4);
  color: var(--ak-fg-muted);
  font-size: var(--ak-fs-sm);
}
</style>
