<template>
  <div class="asset-page h-screen flex">
    <!-- Left Panel - Tree -->
    <div
      class="overflow-hidden border-r border-gray-200"
      :style="{ width: `${leftWidth}px` }"
    >
      <AssetTree :selected-path="selectedPath" @select="handleTreeSelect" />
    </div>

    <!-- Resizer -->
    <div
      ref="resizer"
      class="resizer w-1 cursor-col-resize bg-gray-200 transition-colors hover:bg-blue-400"
      @mousedown="startResize"
    ></div>

    <!-- Right Panel - Content -->
    <div class="flex-1 overflow-hidden">
      <AssetContent
        :path="selectedPath"
        :is-dir="selectedIsDir"
        @navigate="handleNavigate"
      />
    </div>

    <!-- Mobile Menu Toggle (for small screens) -->
    <div v-if="isMobile" class="fixed left-4 top-4 z-50">
      <NButton @click="toggleMobileMenu">
        <template #icon>
          <CarbonMenu v-if="!showMobileMenu" />
          <CarbonClose v-else />
        </template>
      </NButton>
    </div>

    <!-- Mobile Drawer -->
    <NDrawer v-model:show="showMobileMenu" :width="300" placement="left">
      <NDrawerContent title="文件浏览">
        <AssetTree
          :selected-path="selectedPath"
          @select="handleTreeSelectMobile"
        />
      </NDrawerContent>
    </NDrawer>
  </div>
</template>

<script lang="ts" setup>
import { useBreakpoints } from "@vueuse/core";
import CarbonClose from "~icons/carbon/close";
import CarbonMenu from "~icons/carbon/menu";
import { onMounted, onUnmounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import AssetContent from "./components/AssetContent.vue";
import AssetTree from "./components/AssetTree.vue";

const route = useRoute();
const router = useRouter();

const selectedPath = ref("");
const selectedIsDir = ref(false);
const leftWidth = ref(350);
const showMobileMenu = ref(false);

// Responsive design
const breakpoints = useBreakpoints({ mobile: 768 });
const isMobile = breakpoints.smaller("mobile");

// Resizer logic
const isResizing = ref(false);
const resizer = ref<HTMLElement | null>(null);

function startResize(e: MouseEvent) {
  isResizing.value = true;
  e.preventDefault();
}

function onMouseMove(e: MouseEvent) {
  if (!isResizing.value) return;

  const newWidth = e.clientX;
  if (newWidth >= 200 && newWidth <= 600) {
    leftWidth.value = newWidth;
  }
}

function stopResize() {
  isResizing.value = false;
}

// Handle tree selection
function handleTreeSelect(path: string, isDir: boolean) {
  selectedPath.value = path;
  selectedIsDir.value = isDir;

  // Update URL
  const pathParam = path.replace("./asset/", "");
  router.push({
    path: "/asset",
    query: pathParam ? { path: pathParam } : {},
  });
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
  router.push({
    path: "/asset",
    query: pathParam ? { path: pathParam } : {},
  });
}

function toggleMobileMenu() {
  showMobileMenu.value = !showMobileMenu.value;
}

// Initialize from URL
onMounted(() => {
  // Add resize listeners
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", stopResize);

  // Load from URL query
  const pathFromQuery = route.query.path as string;
  if (pathFromQuery) {
    selectedPath.value = `./asset/${pathFromQuery}`;
    selectedIsDir.value = false; // Will be determined by the API
  }
});

onUnmounted(() => {
  document.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseup", stopResize);
});
</script>

<style scoped>
.asset-page {
  position: relative;
}

.resizer {
  user-select: none;
}

@media (max-width: 768px) {
  .asset-page > div:first-child {
    display: none;
  }
  .resizer {
    display: none;
  }
}
</style>
