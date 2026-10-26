<template>
  <PageHeader title="变更" en="Diff">
    <PageTool
      v-if="pair.right"
      icon="copy"
      @click="
        copy(torappuCommand(pair.right, pair.left), '已复制 torappu 命令')
      "
    >
      复制 torappu 命令
    </PageTool>
    <PageTool icon="link" @click="copyLink">复制链接</PageTool>
  </PageHeader>

  <AsToolbar>
    <VersionSelect
      v-model="leftId"
      :options="options"
      placeholder="基准版本"
      label="基准版本"
    />
    <button
      type="button"
      class="ak-btn ak-btn--icon ak-btn--sm"
      title="交换"
      aria-label="交换两边"
      @click="swap"
    >
      <AsIcon name="swap" :size="16" />
    </button>
    <VersionSelect
      v-model="rightId"
      :options="options"
      placeholder="对比版本"
      label="对比版本"
    />
    <span class="diff-sep"></span>
    <KindToggles
      v-model="kinds"
      :counts="loadState === 'ready' ? counts : undefined"
    />
    <template #end>
      <AkSearch
        v-model="keyword"
        size="sm"
        placeholder="筛选路径"
        label="筛选路径"
      />
    </template>
  </AsToolbar>

  <AkEmpty v-if="state === 'error'" code="503" title="暂时拿不到版本列表">
    <template #extra>
      <button type="button" class="ak-btn ak-btn--sm" @click="reload">
        <AsIcon name="refresh" :size="16" />重试
      </button>
    </template>
  </AkEmpty>
  <AkEmpty
    v-else-if="state === 'ready' && !pair.valid"
    icon="info"
    title="选两个版本来比较"
  />
  <AkEmpty v-else-if="sameVersion" icon="info" title="两边是同一个版本" />
  <AkEmpty
    v-else-if="loadState === 'error'"
    icon="warn"
    title="没拿到 bundle 列表"
  >
    <template #extra>
      <button type="button" class="ak-btn ak-btn--sm" @click="loadLists">
        <AsIcon name="refresh" :size="16" />重试
      </button>
    </template>
  </AkEmpty>
  <AkEmpty
    v-else-if="loadState === 'ready' && changes.length === 0"
    icon="check"
    title="两版之间没有变更"
  />

  <SplitView v-else side-label="目录" narrow>
    <template #side>
      <PaneNote v-if="loadState !== 'ready'"
        ><AkSkeleton text :repeat="8"
      /></PaneNote>
      <FileTree
        v-else
        label="有变更的目录"
        :roots="tree.roots"
        :children-of="tree.childrenOf"
        :expanded="expanded"
        :selected="treeSelected"
        :expand-all="q.trim() !== ''"
        :highlight="q"
        @select="onTreeSelect"
        @toggle="toggle"
      />
    </template>

    <PaneBox label="变更列表">
      <template #head>
        <PathBar :segments="dirSegments" />
        <span class="diff-dim">{{ formatCount(rows.length) }} 项</span>
      </template>
      <template #head-end>
        <AkSwitch v-model="withSubdirs" size="sm">含子目录</AkSwitch>
      </template>
      <PaneNote v-if="loadState !== 'ready'"
        ><AkSkeleton text :repeat="10"
      /></PaneNote>
      <AkEmpty
        v-else-if="rows.length === 0"
        icon="search"
        title="没有符合筛选的变更"
      >
        <template #extra>
          <button type="button" class="ak-btn ak-btn--sm" @click="resetFilters">
            清除筛选
          </button>
        </template>
      </AkEmpty>
      <ChangeTable
        v-else
        :rows="pageRows"
        :selected="bundle"
        @select="setQuery({ bundle: $event })"
      />
      <template v-if="rows.length > PAGE_SIZE" #foot>
        <span class="diff-dim">{{ range }}</span>
        <AkPagination
          v-model="page"
          class="diff-pager"
          :item-count="rows.length"
          :page-size="PAGE_SIZE"
        />
      </template>
    </PaneBox>

    <template v-if="detail && bundle && pair.right" #detail>
      <BundleDetail
        :path="bundle"
        :left="pair.left"
        :right="pair.right"
        :before="detail.before"
        :after="detail.after"
        :kind="detail.kind"
        @close="setQuery({ bundle: undefined })"
      />
    </template>
  </SplitView>
</template>

