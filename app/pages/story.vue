<template>
  <PageHeader
    title="剧情资源"
    en="Story"
    :document-title="selectedId ?? '剧情资源'"
  />

  <div class="ak-tabs story-tabs" role="tablist" aria-label="类型">
    <button
      v-for="t in TYPES"
      :key="t.type"
      type="button"
      class="ak-tab"
      :class="{ 'is-active': t.type === type }"
      role="tab"
      :aria-selected="t.type === type"
      @click="selectType(t.type)"
    >
      {{ t.label }}
    </button>
  </div>

  <SplitView :side-label="typeLabel">
    <template #side-head>
      <AkSearch
        v-model="keyword"
        size="sm"
        :placeholder="`搜索${typeLabel}`"
        :label="`搜索${typeLabel}`"
      />
    </template>
    <template #side>
      <PaneNote v-if="listState === 'error' && resources.length === 0">
        没加载出来，<a href="#" @click.prevent="loadResources()">重试</a>
      </PaneNote>
      <PaneNote v-else-if="listState === 'loading' && resources.length === 0">
        <AkSkeleton text :repeat="10" />
      </PaneNote>
      <PaneNote v-else-if="resources.length === 0"
        >没有匹配「{{ keyword.trim() }}」的{{ typeLabel }}</PaneNote
      >
      <ResList v-else>
        <ResItem
          v-for="r in resources"
          :key="r.id"
          :to="{ query: { type, id: r.id } }"
          :title="r.id"
          :count="formatCount(r.scriptCount)"
          :selected="r.id === selectedId"
          :thumb="type === 'background' || type === 'image' ? 'wide' : 'square'"
          :contain="type === 'character' || type === 'item'"
        >
          <template #thumb>
            <img
              v-if="!brokenImages.has(r.id)"
              :src="assetUrl(imagePath(type, r.id))"
              alt=""
              loading="lazy"
              decoding="async"
              @error="markBroken(r.id)"
            />
            <AsIcon
              v-else
              :name="type === 'character' ? 'person' : 'image'"
              :size="18"
            />
          </template>
        </ResItem>
      </ResList>
    </template>
    <template v-if="listCursor" #side-foot>
      <button
        type="button"
        class="ak-btn ak-btn--ghost ak-btn--sm ak-btn--block"
        :disabled="listState === 'loading'"
        @click="loadResources(true)"
      >
        加载更多
      </button>
    </template>

    <PaneBox v-if="selectedId" :label="selectedId">
      <div class="story-head">
        <RouterLink v-if="imageLink" class="story-head__img" :to="imageLink">
          <img
            v-if="!brokenImages.has(selectedId)"
            :src="assetUrl(imagePath(type, selectedId))"
            alt=""
            @error="markBroken(selectedId)"
          />
          <AsIcon
            v-else
            :name="type === 'character' ? 'person' : 'image'"
            :size="32"
          />
        </RouterLink>
        <div class="story-head__text">
          <h2 class="story-head__id">{{ selectedId }}</h2>
          <p class="story-head__count">
            {{
              selected ? `${formatCount(selected.scriptCount)} 个剧本使用` : "…"
            }}
          </p>
          <div class="story-head__actions">
            <RouterLink
              v-if="imageLink"
              class="ak-btn ak-btn--sm"
              :to="imageLink"
            >
              <AsIcon name="image" :size="16" />查看图片
            </RouterLink>
            <button
              type="button"
              class="ak-btn ak-btn--ghost ak-btn--sm"
              @click="copy(selectedId, '已复制 id')"
            >
              <AsIcon name="copy" :size="16" />复制 id
            </button>
          </div>
        </div>
      </div>
      <div class="story-filter">
        <AkSearch
          v-model="scriptFilter"
          size="sm"
          placeholder="筛选剧本"
          label="筛选剧本"
        />
        <span v-if="usageState === 'ready'" class="story-dim">
          已加载 {{ formatCount(usages.length)
          }}{{
            selected?.scriptCount
              ? ` / ${formatCount(selected.scriptCount)}`
              : ""
          }}
        </span>
      </div>
      <AkEmpty
        v-if="usageState === 'error' && usages.length === 0"
        icon="warn"
        title="没加载出来"
      >
        <template #extra>
          <button type="button" class="ak-btn ak-btn--sm" @click="loadUsages()">
            <AsIcon name="refresh" :size="16" />重试
          </button>
        </template>
      </AkEmpty>
      <PaneNote v-else-if="usageState === 'loading' && usages.length === 0"
        ><AkSkeleton text :repeat="8"
      /></PaneNote>
      <AkEmpty
        v-else-if="groups.length === 0"
        icon="search"
        :title="scriptFilter ? '没有符合筛选的剧本' : '没有剧本用到它'"
      />
      <table v-else class="ak-table ak-table--compact">
        <tbody v-for="g in groups" :key="g.dir">
          <tr>
            <th :colspan="type === 'character' ? 3 : 2" class="story-group">
              {{ g.name }}
            </th>
          </tr>
          <tr v-for="u in g.items" :key="u.scriptPath">
            <td class="story-path">
              <span>{{ g.dir }}</span
              >{{ splitPath(u.scriptPath).name }}
            </td>
            <td v-if="type === 'character'" class="story-dim">
              {{ u.displayNames.join("、") }}
            </td>
            <td class="story-num">
              <span class="story-actions">
                <RouterLink
                  class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                  :to="scriptLink(u)"
                  title="打开剧本"
                  aria-label="打开剧本"
                >
                  <AsIcon name="book" :size="16" />
                </RouterLink>
              </span>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="usageCursor" class="story-more">
        <button
          type="button"
          class="ak-btn ak-btn--sm"
          :disabled="usageState === 'loading'"
          @click="loadUsages(true)"
        >
          加载更多
        </button>
      </div>
    </PaneBox>
    <AkEmpty
      v-else-if="listState === 'ready'"
      icon="info"
      :title="`从左边选一个${typeLabel}`"
    />
  </SplitView>
