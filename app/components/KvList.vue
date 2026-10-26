<template>
  <dl class="kv"><slot /></dl>
</template>

<script setup lang="ts">
/** Key / value rows (`<dt>` + `<dd>` pairs in the slot). */
defineSlots<{ default?: () => unknown }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.kv {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  margin: 0;

  :slotted(dt),
  :slotted(dd) {
    margin: 0;
    padding: 10px 12px;
    border-bottom: 1px solid var(--ak-border-subtle);
    font-size: var(--ak-fs-sm);
  }

  :slotted(dt) {
    padding-left: var(--ak-space-4);
    color: var(--ak-fg-muted);
    white-space: nowrap;
  }

  :slotted(dd) {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding-right: var(--ak-space-4);
  }

  :slotted(:nth-last-child(-n + 2)) {
    border-bottom: 0;
  }

  @media (max-width: $bp-phone) {
    grid-template-columns: minmax(0, 1fr);

    :slotted(dt) {
      padding-bottom: 0;
      border-bottom: 0;
    }
  }
}
</style>
