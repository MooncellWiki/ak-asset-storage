<template>
  <span class="change-mark" :class="`is-${kind}`" :title="KIND_LABEL[kind]">{{
    KIND_LETTER[kind]
  }}</span>
</template>

<script setup lang="ts">
import { KIND_LABEL, KIND_LETTER } from "~/common/bundles";
import type { ChangeKind } from "~/common/bundles";

/** The A / M / D box of a change (colour plus letter, never colour alone). */
defineProps<{ kind: ChangeKind }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.change-mark {
  @include kind-colors;

  flex: none;
  display: inline-grid;
  place-items: center;
  width: 20px;
  height: 20px;
  font: 700 11px/1 var(--ak-font-label);
  color: var(--_c);
  background: var(--_bg);
  box-shadow: inset 0 0 0 1px var(--_c);
}
</style>
