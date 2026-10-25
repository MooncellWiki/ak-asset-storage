<template>
  <nav class="pathbar" aria-label="所在位置">
    <template v-for="(s, i) in segments" :key="i">
      <i v-if="i > 0">/</i>
      <b v-if="i === segments.length - 1" aria-current="page">{{ s.label }}</b>
      <RouterLink v-else-if="s.to" :to="s.to">{{ s.label }}</RouterLink>
      <span v-else>{{ s.label }}</span>
    </template>
    <CopyButton
      v-if="copy"
      class="pathbar__copy"
      :text="copy"
      :label="copyLabel ?? '复制路径'"
    />
  </nav>
</template>

<script setup lang="ts">
import CopyButton from "./CopyButton.vue";
import type { RouteLocationRaw } from "vue-router";

export interface PathSegment {
  label: string;
  /** Where the segment leads; the last segment is the current location. */
  to?: RouteLocationRaw;
}

/**
 * A GitHub-style path line: parents are links, the current item is bold,
 * an optional copy button at the end (instead of breadcrumb + title).
 */
defineProps<{
  segments: PathSegment[];
  /** Text copied by the trailing button; no button when absent. */
  copy?: string;
  copyLabel?: string;
}>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.pathbar {
  @include mono(15px);

  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px 6px;
  min-width: 0;
  line-height: 1.4;

  > a {
    color: var(--ak-link);
    text-decoration: none;
    overflow-wrap: anywhere;

    &:hover {
      text-decoration: underline;
    }
  }

  > i {
    font-style: normal;
    color: var(--ak-fg-subtle);
  }

  > b {
    font-weight: 700;
    color: var(--ak-fg);
    overflow-wrap: anywhere;
  }

  &__copy {
    margin-left: 2px;
  }
}
</style>
