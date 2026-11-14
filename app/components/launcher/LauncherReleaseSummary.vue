<template>
  <details class="release-summary" :open="!compact">
    <summary class="release-summary__progress">
      <span class="release-summary__status">
        <span class="ak-sr-only">归档详情</span>
        <AkTag :variant="release.readyAt ? 'success' : 'info'" size="sm" dot>
          {{ release.readyAt ? "归档完成" : "归档中" }}
        </AkTag>
        <span class="release-summary__percent">{{ progress }}%</span>
        <AsIcon
          class="release-summary__caret"
          name="chevron-right"
          :size="12"
        />
      </span>
      <span
        class="ak-progress ak-progress--sm release-summary__bar"
        :class="{ 'ak-progress--success': release.readyAt }"
        :style="{ '--_v': `${progress}%` }"
        role="progressbar"
        aria-label="已归档文件比例"
        :aria-valuenow="progress"
      />
    </summary>
    <dl class="release-summary__stats">
      <dt>文件</dt>
      <dd>
        {{ formatCount(release.archivedFiles) }} /
        {{ formatCount(release.totalFiles) }}
      </dd>
      <dt>数据量</dt>
      <dd>
        {{ formatSize(release.archivedBytes) }} /
        {{ formatSize(release.totalBytes) }}
      </dd>
    </dl>
    <dl class="release-summary__meta">
      <dt>包版本</dt>
      <dd>{{ release.packageVersion }}</dd>
      <dt>游戏版本</dt>
      <dd>{{ release.gameVersion }}</dd>
      <dt>发现时间</dt>
      <dd>
        <time :datetime="release.discoveredAt">{{
          formatDateTime(release.discoveredAt)
        }}</time>
      </dd>
      <dt>清单 MD5</dt>
      <dd>
        <code>{{ release.manifestMd5 }}</code>
        <CopyButton :text="release.manifestMd5" label="复制清单 MD5" />
      </dd>
    </dl>
    <div class="release-summary__foot">
      <a class="ak-btn ak-btn--ghost ak-btn--sm" :href="manifestUrl">
        <AsIcon name="download" :size="16" />下载原始加密清单
      </a>
    </div>
  </details>
</template>

<script setup lang="ts">
import { AkTag } from "@mooncellwiki/prts-design-vue";
import { useMediaQuery } from "@vueuse/core";
import { computed } from "vue";
import { formatCount, formatDateTime, formatSize } from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import CopyButton from "../CopyButton.vue";
import type { components } from "~/common/schema";

const props = defineProps<{
  release: components["schemas"]["LauncherRelease"];
  manifestUrl: string;
}>();
const compact = useMediaQuery("(max-width: 1119px)");
const progress = computed(() => {
  const { archivedFiles, totalFiles, readyAt } = props.release;
  if (!totalFiles) return readyAt ? 100 : 0;
  return Math.floor((archivedFiles / totalFiles) * 10_000) / 100;
});
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.release-summary {
  font-size: var(--ak-fs-sm);

  &__progress,
  &__stats,
  &__meta {
    padding: var(--ak-space-3);
  }

  &__progress {
    display: block;
    list-style: none;
    cursor: pointer;

    &::-webkit-details-marker {
      display: none;
    }
  }

  &__status {
    display: flex;
    align-items: center;
    gap: var(--ak-space-2);
    margin-bottom: var(--ak-space-2);
  }

  &__percent {
    @include num;

    margin-left: auto;
    font-weight: 700;
  }

  &__bar {
    display: block;
  }

  &__caret {
    color: var(--ak-fg-muted);
  }

  &[open] &__caret {
    transform: rotate(90deg);
  }

  &__stats,
  &__meta {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--ak-space-2);
    align-items: baseline;
    margin: 0;

    dt {
      color: var(--ak-fg-muted);
    }

    dd {
      margin: 0;
      font-variant-numeric: tabular-nums;
    }
  }

  &__stats {
    padding-top: 0;

    dd {
      text-align: right;
    }
  }

  &__meta {
    border-top: 1px solid var(--ak-border);

    dd:last-child {
      grid-column: 1 / -1;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--ak-space-2);
    }

    code {
      @include mono(11px);

      min-width: 0;
      overflow-wrap: anywhere;
    }
  }

  &__foot {
    padding: var(--ak-space-2) var(--ak-space-3);
    border-top: 1px solid var(--ak-border);
  }
}
</style>