</template>

<script setup lang="ts">
import { AkEmpty, AkSearch, AkSkeleton } from "@mooncellwiki/prts-design-vue";
import { refDebounced } from "@vueuse/core";
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { client } from "~/common/client";
import { useCopy } from "~/common/clipboard";
import { assetUrl, formatCount, splitPath } from "~/common/utils";
import AsIcon from "~/components/AsIcon.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PaneNote from "~/components/layout/PaneNote.vue";
import SplitView from "~/components/layout/SplitView.vue";
import PageHeader from "~/components/PageHeader.vue";
import ResItem from "~/components/ResItem.vue";
import ResList from "~/components/ResList.vue";
import type { components } from "~/common/schema";

type ResourceType = components["schemas"]["StoryResourceType"];
type Resource = components["schemas"]["StoryResourceSummary"];
type Usage = components["schemas"]["StoryResourceUsageItem"];

const TYPES: { type: ResourceType; label: string }[] = [
  { type: "background", label: "背景" },
  { type: "character", label: "角色" },
  { type: "image", label: "图片" },
  { type: "item", label: "道具" },
];
const PAGE_SIZE = 50;

const route = useRoute();
const router = useRouter();
const copy = useCopy();

const type = computed<ResourceType>(() => {
  const t = route.query.type;
  return TYPES.some((x) => x.type === t) ? (t as ResourceType) : "background";
});
const selectedId = computed(() =>
  typeof route.query.id === "string" ? route.query.id : undefined,
);
const typeLabel = computed(
  () => TYPES.find((x) => x.type === type.value)!.label,
);

function selectType(t: ResourceType) {
  if (t !== type.value) void router.push({ query: { type: t } });
}

/** Extracted image of a story resource under raw/avg. */
function imagePath(t: ResourceType, id: string): string {
  if (t === "background") return `raw/avg/background/${id}.png`;
  if (t !== "character") return `raw/avg/images/${id}.png`;
  // `base#expression` is one png per expression; `base$body` is the shared body png.
  const hash = id.indexOf("#");
  if (hash >= 0)
    return `raw/avg/characters/${id.slice(0, hash)}/${id.slice(hash + 1)}.png`;
  const base = id.slice(0, id.indexOf("$"));
  return `raw/avg/characters/${base}/${id}.png`;
}
const brokenImages = ref(new Set<string>());
function markBroken(id: string) {
  brokenImages.value = new Set(brokenImages.value).add(id);
}

// ── resource list ──
const keyword = ref("");
const q = refDebounced(keyword, 300);
const resources = shallowRef<Resource[]>([]);
const listCursor = ref<string | null>();
const listState = ref<"loading" | "ready" | "error">("loading");
let listSeq = 0;
async function loadResources(more = false) {
  const seq = ++listSeq;
  if (!more) {
    resources.value = [];
    listCursor.value = undefined;
  }
  listState.value = "loading";
  try {
    const { data } = await client.GET("/api/v1/story-resources", {
      params: {
        query: {
          type: type.value,
          q: q.value.trim() || undefined,
          limit: PAGE_SIZE,
          cursor: more ? (listCursor.value ?? undefined) : undefined,
        },
      },
    });
    if (seq !== listSeq) return;
    if (!data) throw new Error("story resources unavailable");
    resources.value = more
      ? [...resources.value, ...data.resources]
      : data.resources;
    listCursor.value = data.nextCursor;
    listState.value = "ready";
  } catch {
    if (seq === listSeq) listState.value = "error";
  }
}
watch([type, q], () => loadResources(), { immediate: true });
watch(type, () => {
  keyword.value = "";
});

