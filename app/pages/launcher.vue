<template>
  <PageHeader title="PC 启动器归档" en="PC Launcher" />

  <AkCollapseItem title="各版本归档进度" @toggle.once="progressOpened = true">
    <LauncherArchiveProgress v-if="progressOpened" ref="archiveProgress">
      <template #actions="{ loading }">
        <button
          type="button"
          class="ak-btn ak-btn--sm"
          :disabled="loading || loadingReleases || refreshing"
          @click="refresh"
        >
          <AsIcon name="refresh" :size="16" />
          {{ refreshing ? "正在刷新…" : "刷新进度" }}
        </button>
      </template>
    </LauncherArchiveProgress>
  </AkCollapseItem>

  <AkEmpty
    v-if="!release && releasesError"
    :icon="false"
    title="加载发布列表失败"
  >
    <template #icon
      ><AsIcon name="warn" :size="48" class="ak-empty__icon"
    /></template>
    <template #extra>
      <button type="button" class="ak-btn ak-btn--sm" @click="loadReleases()">
        <AsIcon name="refresh" :size="16" />重试
      </button>
    </template>
  </AkEmpty>
  <AkEmpty
    v-else-if="!release && !loadingReleases"
    :icon="false"
    title="暂无 PC 发布"
  >
    <template #icon
      ><AsIcon name="empty" :size="48" class="ak-empty__icon"
    /></template>
  </AkEmpty>
  <PaneBox v-else-if="!release" aria-busy="true">
    <PaneNote><AkSkeleton text :repeat="5" /></PaneNote>
  </PaneBox>

  <template v-else>
    <AsToolbar class="launcher-toolbar">
      <div class="launcher-versions">
        <VersionSelect
          v-model="compareModel"
          :options="options"
          class="launcher-select"
          label="基准版本"
          placeholder="基准版本"
        />
        <button
          type="button"
          class="ak-btn ak-btn--icon ak-btn--sm launcher-swap"
          title="交换版本"
          aria-label="交换版本"
          :disabled="compareFrom == null || compareFrom === selected"
          @click="swap"
        >
          <AsIcon name="swap" :size="16" />
        </button>
        <VersionSelect
          v-model="selectedModel"
          :options="options"
          class="launcher-select"
          label="对比版本"
          placeholder="对比版本"
        />
        <button
          v-if="nextBefore != null"
          type="button"
          class="ak-btn ak-btn--ghost ak-btn--sm launcher-older"
          :disabled="loadingReleases || refreshing"
          @click="loadReleases(true)"
        >
          <AsIcon name="history" :size="16" />
          {{ loadingReleases ? "正在加载…" : "更多发布" }}
        </button>
      </div>
      <KindToggles v-model="kinds" :counts="diffReady ? counts : undefined" />
      <template #end>
        <AkSearch
          v-model="keyword"
          size="sm"
          placeholder="筛选路径"
          label="筛选路径"
        />
      </template>
    </AsToolbar>

    <AkMessage v-if="releasesError" class="launcher-message" variant="danger">
      {{ releasesError }}
    </AkMessage>

    <SplitView side-label="文件树">
      <template #side>
        <PaneNote v-if="loadingDiff"><AkSkeleton text :repeat="8" /></PaneNote>
        <FileTree
          v-else-if="diffReady"
          label="版本差异文件树"
          :roots="tree.roots"
          :children-of="tree.childrenOf"
          :expanded="expanded"
          :selected="detail ? changeFileId(detail.path) : directory"
          :expand-all="q.trim() !== ''"
          :highlight="q"
          :empty-note="() => '没有符合筛选的文件'"
          @select="selectTreeNode"
          @toggle="toggle"
        />
      </template>
      <template #side-foot>
        <LauncherReleaseSummary
          class="launcher-progress"
          :release="release"
          :manifest-url="manifestUrl"
        />
      </template>

      <PaneBox
        class="launcher-results"
        label="版本差异"
        :aria-busy="loadingDiff"
      >
        <template #head>
          <h2 class="launcher-heading">
            <AsIcon name="diff" :size="16" />版本差异
          </h2>
          <span v-if="diffReady" class="launcher-count">
            {{ formatCount(rows.length) }} 项
          </span>
        </template>
        <template v-if="diffReady" #head-end>
          <AkSwitch v-model="withSubdirs" size="sm">含子目录</AkSwitch>
        </template>
        <AkEmpty
          v-if="compareFrom == null || compareFrom === selected"
          :icon="false"
          :title="
            compareFrom == null ? '选择基准版本来比较' : '两边是同一个发布'
          "
        >
          <template #icon
            ><AsIcon name="info" :size="48" class="ak-empty__icon"
          /></template>
        </AkEmpty>
        <PaneNote v-else-if="loadingDiff"
          ><AkSkeleton text :repeat="6"
        /></PaneNote>
        <AkEmpty v-else-if="diffError" :icon="false" :title="diffError">
          <template #icon
            ><AsIcon name="warn" :size="48" class="ak-empty__icon"
          /></template>
          <template #extra>
            <button type="button" class="ak-btn ak-btn--sm" @click="loadDiff">
              <AsIcon name="refresh" :size="16" />重试
            </button>
          </template>
        </AkEmpty>
        <AkEmpty
          v-else-if="!rows.length"
          :icon="false"
          :title="noChanges ? '两次发布没有文件差异' : '没有符合筛选的文件'"
        >
          <template #icon>
            <AsIcon
              :name="noChanges ? 'check' : 'search'"
              :size="48"
              class="ak-empty__icon"
            />
          </template>
          <template #extra>
            <button
              v-if="noChanges"
              type="button"
              class="ak-btn ak-btn--sm"
              @click="kinds.same = true"
            >
              显示未变更
            </button>
            <button
              v-else
              type="button"
              class="ak-btn ak-btn--sm"
              @click="resetView"
            >
              清除筛选
            </button>
          </template>
        </AkEmpty>
        <LauncherChangeTable
          v-else
          :changes="pageRows"
          :selected="detail?.path"
          @select="selectedFile = $event"
        />
        <template v-if="rows.length > PAGE_SIZE" #foot>
          <span class="launcher-count">{{ range }}</span>
          <AkPagination
            v-model="page"
            class="launcher-pager"
            :item-count="rows.length"
            :page-size="PAGE_SIZE"
          />
        </template>
      </PaneBox>
      <template v-if="wide && detail && baseRelease" #detail>
        <LauncherFileDetail
          :change="detail"
          :before="baseRelease"
          :after="release"
          @close="closeDetail"
        />
      </template>
    </SplitView>

    <AkDialog
      v-if="!wide && baseRelease"
      class="launcher-detail-dialog"
      label="文件详情"
      size="sm"
      :closable="false"
      :model-value="detail != null"
      @update:model-value="closeDetail"
    >
      <LauncherFileDetail
        v-if="detail"
        :change="detail"
        :before="baseRelease"
        :after="release"
        @close="closeDetail"
      />
    </AkDialog>
  </template>
