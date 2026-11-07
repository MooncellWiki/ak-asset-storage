<template>
  <svg
    class="ak-icon as-icon"
    :style="size ? { width: `${size}px`, height: `${size}px` } : undefined"
    viewBox="0 0 24 24"
    focusable="false"
    :role="label ? 'img' : undefined"
    :aria-label="label"
    :aria-hidden="label ? undefined : 'true'"
    v-html="markup"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  name: IconName;
  label?: string;
  /** Edge length in px; 1em when absent */
  size?: number;
}>();

/*
 * Line icons (24×24, currentColor) from the PRTS Design ak-asset-storage
 * mockups (preview/_src/app-skeleton.html). Decorative unless `label` is set.
 */
const ICONS = {
  search:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M10.5 3a7.5 7.5 0 1 1 0 15 7.5 7.5 0 0 1 0-15zM16 16l5 5"/>',
  sun: '<path fill="none" stroke="currentColor" stroke-width="2" d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm0-6v3m0 14v3M2 12h3m14 0h3M4.9 4.9l2.1 2.1m10 10 2.1 2.1M4.9 19.1 7 17m10-10 2.1-2.1"/>',
  moon: '<path fill="none" stroke="currentColor" stroke-width="2" d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/>',
  monitor:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 4h18v12H3zm5 16h8m-4-4v4"/>',
  more: '<path fill="currentColor" d="M5 10a2 2 0 1 1 0 4 2 2 0 0 1 0-4zm7 0a2 2 0 1 1 0 4 2 2 0 0 1 0-4zm7 0a2 2 0 1 1 0 4 2 2 0 0 1 0-4z"/>',
  link: '<path fill="none" stroke="currentColor" stroke-width="2" d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1.5-1.5"/>',
  list: '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 7h16M4 12h16M4 17h10"/>',
  download:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M12 4v12m-5-5 5 5 5-5M4 20h16"/>',
  grid: '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 4h6v6H4zm10 0h6v6h-6zM4 14h6v6H4zm10 0h6v6h-6z"/>',
  refresh:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5"/>',
  info: '<path fill="none" stroke="currentColor" stroke-width="2" d="M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18zm0 8v5m0-9v1"/>',
  warn: '<path fill="none" stroke="currentColor" stroke-width="2" d="M12 3l10 18H2zm0 7v5m0 3v1"/>',
  check:
    '<path fill="none" stroke="currentColor" stroke-width="2.5" d="M4 12l5 5L20 6"/>',
  x: '<path fill="none" stroke="currentColor" stroke-width="2.5" d="M5 5l14 14M19 5L5 19"/>',
  "chevron-left":
    '<path fill="none" stroke="currentColor" stroke-width="2.5" d="M15 5l-7 7 7 7"/>',
  "chevron-right":
    '<path fill="none" stroke="currentColor" stroke-width="2.5" d="M9 5l7 7-7 7"/>',
  copy: '<path fill="none" stroke="currentColor" stroke-width="2" d="M9 9h11v11H9zM4 15V4h11"/>',
  swap: '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 8h14m-4-4 4 4-4 4M20 16H6m4 4-4-4 4-4"/>',
  folder:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 5h6l2 2h10v12H3z"/>',
  "folder-open":
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 5h6l2 2h9v3M3 5v14h16l2.5-9H6.5L3 19"/>',
  file: '<path fill="none" stroke="currentColor" stroke-width="2" d="M6 3h8l4 4v14H6zm8 0v4h4"/>',
  image:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 5h18v14H3zm3 11 4-5 3 3 2-2 4 4M15 9h.01"/>',
  audio:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 10v4h3l5 4V6L7 10zm11-1a4 4 0 0 1 0 6m2.5-9a8 8 0 0 1 0 12"/>',
  json: '<path fill="none" stroke="currentColor" stroke-width="2" d="M8 4c-2 0-3 1-3 3v2c0 1.5-1 3-2 3 1 0 2 1.5 2 3v2c0 2 1 3 3 3m8-16c2 0 3 1 3 3v2c0 1.5 1 3 2 3-1 0-2 1.5-2 3v2c0 2-1 3-3 3"/>',
  bundle:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 7l9-4 9 4v10l-9 4-9-4zm9 4 9-4M12 11 3 7m9 4v10M7.5 5l9 4"/>',
  diff: '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 4h7v7H4zm9 9h7v7h-7zM7.5 6v3m-1.5-1.5h3M14.5 16.5h3"/>',
  history:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5m4-1v5l3 2"/>',
  docker:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 13h14l2-3h2c0 5-3 8-9 8s-9-3-9-5zm2-3h3v3H5zm4 0h3v3H9zm4 0h3v3h-3zM9 6h3v3H9z"/>',
  external:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M14 4h6v6m0-6L10 14M20 14v6H4V4h6"/>',
  filter:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M3 5h18l-7 8v6l-4 2v-8z"/>',
  book: '<path fill="none" stroke="currentColor" stroke-width="2" d="M4 4h7a3 3 0 0 1 3 3v13a2 2 0 0 0-2-2H4zm16 0h-7a3 3 0 0 0-3 3v13a2 2 0 0 1 2-2h8z"/>',
  person:
    '<path fill="none" stroke="currentColor" stroke-width="2" d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm-8 9c0-4 3.6-6 8-6s8 2 8 6"/>',
  empty:
    '<path fill="none" stroke="currentColor" stroke-width="1.5" d="M3 7l9-4 9 4v10l-9 4-9-4zm9 4 9-4M12 11 3 7m9 4v10"/>',
} as const;

export type IconName = keyof typeof ICONS;

const markup = computed(() => ICONS[props.name]);
</script>

<style scoped lang="scss">
.as-icon {
  width: 1em;
  height: 1em;
  flex: none;
  vertical-align: -0.125em;
}
</style>
