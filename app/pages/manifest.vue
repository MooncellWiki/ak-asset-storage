<template>
  <PageHeader
    title="清单"
    en="Manifest"
    :document-title="path ? path.split('/').at(-1) : '清单'"
  >
    <PageTool
      v-if="current && previous(current)"
      icon="diff"
      :to="diffLink(previous(current), current)"
    >
      对比上一版
    </PageTool>
  </PageHeader>

  <AsToolbar>
    <VersionSelect
      v-model="versionId"
      :options="options"
      placeholder="选择版本"
      label="版本"
    />
    <template #end>
      <AkSearch
        v-model="keyword"
        class="manifest-search"
        size="sm"
        placeholder="搜索这个版本里的资源"
        label="搜索资源"
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
    v-else-if="state === 'ready' && route.query.version && !current"
    icon="warn"
    :title="`没有版本 ${route.query.version}`"
  />
  <AkEmpty
    v-else-if="current && !hasManifest(current)"
    icon="info"
    title="这个版本还没有导入清单"
  />

  <SplitView v-else side-label="目录">
    <template #side>
      <template v-if="searching">
        <PaneNote v-if="searchFailed">搜索失败，稍后再试</PaneNote>
        <PaneNote v-else-if="!resultTree"
          ><AkSkeleton text :repeat="6"
        /></PaneNote>
        <PaneNote v-else-if="resultTree.roots.length === 0"
          >没有匹配「{{ keyword.trim() }}」的资源</PaneNote
        >
        <FileTree
          v-else
          label="搜索结果"
          :roots="resultTree.roots"
          :children-of="resultTree.childrenOf"
          :expanded="tree.expanded.value"
          :selected="path"
          expand-all
          :highlight="keyword"
          :empty-note="() => ''"
          @select="navigate($event.path)"
        />
      </template>
      <PaneNote v-else-if="!roots"><AkSkeleton text :repeat="10" /></PaneNote>
      <FileTree
        v-else
        label="清单目录"
        :roots="roots"
        :children-of="tree.childrenOf"
        :expanded="tree.expanded.value"
        :selected="path"
        :empty-note="(p) => (tree.failed(p) ? '没加载出来' : '空目录')"
        @select="navigate($event.path)"
        @toggle="tree.toggle"
      />
    </template>
    <template v-if="searching && results?.length" #side-foot>
      <span class="manifest-dim">{{ formatCount(results.length) }} 条结果</span>
    </template>

    <PaneBox v-if="path" label="资源">
      <template #head>
        <PathBar :segments="segments" :copy="path" copy-label="复制资源名" />
      </template>
      <template v-if="detail?.bundleHash" #head-end>
        <button
          type="button"
          class="ak-btn ak-btn--primary ak-btn--sm"
          @click="saveBundle"
        >
          <AsIcon name="download" :size="16" />下载 bundle
        </button>
      </template>
      <PaneNote v-if="loading && !detail"
        ><AkSkeleton text :repeat="3"
      /></PaneNote>
      <KvList v-else-if="detail">
        <dt>Bundle</dt>
        <dd>
          <span class="manifest-mono">{{ detail.bundlePath }}</span>
          <span v-if="detail.bundleSize != null" class="manifest-dim">{{
            formatSize(detail.bundleSize)
          }}</span>
          <RouterLink
            v-if="historyLink"
            class="ak-btn ak-btn--ghost ak-btn--xs"
            :to="historyLink"
            >历次变化</RouterLink
          >
        </dd>
        <template v-if="detail.assetPath">
          <dt>资源路径</dt>
          <dd class="manifest-mono">{{ detail.assetPath }}</dd>
        </template>
        <template v-if="detail.shortName">
          <dt>短名</dt>
          <dd class="manifest-mono">{{ detail.shortName }}</dd>
        </template>
        <template v-if="detail.bundleHash">
          <dt>SHA-256</dt>
          <dd class="manifest-hash">
            {{ detail.bundleHash }}
            <CopyButton :text="detail.bundleHash" label="复制哈希" />
          </dd>
        </template>
      </KvList>
    </PaneBox>

    <PaneBox :label="listingTitle">
      <template #head>
        <h2 class="manifest-title">
          <AsIcon name="folder" :size="16" />
          <span class="manifest-mono">{{ listingTitle }}</span>
        </h2>
      </template>
      <template v-if="listing" #head-end>
        <span class="manifest-dim">{{ formatCount(listing.length) }} 项</span>
      </template>
      <PaneNote v-if="!listing"><AkSkeleton text :repeat="6" /></PaneNote>
      <PaneNote v-else-if="listing.length === 0">空目录</PaneNote>
      <table v-else class="ak-table ak-table--compact">
        <tbody>
          <tr
            v-for="e in listing"
            :key="e.path"
            class="manifest-row"
            :class="{ 'is-selected': e.path === path }"
            @click="navigate(e.path)"
          >
            <td>
              <RouterLink
                class="manifest-entry"
                :to="{ query: { version: current?.resVersion, path: e.path } }"
                @click.stop
              >
                <AsIcon :name="e.icon ?? 'file'" :size="16" />
                <span class="manifest-mono">{{ e.name }}</span>
              </RouterLink>
            </td>
            <td class="manifest-dim">{{ e.isDir ? "目录" : "资源" }}</td>
          </tr>
        </tbody>
      </table>
    </PaneBox>
  </SplitView>
