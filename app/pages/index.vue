<template>
  <PageHeader title="版本" en="Versions">
    <PageTool
      v-if="latest && previous(latest)"
      icon="diff"
      :to="diffLink(previous(latest), latest)"
    >
      对比最近两版
    </PageTool>
  </PageHeader>

  <LatestVersion
    v-if="latest"
    :version="latest"
    :has-gamedata="gamedata.has(latest.resVersion)"
  />

  <PaneBox label="全部版本">
    <template #head>
      <AkSearch
        v-model="keyword"
        size="sm"
        placeholder="筛选版本"
        label="筛选版本"
      />
    </template>
    <template #head-end>
      <div class="ak-seg" role="group" aria-label="显示">
        <button
          type="button"
          :aria-pressed="!onlyManifest"
          @click="onlyManifest = false"
        >
          全部
        </button>
        <button
          type="button"
          :aria-pressed="onlyManifest"
          @click="onlyManifest = true"
        >
          有清单
        </button>
      </div>
    </template>

    <AkEmpty v-if="state === 'error'" code="503" title="暂时拿不到版本列表">
      <template #extra>
        <button type="button" class="ak-btn ak-btn--sm" @click="reload">
          <AsIcon name="refresh" :size="16" />重试
        </button>
      </template>
    </AkEmpty>
    <AkEmpty
      v-else-if="state === 'ready' && filtered.length === 0"
      icon="search"
      :title="keyword ? `没有匹配「${keyword}」的版本` : '没有版本'"
    >
      <template v-if="keyword" #extra>
        <button type="button" class="ak-btn ak-btn--sm" @click="keyword = ''">
          清除关键词
        </button>
      </template>
    </AkEmpty>
    <div v-else class="versions">
      <table class="ak-table" :aria-busy="state === 'loading'">
        <thead>
          <tr>
            <th>资源版本</th>
            <th>时间</th>
            <th>状态</th>
            <th><span class="ak-sr-only">操作</span></th>
          </tr>
        </thead>
        <tbody v-if="state === 'loading'">
          <tr v-for="i in 6" :key="i">
            <td><AkSkeleton text :width="220" /></td>
            <td><AkSkeleton text :width="80" /></td>
            <td><AkSkeleton text :width="56" /></td>
            <td></td>
          </tr>
        </tbody>
        <tbody v-for="g in groups" v-else :key="g.client + g.rows[0]!.id">
          <tr>
            <th colspan="4" class="versions__group">{{ g.client }}</th>
          </tr>
          <tr v-for="v in g.rows" :key="v.id">
            <td>
              <RouterLink
                v-if="hasManifest(v)"
                class="versions__res"
                :to="manifestLink(v)"
              >
                {{ v.resVersion }}
              </RouterLink>
              <span v-else class="versions__res">{{ v.resVersion }}</span>
            </td>
            <td class="versions__time">{{ formatResTime(v.resVersion) }}</td>
            <td>
              <AkTag size="sm" :variant="versionStatus(v).variant">{{
                versionStatus(v).label
              }}</AkTag>
            </td>
            <td class="versions__num">
              <span class="versions__actions">
                <template v-for="a in actions(v)" :key="a.icon">
                  <RouterLink
                    v-if="a.to"
                    class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                    :to="a.to"
                    :title="a.label"
                    :aria-label="a.label"
                  >
                    <AsIcon :name="a.icon" :size="16" />
                  </RouterLink>
                  <button
                    v-else
                    type="button"
                    class="ak-btn ak-btn--ghost ak-btn--icon ak-btn--sm"
                    disabled
                    :title="a.label"
                    :aria-label="a.label"
                  >
                    <AsIcon :name="a.icon" :size="16" />
                  </button>
                </template>
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <template v-if="state === 'ready' && filtered.length > PAGE_SIZE" #foot>
      <span class="versions__range">{{ range }}</span>
      <AkPagination
        v-model="page"
        class="versions__pager"
        :item-count="filtered.length"
        :page-size="PAGE_SIZE"
      />
    </template>
  </PaneBox>
</template>

