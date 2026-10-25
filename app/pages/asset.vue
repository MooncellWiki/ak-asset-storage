<template>
  <PageHeader title="文件" en="Files" :document-title="title" />

  <AsToolbar>
    <div class="ak-seg ak-seg--lg" role="group" aria-label="根目录">
      <button
        type="button"
        :aria-pressed="area === 'raw'"
        @click="switchArea('raw')"
      >
        解包产物
      </button>
      <button
        type="button"
        :aria-pressed="area === 'gamedata'"
        @click="switchArea('gamedata')"
      >
        数据表
      </button>
    </div>
    <VersionSelect
      v-if="area === 'gamedata'"
      v-model="versionModel"
      :options="versionOptions"
      label="版本"
    />
    <template #end>
      <AkSearch
        v-model="keyword"
        size="sm"
        placeholder="搜索文件"
        label="搜索文件"
      />
    </template>
  </AsToolbar>

  <SplitView side-label="目录">
    <template #side>
      <template v-if="searching">
        <PaneNote v-if="searchError">{{ searchError }}</PaneNote>
        <PaneNote v-else-if="!resultTree"
          ><AkSkeleton text :repeat="6"
        /></PaneNote>
        <PaneNote v-else-if="resultTree.roots.length === 0"
          >没有匹配「{{ keyword.trim() }}」的文件</PaneNote
        >
        <FileTree
          v-else
          label="搜索结果"
          :roots="resultTree.roots"
          :children-of="resultTree.childrenOf"
          :expanded="tree.expanded.value"
          :selected="relativePath"
          expand-all
          :highlight="keyword"
          :empty-note="() => ''"
          @select="go(`${root}/${$event.path}`)"
        />
      </template>
      <PaneNote v-else-if="tree.failed(root)">没有这个目录</PaneNote>
      <PaneNote v-else-if="!roots"><AkSkeleton text :repeat="10" /></PaneNote>
      <FileTree
        v-else
        label="文件目录"
        :roots="roots"
        :children-of="tree.childrenOf"
        :expanded="tree.expanded.value"
        :selected="path"
        :empty-note="(p) => (tree.failed(p) ? '没加载出来' : '空目录')"
        @select="go($event.path)"
        @toggle="tree.toggle"
      />
    </template>
    <template v-if="searching && results" #side-foot>
      <span class="files-dim">
        {{
          results.truncated
            ? `只列出前 ${results.list.length} 项，换个更具体的关键词`
            : `${formatCount(results.list.length)} 项`
        }}
      </span>
    </template>

    <PaneBox v-if="!current"
      ><PaneNote><AkSkeleton text :repeat="10" /></PaneNote
    ></PaneBox>

    <PaneBox v-else-if="current.kind === 'missing'">
      <template #head><PathBar :segments="segments" /></template>
      <AkEmpty code="404" title="没有这个文件或目录">
        <template #extra>
          <RouterLink
            class="ak-btn ak-btn--sm"
            :to="linkFor({ path: parentPath(path) || root })"
            >回到上一级</RouterLink
          >
        </template>
      </AkEmpty>
    </PaneBox>

    <FilePreview
      v-else-if="current.kind === 'file'"
      :entry="current.entry"
      :siblings="current.siblings"
      :segments="segments"
    />

    <PaneBox v-else :label="title">
      <template #head>
        <PathBar
          :segments="segments"
          :copy="`${absoluteUrl(assetUrl(path))}/`"
          copy-label="复制目录直链"
        />
        <span class="files-dim">{{ formatCount(entries.length) }} 项</span>
      </template>
      <template #head-end>
        <AkSelect
          v-model="sortModel"
          class="files-select"
          size="sm"
          :options="SORT_OPTIONS"
          label="排序"
        />
        <div class="ak-btn-group" role="group" aria-label="显示方式">
          <button
            v-for="v in ['grid', 'list'] as const"
            :key="v"
            type="button"
            class="ak-btn ak-btn--sm ak-btn--icon"
            :class="{ 'is-active': view === v }"
            :aria-pressed="view === v"
            :title="v === 'grid' ? '网格' : '列表'"
            :aria-label="v === 'grid' ? '网格' : '列表'"
            @click="setView(v)"
          >
            <AsIcon :name="v" :size="16" />
          </button>
        </div>
      </template>

      <PaneNote v-if="entries.length === 0">空目录</PaneNote>
      <FileGrid
        v-else-if="view === 'grid'"
        :entries="gridEntries"
        :link-for="linkFor"
      />
      <FileTable
        v-else
        :entries="entries"
        :sort="sortKey"
        :link-for="linkFor"
        @sort="setSort"
        @open="go($event.path)"
      />

      <template v-if="view === 'grid' && entries.length > PAGE_SIZES[0]!" #foot>
        <span class="files-dim">{{ range }}</span>
        <AkPagination
          v-model="page"
          class="files-pager"
          :item-count="entries.length"
          :page-size="pageSize"
        />
        <AkSelect
          v-model="pageSizeModel"
          class="files-select"
          size="sm"
          :options="pageSizeOptions"
          label="每页"
        />
      </template>
    </PaneBox>
  </SplitView>
