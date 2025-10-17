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
        block-line
        :data="treeData"
        :selected-keys="selectedKeys"
        :expanded-keys="expandedKeys"
        :on-update:selected-keys="handleSelect"
        :on-update:expanded-keys="handleExpand"
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
import { ref, watch } from "vue";
import { client } from "~/common/client";
import type { components } from "~/common/schema";
import type { TreeNode } from "../types";

type AssetEntry = components["schemas"]["AssetEntry"];

const props = defineProps<{
  selectedPath?: string;
  treeData: TreeNode[];
  onLoad: (node: TreeNode) => Promise<void>;
}>();

const emit = defineEmits<{
  select: [path: string, isDir: boolean];
}>();

const searchText = ref("");
const isSearching = ref(false);
const searchResults = ref<AssetEntry[]>([]);
const selectedKeys = ref<string[]>([]);
const expandedKeys = ref<string[]>([]);

// Load children for a directory
function handleLoad(node: TreeNode) {
  props.onLoad(node);
}

// Handle node selection
function handleSelect(keys: string[]) {
  if (keys.length > 0) {
    selectedKeys.value = keys;
    const node = findNodeByKey(props.treeData, keys[0]);
    if (node) {
      emit("select", node.path, node.is_dir);
    }
  }
}

// Handle expansion
function handleExpand(keys: string[]) {
  expandedKeys.value = keys;
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
  isSearching.value = false;
  searchText.value = "";

  // Expand tree to show the selected item
  const pathParts = item.path.split("/");
  const expandKeys: string[] = [];

  for (let i = 1; i < pathParts.length; i++) {
    const partialPath = pathParts.slice(0, i).join("/");
    expandKeys.push(partialPath);

    // Load if needed
    const node = findNodeByKey(props.treeData, partialPath);
    if (node && node.is_dir && (!node.children || node.children.length === 0)) {
      await props.onLoad(node);
    }
  }

  expandedKeys.value = expandKeys;
  selectedKeys.value = [item.path];
  emit("select", item.path, item.is_dir);
}

// Helper to find node by key
function findNodeByKey(nodes: TreeNode[], key: string): TreeNode | null {
  for (const node of nodes) {
    if (node.key === key) return node;
    if (node.children) {
      const found = findNodeByKey(node.children, key);
      if (found) return found;
    }
  }
  return null;
}

// Watch for prop changes and load path if needed
watch(
  () => props.selectedPath,
  async (newPath) => {
    if (newPath) {
      selectedKeys.value = [newPath];

      // Expand tree to show the selected item (like handleSearchSelect)
      const pathParts = newPath
        .replace("./asset/", "")
        .split("/")
        .filter(Boolean);
      const expandKeys: string[] = [];

      for (let i = 0; i < pathParts.length; i++) {
        const partialPath = `./asset/${pathParts.slice(0, i + 1).join("/")}`;
        expandKeys.push(partialPath);

        // Load if needed
        const node = findNodeByKey(props.treeData, partialPath);
        if (
          node &&
          node.is_dir &&
          (!node.children || node.children.length === 0)
        ) {
          await props.onLoad(node);
        }
      }

      expandedKeys.value = expandKeys;
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.asset-tree {
  background: #fff;
}
</style>
