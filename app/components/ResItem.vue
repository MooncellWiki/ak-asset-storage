<template>
  <li class="res-item">
    <RouterLink
      class="res-item__link"
      :class="{ 'is-selected': selected }"
      :to="to"
      :aria-current="selected ? 'true' : undefined"
    >
      <span
        v-if="$slots.thumb"
        class="res-item__thumb"
        :class="{ 'is-wide': thumb === 'wide', 'is-contain': contain }"
      >
        <slot name="thumb" />
      </span>
      <AsIcon v-else-if="icon" :name="icon" :size="16" class="res-item__icon" />
      <span class="res-item__text">
        <span class="res-item__title" :title="title">{{ title }}</span>
        <span v-if="sub" class="res-item__sub">{{ sub }}</span>
      </span>
      <span v-if="count" class="res-item__count">{{ count }}</span>
    </RouterLink>
  </li>
</template>

<script setup lang="ts">
import AsIcon from "./AsIcon.vue";
import type { IconName } from "./AsIcon.vue";
import type { RouteLocationRaw } from "vue-router";

/**
 * One row of a ResList: a thumbnail or an icon, a monospace name with an
 * optional second line, and a count / size at the right end.
 */
defineProps<{
  to: RouteLocationRaw;
  title: string;
  sub?: string;
  count?: string;
  selected?: boolean;
  /** Shown when there is no `thumb` slot */
  icon?: IconName;
  /** Thumbnail box: square 44px or wide 64×40 */
  thumb?: "square" | "wide";
  /** Fit images inside the box instead of covering it */
  contain?: boolean;
}>();
defineSlots<{ thumb?: () => unknown }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.res-item {
  margin: 0;

  &__link {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-left: 2px solid transparent;
    border-bottom: 1px solid var(--ak-border-subtle);
    color: var(--ak-fg-secondary);
    text-decoration: none;

    &:visited {
      color: var(--ak-fg-secondary);
    }

    &:hover {
      background: var(--ak-bg-hover);
      color: var(--ak-fg);
      text-decoration: none;
    }

    &.is-selected {
      background: var(--ak-accent-subtle);
      border-left-color: var(--ak-accent);
      color: var(--ak-fg);
    }
  }

  &__thumb {
    @include checker;

    flex: none;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    overflow: hidden;
    border: 1px solid var(--ak-border);

    &.is-wide {
      width: 64px;
      height: 40px;
    }

    :slotted(img) {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }

    &.is-contain :slotted(img) {
      object-fit: contain;
    }

    :slotted(.ak-icon) {
      color: var(--ak-fg-subtle);
    }
  }

  &__icon {
    color: var(--ak-fg-muted);
  }

  &__text {
    flex: 1;
    min-width: 0;
  }

  &__title,
  &__sub {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__title {
    @include mono(12px);
  }

  &__sub {
    font-size: 11px;
    color: var(--ak-fg-subtle);
  }

  &__count {
    flex: none;
    font-size: var(--ak-fs-sm);
    font-variant-numeric: tabular-nums;
    color: var(--ak-fg-muted);

    .is-selected > & {
      color: var(--ak-accent);
    }
  }
}
</style>