<script setup lang="ts">
import {
  AkEmpty,
  AkPagination,
  AkSearch,
  AkSkeleton,
  AkSwitch,
} from "@mooncellwiki/prts-design-vue";
import { refDebounced } from "@vueuse/core";
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  buildChangeTree,
  diffBundles,
  loadVersionBundles,
} from "~/common/bundles";
import { useCopy } from "~/common/clipboard";
import { formatCount, parentPath, splitPath } from "~/common/utils";
import { torappuCommand, useVersions, versionOptions } from "~/common/versions";
import AsIcon from "~/components/AsIcon.vue";
import BundleDetail from "~/components/diff/BundleDetail.vue";
import ChangeTable from "~/components/diff/ChangeTable.vue";
import KindToggles from "~/components/diff/KindToggles.vue";
import AsToolbar from "~/components/layout/AsToolbar.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import VersionSelect from "~/components/layout/VersionSelect.vue";
import PageHeader from "~/components/PageHeader.vue";
import PageTool from "~/components/PageTool.vue";
import PathBar from "~/components/PathBar.vue";
import FileTree from "~/components/tree/FileTree.vue";
import type { Bundle, Change, ChangeKind, DirNode } from "~/common/bundles";
import type { PathSegment } from "~/components/PathBar.vue";
import type { TreeNode } from "~/components/tree/model";

const PAGE_SIZE = 100;

const route = useRoute();
const router = useRouter();
const copy = useCopy();
const { versions, state, latest, byRes, byId, previous, reload } =
  useVersions();

type QueryPatch = Record<string, string | undefined>;
function queryWith(patch: QueryPatch) {
  const query: Record<string, string> = {};
  for (const [k, v] of Object.entries({ ...route.query, ...patch }))
    if (typeof v === "string" && v !== "") query[k] = v;
  return { query };
}
function setQuery(patch: QueryPatch) {
  void router.replace(queryWith(patch));
}

// ── versions: ?diff=<left res>...<right res>, the latest two by default ──
// An empty left side compares against nothing (everything in `right` is
// added); the oldest version's bundle history links there.
const pair = computed(() => {
  const [l = "", r = ""] = String(route.query.diff ?? "").split("...");
  const left = byRes(l);
  const right = byRes(r);
  return { left, right, valid: Boolean(right && (left || l === "")) };
});
watch(
  [state, () => route.query.diff],
  () => {
    if (state.value !== "ready" || route.query.diff || !latest.value) return;
    setQuery({
      diff: `${previous(latest.value)?.resVersion ?? ""}...${latest.value.resVersion}`,
    });
  },
  { immediate: true },
);
const options = computed(() => versionOptions(versions.value));
function pick(side: "left" | "right", id: string | number | null | undefined) {
  const v = byId(Number(id));
  const { left, right } = pair.value;
  const l = side === "left" ? v : left;
  const r = side === "right" ? v : right;
  setQuery({ diff: `${l?.resVersion ?? ""}...${r?.resVersion ?? ""}` });
}
const leftId = computed({
  get: () => pair.value.left?.id ?? null,
  set: (id) => pick("left", id),
});
const rightId = computed({
  get: () => pair.value.right?.id ?? null,
  set: (id) => pick("right", id),
});
function swap() {
  const { left, right } = pair.value;
  setQuery({ diff: `${right?.resVersion ?? ""}...${left?.resVersion ?? ""}` });
}

// ── bundle lists of both versions ──
const before = shallowRef<Bundle[]>();
const after = shallowRef<Bundle[]>();
const loadState = ref<"idle" | "loading" | "ready" | "error">("idle");
let request = 0;
async function loadLists() {
  const { left, right, valid } = pair.value;
  const seq = ++request;
  if (!valid || !right) {
    loadState.value = "idle";
    return;
  }
  loadState.value = "loading";
  try {
    const [a, b] = await Promise.all([
      left ? loadVersionBundles(left.id) : [],
      loadVersionBundles(right.id),
    ]);
    if (seq !== request) return;
    before.value = a;
    after.value = b;
    loadState.value = "ready";
  } catch {
    if (seq === request) loadState.value = "error";
  }
}
watch(
  () => [pair.value.left?.id, pair.value.right?.id, pair.value.valid],
  loadLists,
  { immediate: true },
);

const changes = computed<Change[]>(() =>
  loadState.value === "ready" && before.value && after.value
    ? diffBundles(before.value, after.value)
    : [],
);