</template>

<script setup lang="ts">
import { AkEmpty, AkSearch, AkSkeleton } from "@mooncellwiki/prts-design-vue";
import { refDebounced } from "@vueuse/core";
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { client } from "~/common/client";
import {
  download,
  formatCount,
  formatSize,
  parentPath,
  storageUrl,
} from "~/common/utils";
import {
  diffLink,
  hasManifest,
  useVersions,
  versionOptions,
} from "~/common/versions";
import AsIcon from "~/components/AsIcon.vue";
import CopyButton from "~/components/CopyButton.vue";
import KvList from "~/components/KvList.vue";
import AsToolbar from "~/components/layout/AsToolbar.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import VersionSelect from "~/components/layout/VersionSelect.vue";
import PageHeader from "~/components/PageHeader.vue";
import PageTool from "~/components/PageTool.vue";
import PathBar from "~/components/PathBar.vue";
import FileTree from "~/components/tree/FileTree.vue";
import { useLazyTree } from "~/components/tree/lazyTree";
import { treeFromPaths } from "~/components/tree/model";
import type { components } from "~/common/schema";
import type { PathSegment } from "~/components/PathBar.vue";
import type { TreeNode } from "~/components/tree/model";

type ManifestNode = components["schemas"]["ManifestNode"];
type AssetDetail = components["schemas"]["AssetMappingDetails"];

const route = useRoute();
const router = useRouter();
const { versions, state, latestWithManifest, byRes, byId, previous, reload } =
  useVersions();

// ── version: ?version=<res>, the newest with an imported manifest by default ──
const current = computed(() =>
  byRes(route.query.version as string | undefined),
);
const path = computed(() =>
  typeof route.query.path === "string" ? route.query.path : "",
);
function navigate(to: string) {
  void router.push({
    query: { version: current.value?.resVersion, path: to || undefined },
  });
}
watch(
  [state, () => route.query.version],
  () => {
    if (
      state.value === "ready" &&
      !route.query.version &&
      latestWithManifest.value
    )
      void router.replace({
        query: { ...route.query, version: latestWithManifest.value.resVersion },
      });
  },
  { immediate: true },
);
const options = computed(() =>
  versionOptions(versions.value.filter(hasManifest)),
);
const versionId = computed({
  get: () => current.value?.id ?? null,
  set: (id) => {
    const v = byId(Number(id));
    if (v) void router.push({ query: { version: v.resVersion } });
  },
});

// ── tree, loaded one directory at a time ──
function toNode(n: ManifestNode): TreeNode {
  return {
    name: n.name,
    path: n.path,
    isDir: n.nodeType !== "file",
    icon: n.nodeType === "file" ? "file" : "folder",
  };
}
async function fetchChildren(dir: string): Promise<TreeNode[]> {
  const id = current.value?.id;
  if (id == null) return [];
  const { data } = await client.GET("/api/v1/manifest/{version_id}/children", {
    params: { path: { version_id: id }, query: dir ? { dir } : {} },
  });
  if (!data) throw new Error("manifest children unavailable");
  return data.map(toNode);
}
const tree = useLazyTree(fetchChildren);
const roots = computed(() => tree.childrenOf(""));
watch(
  () => current.value?.id,
  (id) => {
    tree.reset();
    if (id == null) return;
    void tree.load("");
    if (path.value) void tree.reveal(path.value);
  },
  { immediate: true },
);
watch(path, (p) => {
  if (p && current.value) void tree.reveal(p);
});

