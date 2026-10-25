<template>
  <PaneBox tag="aside" label="详情">
    <template #head>
      <h2 class="detail__title">
        <ChangeMark v-if="kind" :kind="kind" />
        <span class="detail__path">{{ path }}</span>
      </h2>
    </template>
    <template #head-end>
      <CopyButton :text="path" label="复制路径" size="sm" />
      <button
        type="button"
        class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
        title="关闭"
        aria-label="关闭"
        @click="emit('close')"
      >
        <AsIcon name="x" :size="16" />
      </button>
    </template>

    <div class="compare">
      <div class="compare__col">
        <div class="compare__ver">{{ left.resVersion }}</div>
        <template v-if="before">
          <div class="compare__size">{{ formatSize(before.fileSize) }}</div>
          <div class="compare__hash" :title="before.fileHash">
            {{ before.fileHash.slice(0, 12) }}…
          </div>
          <button type="button" class="ak-btn ak-btn--sm" @click="save(before)">
            <AsIcon name="download" :size="16" />下载
          </button>
        </template>
        <div v-else class="compare__none">这一版没有</div>
      </div>
      <div class="compare__col is-new">
        <div class="compare__ver">{{ right.resVersion }}</div>
        <template v-if="after">
          <div class="compare__size">
            {{ formatSize(after.fileSize) }}
            <span
              v-if="delta !== undefined"
              class="compare__delta"
              :class="{ 'is-up': delta > 0, 'is-down': delta < 0 }"
            >
              {{ kind ? formatDelta(delta) : "未变化" }}
            </span>
          </div>
          <div class="compare__hash" :title="after.fileHash">
            {{ after.fileHash.slice(0, 12) }}…
          </div>
          <button
            type="button"
            class="ak-btn ak-btn--primary ak-btn--sm"
            @click="save(after)"
          >
            <AsIcon name="download" :size="16" />下载
          </button>
        </template>
        <div v-else class="compare__none">这一版没有</div>
      </div>
    </div>

    <h3 class="detail__sub">历次变化</h3>
    <PaneNote v-if="historyFailed">没拿到这个 bundle 的历史</PaneNote>
    <table v-else class="ak-table ak-table--compact">
      <tbody v-if="!history">
        <tr v-for="i in 3" :key="i">
          <td><AkSkeleton text :width="200" /></td>
          <td class="detail__num"><AkSkeleton text :width="60" /></td>
        </tr>
      </tbody>
      <tbody v-else>
        <tr
          v-for="b in shownHistory"
          :key="b.id"
          :class="{ 'is-selected': b.versionId === right.id }"
        >
          <td>
            <RouterLink
              class="detail__ver"
              :to="historyLink(b)"
              title="对比这一次变化"
            >
              {{ b.versionRes }}
            </RouterLink>
          </td>
          <td class="detail__num">{{ formatSize(b.fileSize) }}</td>
        </tr>
        <tr v-if="history.length === 0">
          <td class="detail__dim" colspan="2">没有记录</td>
        </tr>
      </tbody>
    </table>

    <template v-if="history && history.length > HISTORY_PREVIEW" #foot>
      <button
        type="button"
        class="ak-btn ak-btn--ghost ak-btn--sm"
        @click="showAll = !showAll"
      >
        {{ showAll ? "收起" : `全部 ${history.length} 次` }}
      </button>
    </template>
  </PaneBox>
</template>

<script setup lang="ts">
import { AkSkeleton } from "@mooncellwiki/prts-design-vue";
import { computed, ref, watch } from "vue";
import { loadBundleHistory } from "~/common/bundles";
import {
  bundleFilename,
  download,
  formatDelta,
  formatSize,
  storageUrl,
} from "~/common/utils";
import { diffLink, useVersions } from "~/common/versions";
import AsIcon from "../AsIcon.vue";
import ChangeMark from "../ChangeMark.vue";
import CopyButton from "../CopyButton.vue";
import PaneBox from "../layout/PaneBox.vue";
import PaneNote from "../layout/PaneNote.vue";
import type { Bundle, ChangeKind } from "~/common/bundles";
import type { Version } from "~/common/versions";

