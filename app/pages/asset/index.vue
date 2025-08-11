<template>
  <NButton @click="openSearch">
    <CarbonSearch />
  </NButton>
  <NDataTable
    v-model:expanded-row-keys="expandedRowKeys"
    class="mt-2"
    max-height="80vh"
    :data="data"
    :columns="columns"
    :row-props="rowProps"
    @load="onLoad"
  />
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

  <!-- <div class="h-full">

    <div class="max-h-full w-full flex overflow-y-auto">
      <div class="w-1/2 flex-grow">
        <div class="max-h-full overflow-y-scroll">

        </div>
      </div>
      <NCard
        v-if="previewPath"
        embedded
        :title="previewPath"
        class="max-h-full w-1/2 overflow-y-auto"
      >
        <NButton @click="open(previewPath)">
          <template #icon>
            <CarbonDownload></CarbonDownload>
          </template>
        </NButton>
        <Preview :path="previewPath" />
      </NCard>

    </div>
  </div> -->
</template>
<script lang="ts" setup>
import { useDebounceFn, useDraggable } from "@vueuse/core";
import CarbonCloseLarge from "~icons/carbon/close-large";
import CarbonFitToScreen from "~icons/carbon/fit-to-screen";
import CarbonSearch from "~icons/carbon/search";
import { format, parseISO } from "date-fns";
import { useMessage } from "naive-ui";
import { onMounted, ref, useTemplateRef } from "vue";
import { client } from "~/common/client";
import { toReadableSize } from "~/common/utils";
import type { components } from "~/common/schema";
import Preview from "./components/Preview.vue";
import type { DataTableColumns } from "naive-ui";
import type { RowData } from "naive-ui/es/data-table/src/interface";

const DATE_FORMAT_STRING = "yyyy-MM-dd HH:mm:ss";
interface Entry {
  create_at: string;
  is_dir: boolean;
  modified_at: string;
  name: string;
  path: string;
  key: string;
  isLeaf?: boolean;
  size: number;
  hSize: string;
  children?: Entry[];
}
function makeDisplayable(
  arr: components["schemas"]["AssetEntry"][],
  withLeaf = true,
): Entry[] {
  return arr
    .map((item) => {
      return {
        ...item,
        key: item.path,
        isLeaf: withLeaf ? !item.is_dir : undefined,
        create_at: format(parseISO(item.create_at), DATE_FORMAT_STRING),
        modified_at: format(parseISO(item.modified_at), DATE_FORMAT_STRING),
        hSize: item.is_dir ? "" : toReadableSize(item.size),
      } satisfies Entry;
    })
    .sort((a, b) => {
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
const data = ref<Entry[]>([]);
async function list(path = ""): Promise<Entry[]> {
  const { data } = await client.GET("/api/v1/files/{path}", {
    params: {
      path: {
        path,
      },
    },
  });
  return makeDisplayable(data?.children ?? []);
}
const previewPath = ref("");
const showFileToast = ref(false);
const showFullScreen = ref(false);

const columns: DataTableColumns<Entry> = [
  { key: "name", title: "文件名" },
  { key: "create_at", title: "创建时间" },
  { key: "modified_at", title: "修改时间" },
  { key: "hSize", title: "大小" },
];
onMounted(async () => {
  data.value = await list();
  window.addEventListener("resize", updatewindowHeight);
});
const expandedRowKeys = ref<string[]>([]);
async function onLoad(row: RowData) {
  const resp = await list(row.path.replace("./asset/", ""));
  row.children = resp;
}

function rowProps(row: Entry) {
  return {
    onClick: () => {
      if (row.is_dir) {
        return;
      }
      if (row.path.endsWith(".json")) {
        open(realPath(row));
        return;
      }
      previewPath.value = realPath(row);
      showFileToast.value = true;
      //showFileInfo.value = true;
    },
  };
}
const searchVisible = ref(false);
const message = useMessage();
function searchRowProps(row: Entry) {
  return {
    onClick: async () => {
      const msgHandler = message.loading("", { duration: 0 });
      const arr = row.path.split("/");
      const set = new Set<string>();
      for (let i = 1; i < arr.length; i++) {
        set.add(arr.slice(0, i).join("/"));
      }
      const keys = [...set];
      let curList = data.value;
      for (const p of arr) {
        const entry = curList.find((v) => v.name === p)!;
        if (entry.children) {
          curList = entry.children;
          continue;
        }
        if (entry.is_dir) {
          await onLoad(entry);
          curList = entry.children!;
        } else {
          break;
        }
      }
      msgHandler.destroy();
      expandedRowKeys.value = keys;
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
  searchData.value = makeDisplayable(data ?? [], false);
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
