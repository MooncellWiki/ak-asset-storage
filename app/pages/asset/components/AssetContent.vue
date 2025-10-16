<template>
  <div class="asset-content h-full flex flex-col">
    <!-- Content Area -->
    <div class="flex-1 overflow-auto p-4">
      <!-- Loading State -->
      <div v-if="loading" class="h-full flex items-center justify-center">
        <NSpin size="large" />
      </div>

      <!-- Empty State -->
      <div
        v-else-if="!path"
        class="h-full flex flex-col items-center justify-center text-gray-400"
      >
        <CarbonDocumentBlank class="mb-4 text-6xl" />
        <p>选择一个文件或目录查看内容</p>
      </div>

      <!-- Directory View -->
      <div v-else-if="isDir && dirContent">
        <h3 class="mb-4 text-lg font-semibold">{{ currentName }}</h3>
        <NDataTable
          :columns="columns"
          :data="dirContent.children"
          :row-props="rowProps"
        />
      </div>

      <!-- File View -->
      <div v-else-if="!isDir" class="file-viewer">
        <!-- Header with actions -->
        <div class="mb-4 flex items-center justify-between">
          <h3 class="text-lg font-semibold">{{ currentName }}</h3>
          <div class="flex gap-2">
            <NButton v-if="canCopy" size="small" @click="copyContent">
              <template #icon><CarbonCopy /></template>
              复制
            </NButton>
            <NButton size="small" @click="downloadFile">
              <template #icon><CarbonDownload /></template>
              下载
            </NButton>
          </div>
        </div>

        <!-- Image Preview -->
        <div v-if="isImage" class="flex justify-center">
          <img
            :src="fileUrl"
            :alt="currentName"
            class="preview-img max-w-full"
          />
        </div>

        <!-- Audio Preview -->
        <audio
          v-else-if="isAudio"
          controls
          :src="fileUrl"
          class="w-full"
        ></audio>

        <!-- Text/Code Preview with Syntax Highlighting -->
        <div v-else-if="isText || isCode" class="code-viewer">
          <div
            v-if="highlightedCode"
            class="shiki-container"
            v-html="highlightedCode"
          ></div>
          <pre v-else class="overflow-x-auto rounded bg-gray-50 p-4">{{
            fileContent
          }}</pre>
        </div>

        <!-- Markdown Preview -->
        <div
          v-else-if="isMarkdown"
          class="markdown-body prose max-w-none"
          v-html="renderedMarkdown"
        ></div>

        <!-- Binary/Unknown File -->
        <div v-else class="p-8 text-center">
          <CarbonDocumentUnknown class="mx-auto mb-4 text-6xl text-gray-400" />
          <p class="mb-4 text-gray-600">无法预览此文件类型</p>
          <div class="mb-4 text-sm text-gray-500">
            <p>文件名: {{ currentName }}</p>
            <p v-if="fileSize">大小: {{ fileSize }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import CarbonCopy from "~icons/carbon/copy";
import CarbonDocumentBlank from "~icons/carbon/document-blank";
import CarbonDocumentUnknown from "~icons/carbon/document-unknown";
import CarbonDownload from "~icons/carbon/download";
import { format, parseISO } from "date-fns";
import MarkdownIt from "markdown-it";
import { useMessage } from "naive-ui";
import { codeToHtml } from "shiki";
import { computed, ref, watch } from "vue";
import { client } from "~/common/client";
import { toReadableSize } from "~/common/utils";
import type { components } from "~/common/schema";
import type { DataTableColumns } from "naive-ui";

type AssetEntry = components["schemas"]["AssetEntry"];
type AssetDir = components["schemas"]["AssetDir"];

const props = defineProps<{
  path: string;
  isDir: boolean;
  dirContent: AssetDir | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  navigate: [path: string, isDir: boolean];
}>();

const message = useMessage();
const md = new MarkdownIt();

const fileContent = ref("");
const highlightedCode = ref("");
const renderedMarkdown = ref("");
const fileSize = ref("");

const DATE_FORMAT_STRING = "yyyy-MM-dd HH:mm:ss";

// Computed properties
const pathParts = computed(() => {
  if (!props.path) return [];
  return props.path.replace("./asset/", "").split("/").filter(Boolean);
});

const currentName = computed(() => {
  if (!props.path) return "";
  return pathParts.value.at(-1) || "根目录";
});

const fileUrl = computed(() => {
  if (!props.path) return "";
  if (props.path.startsWith("raw")) {
    return `${location.origin}/${props.path.replace("raw", "assets")}`;
  }
  return `${location.origin}/${props.path}`;
});

const fileExtension = computed(() => {
  if (!props.path) return "";
  return props.path.slice(props.path.lastIndexOf(".") + 1).toLowerCase();
});

const isImage = computed(() =>
  ["png", "jpg", "jpeg", "webp", "gif", "svg"].includes(fileExtension.value),
);
const isAudio = computed(() =>
  ["mp3", "wav", "ogg"].includes(fileExtension.value),
);
const isMarkdown = computed(() =>
  ["md", "markdown"].includes(fileExtension.value),
);
const isText = computed(() =>
  ["txt", "log", "atlas"].includes(fileExtension.value),
);
const isCode = computed(() => {
  return [
    "js",
    "ts",
    "jsx",
    "tsx",
    "vue",
    "json",
    "xml",
    "html",
    "css",
    "scss",
    "py",
    "java",
    "c",
    "cpp",
    "rs",
    "go",
    "sh",
    "yaml",
    "yml",
    "toml",
  ].includes(fileExtension.value);
});

const canCopy = computed(
  () => isText.value || isCode.value || isMarkdown.value,
);

// Columns for directory listing
const columns: DataTableColumns<AssetEntry> = [
  {
    key: "name",
    title: "文件名",
    render: (row) => {
      return row.is_dir ? `📁 ${row.name}` : `📄 ${row.name}`;
    },
  },
  {
    key: "create_at",
    title: "创建时间",
    render: (row) => format(parseISO(row.create_at), DATE_FORMAT_STRING),
  },
  {
    key: "modified_at",
    title: "修改时间",
    render: (row) => format(parseISO(row.modified_at), DATE_FORMAT_STRING),
  },
  {
    key: "size",
    title: "大小",
    render: (row) => (row.is_dir ? "-" : toReadableSize(row.size)),
  },
];

function rowProps(row: AssetEntry) {
  return {
    style: "cursor: pointer;",
    onClick: () => {
      emit("navigate", row.path, row.is_dir);
    },
  };
}

// Load file content for preview
async function loadFileContent() {
  highlightedCode.value = "";
  renderedMarkdown.value = "";
  fileContent.value = "";

  if (!props.path || props.isDir) return;

  try {
    // Load file content for preview
    if (isText.value || isCode.value || isMarkdown.value) {
      const response = await fetch(fileUrl.value);
      const text = await response.text();
      fileContent.value = text;

      if (isMarkdown.value) {
        renderedMarkdown.value = md.render(text);
      } else if (isCode.value) {
        await highlightCode(text, fileExtension.value);
      }
    }

    // Get file size
    const { data } = await client.GET("/api/v1/files/{path}", {
      params: { path: { path: props.path.replace("./asset/", "") } },
    });
    if (data?.dir) {
      fileSize.value = toReadableSize(data.dir.size);
    }
  } catch (error) {
    console.error("Error loading content:", error);
    message.error("加载内容失败");
  }
}

// Syntax highlighting
async function highlightCode(code: string, lang: string) {
  try {
    highlightedCode.value = await codeToHtml(code, {
      lang: lang as any,
      theme: "github-light",
    });
  } catch (error) {
    console.error("Highlighting error:", error);
    // Fallback to plain text
    highlightedCode.value = "";
  }
}

// Copy content to clipboard
async function copyContent() {
  try {
    await navigator.clipboard.writeText(fileContent.value);
    message.success("已复制到剪贴板");
  } catch {
    message.error("复制失败");
  }
}

// Download file
function downloadFile() {
  window.open(fileUrl.value, "_blank");
}

// Watch for path changes
watch(
  () => props.path,
  () => {
    if (props.path && !props.isDir) {
      loadFileContent();
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.asset-content {
  background: #fff;
}

.preview-img {
  background-image:
    linear-gradient(45deg, rgb(204, 204, 204) 25%, transparent 25%),
    linear-gradient(135deg, rgb(204, 204, 204) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgb(204, 204, 204) 75%),
    linear-gradient(135deg, transparent 75%, rgb(204, 204, 204) 75%);
  background-size: 24px 24px;
  background-position:
    0px 0px,
    12px 0px,
    12px -12px,
    0px 12px;
}

.shiki-container {
  border-radius: 6px;
  overflow-x: auto;
}

.shiki-container :deep(pre) {
  padding: 1rem;
  margin: 0;
}

.markdown-body {
  padding: 1rem 0;
}

.markdown-body :deep(pre) {
  background: #f6f8fa;
  padding: 1rem;
  border-radius: 6px;
  overflow-x: auto;
}

.markdown-body :deep(code) {
  background: #f6f8fa;
  padding: 0.2em 0.4em;
  border-radius: 3px;
  font-size: 85%;
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
}
</style>
