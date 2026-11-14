<template>
  <PageHeader title="PC 启动器归档" en="PC Launcher">
    <PageTool
      icon="refresh"
      :disabled="loadingReleases || refreshing"
      @click="refresh"
    >
      {{ refreshing ? "正在刷新…" : "刷新进度" }}
    </PageTool>
  </PageHeader>

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

  <AkTabs v-else v-model="tab" label="PC 归档内容">
    <AkTabPane v-for="view in views" :key="view.name" :name="view.name">
      <template #tab>
        <span class="launcher-tab"
          ><AsIcon :name="view.icon" :size="16" />{{ view.title }}</span
        >
      </template>

      <AsToolbar class="launcher-toolbar">
        <div
          class="launcher-versions"
          :class="{ 'launcher-versions--compare': view.name === 'diff' }"
        >
          <label v-if="view.name === 'diff'" class="launcher-picker">
            基准版本
            <VersionSelect
              v-model="compareModel"
              :options="options"
              class="launcher-select"
              label="基准版本"
              placeholder="选择基准版本"
            />
          </label>
          <button
            v-if="view.name === 'diff'"
            type="button"
            class="ak-btn ak-btn--icon ak-btn--sm launcher-swap"
            title="交换版本"
            aria-label="交换版本"
            :disabled="compareFrom == null || compareFrom === selected"
            @click="swap"
          >
            <AsIcon name="swap" :size="16" />
          </button>
          <label class="launcher-picker">
            {{ view.name === "diff" ? "对比版本" : "PC 发布" }}
            <VersionSelect
              v-model="selectedModel"
              :options="options"
              class="launcher-select"
              :label="view.name === 'diff' ? '对比版本' : 'PC 发布'"
            />
          </label>
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
        <template v-if="view.name === 'files'" #end>
          <form
            class="launcher-search"
            aria-label="搜索历史路径"
            @submit.prevent="loadFiles('search')"
          >
            <AkSearch
              v-model="search"
              size="sm"
              placeholder="搜索路径（区分大小写）"
              label="搜索历史路径"
            />
            <button
              type="submit"
              class="ak-btn ak-btn--sm"
              :disabled="loadingFiles"
            >
              搜索
            </button>
          </form>
        </template>
      </AsToolbar>

      <AkMessage v-if="releasesError" class="launcher-message" variant="danger">
        {{ releasesError }}
      </AkMessage>

      <SplitView side-label="发布归档进度">
        <template #side-head>
          <h2 class="launcher-heading">
            <AsIcon name="bundle" :size="16" />{{
              view.name === "diff" ? "对比版本归档进度" : "归档进度"
            }}
          </h2>
          <span class="launcher-release-id">#{{ release.id }}</span>
        </template>
        <template #side>
          <LauncherReleaseSummary
            :release="release"
            :manifest-url="manifestUrl"
          />
        </template>

        <PaneBox
          v-if="view.name === 'files'"
          label="文件清单"
          :aria-busy="loadingFiles"
        >
          <template #head>
            <h2 class="launcher-heading">
              <AsIcon name="file" :size="16" />历史文件
            </h2>
          </template>
          <template v-if="files.length > 0" #head-end>
            <span class="launcher-count"
              >{{ nextFile != null ? "已加载 " : ""
              }}{{ formatCount(files.length) }} 个文件</span
            >
          </template>
          <AkMessage
            v-if="filesError && files.length"
            class="launcher-message launcher-message--inset"
            variant="danger"
          >
            {{ filesError }}
          </AkMessage>
          <PaneNote v-if="loadingFiles && !files.length"
            ><AkSkeleton text :repeat="6"
          /></PaneNote>
          <AkEmpty
            v-else-if="filesError && !files.length"
            :icon="false"
            :title="filesError"
          >
            <template #icon
              ><AsIcon name="warn" :size="48" class="ak-empty__icon"
            /></template>
            <template #extra>
              <button
                type="button"
                class="ak-btn ak-btn--sm"
                @click="loadFiles()"
              >
                <AsIcon name="refresh" :size="16" />重试
              </button>
            </template>
          </AkEmpty>
          <AkEmpty
            v-else-if="!files.length"
            :icon="false"
            :title="fileSearch ? '没有匹配的文件' : '清单中没有文件'"
          >
            <template #icon
              ><AsIcon
                :name="fileSearch ? 'search' : 'empty'"
                :size="48"
                class="ak-empty__icon"
            /></template>
            <template v-if="fileSearch" #extra>
              <button
                type="button"
                class="ak-btn ak-btn--sm"
                @click="clearSearch"
              >
                清除搜索
              </button>
            </template>
          </AkEmpty>
          <LauncherFileTable v-else :files="files" />
          <template v-if="nextFile != null" #foot>
            <button
              type="button"
              class="ak-btn ak-btn--sm launcher-more"
              :disabled="loadingFiles"
              @click="loadFiles('more')"
            >
              {{ loadingFiles ? "正在加载…" : "更多文件" }}
            </button>
          </template>
        </PaneBox>

        <PaneBox v-else label="版本差异" :aria-busy="loadingDiff">
          <template #head>
            <h2 class="launcher-heading">
              <AsIcon name="diff" :size="16" />变更列表
            </h2>
          </template>
          <template v-if="changes.length > 0" #head-end>
            <span class="launcher-count"
              >{{ nextDiff != null ? "已加载 " : ""
              }}{{ formatCount(changes.length) }} 项变更</span
            >
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
          <template v-else>
            <AkMessage
              v-if="diffError && changes.length"
              class="launcher-message launcher-message--inset"
              variant="danger"
            >
              {{ diffError }}
            </AkMessage>
            <PaneNote v-if="loadingDiff && !changes.length"
              ><AkSkeleton text :repeat="6"
            /></PaneNote>
            <AkEmpty
              v-else-if="diffError && !changes.length"
              :icon="false"
              :title="diffError"
            >
              <template #icon
                ><AsIcon name="warn" :size="48" class="ak-empty__icon"
              /></template>
              <template #extra>
                <button
                  type="button"
                  class="ak-btn ak-btn--sm"
                  @click="loadDiff()"
                >
                  <AsIcon name="refresh" :size="16" />重试
                </button>
              </template>
            </AkEmpty>
            <AkEmpty
              v-else-if="!changes.length"
              :icon="false"
              title="两次发布没有文件差异"
            >
              <template #icon
                ><AsIcon name="check" :size="48" class="ak-empty__icon"
              /></template>
            </AkEmpty>
            <LauncherChangeTable v-else :changes="changes" />
          </template>
          <template v-if="changes.length > 0" #foot>
            <span class="launcher-count"
              >改名会显示为删除旧路径和新增路径。</span
            >
            <button
              v-if="nextDiff != null"
              type="button"
              class="ak-btn ak-btn--sm launcher-more"
              :disabled="loadingDiff"
              @click="loadDiff(true)"
            >
              {{ loadingDiff ? "正在加载…" : "更多差异" }}
            </button>
          </template>
        </PaneBox>
      </SplitView>
    </AkTabPane>
  </AkTabs>
