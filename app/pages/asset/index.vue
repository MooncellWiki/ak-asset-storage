<template>
  <div class="flex flex-col gap-4">
    <!-- Search Button and Breadcrumb -->
    <div class="flex items-center justify-between">
      <NBreadcrumb>
        <NBreadcrumbItem @click="navigateTo('')">
          <div class="flex cursor-pointer items-center gap-1">
            <CarbonFolderShared />
            <span>asset</span>
          </div>
        </NBreadcrumbItem>
        <NBreadcrumbItem
          v-for="(part, index) in pathParts"
          :key="index"
          @click="navigateTo(pathParts.slice(0, index + 1).join('/'))"
        >
          <div class="cursor-pointer">{{ part }}</div>
        </NBreadcrumbItem>
      </NBreadcrumb>
      <NButton @click="openSearch">
        <template #icon>
          <CarbonSearch />
        </template>
        搜索
      </NButton>
    </div>

    <!-- File/Folder List -->
    <NCard>
      <NDataTable
        :data="data"
        :columns="columns"
        :row-props="rowProps"
        :loading="loading"
      />
    </NCard>
  </div>

  <!-- File Preview Modal -->
  <div
    ref="el"
    class="fixed right-2 top-2 z-10"
    :style="[
      style,
      showFullScreen && {
        left: 0,
        top: 0,
      },
    ]"
  >
    <NCard
      class="max-w-400px min-w-400px b b-#ddd b-solid shadow-lg transition-300"
      :class="{
        'opacity-0 pointer-events-none': !showFileToast,
        'min-w-screen min-h-screen max-h-screen max-w-screen': showFullScreen,
      }"
    >
      <template #header>
        <div class="flex items-center gap-2">
          <NButton tertiary @click="() => (showFullScreen = !showFullScreen)">
            <template #icon>
              <CarbonFitToScreen></CarbonFitToScreen>
            </template>
          </NButton>
          <div
            class="pl-2 font-size-sm line-height-1em"
            @pointerdown="(e) => e.stopPropagation()"
          >
            {{ previewPath }}
          </div>
          <NButton quaternary @click="() => (showFileToast = false)">
            <template #icon>
              <CarbonCloseLarge></CarbonCloseLarge>
            </template>
          </NButton>
        </div>
      </template>
      <div class="flex flex-col items-center">
        <Preview
          :path="previewPath"
          class="max-h-80% max-w-300px overflow-y-auto"
        />
      </div>
    </NCard>
  </div>

  <!-- Search Modal -->
  <NModal v-model:show="searchVisible">
    <NCard class="container">
      <NMessageProvider>
        <NInput
          v-model:value="searchText"
          placeholder="搜索"
          clearable
          size="small"
          class="m-2"
          @update:value="search"
        >
          <template #suffix> <CarbonSearch /> </template
        ></NInput>
        <NDataTable
          max-height="80vh"
          :data="searchData"
          :columns="searchColumns"
          :row-props="searchRowProps"
        ></NDataTable>
      </NMessageProvider>
    </NCard>
  </NModal>
