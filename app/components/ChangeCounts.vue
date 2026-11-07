<template>
  <span class="change-counts">
    <span v-if="counts.add" class="is-add">+{{ counts.add }}</span>
    <span v-if="counts.chg" class="is-chg">~{{ counts.chg }}</span>
    <span v-if="counts.rem" class="is-rem">−{{ counts.rem }}</span>
  </span>
</template>

<script setup lang="ts">
import type { ChangeKind } from "~/common/bundles";

/** +added ~modified −removed, each in its change colour; zeros are left out. */
defineProps<{ counts: Record<ChangeKind, number> }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.change-counts {
  display: inline-flex;
  gap: 6px;
  font-variant-numeric: tabular-nums;

  > span {
    @include kind-colors;

    color: var(--_c);
    font-weight: 600;
  }
}
</style>