</template>

<script setup lang="ts">
import {
  AkEmpty,
  AkMessage,
  AkSearch,
  AkSkeleton,
  AkTabPane,
  AkTabs,
} from "@mooncellwiki/prts-design-vue";
import { useIntervalFn } from "@vueuse/core";
import { computed, onMounted, ref, watch } from "vue";
import { client } from "~/common/client";
import { formatCount } from "~/common/utils";
import AsIcon from "~/components/AsIcon.vue";
import LauncherChangeTable from "~/components/launcher/LauncherChangeTable.vue";
import LauncherFileTable from "~/components/launcher/LauncherFileTable.vue";
import LauncherReleaseSummary from "~/components/launcher/LauncherReleaseSummary.vue";
import AsToolbar from "~/components/layout/AsToolbar.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import VersionSelect from "~/components/layout/VersionSelect.vue";
import PageHeader from "~/components/PageHeader.vue";
import PageTool from "~/components/PageTool.vue";
import type { components } from "~/common/schema";

type Release = components["schemas"]["LauncherRelease"];
type File = components["schemas"]["LauncherFile"];
type Change = components["schemas"]["LauncherDiff"];

const PAGE_SIZE = 100;
const views = [
  { name: "files", title: "文件清单", icon: "list" },
  { name: "diff", title: "版本差异", icon: "diff" },
] as const;
const tab = ref<string | number>("files");
const releases = ref<Release[]>([]);
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

