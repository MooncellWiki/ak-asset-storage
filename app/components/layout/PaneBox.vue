<template>
  <component :is="tag ?? 'section'" class="pane" :aria-label="label">
    <div v-if="$slots.head || $slots['head-end']" class="pane__head">
      <slot name="head" />
      <div v-if="$slots['head-end']" class="pane__head-end">
        <slot name="head-end" />
      </div>
    </div>
    <slot />
    <div v-if="$slots.foot" class="pane__foot"><slot name="foot" /></div>
  </component>
</template>

<script setup lang="ts">
/**
 * A bordered surface (the mockups' `.as-pane`): optional head bar, flush
 * content, optional foot bar. `head-end` sits at the right end of the head.
 */
defineProps<{ label?: string; tag?: "section" | "aside" }>();
defineSlots<{
  default?: () => unknown;
  head?: () => unknown;
  "head-end"?: () => unknown;
  foot?: () => unknown;
}>();
</script>

<style scoped lang="scss">
.pane {
  min-width: 0;
  background: var(--ak-bg-surface);
  border: 1px solid var(--ak-border);

  & + & {
    margin-top: var(--ak-space-4);
  }

  // Tables sit edge to edge inside the pane.
  :slotted(:where(.ak-table)) {
    border: 0;
  }

  :slotted(:where(.ak-empty)) {
    margin: var(--ak-space-4);
  }

  &__head,
  &__foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ak-space-2) var(--ak-space-3);
    padding: var(--ak-space-3) var(--ak-space-4);
  }

  &__head {
    border-bottom: 1px solid var(--ak-border);

    :slotted(:where(.ak-search)) {
      width: min(100%, 240px);
    }
  }

  &__head-end {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ak-space-2) var(--ak-space-3);
    margin-left: auto;
  }

  &__foot {
    border-top: 1px solid var(--ak-border);
  }
}
</style>