</template>

<script setup lang="ts">
import {
  AkCollapseItem,
  AkDialog,
  AkEmpty,
  AkMessage,
  AkPagination,
  AkSearch,
  AkSkeleton,
  AkSwitch,
} from "@mooncellwiki/prts-design-vue";
import { useMediaQuery } from "@vueuse/core";
import {
  computed,
  onMounted,
  ref,
  shallowRef,
  useTemplateRef,
  watch,
} from "vue";
import { client } from "~/common/client";
import { fetchLauncherReleases } from "~/common/launcher";
import { formatCount, parentPath } from "~/common/utils";
import AsIcon from "~/components/AsIcon.vue";
import {
  buildChangeTree,
  changeFileId,
  useChangeFilters,
} from "~/components/diff/changes";
import KindToggles from "~/components/diff/KindToggles.vue";
import LauncherArchiveProgress from "~/components/launcher/LauncherArchiveProgress.vue";
import LauncherChangeTable from "~/components/launcher/LauncherChangeTable.vue";
import LauncherFileDetail from "~/components/launcher/LauncherFileDetail.vue";
import LauncherReleaseSummary from "~/components/launcher/LauncherReleaseSummary.vue";
import AsToolbar from "~/components/layout/AsToolbar.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import VersionSelect from "~/components/layout/VersionSelect.vue";
import PageHeader from "~/components/PageHeader.vue";
import FileTree from "~/components/tree/FileTree.vue";
import type { LauncherRelease } from "~/common/launcher";
import type { LauncherChange } from "~/components/launcher/LauncherChangeTable.vue";
import type { TreeNode } from "~/components/tree/model";

