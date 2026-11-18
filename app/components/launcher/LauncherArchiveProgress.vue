<template>
  <div class="archive-progress">
    <div class="archive-progress__tools">
      <div class="ak-seg" role="group" aria-label="归档状态">
        <button
          type="button"
          :aria-pressed="pendingOnly"
          @click="setFilter(true)"
        >
          未完成
        </button>
        <button
          type="button"
          :aria-pressed="!pendingOnly"
          @click="setFilter(false)"
        >
          全部
        </button>
      </div>
      <div
        v-if="cursors.length || page?.nextBefore != null"
        class="archive-progress__pages"
      >
        <button
          type="button"
          class="ak-btn ak-btn--sm"
          :disabled="!cursors.length"
          @click="cursors.pop()"
        >
          上一页
        </button>
        <button
          type="button"
          class="ak-btn ak-btn--sm"
          :disabled="page?.nextBefore == null"
          @click="nextPage"
        >
          下一页
        </button>
      </div>
    </div>

    <AkSkeleton
      v-if="isLoading"
      text
      :repeat="3"
      aria-label="正在加载归档进度"
    />
    <AkMessage v-else-if="error" variant="danger">
      {{ error instanceof Error ? error.message : "加载归档进度失败" }}
      <button type="button" class="ak-btn ak-btn--sm" @click="loadPage()">
        重试
      </button>
    </AkMessage>
    <div v-else-if="page?.releases.length" class="archive-progress__table">
      <table class="ak-table ak-table--compact" aria-label="各版本归档进度">
        <thead>
          <tr>
            <th scope="col">包版本</th>
            <th scope="col">文件进度</th>
            <th scope="col">已归档 / 总数据量</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="release in page.releases" :key="release.id">
            <td>
              <div class="archive-progress__version">
                {{ release.packageVersion }} <small>#{{ release.id }}</small>
              </div>
              <AkTag
                :variant="release.readyAt ? 'success' : 'info'"
                size="sm"
                dot
              >
                {{ release.readyAt ? "归档完成" : "归档中" }}
              </AkTag>
            </td>
            <td>
              <AkProgress
                size="sm"
                :variant="release.readyAt ? 'success' : 'accent'"
                :percentage="archivedFilePercent(release)"
                :label="`${formatCount(release.archivedFiles)} / ${formatCount(release.totalFiles)}`"
              />
            </td>
            <td>
              <span>{{ formatSize(release.archivedBytes) }}</span> /
              <span>{{ formatSize(release.totalBytes) }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else role="status" class="archive-progress__empty">
      {{
        cursors.length
          ? "本页已无更多发布"
          : pendingOnly
            ? "暂无未完成的归档"
            : "暂无 PC 发布"
      }}
    </p>
  </div>
</template>

<script setup lang="ts">
import {
  AkMessage,
  AkProgress,
  AkSkeleton,
  AkTag,
} from "@mooncellwiki/prts-design-vue";
import { useAsyncState } from "@vueuse/core";
import { computed, ref, watch } from "vue";
import { archivedFilePercent, fetchLauncherReleases } from "~/common/launcher";
import { formatCount, formatSize } from "~/common/utils";

const pendingOnly = ref(true);
const cursors = ref<number[]>([]);
const before = computed(() => cursors.value.at(-1));
const {
  state: page,
  error,
  isLoading,
  executeImmediate: loadPage,
} = useAsyncState(
  () =>
    fetchLauncherReleases({
      before: before.value,
      pending: pendingOnly.value,
      limit: 10,
    }),
  undefined,
);
watch([pendingOnly, before], () => void loadPage());

function setFilter(pending: boolean) {
  if (pendingOnly.value === pending) return;
  cursors.value = [];
  pendingOnly.value = pending;
}

function nextPage() {
  if (page.value?.nextBefore != null) cursors.value.push(page.value.nextBefore);
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.archive-progress {
  &__tools,
  &__pages {
    display: flex;
    align-items: center;
    gap: var(--ak-space-2);
  }

  &__tools {
    flex-wrap: wrap;
    margin-bottom: var(--ak-space-2);
  }

  &__pages {
    margin-inline-start: auto;
  }

  &__table {
    overflow-x: auto;
    font-variant-numeric: tabular-nums;

    td:last-child span {
      white-space: nowrap;
    }

    :deep(.ak-progress-label) {
      flex-wrap: wrap;
      gap: var(--ak-space-1);
    }
  }

  &__version {
    @include mono;

    overflow-wrap: anywhere;

    small {
      color: var(--ak-fg-muted);
    }
  }

  &__empty {
    margin: 0;
    color: var(--ak-fg-muted);
  }
}
</style>
