<template>
  <div class="asset-tree h-full flex flex-col">
    <div class="border-b border-gray-200 p-2">
      <NInput
        v-model:value="searchText"
        placeholder="输入资源路径关键词..."
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
        :on-load="props.onLoad"
        :pattern="searchText"
      />
      <div v-else class="m-2 space-y-2">
        <NAlert v-if="searchError" type="error" :show-icon="false">
          {{ searchError }}
        </NAlert>
        <NAlert v-if="truncated" type="warning" :show-icon="false">
          仅显示前 {{ searchResults.length }} 条结果，请输入更具体的关键词。
        </NAlert>
        <div v-if="searchLoading" class="p-2 text-gray-500">搜索中...</div>
        <NEmpty
          v-else-if="!searchError && !searchResults.length"
          description="没有匹配的资源"
          class="py-4"
        />
        <NList v-if="searchResults.length" bordered>
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
  </div>
</template>

<script setup lang="ts">
import { useDebounceFn } from "@vueuse/core";
import { computed, ref, watch } from "vue";
import CarbonDocument from "~icons/carbon/document";
import CarbonFolder from "~icons/carbon/folder";
import CarbonSearch from "~icons/carbon/search";
import { client } from "~/common/client";
import { getParentPaths } from "~/common/utils";
import type { TreeNode } from "../types";
import type { components } from "~/common/schema";

type AssetEntry = components["schemas"]["AssetEntry"];

const props = defineProps<{
  treeData: TreeNode[];
  onLoad: (node: TreeNode) => Promise<void>;
}>();
const selectedPath = defineModel<string>({ required: true });
const searchText = ref("");
const isSearching = ref(false);
const searchResults = ref<AssetEntry[]>([]);
const truncated = ref(false);
const searchLoading = ref(false);
const searchError = ref("");
let searchRequest = 0;
const selectedKeys = computed<string[]>({
  get() {
    return [selectedPath.value];
  },
  set(v) {
    selectedPath.value = v[0] || "";
  },
});
const expandedKeys = ref<string[]>([]);

const search = useDebounceFn(async (query: string, request: number) => {
  if (request !== searchRequest) return;
  try {
    const { data, response } = await client.GET("/api/v1/files", {
      params: { query: { path: query } },
    });
    if (request !== searchRequest) return;
    if (!data) {
      searchError.value =
        response.status === 503
          ? "搜索繁忙或索引准备中，请稍后重试。"
          : "搜索失败，请稍后重试。";
      return;
    }
    searchResults.value = data.results;
    truncated.value = data.truncated;
  } catch {
    if (request === searchRequest)
      searchError.value = "无法连接搜索服务，请稍后重试。";
  } finally {
    if (request === searchRequest) searchLoading.value = false;
  }
}, 500);

function onSearch() {
  const request = ++searchRequest;
  const query = searchText.value;
  isSearching.value = Boolean(query);
  searchResults.value = [];
  truncated.value = false;
  searchError.value = "";
  searchLoading.value = Boolean(query);
  if (query) void search(query, request);
}

function handleSearchSelect(item: AssetEntry) {
  selectedPath.value = item.path;
}

watch(
  selectedPath,
  (newPath) => {
    expandedKeys.value = newPath ? getParentPaths(newPath) : [];
  },
  { immediate: true },
);
</script>