// ── selected asset / directory ──
const detail = shallowRef<AssetDetail>();
const listing = shallowRef<TreeNode[]>();
const listingDir = ref("");
const loading = ref(false);
let request = 0;
watch(
  [() => current.value?.id, path],
  async ([id, p]) => {
    const seq = ++request;
    detail.value = undefined;
    listing.value = undefined;
    if (id == null) return;
    loading.value = true;
    const detailRequest = p
      ? client
          .GET("/api/v1/manifest/{version_id}/detail", {
            params: { path: { version_id: id }, query: { asset_name: p } },
          })
          .then(({ data }) => data)
          .catch(() => undefined)
      : Promise.resolve(undefined);
    const children = await tree.load(p);
    const found = await detailRequest;
    if (seq !== request) return;
    detail.value = found;
    // A directory lists its own children; an asset lists its siblings.
    if (children.length > 0 || !found) {
      listing.value = children;
      listingDir.value = p;
    } else {
      listingDir.value = parentPath(p);
      listing.value = await tree.load(listingDir.value);
    }
    if (seq === request) loading.value = false;
  },
  { immediate: true },
);

const segments = computed<PathSegment[]>(() => {
  const version = current.value?.resVersion;
  const parts = path.value ? path.value.split("/") : [];
  return [
    { label: version ?? "清单", to: { query: { version } } },
    ...parts.map((name, i) => ({
      label: name,
      to: { query: { version, path: parts.slice(0, i + 1).join("/") } },
    })),
  ];
});
const listingTitle = computed(
  () => listingDir.value.split("/").at(-1) || "根目录",
);

function saveBundle() {
  const d = detail.value;
  if (d?.bundleHash) download(storageUrl(d.bundleHash), `${d.bundlePath}.zip`);
}
const historyLink = computed(() =>
  detail.value && current.value
    ? diffLink(previous(current.value), current.value, {
        bundle: detail.value.bundlePath,
      })
    : undefined,
);

// ── search: results filter the tree (non-matches hidden, matches expanded) ──
const keyword = ref("");
const q = refDebounced(keyword, 300);
const results = shallowRef<ManifestNode[]>();
const searchFailed = ref(false);
let searchSeq = 0;
watch([q, () => current.value?.id], async ([text, id]) => {
  const seq = ++searchSeq;
  results.value = undefined;
  searchFailed.value = false;
  if (!text.trim() || id == null) return;
  try {
    const { data } = await client.GET("/api/v1/manifest/{version_id}/search", {
      params: { path: { version_id: id }, query: { q: text.trim() } },
    });
    if (seq !== searchSeq) return;
    if (data) results.value = data;
    else searchFailed.value = true;
  } catch {
    if (seq === searchSeq) searchFailed.value = true;
  }
});
const resultTree = computed(() =>
  results.value
    ? treeFromPaths(
        results.value.map((n) => ({
          path: n.path,
          isDir: n.nodeType !== "file",
          icon: toNode(n).icon,
        })),
      )
    : undefined,
);
const searching = computed(() => keyword.value.trim() !== "");
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.manifest-search {
  width: min(100%, 400px);
}

.manifest-mono {
  @include mono;

  overflow-wrap: anywhere;
}

.manifest-hash {
  @include mono(12px);

  color: var(--ak-fg-muted);
  overflow-wrap: anywhere;
}

.manifest-dim {
  color: var(--ak-fg-muted);
}

.manifest-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin: 0;
  font-size: var(--ak-fs-body);
  font-weight: 700;
}

.manifest-row {
  cursor: pointer;
}

.manifest-entry {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  color: inherit;
  text-decoration: none;

  &:visited {
    color: inherit;
  }

  .ak-icon {
    color: var(--ak-fg-muted);
  }
}
</style>