<script setup lang="ts">
import {
  AkEmpty,
  AkPagination,
  AkSearch,
  AkSkeleton,
  AkTag,
} from "@mooncellwiki/prts-design-vue";
import { computed, ref, watch } from "vue";
import { useGamedataDirs } from "~/common/gamedata";
import { formatCount, formatResTime } from "~/common/utils";
import {
  diffLink,
  hasManifest,
  useVersions,
  versionStatus,
} from "~/common/versions";
import AsIcon from "~/components/AsIcon.vue";
import PaneBox from "~/components/layout/PaneBox.vue";
import PageHeader from "~/components/PageHeader.vue";
import PageTool from "~/components/PageTool.vue";
import LatestVersion from "~/components/versions/LatestVersion.vue";
import type { Version } from "~/common/versions";

const PAGE_SIZE = 50;

const { versions, state, latest, previous, reload } = useVersions();
const gamedataDirs = useGamedataDirs();
const gamedata = computed(() => new Set(gamedataDirs.value));

const keyword = ref("");
const onlyManifest = ref(false);
const page = ref(1);

const filtered = computed(() => {
  const q = keyword.value.trim();
  return versions.value.filter(
    (v) =>
      (!onlyManifest.value || hasManifest(v)) &&
      (!q || v.resVersion.includes(q) || v.clientVersion.includes(q)),
  );
});
watch([keyword, onlyManifest], () => {
  page.value = 1;
});

/** The current page, grouped by client version. */
const groups = computed(() => {
  const rows = filtered.value.slice(
    (page.value - 1) * PAGE_SIZE,
    page.value * PAGE_SIZE,
  );
  const result: { client: string; rows: Version[] }[] = [];
  for (const v of rows) {
    const last = result.at(-1);
    if (last?.client === v.clientVersion) last.rows.push(v);
    else result.push({ client: v.clientVersion, rows: [v] });
  }
  return result;
});
const range = computed(() => {
  const from = (page.value - 1) * PAGE_SIZE + 1;
  const to = Math.min(page.value * PAGE_SIZE, filtered.value.length);
  return `${formatCount(from)}–${formatCount(to)} / ${formatCount(filtered.value.length)}`;
});

function manifestLink(v: Version) {
  return {
    path: "/manifest",
    query: { version: v.resVersion },
  };
}
function gamedataLink(v: Version) {
  return {
    path: "/asset",
    query: { path: `gamedata/${v.resVersion}` },
  };
}

/** Row actions; unavailable ones stay in place, disabled, so the column lines up. */
function actions(v: Version) {
  const prev = previous(v);
  return [
    {
      icon: "diff" as const,
      label: prev ? "对比上一版" : "没有上一版",
      to: prev && diffLink(prev, v),
    },
    {
      icon: "bundle" as const,
      label: hasManifest(v) ? "清单" : "没有导入清单",
      to: hasManifest(v) ? manifestLink(v) : undefined,
    },
    {
      icon: "json" as const,
      label: gamedata.value.has(v.resVersion) ? "数据表" : "没有数据表",
      to: gamedata.value.has(v.resVersion) ? gamedataLink(v) : undefined,
    },
  ];
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.versions {
  overflow-x: auto;

  &__group {
    @include group-row;

    font: 700 var(--ak-fs-body) / 1 var(--ak-font-label);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }

  &__res {
    @include mono;

    font-weight: 600;
    color: var(--ak-fg);
    white-space: nowrap;
    text-decoration: none;

    &:visited {
      color: var(--ak-fg);
    }

    &:is(a):hover {
      color: var(--ak-link);
      text-decoration: underline;
    }
  }

  &__time {
    white-space: nowrap;
  }

  &__num {
    @include num;
  }

  &__actions {
    @include row-actions;

    .ak-btn--icon.ak-btn--sm {
      width: 28px;
      min-height: 28px;
    }
  }

  @media (max-width: $bp-phone) {
    .ak-table {
      white-space: nowrap;
    }
  }
}

.versions__range {
  color: var(--ak-fg-muted);
}

.versions__pager {
  margin-left: auto;
}
</style>
