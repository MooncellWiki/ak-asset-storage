<template>
  <div
    class="split"
    :class="{ 'split--narrow': narrow, 'split--detail': $slots.detail }"
  >
    <aside class="split__side" :aria-label="sideLabel">
      <div v-if="$slots['side-head']" class="split__side-head">
        <slot name="side-head" />
      </div>
      <div class="split__side-body"><slot name="side" /></div>
      <div v-if="$slots['side-foot']" class="split__side-foot">
        <slot name="side-foot" />
      </div>
    </aside>
    <div class="split__main"><slot /></div>
    <div v-if="$slots.detail" class="split__detail"><slot name="detail" /></div>
  </div>
</template>

<script setup lang="ts">
/**
 * Side pane + main column, optionally a detail column (the diff page).
 * The side pane sticks under the header and scrolls on its own; below
 * 1400px the detail column drops under the main one, below 1120px
 * everything stacks.
 */
defineProps<{
  sideLabel: string;
  /** Narrower side column, for the three-column diff layout */
  narrow?: boolean;
}>();
defineSlots<{
  default?: () => unknown;
  side?: () => unknown;
  "side-head"?: () => unknown;
  "side-foot"?: () => unknown;
  detail?: () => unknown;
}>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

$sticky-top: calc(var(--ak-header-h) + var(--ak-space-4));
$sticky-height: calc(100vh - var(--ak-header-h) - var(--ak-space-8));

.split {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: var(--ak-space-4);
  align-items: start;

  &--narrow {
    grid-template-columns: 240px minmax(0, 1fr);
  }

  &--detail {
    grid-template-columns: 240px minmax(0, 1fr) 360px;
  }

  &__side {
    position: sticky;
    top: $sticky-top;
    max-height: $sticky-height;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--ak-bg-surface);
    border: 1px solid var(--ak-border);
  }

  &__side-head,
  &__side-foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: var(--ak-space-2) var(--ak-space-3);

    :slotted(:where(.ak-search)) {
      flex: 1;
    }
  }

  &__side-head {
    border-bottom: 1px solid var(--ak-border);
  }

  &__side-foot {
    border-top: 1px solid var(--ak-border);
  }

  // A column so a FileTree (flex: 1, min-height: 0) scrolls itself and
  // virtualises; other content scrolls here.
  &__side-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
    padding: 4px 0;
  }

  &__main {
    min-width: 0;
  }

  &__detail {
    position: sticky;
    top: $sticky-top;
    max-height: $sticky-height;
    min-width: 0;
    overflow: auto;
  }

  @media (max-width: $bp-wide) {
    &--detail {
      grid-template-columns: 260px minmax(0, 1fr);
    }

    &__detail {
      grid-column: 1 / -1;
      position: static;
      max-height: none;
    }
  }

  @media (max-width: $bp-narrow) {
    &,
    &--narrow,
    &--detail {
      grid-template-columns: minmax(0, 1fr);
    }

    &__side {
      position: static;
      max-height: 360px;
    }
  }
}
</style>