const files = ref<File[]>([]);
const nextFile = ref<string | null>();
const search = ref("");
const fileSearch = ref("");
const loadedFileRelease = ref<number>();
const loadingFiles = ref(false);
const filesError = ref("");
let fileRequest = 0;

const changes = ref<Change[]>([]);
const loadedDiff = ref<{ from: number; to: number }>();
const nextDiff = ref<string | null>();
const loadingDiff = ref(false);
const diffError = ref("");
let diffRequest = 0;

function swap() {
  [selected.value, compareFrom.value] = [compareFrom.value, selected.value];
}

/** Mirrors the API's order: newest package version first, then newest id. */
function versionKey(version: string) {
  return (version.match(/\d{1,18}/g) ?? []).map(BigInt);
}
function compareReleases(a: Release, b: Release) {
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
    const { data, response } = await client.GET("/api/v1/launcher/releases", {
      params: {
        query: {
          limit: PAGE_SIZE,
          before: more ? (nextBefore.value ?? undefined) : undefined,
        },
      },
    });
    if (!data) throw new Error(`加载发布列表失败（${response.status}）`);
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

async function loadFiles(mode: "search" | "more" | "refresh" = "search") {
  const id = selected.value;
  if (id == null) return;
  if (
    mode !== "search" &&
    (loadingFiles.value || loadedFileRelease.value !== id)
  )
    return;
  const request = ++fileRequest;
  if (mode === "search") {
    fileSearch.value = search.value;
    loadedFileRelease.value = undefined;
    files.value = [];
    nextFile.value = null;
  }
  const query = fileSearch.value;
  const count =
    mode === "refresh" ? Math.max(files.value.length, PAGE_SIZE) : PAGE_SIZE;
  let after = mode === "more" ? (nextFile.value ?? undefined) : undefined;
  const updated: File[] = [];
  loadingFiles.value = true;
  filesError.value = "";
  try {
    // Refresh archival status without dropping pages the user has already loaded.
    do {
      const { data, response } = await client.GET(
        "/api/v1/launcher/releases/{id}/files",
        {
          params: {
            path: { id },
            query: { q: query, after, limit: PAGE_SIZE },
          },
        },
      );
      if (request !== fileRequest || id !== selected.value) return;
      if (!data) throw new Error(`加载清单失败（${response.status}）`);
      updated.push(...data.files);
      after = data.nextAfter ?? undefined;
    } while (after != null && updated.length < count);
    files.value = mode === "more" ? [...files.value, ...updated] : updated;
    nextFile.value = after ?? null;
    loadedFileRelease.value = id;
  } catch (err) {
    if (request === fileRequest && id === selected.value)
      filesError.value = err instanceof Error ? err.message : "加载清单失败";
  } finally {
    if (request === fileRequest) loadingFiles.value = false;
  }
}

function clearSearch() {
  search.value = "";
  void loadFiles();
}

async function loadDiff(more = false) {
  const from = compareFrom.value;
  const to = selected.value;
  const request = ++diffRequest;
  if (!more) {
    changes.value = [];
    nextDiff.value = null;
    loadedDiff.value = undefined;
  }
  diffError.value = "";
  loadingDiff.value = from != null && to != null && from !== to;
  if (from == null || to == null || from === to) return;
  try {
    const { data, response } = await client.GET("/api/v1/launcher/diff", {
      params: {
        query: {
          from,
          to,
          after: more ? (nextDiff.value ?? undefined) : undefined,
        },
      },
    });
    if (
      request !== diffRequest ||
      from !== compareFrom.value ||
      to !== selected.value
    )
      return;
    if (!data) throw new Error(`比较发布失败（${response.status}）`);
    changes.value = more ? [...changes.value, ...data.changes] : data.changes;
    nextDiff.value = data.nextAfter;
    loadedDiff.value = { from, to };
  } catch (err) {
    if (
      request === diffRequest &&
      from === compareFrom.value &&
      to === selected.value
    )
      diffError.value = err instanceof Error ? err.message : "比较发布失败";
  } finally {
    if (request === diffRequest) loadingDiff.value = false;
  }
}

async function refresh() {
  if (loadingReleases.value || refreshing.value) return;
  refreshing.value = true;
  try {
    const latestPage = await loadReleases();
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
    if (tab.value === "files") await loadFiles("refresh");
  } catch (err) {
    releasesError.value = err instanceof Error ? err.message : "刷新进度失败";
  } finally {
    refreshing.value = false;
  }
}

watch([tab, selected, releases], () => {
  if (tab.value !== "diff" || compareFrom.value != null) return;
  const index = releases.value.findIndex((r) => r.id === selected.value);
  if (index >= 0) compareFrom.value = releases.value[index + 1]?.id;
});
watch([selected, tab], ([id, name]) => {
  if (id == null || name !== "files") return;
  if (loadedFileRelease.value !== id) {
    void loadFiles();
  } else if (files.value.some((file) => !file.sha256)) {
    void loadFiles("refresh");
  }
});
watch([selected, compareFrom, tab], ([to, from, name]) => {
  if (
    name === "diff" &&
    (loadedDiff.value?.to !== to || loadedDiff.value?.from !== from)
  )
    void loadDiff();
});
onMounted(() => void loadReleases());
useIntervalFn(() => {
  if (release.value && !release.value.readyAt && !loadingFiles.value)
    void refresh();
}, 15_000);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-message {
  margin-bottom: var(--ak-space-4);
}

.launcher-tab,
.launcher-heading {
  display: inline-flex;
  align-items: center;
  gap: var(--ak-space-2);
}

.launcher-toolbar {
  align-items: flex-end;
}

.launcher-heading {
  margin: 0;
  font-size: var(--ak-fs-sm);
  font-weight: 700;
}

.launcher-versions {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--ak-space-2);
  min-width: 0;
  max-width: 100%;
}

