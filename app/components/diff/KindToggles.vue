<template>
  <div class="kinds" role="group" aria-label="变更类型">
    <button
      v-for="k in KINDS"
      :key="k"
      type="button"
      class="kinds__kind"
      :class="`is-${k}`"
      :aria-pressed="model[k]"
      @click="model = { ...model, [k]: !model[k] }"
    >
      {{ KIND_LABEL[k] }}
      <b>{{ counts ? formatCount(counts[k]) : "–" }}</b>
    </button>
  </div>
</template>

<script setup lang="ts">
import { KIND_LABEL } from "~/common/bundles";
import { formatCount } from "~/common/utils";
import type { ChangeKind } from "~/common/bundles";

/**
 * Added / modified / removed toggles. Each cell is its own change colour:
 * pressed = tinted with the colour, released = grey with the count struck out.
 */
defineProps<{ counts?: Record<ChangeKind, number> }>();
const model = defineModel<Record<ChangeKind, boolean>>({ required: true });

const KINDS: ChangeKind[] = ["add", "chg", "rem"];
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.kinds {
  display: inline-flex;
  border: 1px solid var(--ak-border-strong);

  &__kind {
    @include kind-colors;

    all: unset;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 12px;
    font-size: var(--ak-fs-xs);
    color: var(--ak-fg-muted);
    cursor: pointer;
    transition:
      background var(--ak-dur-fast),
      color var(--ak-dur-fast);

    & + & {
      border-left: 1px solid var(--ak-border);
    }

    &:hover {
      background: var(--ak-bg-hover);
      color: var(--ak-fg);
    }

    &:focus-visible {
      outline: 2px solid var(--ak-focus);
      outline-offset: -2px;
    }

    &[aria-pressed="true"] {
      background: var(--_bg);
      color: var(--_c);
      box-shadow: inset 0 -2px var(--_c);
    }

    > b {
      font-weight: 700;
      font-variant-numeric: tabular-nums;
    }

    &[aria-pressed="false"] > b {
      text-decoration: line-through;
    }
  }
}
</style>