// Open the first resource when none is chosen.
watch([resources, selectedId], ([list, id]) => {
  if (!id && list[0] && !q.value)
    void router.replace({ query: { type: type.value, id: list[0].id } });
});

// ── selected resource ──
const selected = shallowRef<Resource>();
watch(
  [type, selectedId],
  async ([t, id]) => {
    selected.value = undefined;
    if (!id) return;
    const known = resources.value.find((r) => r.id === id && r.type === t);
    if (known) {
      selected.value = known;
      return;
    }
    // Opened from a link: look the id up for its script count.
    const { data } = await client.GET("/api/v1/story-resources", {
      params: { query: { type: t, q: id, limit: 200 } },
    });
    if (selectedId.value === id) {
      selected.value = data?.resources.find((r) => r.id === id) ?? {
        id,
        type: t,
        scriptCount: 0,
      };
    }
  },
  { immediate: true },
);

const usages = shallowRef<Usage[]>([]);
const usageCursor = ref<string | null>();
const usageState = ref<"idle" | "loading" | "ready" | "error">("idle");
let usageSeq = 0;
async function loadUsages(more = false) {
  const id = selectedId.value;
  const seq = ++usageSeq;
  if (!more) {
    usages.value = [];
    usageCursor.value = undefined;
  }
  if (!id) {
    usageState.value = "idle";
    return;
  }
  usageState.value = "loading";
  try {
    const { data } = await client.GET("/api/v1/story-resource-usages", {
      params: {
        query: {
          type: type.value,
          id,
          limit: PAGE_SIZE,
          cursor: more ? (usageCursor.value ?? undefined) : undefined,
        },
      },
    });
    if (seq !== usageSeq) return;
    if (!data) throw new Error("story usages unavailable");
    usages.value = more ? [...usages.value, ...data.usages] : data.usages;
    usageCursor.value = data.nextCursor;
    usageState.value = "ready";
  } catch {
    if (seq === usageSeq) usageState.value = "error";
  }
}
watch([type, selectedId], () => loadUsages(), { immediate: true });

const scriptFilter = ref("");
/** Loaded scripts grouped by their directory (one activity / chapter each). */
const groups = computed(() => {
  const needle = scriptFilter.value.trim().toLowerCase();
  const result: { dir: string; name: string; items: Usage[] }[] = [];
  for (const u of usages.value) {
    if (
      needle &&
      !u.scriptPath.toLowerCase().includes(needle) &&
      !u.displayNames.some((n) => n.toLowerCase().includes(needle))
    )
      continue;
    const { dir } = splitPath(u.scriptPath);
    const last = result.at(-1);
    if (last?.dir === dir) last.items.push(u);
    else
      result.push({
        dir,
        name: dir.replace(/\/$/u, "").split("/").at(-1) || "/",
        items: [u],
      });
  }
  return result;
});
function scriptLink(u: Usage) {
  return {
    path: "/asset",
    query: { path: `gamedata/latest/story/${u.scriptPath}.txt` },
  };
}
const imageLink = computed(() =>
  selectedId.value
    ? {
        path: "/asset",
        query: { path: imagePath(type.value, selectedId.value) },
      }
    : undefined,
);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.story-tabs {
  margin-bottom: var(--ak-space-4);
}

.story-head {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: var(--ak-space-5);
  padding: var(--ak-space-4);
  border-bottom: 1px solid var(--ak-border);

  &__img {
    @include checker;

    flex: none;
    display: grid;
    place-items: center;
    width: 280px;
    max-width: 100%;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border: 1px solid var(--ak-border);
    color: var(--ak-fg-subtle);

    > img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: contain;
    }
  }

  &__text {
    flex: 1;
    min-width: 240px;
  }

  &__id {
    @include mono(var(--ak-fs-h3));

    margin: 0;
    font-weight: 700;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  &__count {
    margin: 4px 0 12px;
    font-size: var(--ak-fs-sm);
    color: var(--ak-fg-muted);
  }

  &__actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ak-space-2);
  }
}

.story-filter {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--ak-space-2) var(--ak-space-3);
  padding: var(--ak-space-3) var(--ak-space-4);
  border-bottom: 1px solid var(--ak-border);

  .ak-search {
    width: min(100%, 260px);
  }
}

.story-group {
  @include group-row;
}

.story-path {
  @include mono;

  overflow-wrap: anywhere;

  > span {
    color: var(--ak-fg-muted);
  }
}

.story-dim {
  color: var(--ak-fg-muted);
}

.story-num {
  @include num;
}

.story-actions {
  @include row-actions;
}

.story-more {
  display: flex;
  justify-content: center;
  padding: var(--ak-space-3);
}
</style>
