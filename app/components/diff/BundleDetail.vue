<template>
  <ChangeDetail
    :path="path"
    :kind="kind"
    :before="beforeSide"
    :after="afterSide"
    @close="emit('close')"
  >
    <template v-if="before" #before-action>
      <button type="button" class="ak-btn ak-btn--sm" @click="save(before)">
        <AsIcon name="download" :size="16" />下载
      </button>
    </template>
    <template v-if="after" #after-action>
      <button
        type="button"
        class="ak-btn ak-btn--primary ak-btn--sm"
        @click="save(after)"
      >
        <AsIcon name="download" :size="16" />下载
      </button>
    </template>

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
  </ChangeDetail>
</template>

<script setup lang="ts">
import { AkSkeleton } from "@mooncellwiki/prts-design-vue";
import { computed, ref, watch } from "vue";
import { loadBundleHistory } from "~/common/bundles";
import {
  bundleFilename,
  download,
  formatSize,
  storageUrl,
} from "~/common/utils";
import { diffLink, useVersions } from "~/common/versions";
import AsIcon from "../AsIcon.vue";
import PaneNote from "../layout/PaneNote.vue";
import ChangeDetail from "./ChangeDetail.vue";
import type { Bundle, ChangeKind } from "~/common/bundles";
import type { Version } from "~/common/versions";

/**
 * The selected bundle: old and new side by side (version, size, hash,
 * download), then every version in which its content changed.
 */
const props = defineProps<{
  path: string;
  /** Undefined when comparing against nothing (the oldest version) */
  left?: Version;
  right: Version;
  before?: Bundle;
  after?: Bundle;
  kind?: ChangeKind;
}>();
const emit = defineEmits<{ close: [] }>();

const { byId, previous } = useVersions();

const beforeSide = computed(() => ({
  version: props.left?.resVersion ?? "—",
  size: props.before?.fileSize,
  hash: props.before?.fileHash,
  missing: props.left ? "这一版没有" : "此前没有",
}));
const afterSide = computed(() => ({
  version: props.right.resVersion,
  size: props.after?.fileSize,
  hash: props.after?.fileHash,
}));

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
</style>
