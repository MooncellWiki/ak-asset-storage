<template>
  <!-- Empty State -->
  <div
    v-if="!path"
    class="h-full flex flex-col items-center justify-center text-gray-400"
  >
    <CarbonDocumentBlank class="mb-4 text-6xl" />
    <p>选择一个文件或目录查看内容</p>
  </div>

  <!-- Directory View -->
  <div v-else-if="node!.is_dir && node!.children">
    <h3 class="mb-4 text-lg font-semibold">{{ currentName }}</h3>
    <NDataTable
      :columns="columns"
      :data="node!.children"
      :row-props="rowProps"
    />
  </div>

  <!-- File View -->
  <div v-else-if="!node!.is_dir" class="file-viewer">
    <!-- Header with actions -->
    <div class="sticky top-0 flex items-center justify-between bg-white">
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
      <img :src="fileUrl" :alt="currentName" class="preview-img max-w-full" />
    </div>

    <!-- Audio Preview -->
    <audio v-else-if="isAudio" controls :src="fileUrl" class="w-full"></audio>

    <!-- Text/Code Preview with Syntax Highlighting -->
    <div v-else-if="isCode" class="code-viewer">
      <div
        v-if="highlightedCode"
        class="shiki-container"
        v-html="highlightedCode"
      ></div>
    </div>
    <div v-else-if="isText">
      <pre>{{ fileContent }}</pre>
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
        <p v-if="node!.size > 0">大小: {{ toReadableSize(node!.size) }}</p>
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
import { toReadableSize } from "~/common/utils";
import type { components } from "~/common/schema";
import type { TreeNode } from "../types";
import type { DataTableColumns } from "naive-ui";

type AssetEntry = components["schemas"]["AssetEntry"];

const props = defineProps<{
  node: TreeNode;
}>();

const path = defineModel<string>({ required: true });

const message = useMessage();
const md = new MarkdownIt();

const fileContent = ref("");
const highlightedCode = ref("");
const renderedMarkdown = ref("");

const DATE_FORMAT_STRING = "yyyy-MM-dd HH:mm:ss";

const currentName = computed(() => {
  return path.value.split("/").at(-1) || "根目录";
});

const fileUrl = computed(() => {
  if (path.value.startsWith("raw")) {
    return `${location.origin}/${path.value.replace("raw", "assets")}`;
  }
  return `${location.origin}/${path.value}`;
});

const fileExtension = computed(() => {
  return path.value.slice(path.value.lastIndexOf(".") + 1).toLowerCase();
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
      const name = row.path.split("/").at(-1) || "根目录";
      return row.is_dir ? `📁 ${name}` : `📄 ${name}`;
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
      path.value = row.path;
    },
  };
}

// Load file content for preview
async function loadFileContent() {
  highlightedCode.value = "";
  renderedMarkdown.value = "";
  fileContent.value = "";

  if (!path.value || props.node!.is_dir) return;

  try {
    // Load file content for preview
    if (isText.value || isCode.value || isMarkdown.value) {
      const response = await fetch(fileUrl.value);
      let text = await response.text();
      fileContent.value = text;

      if (isMarkdown.value) {
        renderedMarkdown.value = md.render(text);
      } else if (isCode.value) {
        if (fileExtension.value === "json") {
          try {
            text = JSON.stringify(JSON.parse(text), undefined, "  ");
          } catch {}
        }
        await highlightCode(text, fileExtension.value);
      }
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
    highlightedCode.value = `<pre>${code}</pre>`;
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
  path,
  () => {
    if (path.value && !props.node!.is_dir) {
      loadFileContent();
    }
  },
  { immediate: true },
);
</script>

<style scoped>
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
