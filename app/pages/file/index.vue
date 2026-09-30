<template>
  <NCard class="mb-2">
    <NForm :label-width="80" label-placement="left">
      <NGrid :cols="24" :x-gap="24">
        <NFormItemGi label="path" :span="6">
          <NInput v-model:value="model.path"></NInput>
        </NFormItemGi>
        <NFormItemGi label="hash" :span="6">
          <NInput v-model:value="model.hash"></NInput>
        </NFormItemGi>
        <NFormItemGi label="version" :span="6">
          <NSelect
            v-model:value="model.version"
            :options="versionOpts"
            clearable
          ></NSelect>
        </NFormItemGi>
        <NFormItemGi :span="6">
          <NButton type="primary" @click="search">搜索</NButton>
        </NFormItemGi>
      </NGrid>
    </NForm>
    <NDataTable
      virtual-scroll
      max-height="500px"
      size="small"
      :data="list"
      :columns="columns"
    ></NDataTable>
    <div v-if="nextCursor && !formDirty" class="mt-2 flex justify-center">
      <NButton :loading="loading" :disabled="loading" @click="loadMore">
        加载更多
      </NButton>
    </div>
    <div
      v-else-if="nextCursor && formDirty"
      class="mt-2 text-center text-xs opacity-60"
    >
      搜索条件已修改，重新搜索后才能继续加载
    </div>
  </NCard>
  <NModal v-model:show="detailVisible" display-directive="if">
    <NCard class="w-fit">
      <FileDetailVue :detail="detail!" />
    </NCard>
  </NModal>
</template>
<script lang="ts" setup>
import { NButton } from "naive-ui";
import { computed, h, onBeforeMount, ref } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import FileDetailVue from "../components/FileDetail.vue";
import type { TableColumns } from "naive-ui/es/data-table/src/interface";
import type { components } from "~/common/schema";

const model = ref<{
  path?: string;
  hash?: string;
  version?: number;
}>({});
const { versionOpts, load } = useVersionSelect();
onBeforeMount(() => {
  load();
});
const list = ref<components["schemas"]["BundleDetailsDto"][]>([]);
const nextCursor = ref<string | null | undefined>();
const loading = ref(false);
// The cursor is only valid for the conditions that produced it; when the
// inputs drift from the last fetched query, load-more is hidden until the
// user searches again (the backend rejects the mix as 400 regardless).
const lastQuery = ref<string>();

function snapshot(): string {
  return JSON.stringify({
    path: model.value.path || undefined,
    hash: model.value.hash || undefined,
    version: model.value.version ?? undefined,
  });
}

const formDirty = computed(() => snapshot() !== lastQuery.value);

// Monotonic request id: only the newest request may apply its response, so
// a slow page of an older search can neither append to nor overwrite the
// list/cursor of a newer one.
let requestSeq = 0;

// Results are paginated (newest version first, then path ascending); a
// non-null nextCursor means more pages exist for the current filter.
async function fetchPage(cursor?: string) {
  // Capture what this request actually searches BEFORE dispatch: reading
  // the form again on response would mislabel results as whatever the
  // form drifted to while the request was in flight.
  const query = snapshot();
  const seq = ++requestSeq;
  loading.value = true;
  try {
    const { data } = await client.GET("/api/v1/bundle", {
      params: { query: { ...model.value, limit: 100, cursor } },
    });
    if (seq !== requestSeq) {
      return;
    }
    const page = data ?? { bundles: [] };
    if (cursor) {
      list.value.push(...page.bundles);
    } else {
      list.value = page.bundles;
    }
    nextCursor.value = page.nextCursor;
    lastQuery.value = query;
  } finally {
    if (seq === requestSeq) {
      loading.value = false;
    }
  }
}

async function search() {
  if (model.value.path || model.value.hash || model.value.version) {
    await fetchPage();
  }
}

async function loadMore() {
  if (nextCursor.value) {
    await fetchPage(nextCursor.value);
  }
}
const columns: TableColumns<components["schemas"]["BundleDetailsDto"]> = [
  { title: "path", key: "path" },
  { title: "hash", key: "fileHash", width: 550 },
  { title: "clientVersion", key: "versionClient", width: 140 },
  { title: "resVersion", key: "versionRes", width: 250 },
  {
    key: "action",
    title: "操作",
    width: 140,
    render(row) {
      return h(NButton, { onClick: () => showDetail(row) }, () => "详情");
    },
  },
];
const detail = ref<components["schemas"]["BundleDetailsDto"]>();
const detailVisible = ref(false);
function showDetail(data: components["schemas"]["BundleDetailsDto"]) {
  detail.value = data;
  detailVisible.value = true;
}
</script>