.launcher-picker {
  display: grid;
  gap: var(--ak-space-1);
  min-width: 0;
  max-width: 100%;
  color: var(--ak-fg-secondary);
  font-size: var(--ak-fs-xs);
  font-weight: 600;
}

.launcher-select {
  width: 20rem;
  min-width: 0;
}

.launcher-search {
  display: flex;
  align-items: center;
  gap: var(--ak-space-2);
  max-width: 100%;

  .ak-search {
    width: 15rem;
    min-width: 0;
  }
}

.launcher-count {
  color: var(--ak-fg-muted);
  font-variant-numeric: tabular-nums;
}

.launcher-release-id {
  @include mono(12px);

  margin-left: auto;
  color: var(--ak-fg-muted);
}

.launcher-message--inset {
  margin: var(--ak-space-3) var(--ak-space-4);
}

.launcher-more {
  margin-left: auto;
}

@media (max-width: $bp-phone) {
  .launcher-versions,
  .launcher-select,
  .launcher-search {
    width: 100%;
  }

  .launcher-picker {
    flex: 1 1 12rem;
  }

  .launcher-older {
    padding-inline: var(--ak-space-2);
  }

  .launcher-toolbar :deep(.toolbar__end) {
    width: 100%;
  }

  .launcher-search .ak-search {
    flex: 1;
  }

  .launcher-versions--compare {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;

    .launcher-picker {
      grid-column: 1;
    }

    .launcher-swap {
      grid-area: 1 / 2 / 3 / 3;
    }

    .launcher-older {
      justify-self: start;
    }
  }
}
</style>