const PAGE_SIZE = 100;
const CHANGE_KINDS = new Map<string, LauncherChange["kind"]>([
  ["added", "add"],
  ["modified", "chg"],
  ["deleted", "rem"],
  ["unchanged", "same"],
]);
const releases = ref<LauncherRelease[]>([]);
const progressOpened = ref(false);
const archiveProgress = useTemplateRef("archiveProgress");
const selected = ref<number>();
const compareFrom = ref<number>();

/** VersionSelect models use null for the placeholder; release ids stay numeric. */
function idModel(id: typeof selected) {
  return computed({
    get: () => id.value ?? null,
    set: (value) => {
      id.value = value == null ? undefined : Number(value);
    },
  });
}
const selectedModel = idModel(selected);
const compareModel = idModel(compareFrom);
const nextBefore = ref<number | null>();
const loadingReleases = ref(true);
const refreshing = ref(false);
const releasesError = ref("");
const release = computed(() =>
  releases.value.find((r) => r.id === selected.value),
);
const baseRelease = computed(() =>
  releases.value.find((r) => r.id === compareFrom.value),
);
const options = computed(() =>
  releases.value.map((r) => ({
    value: r.id,
    label: `#${r.id} · ${r.packageVersion} / ${r.gameVersion} · ${r.manifestMd5.slice(0, 10)}`,
  })),
);
const manifestUrl = computed(() => {
  const base = (import.meta.env.VITE_API_PATH ?? "").replace(/\/$/, "");
  return `${base}/api/v1/launcher/releases/${selected.value}/manifest`;
});

const changes = shallowRef<LauncherChange[]>([]);
const diffReady = ref(false);
const loadingDiff = ref(false);
const diffError = ref("");
let diffRequest = 0;

const { kinds, keyword, q, counts, visible, resetFilters } =
  useChangeFilters(changes);
const tree = computed(() => buildChangeTree(visible.value));
const expanded = ref(new Set([""]));
const directory = ref("");
const selectedFile = ref<string>();
const detail = computed(() =>
  selectedFile.value == null
    ? undefined
    : visible.value.find((change) => change.path === selectedFile.value),
);
const wide = useMediaQuery("(width > 1399px)");
const withSubdirs = ref(true);

watch(tree, ({ childrenOf }) => {
  if (!childrenOf(directory.value)) {
    directory.value = "";
    expanded.value.add("");
  }
});
watch(detail, (change) => {
  if (!change) closeDetail();
});

function closeDetail() {
  selectedFile.value = undefined;
}

function selectTreeNode(node: TreeNode) {
  if (node.isDir) {
    directory.value = node.path;
    closeDetail();
  } else {
    selectedFile.value = node.path;
  }
}

function toggle(path: string, open: boolean) {
  if (open) expanded.value.add(path);
  else expanded.value.delete(path);
}

function resetView() {
  resetFilters();
  directory.value = "";
  closeDetail();
  withSubdirs.value = true;
  expanded.value.add("");
}

const noChanges = computed(
  () =>
    !kinds.value.same &&
    changes.value.every((change) => change.kind === "same"),
);
const rows = computed(() => {
  const dir = directory.value;
  if (!withSubdirs.value) {
    return visible.value.filter((change) => parentPath(change.path) === dir);
  }
  return dir
    ? visible.value.filter((change) => change.path.startsWith(`${dir}/`))
    : visible.value;
});
const page = ref(1);
watch(rows, () => {
  page.value = 1;
});
const pageRows = computed(() =>
  rows.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE),
);
const range = computed(
  () =>
    `${formatCount((page.value - 1) * PAGE_SIZE + 1)}–${formatCount(
      Math.min(page.value * PAGE_SIZE, rows.value.length),
    )} / ${formatCount(rows.value.length)}`,
);

function swap() {
  [selected.value, compareFrom.value] = [compareFrom.value, selected.value];
}

/** Mirrors the API's order: newest package version first, then newest id. */
function versionKey(version: string) {
  return (version.match(/\d{1,18}/g) ?? []).map(BigInt);
}
function compareReleases(a: LauncherRelease, b: LauncherRelease) {
  const x = versionKey(a.packageVersion);
  const y = versionKey(b.packageVersion);
  for (let i = 0; i < Math.min(x.length, y.length); i++)
    if (x[i] !== y[i]) return x[i]! > y[i]! ? -1 : 1;
  return y.length - x.length || b.id - a.id;
}