</template>

<script setup lang="ts">
import {
  AkEmpty,
  AkPagination,
  AkSearch,
  AkSelect,
  AkSkeleton,
} from "@mooncellwiki/prts-design-vue";
import { refDebounced } from "@vueuse/core";
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { client } from "~/common/client";
import { useGamedataDirs } from "~/common/gamedata";
import {
  absoluteUrl,
  assetUrl,
  fileKind,
  formatCount,
  parentPath,
} from "~/common/utils";
import AsIcon from "~/components/AsIcon.vue";
import { entryIcon, sortEntries } from "~/components/files/entries";
import FileGrid from "~/components/files/FileGrid.vue";
import FilePreview from "~/components/files/FilePreview.vue";
import FileTable from "~/components/files/FileTable.vue";
import AsToolbar from "~/components/layout/AsToolbar.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import VersionSelect from "~/components/layout/VersionSelect.vue";
import PageHeader from "~/components/PageHeader.vue";
import PathBar from "~/components/PathBar.vue";
import FileTree from "~/components/tree/FileTree.vue";
import { useLazyTree } from "~/components/tree/lazyTree";
import { treeFromPaths } from "~/components/tree/model";
import type { AssetEntry, SortKey } from "~/components/files/entries";
import type { PathSegment } from "~/components/PathBar.vue";
import type { TreeNode } from "~/components/tree/model";

const route = useRoute();
const router = useRouter();
const gamedataDirs = useGamedataDirs();

const LATEST = "latest";

// ── location: ?path=raw/… or ?path=gamedata/<version>/… ──
const path = computed(() => {
  const p =
    typeof route.query.path === "string"
      ? route.query.path.replace(/^\/+|\/+$/gu, "")
      : "";
  if (p === "gamedata") return `gamedata/${LATEST}`;
  return p === "raw" || p.startsWith("raw/") || p.startsWith("gamedata/")
    ? p
    : "raw";
});
const area = computed(() =>
  path.value.startsWith("gamedata/") ? "gamedata" : "raw",
);
const gamedataVersion = computed(() =>
  area.value === "gamedata" ? path.value.split("/")[1]! : undefined,
);
/** The tree root: `raw` or `gamedata/<version>` */
const root = computed(() =>
  area.value === "gamedata" ? `gamedata/${gamedataVersion.value}` : "raw",
);
const title = computed(() => path.value.split("/").at(-1) ?? "文件");

function linkFor(e: { path: string }) {
  return {
    query: { ...route.query, path: e.path },
  };
}
function go(p: string) {
  void router.push(linkFor({ path: p }));
}
function switchArea(to: "raw" | "gamedata") {
  if (to !== area.value) go(to === "raw" ? "raw" : `gamedata/${LATEST}`);
}
const versionOptions = computed(() => [
  { label: "latest", value: LATEST },
  ...gamedataDirs.value.map((d) => ({ label: d, value: d })),
]);
const versionModel = computed({
  get: () => gamedataVersion.value ?? null,
  set: (v) => {
    if (v == null) return;
    // Stay at the same place inside the version, so tables compare across versions.
    const rest = path.value.split("/").slice(2).join("/");
    go(rest ? `gamedata/${v}/${rest}` : `gamedata/${v}`);
  },
});

// ── directory listings, shared by the tree and the main pane ──
const listings = new Map<string, Promise<AssetEntry[]>>();
function listDir(dir: string): Promise<AssetEntry[]> {
  let request = listings.get(dir);
  if (!request) {
    request = client
      .GET("/api/v1/files/{path}", { params: { path: { path: dir } } })
      .then(({ data }) => {
        if (!data) throw new Error(`no directory at ${dir}`);
        return data.children;
      });
    request.catch(() => listings.delete(dir));
    listings.set(dir, request);
  }
  return request;
}

const tree = useLazyTree(async (dir) =>
  (await listDir(dir)).map<TreeNode>((e) => ({
    name: e.name,
    path: e.path,
    isDir: e.is_dir,
    icon: entryIcon(e),
  })),
);
const roots = computed(() => tree.childrenOf(root.value));
watch(
  root,
  (r) => {
    tree.reset();
    void tree.load(r);
  },
  { immediate: true },
);
watch(
  path,
  (p) => {
    if (p !== root.value) void tree.reveal(p, root.value);
  },
  { immediate: true },
);

// ── what the path points at ──
type Current =
  | { kind: "dir"; entries: AssetEntry[] }
  | { kind: "file"; entry: AssetEntry; siblings: AssetEntry[] }
  | { kind: "missing" };
