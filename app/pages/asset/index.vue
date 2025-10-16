<template>
  <div class="asset-page h-screen flex flex-col">
    <!-- Header with Breadcrumb and Mobile Menu Button -->
    <div class="border-b border-gray-200 p-3">
      <div class="flex items-center gap-2">
        <!-- Mobile Menu Button -->
        <NButton v-if="isMobile" quaternary @click="toggleMobileMenu">
          <template #icon>
            <CarbonMenu v-if="!showMobileMenu" />
            <CarbonClose v-else />
          </template>
        </NButton>

        <!-- Breadcrumb Navigation -->
        <NBreadcrumb v-if="selectedPath">
          <NBreadcrumbItem @click="handleBreadcrumbClick('')">
            <div class="flex items-center gap-1">
              <CarbonHome />
              <span>根目录</span>
            </div>
          </NBreadcrumbItem>
          <NBreadcrumbItem
            v-for="(part, idx) in pathParts"
            :key="idx"
            @click="
              handleBreadcrumbClick(pathParts.slice(0, idx + 1).join('/'))
            "
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
              :selected-path="selectedPath"
              :tree-data="treeData"
              @select="handleTreeSelect"
              @load="handleTreeLoad"
            />
          </div>
        </template>
        <template #2>
          <div class="h-full overflow-hidden">
            <AssetContent
              :path="selectedPath"
              :is-dir="selectedIsDir"
              :dir-content="dirContent"
              :loading="contentLoading"
              @navigate="handleNavigate"
            />
          </div>
        </template>
      </NSplit>

      <!-- Mobile: Content Only -->
      <div v-else class="h-full overflow-hidden">
        <AssetContent
          :path="selectedPath"
          :is-dir="selectedIsDir"
          :dir-content="dirContent"
          :loading="contentLoading"
          @navigate="handleNavigate"
        />
      </div>
    </div>

    <!-- Mobile Drawer -->
    <NDrawer v-model:show="showMobileMenu" :width="300" placement="left">
      <NDrawerContent title="文件浏览">
        <AssetTree
          :selected-path="selectedPath"
          :tree-data="treeData"
          @select="handleTreeSelectMobile"
          @load="handleTreeLoad"
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
import type { components } from "~/common/schema";
import AssetContent from "./components/AssetContent.vue";
import AssetTree from "./components/AssetTree.vue";

type AssetDir = components["schemas"]["AssetDir"];

interface TreeNode {
  key: string;
  label: string;
  path: string;
  is_dir: boolean;
  isLeaf?: boolean;
  children?: TreeNode[];
}

// Responsive design
const breakpoints = useBreakpoints({ mobile: 768 });
const isMobile = breakpoints.smaller("mobile");

// Use route query for path synchronization
const pathQuery = useRouteQuery<string>("path", "");

// State
const selectedPath = ref("");
const selectedIsDir = ref(false);
const showMobileMenu = ref(false);
const treeData = ref<TreeNode[]>([]);
const dirContent = ref<AssetDir | null>(null);
const contentLoading = ref(false);

// Computed
const pathParts = computed(() => {
  if (!selectedPath.value) return [];
  return selectedPath.value.replace("./asset/", "").split("/").filter(Boolean);
});

// Load root directory
async function loadRoot() {
  try {
    const { data, error } = await client.GET("/api/v1/files/{path}", {
      params: { path: { path: "" } },
    });

    if (error) {
      console.error("Failed to load root directory:", error);
      treeData.value = [];
      return;
    }

    if (data?.children) {
      treeData.value = data.children.map((item) => ({
        key: item.path,
        label: item.name,
        path: item.path,
        is_dir: item.is_dir,
        isLeaf: !item.is_dir,
        children: undefined,
      }));
    }
  } catch (error) {
    console.error("Error loading root:", error);
    treeData.value = [];
  }
}

// Load children for a directory (for tree lazy loading)
async function handleTreeLoad(node: TreeNode) {
  const { data } = await client.GET("/api/v1/files/{path}", {
    params: { path: { path: node.path.replace("./asset/", "") } },
  });

  if (data?.children) {
    node.children = data.children.map((item) => ({
      key: item.path,
      label: item.name,
      path: item.path,
      is_dir: item.is_dir,
      isLeaf: !item.is_dir,
      children: undefined,
    }));
  }
}

// Load content for display
async function loadContent(path: string, isDir: boolean) {
  if (!path) {
    dirContent.value = null;
    return;
  }

  contentLoading.value = true;
  try {
    if (isDir) {
      const { data } = await client.GET("/api/v1/files/{path}", {
        params: { path: { path: path.replace("./asset/", "") } },
      });
      dirContent.value = data || null;
    } else {
      // For files, just clear dirContent (AssetContent will handle file loading)
      dirContent.value = null;
    }
  } catch (error) {
    console.error("Error loading content:", error);
    dirContent.value = null;
  } finally {
    contentLoading.value = false;
  }
}

// Handle tree selection
function handleTreeSelect(path: string, isDir: boolean) {
  selectedPath.value = path;
  selectedIsDir.value = isDir;

  // Update URL
  const pathParam = path.replace("./asset/", "");
  pathQuery.value = pathParam;

  // Load content
  loadContent(path, isDir);
}

function handleTreeSelectMobile(path: string, isDir: boolean) {
  handleTreeSelect(path, isDir);
  showMobileMenu.value = false;
}

// Handle navigation from content
function handleNavigate(path: string, isDir: boolean) {
  selectedPath.value = path;
  selectedIsDir.value = isDir;

  // Update URL
  const pathParam = path.replace("./asset/", "");
  pathQuery.value = pathParam;

  // Load content
  loadContent(path, isDir);
}

// Handle breadcrumb click
function handleBreadcrumbClick(path: string) {
  const fullPath = path ? `./asset/${path}` : "";
  handleNavigate(fullPath, true);
}

function toggleMobileMenu() {
  showMobileMenu.value = !showMobileMenu.value;
}

// Watch for URL query changes
watch(
  pathQuery,
  (newPath) => {
    if (newPath) {
      const fullPath = `./asset/${newPath}`;
      if (fullPath !== selectedPath.value) {
        selectedPath.value = fullPath;
        // Determine if it's a directory or file (simple heuristic)
        selectedIsDir.value = !newPath.includes(".");
        loadContent(fullPath, selectedIsDir.value);
      }
    } else {
      selectedPath.value = "";
      selectedIsDir.value = false;
      dirContent.value = null;
    }
  },
  { immediate: true },
);

// Initialize
loadRoot();
</script>

<style scoped>
.asset-page {
  position: relative;
}
</style>