async function loadReleases(more = false) {
  loadingReleases.value = true;
  releasesError.value = "";
  try {
    const data = await fetchLauncherReleases({
      limit: PAGE_SIZE,
      before: more ? (nextBefore.value ?? undefined) : undefined,
    });
    const hadReleases = releases.value.length > 0;
    const combined = new Map(releases.value.map((r) => [r.id, r]));
    for (const item of data.releases) combined.set(item.id, item);
    releases.value = [...combined.values()].sort(compareReleases);
    if (more || !hadReleases) nextBefore.value = data.nextBefore;
    selected.value ??= data.releases[0]?.id;
    return data.releases;
  } catch (err) {
    releasesError.value =
      err instanceof Error ? err.message : "加载发布列表失败";
  } finally {
    loadingReleases.value = false;
  }
}

async function loadDiff() {
  const from = compareFrom.value;
  const to = selected.value;
  const request = ++diffRequest;
  changes.value = [];
  directory.value = "";
  closeDetail();
  expanded.value = new Set([""]);
  diffReady.value = false;
  diffError.value = "";
  loadingDiff.value = from != null && to != null && from !== to;
  if (from == null || to == null || from === to) return;
  try {
    const { data, response } = await client.GET("/api/v1/launcher/diff/files", {
      params: { query: { from, to } },
    });
    if (request !== diffRequest) return;
    if (!data) throw new Error(`比较发布失败（${response.status}）`);
    changes.value = data.map((change) => {
      const kind = CHANGE_KINDS.get(change.change);
      if (!kind) throw new Error("比较发布失败（未知的文件变更类型）");
      return { ...change, kind };
    });
    diffReady.value = true;
  } catch (err) {
    if (request === diffRequest)
      diffError.value = err instanceof Error ? err.message : "比较发布失败";
  } finally {
    if (request === diffRequest) loadingDiff.value = false;
  }
}

async function refresh() {
  if (loadingReleases.value || refreshing.value) return;
  refreshing.value = true;
  try {
    const [latestPage] = await Promise.all([
      loadReleases(),
      archiveProgress.value?.refresh(),
    ]);
    const current = release.value;
    if (current && !latestPage?.some((item) => item.id === current.id)) {
      const { data, response } = await client.GET(
        "/api/v1/launcher/releases/{id}",
        {
          params: { path: { id: current.id } },
        },
      );
      if (!data) throw new Error(`刷新进度失败（${response.status}）`);
      releases.value = releases.value.map((item) =>
        item.id === data.id ? data : item,
      );
    }
  } catch (err) {
    releasesError.value = err instanceof Error ? err.message : "刷新进度失败";
  } finally {
    refreshing.value = false;
  }
}

watch([selected, releases], () => {
  if (compareFrom.value != null) return;
  const index = releases.value.findIndex((r) => r.id === selected.value);
  if (index >= 0) compareFrom.value = releases.value[index + 1]?.id;
});
watch([selected, compareFrom], loadDiff);
onMounted(() => void loadReleases());
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-message {
  margin-bottom: var(--ak-space-4);
}

.launcher-heading {
  display: inline-flex;
  align-items: center;
  gap: var(--ak-space-2);
  margin: 0;
  font-size: var(--ak-fs-sm);
  font-weight: 700;
}

.launcher-versions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ak-space-2);
  max-width: 100%;
}

.launcher-select {
  width: 20rem;
  min-width: 0;
}

.launcher-count {
  color: var(--ak-fg-muted);
  font-variant-numeric: tabular-nums;
}

.launcher-progress {
  width: 100%;
}

.launcher-pager {
  margin-left: auto;
}

.launcher-results {
  container-type: inline-size;
}

.launcher-detail-dialog {
  :deep(.ak-dialog__body) {
    padding: 0;
  }

  :deep(.pane) {
    border: 0;
  }
}

@media (max-width: $bp-phone) {
  .launcher-versions,
  .launcher-toolbar :deep(.toolbar__end) {
    width: 100%;
  }

  .launcher-versions {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .launcher-select {
    grid-column: 1;
    width: 100%;
  }

  .launcher-swap {
    grid-area: 1 / 2 / 3 / 3;
  }

  .launcher-older {
    justify-self: start;
  }
}
</style>
