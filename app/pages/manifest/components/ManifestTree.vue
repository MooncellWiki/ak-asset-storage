<template>
  <div class="h-full flex flex-col">
    <div class="border-b border-gray-200 p-2">
      <NInput
        v-model:value="searchText"
        placeholder="搜索资源..."
        clearable
        size="small"
        @update:value="onSearch"
      >
        <template #suffix>
          <CarbonSearch />
        </template>
      </NInput>
    </div>

    <div class="flex-1 overflow-auto">
      <NTree
        v-if="!isSearching"
        v-model:selected-keys="selectedKeys"
        v-model:expanded-keys="expandedKeys"
        block-line
        :data="treeData"
        :on-load="handleLoad"
        :render-prefix="renderPrefix"
      />
      <NList v-else bordered class="m-2">
        <NListItem
          v-for="item in searchResults"
          :key="item.path"
          class="cursor-pointer hover:bg-gray-100"
          @click="handleSearchSelect(item)"
        >
          <div class="flex items-center gap-2">
            <CarbonDocument
              v-if="item.nodeType === 'file'"
              class="text-gray-500"
            />
            <CarbonFolder v-else class="text-blue-500" />
            <div class="flex-1">
              <div class="font-medium">{{ item.name }}</div>
              <div class="text-xs text-gray-500">{{ item.path }}</div>
            </div>
          </div>
        </NListItem>
      </NList>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDebounceFn } from "@vueuse/core";
import CarbonDocument from "~icons/carbon/document";
import CarbonFolder from "~icons/carbon/folder";
import CarbonSearch from "~icons/carbon/search";
import { computed, h, ref, watch } from "vue";
import { client } from "~/common/client";
import type { components } from "~/common/schema";
import type { TreeOption, TreeRenderProps } from "naive-ui";

type ManifestNodeDto = components["schemas"]["ManifestNodeDto"];

const props = defineProps<{
  versionId?: number;
  treeData: TreeOption[];
}>();

const selectedPath = defineModel<string>({ required: true });

const searchText = ref("");
const isSearching = ref(false);
const searchResults = ref<ManifestNodeDto[]>([]);
const expandedKeys = ref<string[]>([]);

const selectedKeys = computed<string[]>({
  get() {
    return selectedPath.value ? [selectedPath.value] : [];
  },
  set(v) {
    selectedPath.value = v[0] || "";
  },
});

function isDir(nodeType: string) {
  return nodeType === "directory" || nodeType === "both";
}

function renderPrefix({ option }: TreeRenderProps) {
  if (option.isLeaf) {
    return h(CarbonDocument, { class: "text-gray-500" });
  }
  return h(CarbonFolder, { class: "text-blue-500" });
}

async function handleLoad(node: TreeOption) {
  if (props.versionId == null) return;
  const { data } = await client.GET("/api/v1/manifest/{version_id}/children", {
    params: {
      path: { version_id: props.versionId },
      query: { dir: node.key as string },
    },
  });
  node.children = (data ?? []).map(toTreeOption);
}

function toTreeOption(node: ManifestNodeDto): TreeOption {
  const dir = isDir(node.nodeType);
  return {
    key: node.path,
    label: node.name,
    isLeaf: !dir,
    nodeType: node.nodeType,
  };
}

const onSearch = useDebounceFn(async () => {
  if (!searchText.value || props.versionId == null) {
    isSearching.value = false;
    searchResults.value = [];
    return;
  }
  isSearching.value = true;
  const { data } = await client.GET("/api/v1/manifest/{version_id}/search", {
    params: {
      path: { version_id: props.versionId },
      query: { q: searchText.value },
    },
  });
  searchResults.value = data ?? [];
}, 500);

function handleSearchSelect(item: ManifestNodeDto) {
  selectedKeys.value = [item.path];
  isSearching.value = false;
  searchText.value = "";
}

watch(
  selectedPath,
  (newPath) => {
    if (!newPath) return;
    const parts = newPath.split("/");
    if (parts.length <= 1) return;
    const result: string[] = [];
    let cur = "";
    for (let i = 0; i < parts.length - 1; i++) {
      cur = cur ? `${cur}/${parts[i]}` : parts[i]!;
      result.push(cur);
    }
    expandedKeys.value = result;
  },
  { immediate: true },
);
</script>
