<template>
  <PageHeader title="PC 启动器归档" en="PC Launcher">
    <button
      type="button"
      class="ak-btn ak-btn--sm"
      :disabled="loading"
      @click="refresh"
    >
      刷新进度
    </button>
  </PageHeader>
  <p v-if="error" role="alert" class="launcher-error">{{ error }}</p>
  <p v-if="!releases.length && !loading" role="status">
    暂无 PC 发布。启用 launcher 配置后，worker 会独立追踪并归档。
  </p>
  <div class="launcher-tools">
    <label for="launcher-release">PC 发布</label>
    <select id="launcher-release" v-model="selected" class="launcher-select">
      <option :value="undefined" disabled>
        选择 PC 发布（同包版本重发按清单区分）
      </option>
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
      >
        {{ option.label }}
      </option>
    </select>
    <button
      v-if="nextBefore"
      type="button"
      class="ak-btn ak-btn--ghost ak-btn--sm"
      :disabled="loading"
      @click="loadReleases(true)"
    >
      更多发布
    </button>
  </div>
  <template v-if="release">
    <section class="launcher-summary" aria-label="归档进度">
      <h2>包 {{ release.packageVersion }} / 游戏 {{ release.gameVersion }}</h2>
      <p class="launcher-dim">
        清单 MD5：<code>{{ release.manifestMd5 }}</code> · 发现于
        {{ new Date(release.discoveredAt).toLocaleString() }}
      </p>
      <progress
        class="launcher-progress"
        :value="progress"
        max="100"
        aria-label="已归档文件比例"
      >
        {{ progress }}%
      </progress>
      <div class="launcher-tools">
        <span
          >{{ release.readyAt ? "归档完成" : "归档中" }}：{{
            release.archivedFiles
          }}
          / {{ release.totalFiles }} 文件，{{
            formatSize(release.archivedBytes)
          }}
          / {{ formatSize(release.totalBytes) }}</span
        >
        <a
          class="ak-btn ak-btn--ghost ak-btn--sm"
          :href="apiUrl(`/api/v1/launcher/releases/${release.id}/manifest`)"
          >下载原始加密清单</a
        >
      </div>
    </section>
    <div class="ak-tabs launcher-tabs" role="tablist" aria-label="PC 归档内容">
      <button
        id="launcher-files-tab"
        type="button"
        class="ak-tab"
        :class="{ 'is-active': tab === 'files' }"
        role="tab"
        :aria-selected="tab === 'files'"
        aria-controls="launcher-files"
        @click="tab = 'files'"
      >
        文件清单
      </button>
      <button
        id="launcher-diff-tab"
        type="button"
        class="ak-tab"
        :class="{ 'is-active': tab === 'diff' }"
        role="tab"
        :aria-selected="tab === 'diff'"
        aria-controls="launcher-diff"
        @click="tab = 'diff'"
      >
        版本差异
      </button>
    </div>
    <section
      v-if="tab === 'files'"
      id="launcher-files"
      role="tabpanel"
      aria-labelledby="launcher-files-tab"
    >
      <form class="launcher-tools" @submit.prevent="loadFiles()">
        <AkSearch
          v-model="search"
          placeholder="按历史路径搜索（区分大小写）"
          label="按历史路径搜索"
        />
        <button
          type="submit"
          class="ak-btn ak-btn--sm"
          :disabled="loadingFiles"
        >
          搜索
        </button>
      </form>
      <p v-if="loadingFiles" role="status">加载清单…</p>
      <div class="launcher-table">
        <table class="ak-table ak-table--compact">
          <thead>
            <tr>
              <th scope="col">历史路径</th>
              <th scope="col">大小</th>
              <th scope="col">MD5</th>
              <th scope="col">归档</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="file in files" :key="file.path">
              <td class="launcher-path">{{ file.path }}</td>
              <td class="launcher-size">{{ formatSize(file.size) }}</td>
              <td class="launcher-hash">{{ file.md5 }}</td>
              <td>
                <a
                  v-if="file.sha256"
                  class="ak-btn ak-btn--ghost ak-btn--sm"
                  :href="downloadUrl(file.path)"
                  >下载原始文件</a
                >
                <span
                  v-else
                  class="launcher-pending"
                  :title="file.lastError ?? undefined"
                  >{{
                    file.lastError ? `等待重试 (${file.attempts})` : "待归档"
                  }}</span
                >
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-if="!loadingFiles && !files.length">没有匹配的文件</p>
      <button
        v-if="nextFile"
        type="button"
        class="ak-btn ak-btn--sm launcher-more"
        :disabled="loadingFiles"
        @click="loadFiles('more')"
      >
        更多文件
      </button>
    </section>
    <section
      v-else
      id="launcher-diff"
      role="tabpanel"
      aria-labelledby="launcher-diff-tab"
    >
      <div class="launcher-tools">
        <label for="launcher-compare">与旧发布比较</label>
        <select
          id="launcher-compare"
          v-model="compareFrom"
          class="launcher-select"
        >
          <option :value="undefined" disabled>
            选择旧发布，与当前发布比较
          </option>
          <option
            v-for="option in options"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>
      </div>
      <p class="launcher-dim">
        清单入库即可比较。改名显示为删除旧路径和新增路径，内容按 MD5
        与大小复用。
      </p>
      <p v-if="loadingDiff" role="status">比较发布…</p>
      <div class="launcher-table">
        <table class="ak-table ak-table--compact">
          <thead>
            <tr>
              <th scope="col">路径</th>
              <th scope="col">变化</th>
              <th scope="col">旧 MD5</th>
              <th scope="col">新 MD5</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="change in changes" :key="change.path">
              <td class="launcher-path">{{ change.path }}</td>
              <td>
                {{
                  { added: "新增", deleted: "删除", modified: "修改" }[
                    change.change
                  ] ?? change.change
                }}
              </td>
              <td class="launcher-hash">{{ change.oldMd5 ?? "—" }}</td>
              <td class="launcher-hash">{{ change.newMd5 ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-if="compareFrom != null && !loadingDiff && !changes.length">
        两次发布没有文件差异
      </p>
      <button
        v-if="nextDiff"
        type="button"
        class="ak-btn ak-btn--sm launcher-more"
        :disabled="loadingDiff"
        @click="loadDiff(true)"
      >
        更多差异
      </button>
    </section>
  </template>
</template>

<script setup lang="ts">
import { AkSearch } from "@mooncellwiki/prts-design-vue";
import { useIntervalFn } from "@vueuse/core";
import { computed, onMounted, ref, watch } from "vue";
import { client } from "~/common/client";
import PageHeader from "~/components/PageHeader.vue";
import type { components } from "~/common/schema";

type Release = components["schemas"]["LauncherRelease"];
type File = components["schemas"]["LauncherFile"];
type Change = components["schemas"]["LauncherDiff"];

const releases = ref<Release[]>([]);
const selected = ref<number>();
const compareFrom = ref<number>();
const nextBefore = ref<number | null>();
const files = ref<File[]>([]);
const changes = ref<Change[]>([]);
const nextFile = ref<string | null>();
const nextDiff = ref<string | null>();
const search = ref("");
let fileSearch = "";
let fileRequest = 0;
const tab = ref("files");
const error = ref("");
const loading = ref(false);
const loadingFiles = ref(false);
const loadingDiff = ref(false);
const release = computed(() =>
  releases.value.find((r) => r.id === selected.value),
);
const progress = computed(() =>
  release.value?.totalFiles
    ? Math.round(
        (release.value.archivedFiles / release.value.totalFiles) * 10000,
      ) / 100
    : 0,
);
const options = computed(() =>
  releases.value.map((r) => ({
    value: r.id,
    label: `${r.packageVersion} / ${r.gameVersion} · ${r.manifestMd5.slice(0, 10)} · #${r.id}`,
  })),
);

function apiUrl(path: string) {
  return `${(import.meta.env.VITE_API_PATH ?? "").replace(/\/$/, "")}${path}`;
}

function downloadUrl(path: string) {
  return apiUrl(
    `/api/v1/launcher/releases/${selected.value}/download?${new URLSearchParams({ path })}`,
  );
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(2)} KiB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(2)} MiB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GiB`;
}

async function loadReleases(more = false) {
  loading.value = true;
  try {
    const { data, response } = await client.GET("/api/v1/launcher/releases", {
      params: {
        query: {
          limit: 100,
          before: more ? (nextBefore.value ?? undefined) : undefined,
        },
      },
    });
    if (!data) throw new Error(`加载发布失败：${response.status}`);
    const hadReleases = releases.value.length > 0;
    const combined = new Map(releases.value.map((r) => [r.id, r]));
    for (const item of data.releases) combined.set(item.id, item);
    releases.value = [...combined.values()].sort((a, b) => b.id - a.id);
    if (more || !hadReleases) nextBefore.value = data.nextBefore;
    selected.value ??= data.releases[0]?.id;
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function loadFiles(mode: "replace" | "more" | "refresh" = "replace") {
  if (mode === "refresh" && loadingFiles.value) return;
  const id = selected.value;
  if (id == null) return;
  const request = ++fileRequest;
  const query = mode === "replace" ? search.value : fileSearch;
  const count = mode === "refresh" ? Math.max(files.value.length, 100) : 100;
  let after = mode === "more" ? (nextFile.value ?? undefined) : undefined;
  const updated: File[] = [];
  loadingFiles.value = true;
  try {
    // Manifests are immutable: reload the same pages to update archive status
    // without dropping the rows the user has already expanded.
    do {
      const { data, response } = await client.GET(
        "/api/v1/launcher/releases/{id}/files",
        {
          params: {
            path: { id },
            query: { q: query, after, limit: 100 },
          },
        },
      );
      if (request !== fileRequest || id !== selected.value) return;
      if (!data) throw new Error(`加载清单失败：${response.status}`);
      updated.push(...data.files);
      after = data.nextAfter ?? undefined;
    } while (after != null && updated.length < count);
    files.value = mode === "more" ? [...files.value, ...updated] : updated;
    nextFile.value = after ?? null;
    fileSearch = query;
  } catch (err) {
    if (request === fileRequest && id === selected.value)
      error.value = String(err);
  } finally {
    if (request === fileRequest) loadingFiles.value = false;
  }
}

async function loadDiff(more = false) {
  const from = compareFrom.value;
  const to = selected.value;
  if (from == null || to == null) return;
  loadingDiff.value = true;
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
    if (to !== selected.value || from !== compareFrom.value) return;
    if (!data) throw new Error(`比较发布失败：${response.status}`);
    changes.value = more ? [...changes.value, ...data.changes] : data.changes;
    nextDiff.value = data.nextAfter;
  } catch (err) {
    error.value = String(err);
  } finally {
    loadingDiff.value = false;
  }
}

async function refresh() {
  error.value = "";
  await loadReleases();
  const id = selected.value;
  if (id != null) {
    try {
      const { data, response } = await client.GET(
        "/api/v1/launcher/releases/{id}",
        {
          params: { path: { id } },
        },
      );
      if (!data) throw new Error(`刷新发布失败：${response.status}`);
      releases.value = releases.value.map((item) =>
        item.id === id ? data : item,
      );
    } catch (err) {
      error.value = String(err);
    }
  }
  if (tab.value === "files") await loadFiles("refresh");
}

watch(selected, () => {
  files.value = [];
  changes.value = [];
  nextFile.value = null;
  nextDiff.value = null;
  loadFiles();
  loadDiff();
});
watch(compareFrom, () => {
  changes.value = [];
  nextDiff.value = null;
  loadDiff();
});
onMounted(loadReleases);
useIntervalFn(() => {
  if (
    release.value &&
    !release.value.readyAt &&
    !loading.value &&
    !loadingFiles.value
  )
    refresh();
}, 15000);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-tools {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ak-space-3);
  margin-bottom: var(--ak-space-3);

  .ak-search {
    flex: 1;
    min-width: 200px;
  }
}
.launcher-select {
  @include mono;
  flex: 1;
  min-width: 200px;
  max-width: 100%;
  padding: var(--ak-space-2) var(--ak-space-3);
  border: 1px solid var(--ak-border);
  background: var(--ak-bg-surface);
  color: var(--ak-fg);
}
.launcher-summary {
  padding: var(--ak-space-4);
  border: 1px solid var(--ak-border);
  background: var(--ak-bg-surface);
  margin: var(--ak-space-4) 0;

  h2 {
    margin: 0 0 var(--ak-space-3);
    font-size: var(--ak-fs-h3);
  }
  p {
    overflow-wrap: anywhere;
  }
}
.launcher-progress {
  width: 100%;
  height: 12px;
  margin-bottom: var(--ak-space-3);
  accent-color: var(--ak-accent);
}
.launcher-tabs {
  margin-bottom: var(--ak-space-4);
}
.launcher-table {
  overflow-x: auto;
  border: 1px solid var(--ak-border);
}
.launcher-path {
  @include mono;
  min-width: 280px;
  overflow-wrap: anywhere;
}
.launcher-hash {
  @include mono;
  white-space: nowrap;
}
.launcher-size {
  @include num;
}
.launcher-dim {
  color: var(--ak-fg-muted);
}
.launcher-error {
  color: var(--ak-danger);
}
.launcher-pending {
  color: var(--ak-warning);
  white-space: nowrap;
}
.launcher-more {
  margin-top: var(--ak-space-3);
}
</style>
