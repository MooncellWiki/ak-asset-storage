<template>
  <div
    v-if="!path"
    class="h-full flex flex-col items-center justify-center text-gray-400"
  >
    <CarbonDocumentBlank class="mb-4 text-6xl" />
    <p>选择一个文件或目录查看内容</p>
  </div>

  <NSpin v-else-if="loading" class="mt-8 w-full" />

  <template v-else>
    <h3 class="mb-4 text-lg font-semibold">{{ path || "根目录" }}</h3>

    <NCard v-if="detail" hoverable class="mb-4">
      <NDescriptions bordered :column="1" label-placement="left">
        <NDescriptionsItem label="Bundle Path">
          {{ detail.bundlePath }}
        </NDescriptionsItem>
        <NDescriptionsItem label="Asset Path">
          {{ detail.assetPath ?? "-" }}
        </NDescriptionsItem>
        <NDescriptionsItem label="Short Name">
          {{ detail.shortName ?? "-" }}
        </NDescriptionsItem>
      </NDescriptions>
    </NCard>

    <template v-if="children.length > 0">
      <NDataTable
        :columns="dirColumns"
        :data="children"
        :row-props="dirRowProps"
      />
    </template>
    <NEmpty v-else-if="!detail" description="空目录" />
  </template>
</template>

<script setup lang="ts">
import CarbonDocumentBlank from "~icons/carbon/document-blank";
import { ref, watch } from "vue";
import { client } from "~/common/client";
import type { components } from "~/common/schema";
import type { DataTableColumns } from "naive-ui";

const props = defineProps<{
  versionId?: number;
}>();

const path = defineModel<string>({ required: true });

const loading = ref(false);
const detail = ref<components["schemas"]["AssetMappingDetailDto"]>();
const children = ref<DirRow[]>([]);

function isDir(nodeType: string) {
  return nodeType === "directory" || nodeType === "both";
}

interface DirRow {
  name: string;
  path: string;
  isDir: boolean;
}

const dirColumns: DataTableColumns<DirRow> = [
  {
    key: "name",
    title: "名称",
    render: (row) => (row.isDir ? `📁 ${row.name}` : `📄 ${row.name}`),
  },
  {
    key: "type",
    title: "类型",
    render: (row) => (row.isDir ? "目录" : "文件"),
  },
];

function dirRowProps(row: DirRow) {
  return {
    style: "cursor: pointer;",
    onClick: () => {
      path.value = row.path;
    },
  };
}

async function loadDetail() {
  if (!path.value || props.versionId == null) {
    detail.value = undefined;
    return;
  }
  const { data } = await client.GET("/api/v1/manifest/{version_id}/detail", {
    params: {
      path: { version_id: props.versionId },
      query: { asset_name: path.value },
    },
  });
  detail.value = data ?? undefined;
}

async function loadChildren() {
  if (!path.value || props.versionId == null) {
    children.value = [];
    return;
  }
  const { data } = await client.GET("/api/v1/manifest/{version_id}/children", {
    params: {
      path: { version_id: props.versionId },
      query: { dir: path.value },
    },
  });
  children.value = (data ?? []).map((n) => ({
    name: n.name,
    path: n.path,
    isDir: isDir(n.nodeType),
  }));
}

watch(
  [path, () => props.versionId],
  async () => {
    if (!path.value || props.versionId == null) {
      detail.value = undefined;
      children.value = [];
      return;
    }
    loading.value = true;
    await Promise.all([loadDetail(), loadChildren()]);
    loading.value = false;
  },
  { immediate: true },
);
</script>
