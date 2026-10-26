<template>
  <div class="changes">
    <table class="ak-table ak-table--compact">
      <thead>
        <tr>
          <th class="changes__mark"><span class="ak-sr-only">变更</span></th>
          <th>路径</th>
          <th class="changes__num">大小</th>
          <th class="changes__num">变化</th>
          <th><span class="ak-sr-only">操作</span></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="c in rows"
          :key="c.path"
          class="changes__row"
          :class="[`is-${c.kind}`, { 'is-selected': c.path === selected }]"
          :aria-selected="c.path === selected"
          @click="emit('select', c.path)"
        >
          <td class="changes__mark"><ChangeMark :kind="c.kind" /></td>
          <td class="changes__path" :title="c.path">
            <span>{{ splitPath(c.path).dir }}</span
            >{{ splitPath(c.path).name }}
          </td>
          <td class="changes__num">{{ formatSize(size(c)) }}</td>
          <td
            v-if="c.kind === 'chg'"
            class="changes__num changes__delta"
            :class="{ 'is-up': delta(c) > 0, 'is-down': delta(c) < 0 }"
          >
            {{ formatDelta(delta(c)) }}
          </td>
          <td v-else class="changes__num changes__dim">
            {{ KIND_LABEL[c.kind] }}
          </td>
          <td class="changes__num">
            <span class="changes__actions">
              <button
                type="button"
                class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                title="下载"
                aria-label="下载"
                @click.stop="save(c)"
              >
                <AsIcon name="download" :size="16" />
              </button>
            </span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { KIND_LABEL } from "~/common/bundles";
import {
  bundleFilename,
  download,
  formatDelta,
  formatSize,
  splitPath,
  storageUrl,
} from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import ChangeMark from "../ChangeMark.vue";
import type { Change } from "~/common/bundles";

/** Changed bundles: mark, two-line path (directory small, file name), size, size change. */
defineProps<{ rows: Change[]; selected?: string }>();
const emit = defineEmits<{ select: [path: string] }>();

function size(c: Change) {
  return (c.new ?? c.old)!.fileSize;
}
function delta(c: Change) {
  return c.new!.fileSize - c.old!.fileSize;
}
function save(c: Change) {
  const b = (c.new ?? c.old)!;
  download(storageUrl(b.fileHash), bundleFilename(b));
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.changes {
  overflow-x: auto;

  &__row {
    @include kind-colors;

    cursor: pointer;

    // 3px bar of the change colour at the start of the row.
    > td:first-child {
      box-shadow: inset 3px 0 var(--_c);
    }

    &.is-rem > td {
      color: var(--ak-fg-muted);
    }
  }

  &__mark {
    width: 28px;
  }

  &__path {
    @include mono;

    width: 100%;
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    > span {
      display: block;
      font-size: 11px;
      line-height: 1.3;
      color: var(--ak-fg-muted);
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  &__num {
    @include num;
  }

  &__dim {
    color: var(--ak-fg-muted);
  }

  &__delta {
    &.is-up {
      color: var(--ak-rt-vup, var(--ak-accent));
    }

    &.is-down {
      color: var(--ak-rt-vdown, var(--ak-warning));
    }
  }

  &__actions {
    @include row-actions;
  }
}
</style>