const current = shallowRef<Current>();
let request = 0;
watch(
  path,
  async (p) => {
    const seq = ++request;
    current.value = undefined;
    try {
      if (p === root.value) {
        const entries = await listDir(p);
        if (seq === request) current.value = { kind: "dir", entries };
        return;
      }
      const siblings = await listDir(parentPath(p));
      const entry = siblings.find((e) => e.path === p);
      if (!entry) {
        if (seq === request) current.value = { kind: "missing" };
      } else if (entry.is_dir) {
        const entries = await listDir(p);
        if (seq === request) current.value = { kind: "dir", entries };
      } else if (seq === request) {
        current.value = { kind: "file", entry, siblings };
      }
    } catch {
      if (seq === request) current.value = { kind: "missing" };
    }
  },
  { immediate: true },
);

const segments = computed<PathSegment[]>(() => {
  const parts = path.value.split("/");
  const first = area.value === "gamedata" ? 2 : 1;
  return parts.slice(first - 1).map((label, i) => ({
    label,
    to: linkFor({ path: parts.slice(0, first + i).join("/") }),
  }));
});

// ── directory view: grid (paged) or list (sortable, not paged) ──
const sortKey = computed<SortKey>(() => {
  const s = route.query.sort;
  return s === "mtime" || s === "size" ? s : "name";
});
const SORT_OPTIONS = [
  { label: "按名称", value: "name" },
  { label: "按修改时间", value: "mtime" },
  { label: "按大小", value: "size" },
];
function setSort(key: string | number | null | undefined) {
  void router.replace({
    query: { ...route.query, sort: key === "name" ? undefined : String(key) },
  });
}
const sortModel = computed({ get: () => sortKey.value, set: setSort });
const entries = computed(() =>
  current.value?.kind === "dir"
    ? sortEntries(current.value.entries, sortKey.value)
    : [],
);
/** Grid where there is something to look at, list otherwise. */
const view = computed<"grid" | "list">(() => {
  const v = route.query.view;
  if (v === "grid" || v === "list") return v;
  return entries.value.some((e) => fileKind(e.name) === "image")
    ? "grid"
    : "list";
});
function setView(v: "grid" | "list") {
  void router.replace({ query: { ...route.query, view: v } });
}
const PAGE_SIZES = [60, 120, 240];
const pageSize = ref(60);
const page = ref(1);
watch([path, sortKey, pageSize], () => {
  page.value = 1;
});
const gridEntries = computed(() =>
  entries.value.slice(
    (page.value - 1) * pageSize.value,
    page.value * pageSize.value,
  ),
);
const pageSizeOptions = PAGE_SIZES.map((n) => ({
  label: `每页 ${n}`,
  value: n,
}));
const pageSizeModel = computed({
  get: () => pageSize.value,
  set: (v) => {
    pageSize.value = Number(v);
  },
});
const range = computed(
  () =>
    `${formatCount((page.value - 1) * pageSize.value + 1)}–${formatCount(
      Math.min(page.value * pageSize.value, entries.value.length),
    )} / ${formatCount(entries.value.length)}`,
);

// ── search: results filter the tree (non-matches hidden, matches expanded) ──
const keyword = ref("");
const q = refDebounced(keyword, 300);
const results = shallowRef<{ list: AssetEntry[]; truncated: boolean }>();
const searchError = ref("");
let searchSeq = 0;
watch([q, root], async ([text, r]) => {
  const seq = ++searchSeq;
  results.value = undefined;
  searchError.value = "";
  if (!text.trim()) return;
  try {
    const { data, response } = await client.GET("/api/v1/files", {
      params: { query: { path: text.trim(), prefix: `${r}/`, limit: 200 } },
    });
    if (seq !== searchSeq) return;
    if (data) results.value = { list: data.results, truncated: data.truncated };
    else
      searchError.value =
        response.status === 503
          ? "搜索繁忙或索引还在准备，稍后再试"
          : "搜索失败，稍后再试";
  } catch {
    if (seq === searchSeq) searchError.value = "连不上搜索服务";
  }
});
/**
 * Result path inside the current area. `gamedata/latest/` resolves to the
 * version it links to, so gamedata paths drop their version segment.
 */
function inArea(p: string): string | undefined {
  if (area.value === "raw")
    return p.startsWith("raw/") ? p.slice(4) : undefined;
  return p.startsWith("gamedata/")
    ? p.split("/").slice(2).join("/") || undefined
    : undefined;
}
const resultTree = computed(() => {
  if (!results.value) return undefined;
  const entries = results.value.list.flatMap((e) => {
    const rel = inArea(e.path);
    return rel ? [{ path: rel, isDir: e.is_dir, icon: entryIcon(e) }] : [];
  });
  return treeFromPaths(entries);
});
const searching = computed(() => keyword.value.trim() !== "");
const relativePath = computed(() =>
  path.value === root.value ? "" : path.value.slice(root.value.length + 1),
);
</script>

<style scoped lang="scss">
.files-dim {
  color: var(--ak-fg-muted);
}

.files-select {
  width: auto;
}

.files-pager {
  margin-left: auto;
}
</style>
