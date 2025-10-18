<template>
  <div class="asset-tree h-full flex flex-col">
    <div class="border-b border-gray-200 p-2">
      <NInput
        v-model:value="searchText"
        placeholder="搜索文件..."
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
        :pattern="searchText"
      />
      <NList v-else bordered class="m-2">
        <NListItem
          v-for="item in searchResults"
          :key="item.path"
          class="cursor-pointer hover:bg-gray-100"
          @click="handleSearchSelect(item)"
        >
          <div class="flex items-center gap-2">
            <CarbonDocument v-if="!item.is_dir" class="text-gray-500" />
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
import { computed, ref, watch } from "vue";
import { client } from "~/common/client";
import type { components } from "~/common/schema";
import type { TreeNode } from "../types";
import type { TreeOption } from "naive-ui";

type AssetEntry = components["schemas"]["AssetEntry"];

const props = defineProps<{
  treeData: TreeNode[];
  onLoad: (node: TreeNode) => Promise<void>;
}>();
const selectedPath = defineModel<string>({ required: true });
const searchText = ref("");
const isSearching = ref(false);
const searchResults = ref<AssetEntry[]>([]);
const selectedKeys = computed<string[]>({
  get() {
    return [selectedPath.value];
  },
  set(v) {
    selectedPath.value = v[0] || "";
  },
});
const expandedKeys = ref<string[]>([]);

// Load children for a directory
async function handleLoad(node: TreeOption) {
  await props.onLoad(node as TreeNode);
}

// Search files
const onSearch = useDebounceFn(async () => {
  if (!searchText.value) {
    isSearching.value = false;
    searchResults.value = [];
    return;
  }

  isSearching.value = true;
  const { data } = await client.GET("/api/v1/files", {
    params: { query: { path: searchText.value } },
  });

  searchResults.value = data ?? [];
}, 500);

// Handle search result selection
async function handleSearchSelect(item: AssetEntry) {
  selectedKeys.value = [item.path];
}

// Watch for prop changes and load path if needed
watch(
  selectedPath,
  async (newPath) => {
    const result = [];
    let cur = "";
    const parts = newPath.split("/");
    // 选到最顶级的了
    if (parts.length === 1) {
      expandedKeys.value = [];
      return;
    }
    for (let i = 0; i < parts.length - 1; i++) {
      cur = cur ? `${cur}/${parts[i]}` : parts[i]!;
      result.push(cur);
    }
    expandedKeys.value = result;
  },
  { immediate: true },
);
</script>
