<template>
  <RouterLink v-if="to" class="page-tool" :to="to">
    <AsIcon :name="icon" :size="18" class="page-tool__icon" /><slot />
  </RouterLink>
  <button v-else type="button" class="page-tool" @click="$emit('click')">
    <AsIcon :name="icon" :size="18" class="page-tool__icon" /><slot />
  </button>
</template>

<script setup lang="ts">
import AsIcon from "./AsIcon.vue";
import type { IconName } from "./AsIcon.vue";
import type { RouteLocationRaw } from "vue-router";

/** A ghost action in the page header's tool cluster: a link with `to`, a button otherwise. */
defineProps<{ icon: IconName; to?: RouteLocationRaw }>();
defineEmits<{ click: [] }>();
defineSlots<{ default?: () => unknown }>();
</script>

<style scoped lang="scss">
.page-tool {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 10px 0 8px;
  box-sizing: border-box;
  border: 1px solid transparent;
  background: transparent;
  font: inherit;
  font-size: var(--ak-fs-sm);
  font-weight: 600;
  line-height: 1;
  color: var(--ak-fg-secondary);
  text-decoration: none;
  white-space: nowrap;
  cursor: pointer;
  transition:
    background var(--ak-dur-fast),
    color var(--ak-dur-fast);

  &:visited {
    color: var(--ak-fg-secondary);
  }

  &:hover {
    color: var(--ak-fg);
    background: var(--ak-bg-hover);
    text-decoration: none;
  }

  &:focus-visible {
    outline: 2px solid var(--ak-focus);
    outline-offset: -2px;
  }

  &__icon {
    color: var(--ak-fg-muted);
  }

  &:hover &__icon {
    color: var(--ak-fg);
  }
}
</style>
