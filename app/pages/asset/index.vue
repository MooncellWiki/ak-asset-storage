<template>
  <div class="h-[calc(100vh-4.5rem)] flex flex-col overflow-auto">
    <!-- Header with Breadcrumb and Mobile Menu Button -->
    <div class="border-b border-gray-200 p-3">
      <div class="flex items-center gap-2 overflow-auto">
        <!-- Mobile Menu Button -->
        <NButton v-if="isMobile" quaternary @click="toggleMobileMenu">
          <template #icon>
            <CarbonMenu v-if="!showMobileMenu" />
            <CarbonClose v-else />
          </template>
        </NButton>

        <!-- Breadcrumb Navigation -->
        <NBreadcrumb v-if="selectedPath">
          <NBreadcrumbItem @click="handleNavigation('')">
            <NIcon>
              <CarbonHome />
            </NIcon>
            <span>根目录</span>
          </NBreadcrumbItem>
          <NBreadcrumbItem
            v-for="(part, idx) in pathParts"
            :key="idx"
            @click="handleNavigation(pathParts.slice(0, idx + 1).join('/'))"
          >
            {{ part }}
          </NBreadcrumbItem>
        </NBreadcrumb>
        <div v-else class="text-gray-400">资产浏览器</div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 overflow-hidden">
      <NSplit
        v-if="!isMobile"
        direction="horizontal"
        :default-size="0.25"
        :min="0.15"
        :max="0.4"
      >
        <template #1>
          <div class="h-full overflow-hidden">
            <AssetTree
              v-model="selectedPath"
              :tree-data="treeData"
              :on-load="handleTreeLoad"
            />
          </div>
        </template>
        <template #2>
          <div class="relative h-full overflow-auto px-4">
            <div
              v-if="contentLoading"
              class="h-full flex items-center justify-center"
            >
              <NSpin size="large"> </NSpin>
            </div>
            <AssetContent v-else v-model="selectedPath" :node="selectedNode!" />
          </div>
        </template>
      </NSplit>

      <!-- Mobile: Content Only -->
      <div v-else class="relative h-full overflow-auto px-4">
        <div
          v-if="contentLoading"
          class="h-full flex items-center justify-center"
        >
          <NSpin size="large"> </NSpin>
        </div>
        <AssetContent v-else v-model="selectedPath" :node="selectedNode!" />
      </div>
    </div>

    <!-- Mobile Drawer -->
    <NDrawer v-model:show="showMobileMenu" :width="300" placement="left">
      <NDrawerContent title="文件浏览">
        <AssetTree
          v-model="selectedPath"
          :tree-data="treeData"
          :on-load="handleTreeLoad"
          @update:model-value="handleTreeSelectMobile"
        />
      </NDrawerContent>
    </NDrawer>
  </div>
</template>

<script lang="ts" setup>
import { useBreakpoints } from "@vueuse/core";
import { useRouteQuery } from "@vueuse/router";
import CarbonClose from "~icons/carbon/close";
import CarbonHome from "~icons/carbon/home";
import CarbonMenu from "~icons/carbon/menu";
import { computed, ref, watch } from "vue";
import { client } from "~/common/client";
import AssetContent from "./components/AssetContent.vue";
import AssetTree from "./components/AssetTree.vue";
import type { TreeNode } from "./types";

// Responsive design
const breakpoints = useBreakpoints({ mobile: 768 });
const isMobile = breakpoints.smaller("mobile");

// Use route query for path synchronization
const selectedPath = useRouteQuery<string>("path", "");
const selectedNode = ref<TreeNode>();
// State
const showMobileMenu = ref(false);
const treeData = ref<TreeNode[]>([]);
const contentLoading = ref(false);

// Computed
const pathParts = computed(() => {
  if (!selectedPath.value) return [];
  return selectedPath.value.split("/").filter(Boolean);
});

// Unified function to load directory data
async function loadDirectory(path: string): Promise<TreeNode[]> {
  try {
    const { data, error } = await client.GET("/api/v1/files/{path}", {
      params: { path: { path } },
    });

    if (error) {
      console.error(`Failed to load directory ${path}:`, error);
      return [];
    }

    if (data?.children) {
      return data.children.map((item) => ({
        key: item.path,
        label: item.name,
        path: item.path,
        is_dir: item.is_dir,
        size: item.size,
        isLeaf: !item.is_dir,
        children: undefined,
        create_at: item.create_at,
        modified_at: item.modified_at,
      }));
    }
    return [];
  } catch (error) {
    console.error(`Error loading directory ${path}:`, error);
    return [];
  }
}

// Load children for a directory (for tree lazy loading)
async function handleTreeLoad(node: TreeNode) {
  node.children = await loadDirectory(node.path);
}

// Unified function to handle navigation with tree validation
async function handleNavigation(path: string) {
  selectedPath.value = path;
  await ensurePathInTree(path);
}

// Ensure the path exists in the tree by loading necessary parent directories
async function ensurePathInTree(targetPath: string) {
  contentLoading.value = true;
  if (treeData.value.length === 0) {
    treeData.value = await loadDirectory("");
  }
  if (!targetPath) {
    contentLoading.value = false;
    return;
  }
  let list = treeData.value;
  const parts = targetPath.split("/");
  let cur = "";
  for (const part of parts) {
    cur = cur ? `${cur}/${part}` : part;
    const node = list.find((v) => v.path === cur);
    if (!node) {
      throw new Error(`Path not found in tree: ${cur}`);
    }
    if (targetPath === cur) {
      if (node.is_dir) {
        node.children = await loadDirectory(cur);
      }
      selectedNode.value = node;
      contentLoading.value = false;
      return;
    }
    if (!node.is_dir) {
      throw new Error(`${cur} is not a directory`);
    }
    if (!Array.isArray(node.children)) {
      node.children = await loadDirectory(cur);
    }
    list = node.children;
  }
  contentLoading.value = false;
}

function handleTreeSelectMobile() {
  showMobileMenu.value = false;
}

function toggleMobileMenu() {
  showMobileMenu.value = !showMobileMenu.value;
}

// Watch for URL query changes
watch(
  selectedPath,
  (newPath) => {
    ensurePathInTree(newPath);
  },
  { immediate: true },
);
</script>