// ── filters: change kinds and a path substring ──
const ALL_KINDS: Record<ChangeKind, boolean> = {
  add: true,
  chg: true,
  rem: true,
};
const kinds = ref({ ...ALL_KINDS });
const keyword = ref("");
const q = refDebounced(keyword, 150);
const matched = computed(() => {
  const needle = q.value.trim().toLowerCase();
  return needle
    ? changes.value.filter((c) => c.path.toLowerCase().includes(needle))
    : changes.value;
});
const counts = computed(() => {
  const result: Record<ChangeKind, number> = { add: 0, chg: 0, rem: 0 };
  for (const c of matched.value) result[c.kind]++;
  return result;
});
const visible = computed(() =>
  matched.value.filter((c) => kinds.value[c.kind]),
);
/** The change tree as FileTree nodes: directories with counts, files with their mark. */
const tree = computed(() => {
  const root = buildChangeTree(visible.value);
  const children = new Map<string, TreeNode[]>();
  const walk = (dir: DirNode) => {
    const dirs = [...dir.dirs.values()].sort((a, b) =>
      a.name < b.name ? -1 : 1,
    );
    children.set(dir.path, [
      ...dirs.map((d) => ({
        name: d.name,
        path: d.path,
        isDir: true,
        counts: d.counts,
      })),
      ...dir.files.map((f) => ({
        name: splitPath(f.path).name,
        path: f.path,
        isDir: false,
        mark: f.kind,
      })),
    ]);
    dirs.forEach(walk);
  };
  walk(root);
  return {
    roots: [
      { name: "全部", path: "", isDir: true, counts: root.counts, keep: true },
    ],
    childrenOf: (path: string) => children.get(path),
  };
});
function resetFilters() {
  keyword.value = "";
  kinds.value = { ...ALL_KINDS };
}

// ── tree: ?dir=<directory>&bundle=<path> ──
const dir = computed(() =>
  typeof route.query.dir === "string" ? route.query.dir : "",
);
const bundle = computed(() =>
  typeof route.query.bundle === "string" ? route.query.bundle : undefined,
);
const expanded = ref(new Set<string>([""]));
function toggle(path: string, open = !expanded.value.has(path)) {
  const next = new Set(expanded.value);
  if (open) next.add(path);
  else next.delete(path);
  expanded.value = next;
}
watch(
  dir,
  (d) => {
    const next = new Set(expanded.value);
    for (let p = d; p; p = parentPath(p)) next.add(p);
    expanded.value = next;
  },
  { immediate: true },
);
/** The tree highlights whatever was picked last: a directory or a bundle. */
const treeSelected = ref(bundle.value ?? dir.value);
watch(dir, (d) => {
  treeSelected.value = d;
});
watch(bundle, (b) => {
  if (b) treeSelected.value = b;
});
function onTreeSelect(node: TreeNode) {
  if (node.isDir) setQuery({ dir: node.path || undefined });
  else setQuery({ bundle: node.path });
}

// ── table: changes in the selected directory, added first ──
const withSubdirs = ref(true);
const rows = computed(() => {
  const d = dir.value;
  const inDir = (c: Change) =>
    withSubdirs.value
      ? d === "" || c.path.startsWith(`${d}/`)
      : parentPath(c.path) === d;
  const order = { add: 0, chg: 1, rem: 2 };
  return visible.value
    .filter(inDir)
    .sort((a, b) => order[a.kind] - order[b.kind]);
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

const dirSegments = computed<PathSegment[]>(() => {
  const segments: PathSegment[] = [
    { label: "全部", to: queryWith({ dir: undefined }) },
  ];
  const parts = dir.value ? dir.value.split("/") : [];
  parts.forEach((name, i) => {
    segments.push({
      label: name,
      to: queryWith({ dir: parts.slice(0, i + 1).join("/") }),
    });
  });
  return segments;
});

// ── detail: the selected bundle, changed or not ──
const detail = computed(() => {
  const path = bundle.value;
  if (!path || loadState.value !== "ready") return undefined;
  const change = changes.value.find((c) => c.path === path);
  if (change)
    return { kind: change.kind, before: change.old, after: change.new };
  return {
    kind: undefined,
    before: before.value?.find((b) => b.path === path),
    after: after.value?.find((b) => b.path === path),
  };
});

const sameVersion = computed(
  () => pair.value.left && pair.value.left.id === pair.value.right?.id,
);
function copyLink() {
  void copy(location.href, "已复制链接");
}
</script>

<style scoped lang="scss">
.diff-sep {
  width: 1px;
  height: 24px;
  margin: 0 var(--ak-space-1);
  background: var(--ak-border);
}

.diff-dim {
  color: var(--ak-fg-muted);
}

.diff-pager {
  margin-left: auto;
}
</style>