/**
 * The selected bundle: old and new side by side (version, size, hash,
 * download), then every version in which its content changed.
 */
const props = defineProps<{
  path: string;
  left: Version;
  right: Version;
  before?: Bundle;
  after?: Bundle;
  kind?: ChangeKind;
}>();
const emit = defineEmits<{ close: [] }>();

const { byId, previous } = useVersions();

const delta = computed(() =>
  props.before && props.after
    ? props.after.fileSize - props.before.fileSize
    : undefined,
);

function save(b: Bundle) {
  download(storageUrl(b.fileHash), bundleFilename(b));
}

const HISTORY_PREVIEW = 5;
const history = ref<Bundle[]>();
const historyFailed = ref(false);
const showAll = ref(false);
let request = 0;
watch(
  () => props.path,
  async (path) => {
    const seq = ++request;
    history.value = undefined;
    historyFailed.value = false;
    showAll.value = false;
    try {
      const rows = await loadBundleHistory(path);
      if (seq === request) history.value = rows;
    } catch {
      if (seq === request) historyFailed.value = true;
    }
  },
  { immediate: true },
);
const shownHistory = computed(() =>
  showAll.value ? history.value : history.value?.slice(0, HISTORY_PREVIEW),
);

function historyLink(b: Bundle) {
  const v = byId(b.versionId);
  return diffLink(previous(v), v, { bundle: props.path });
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.detail {
  &__title {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    font-size: var(--ak-fs-body);
    font-weight: 700;
  }

  &__path {
    @include mono;

    overflow-wrap: anywhere;
  }

  &__sub {
    margin: 0;
    padding: var(--ak-space-3) var(--ak-space-4) var(--ak-space-2);
    font-size: var(--ak-fs-sm);
    font-weight: 700;
  }

  &__ver {
    @include mono;

    font-weight: 600;
    color: var(--ak-fg);
    white-space: nowrap;
    text-decoration: none;

    &:visited {
      color: var(--ak-fg);
    }

    &:hover {
      color: var(--ak-link);
      text-decoration: underline;
    }
  }

  &__num {
    @include num;
  }

  &__dim {
    color: var(--ak-fg-muted);
  }
}

// Old / new columns; the new side is tinted.
.compare {
  display: grid;
  grid-template-columns: 1fr 1fr;
  border-bottom: 1px solid var(--ak-border);

  &__col {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
    padding: var(--ak-space-4);

    & + & {
      border-left: 1px solid var(--ak-border);
    }

    &.is-new {
      background: var(--ak-accent-subtle);
    }
  }

  &__ver {
    font: 600 12px/1.3 var(--ak-font-mono);
    color: var(--ak-fg-muted);
    overflow-wrap: anywhere;
  }

  &__size {
    font: 700 var(--ak-fs-h3) / 1.2 var(--ak-font-label);
    font-variant-numeric: tabular-nums;
  }

  &__delta {
    display: block;
    font-size: var(--ak-fs-sm);
    font-weight: 600;
    color: var(--ak-fg-muted);

    &.is-up {
      color: var(--ak-rt-vup, var(--ak-accent));
    }

    &.is-down {
      color: var(--ak-rt-vdown, var(--ak-warning));
    }
  }

  &__hash {
    margin-bottom: 8px;
    font: 12px/1.3 var(--ak-font-mono);
    color: var(--ak-fg-muted);
  }

  &__none {
    font-size: var(--ak-fs-sm);
    color: var(--ak-fg-subtle);
  }

  @media (max-width: $bp-phone) {
    grid-template-columns: 1fr;

    &__col + &__col {
      border-left: 0;
    }
  }
}
</style>
