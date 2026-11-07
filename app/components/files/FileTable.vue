<template>
  <div class="files">
    <table class="ak-table ak-table--compact">
      <thead>
        <tr>
          <th :aria-sort="sort === 'name' ? 'ascending' : undefined">
            <button
              type="button"
              class="ak-table__sort"
              @click="emit('sort', 'name')"
            >
              名称
            </button>
          </th>
          <th
            class="files__num"
            :aria-sort="sort === 'size' ? 'descending' : undefined"
          >
            <button
              type="button"
              class="ak-table__sort"
              @click="emit('sort', 'size')"
            >
              大小
            </button>
          </th>
          <th :aria-sort="sort === 'mtime' ? 'descending' : undefined">
            <button
              type="button"
              class="ak-table__sort"
              @click="emit('sort', 'mtime')"
            >
              修改时间
            </button>
          </th>
          <th><span class="ak-sr-only">操作</span></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="e in entries"
          :key="e.path"
          class="files__row"
          @click="emit('open', e)"
        >
          <td>
            <RouterLink class="files__name" :to="linkFor(e)" @click.stop>
              <AsIcon :name="entryIcon(e)" :size="16" />
              <span>{{ e.name }}</span>
            </RouterLink>
          </td>
          <td class="files__num">{{ e.is_dir ? "" : formatSize(e.size) }}</td>
          <td class="files__time">{{ formatDateTime(e.modified_at) }}</td>
          <td class="files__num">
            <span v-if="!e.is_dir" class="files__actions">
              <button
                type="button"
                class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                title="复制直链"
                aria-label="复制直链"
                @click.stop="copy(absoluteUrl(assetUrl(e.path)), '已复制直链')"
              >
                <AsIcon name="link" :size="16" />
              </button>
              <a
                class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                :href="assetUrl(e.path)"
                :download="e.name"
                title="下载"
                aria-label="下载"
                @click.stop
              >
                <AsIcon name="download" :size="16" />
              </a>
            </span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { useCopy } from "~/common/clipboard";
import {
  absoluteUrl,
  assetUrl,
  formatDateTime,
  formatSize,
} from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import { entryIcon } from "./entries";
import type { AssetEntry, SortKey } from "./entries";

/** Directory as a list: name, size, modified time; headers sort, rows link. */
defineProps<{
  entries: AssetEntry[];
  sort: SortKey;
  linkFor: (e: AssetEntry) => object;
}>();
const emit = defineEmits<{ sort: [key: SortKey]; open: [e: AssetEntry] }>();

const copy = useCopy();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.files {
  overflow-x: auto;

  &__row {
    cursor: pointer;
  }

  &__name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: inherit;
    text-decoration: none;

    &:visited {
      color: inherit;
    }

    > span {
      @include mono;
    }

    .ak-icon {
      color: var(--ak-fg-muted);
    }
  }

  &__num {
    @include num;
  }

  &__time {
    white-space: nowrap;
    color: var(--ak-fg-muted);
  }

  &__actions {
    @include row-actions;
  }
}
</style>
