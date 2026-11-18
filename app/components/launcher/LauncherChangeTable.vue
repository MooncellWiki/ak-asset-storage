<template>
  <table
    class="ak-table ak-table--compact launcher-changes"
    aria-label="版本差异表"
    role="table"
  >
    <thead class="launcher-changes__columns" role="rowgroup">
      <tr role="row">
        <th scope="col" class="launcher-changes__mark">
          <span class="ak-sr-only">标记</span>
        </th>
        <th scope="col">路径</th>
        <th scope="col" class="launcher-changes__delta">变化</th>
        <th scope="col" class="launcher-changes__version">基准版本</th>
        <th scope="col" class="launcher-changes__version">对比版本</th>
      </tr>
    </thead>
    <tbody role="rowgroup">
      <tr
        v-for="row in rows"
        :key="row.path"
        class="launcher-changes__row"
        :class="[`is-${row.kind}`, { 'is-selected': row.path === selected }]"
        role="row"
        @click="emit('select', row.path)"
      >
        <td class="launcher-changes__mark" role="cell">
          <ChangeMark :kind="row.kind" aria-hidden="true" />
        </td>
        <td class="launcher-changes__path" role="cell">
          <button
            type="button"
            class="launcher-changes__file"
            :aria-expanded="row.path === selected"
            @click.stop="emit('select', row.path)"
          >
            <LauncherPath :path="row.path" />
          </button>
          <span class="launcher-changes__change">
            <span v-if="row.delta != null" class="ak-sr-only">{{
              row.label
            }}</span>
            <span>{{
              row.delta != null ? formatDelta(row.delta) : row.label
            }}</span>
          </span>
        </td>
        <td class="launcher-changes__delta" role="cell">
          <span v-if="row.delta != null" class="ak-sr-only">{{
            row.label
          }}</span>
          <span>{{
            row.delta != null ? formatDelta(row.delta) : row.label
          }}</span>
        </td>
        <td
          v-for="side in row.sides"
          :key="side.label"
          class="launcher-changes__values"
          role="cell"
        >
          <span class="launcher-changes__side-label">{{ side.label }}</span>
          <span v-if="side.size != null" class="launcher-changes__size">{{
            formatSize(side.size)
          }}</span>
          <code v-if="side.md5" class="launcher-changes__hash">{{
            side.md5
          }}</code>
          <span
            v-if="side.size == null && !side.md5"
            class="launcher-changes__missing"
            >—</span
          >
        </td>
      </tr>
    </tbody>
  </table>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { KIND_LABEL } from "~/common/bundles";
import { formatDelta, formatSize } from "~/common/utils";
import ChangeMark from "../ChangeMark.vue";
import LauncherPath from "./LauncherPath.vue";
import type { ChangeKind } from "~/common/bundles";
import type { components } from "~/common/schema";

export type LauncherChange = components["schemas"]["LauncherDiff"] & {
  kind: ChangeKind;
};

const props = defineProps<{ changes: LauncherChange[]; selected?: string }>();
const emit = defineEmits<{ select: [path: string] }>();
const rows = computed(() =>
  props.changes.map((change) => {
    const { path, kind, oldMd5, newMd5, oldSize, newSize } = change;
    return {
      path,
      kind,
      label: KIND_LABEL[kind],
      delta:
        kind === "chg" && oldSize != null && newSize != null
          ? newSize - oldSize
          : undefined,
      sides: [
        { label: "基准版本", size: oldSize, md5: oldMd5 },
        { label: "对比版本", size: newSize, md5: newMd5 },
      ],
    };
  }),
);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-changes {
  table-layout: fixed;

  &__row {
    @include kind-colors;

    cursor: pointer;

    > td:first-child {
      box-shadow: inset 3px 0 var(--_c, transparent);
    }
  }

  &__mark {
    width: 2.25rem;
  }

  &__file {
    display: block;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: inherit;
    cursor: pointer;

    &:focus-visible {
      outline: 2px solid var(--ak-focus);
      outline-offset: 2px;
    }
  }

  &__delta {
    @include num;

    width: 6rem;
  }

  td#{&}__delta {
    color: var(--_c, var(--ak-fg-muted));
  }

  &__version {
    width: clamp(8rem, 23cqw, 14rem);
  }

  &__change,
  &__side-label {
    display: none;
  }

  &__size {
    font-variant-numeric: tabular-nums;
  }

  &__hash {
    @include mono(11px);

    display: block;
    overflow-wrap: anywhere;
    color: var(--ak-fg-muted);
  }

  &__missing {
    color: var(--ak-fg-muted);
  }

  @media (max-width: 759px) {
    &__columns {
      display: none;
    }

    &,
    tbody {
      display: block;
    }

    &__row {
      display: grid;
      grid-template-columns: max-content repeat(2, minmax(0, 1fr));
      border-bottom: 1px solid var(--ak-border);
      box-shadow: inset 3px 0 var(--_c, transparent);

      &:last-child {
        border-bottom: 0;
      }

      > td {
        border-bottom: 0;
      }

      > td:first-child {
        width: auto;
        padding-left: var(--ak-space-3);
        box-shadow: none;
      }
    }

    &__path {
      grid-column: 2 / -1;
    }

    &__delta {
      display: none;
    }

    &__values:nth-last-child(2) {
      grid-column: 2;
    }

    &__change,
    &__side-label {
      display: block;
      color: var(--ak-fg-muted);
      font-size: var(--ak-fs-xs);
    }

    &__hash {
      line-height: 1.3;
    }

    &__change {
      color: var(--_c, var(--ak-fg-muted));
    }
  }
}
</style>