</template>
<script lang="ts" setup>
import { useDebounceFn, useDraggable } from "@vueuse/core";
import CarbonCloseLarge from "~icons/carbon/close-large";
import CarbonDocument from "~icons/carbon/document";
import CarbonFitToScreen from "~icons/carbon/fit-to-screen";
import CarbonFolder from "~icons/carbon/folder";
import CarbonFolderShared from "~icons/carbon/folder-shared";
import CarbonSearch from "~icons/carbon/search";
import { format, parseISO } from "date-fns";
import { computed, h, onMounted, ref, useTemplateRef, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { client } from "~/common/client";
import { toReadableSize } from "~/common/utils";
import type { components } from "~/common/schema";
import Preview from "./components/Preview.vue";
import type { DataTableColumns } from "naive-ui";

const DATE_FORMAT_STRING = "yyyy-MM-dd HH:mm:ss";
interface Entry {
  create_at: string;
  is_dir: boolean;
  modified_at: string;
  name: string;
  path: string;
  key: string;
  size: number;
  hSize: string;
}

function makeDisplayable(arr: components["schemas"]["AssetEntry"][]): Entry[] {
  return arr
    .map((item) => {
      return {
        ...item,
        key: item.path,
        create_at: format(parseISO(item.create_at), DATE_FORMAT_STRING),
        modified_at: format(parseISO(item.modified_at), DATE_FORMAT_STRING),
        hSize: item.is_dir ? "" : toReadableSize(item.size),
      } satisfies Entry;
    })
    .sort((a, b) => {
      // Sort directories first, then by name
      if (a.is_dir && !b.is_dir) return -1;
      if (!a.is_dir && b.is_dir) return 1;
      return a.name > b.name ? 1 : -1;
    });
}

function realPath(e: Entry): string {
  if (e.path.startsWith("raw")) {
    return `${location.origin}/${e.path.replace("raw", "assets")}`;
  }
  return `${location.origin}/${e.path}`;
}

function open(p: string) {
  window.open(p);
}

const route = useRoute();
const router = useRouter();
const data = ref<Entry[]>([]);
const loading = ref(false);

// Get current path from route query parameter
const currentPath = computed(() => {
  const path = (route.query.path as string) || "";
  return path;
});

// Split path into breadcrumb parts
const pathParts = computed(() => {
  const path = currentPath.value;
  if (!path) return [];
  return path.split("/").filter(Boolean);
});

// Navigate to a path
function navigateTo(path: string) {
  router.push({
    path: "/asset",
    query: path ? { path } : {},
  });
}

// Load data when path changes
async function loadData() {
  loading.value = true;
  try {
    const { data, error } = await client.GET("/api/v1/files/{path}", {
      params: {
        path: {
          path: currentPath.value,
        },
      },
    });
    if (error) {
      console.error("Failed to load data:", error);
      data.value = [];
    } else {
      data.value = makeDisplayable(data?.children ?? []);
    }
  } catch (error) {
    console.error("Failed to load data:", error);
    data.value = [];
  } finally {
    loading.value = false;
  }
}

const previewPath = ref("");
const showFileToast = ref(false);
const showFullScreen = ref(false);

const columns: DataTableColumns<Entry> = [
  {
    key: "name",
    title: "名称",
    render: (row) => {
      return h("div", { class: "flex items-center gap-2" }, [
        row.is_dir
          ? h(CarbonFolder, { class: "text-blue-500" })
          : h(CarbonDocument, { class: "text-gray-500" }),
        h("span", row.name),
      ]);
    },
  },
  { key: "create_at", title: "创建时间" },
  { key: "modified_at", title: "修改时间" },
  { key: "hSize", title: "大小" },
];

onMounted(async () => {
  await loadData();
  window.addEventListener("resize", updatewindowHeight);
});

// Watch for path changes
watch(currentPath, async () => {
  await loadData();
});

function rowProps(row: Entry) {
  return {
    onClick: () => {
      if (row.is_dir) {
        // Navigate into the directory
        const newPath = currentPath.value
          ? `${currentPath.value}/${row.name}`
          : row.name;
        navigateTo(newPath);
        return;
      }

      // For files, show preview
      if (row.path.endsWith(".json")) {
        open(realPath(row));
        return;
      }
      previewPath.value = realPath(row);
      showFileToast.value = true;
    },
    style: {
      cursor: "pointer",
    },
  };
}

const searchVisible = ref(false);

function searchRowProps(row: Entry) {
  return {
    onClick: async () => {
      // Extract the directory path from the file path
      const pathParts = row.path.replace("./asset/", "").split("/");
      pathParts.pop(); // Remove the filename
      const dirPath = pathParts.join("/");

      navigateTo(dirPath);
      searchVisible.value = false;
    },
  };
}

function openSearch() {
  searchVisible.value = true;
}

const searchText = ref("");
const searchData = ref<Entry[]>([]);

const search = useDebounceFn(async () => {
  if (!searchText.value) {
    searchData.value = [];
    return;
  }
  const { data } = await client.GET("/api/v1/files", {
    params: {
      query: { path: searchText.value },
    },
  });
  searchData.value = makeDisplayable(data ?? []);
}, 500);

const searchColumns: DataTableColumns<Entry> = [
  { key: "name", title: "文件名" },
  { key: "create_at", title: "创建时间" },
  { key: "modified_at", title: "修改时间" },
  { key: "hSize", title: "大小" },
  { key: "path", title: "路径" },
];

const windowHeight = ref(window.innerHeight);
function updatewindowHeight() {
  windowHeight.value = window.innerHeight;
}

const el = useTemplateRef<HTMLElement>("el");
const { style } = useDraggable(el, {
  initialValue: { x: window.innerWidth - 400 - 40, y: 40 },
  preventDefault: true,
  capture: false,
  disabled: showFullScreen,
});
</script>
