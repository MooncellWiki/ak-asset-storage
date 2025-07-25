<template>
  <div class="w-[600px]">
    <div>path: {{ detail?.path }}</div>
    <div>sha256: {{ detail?.fileHash }}</div>
    <div>size: {{ fmtSize(detail?.fileSize) }}</div>
    <div>version: {{ detail?.versionClient }} - {{ detail?.versionRes }}</div>
    <div class="mt-2">
      <NButton @click="download">下载</NButton>
    </div>
  </div>
</template>

<script lang="ts" setup>
import type { components } from "~/common/schema";

const props = defineProps<{
  detail: components["schemas"]["BundleDetailsDto"];
}>();

function download() {
  const path = `/storage/${props.detail.fileHash.slice(0, 2)}/${props.detail.fileHash.slice(2, 4)}/${props.detail.fileHash.slice(4)}`;
  const a = document.createElement("a");
  a.href = path;
  a.download = `${props.detail.versionClient}_${props.detail.versionRes}_${props.detail.path}.zip`;
  a.click();
}
function fmtSize(size?: number) {
  if (!size) {
    return "";
  }
  if (size > 1024 * 1024) {
    return `${(size / 1024 / 1024).toFixed(2)}MiB`;
  }
  if (size > 1024) {
    return `${(size / 1024).toFixed(2)}KiB`;
  }
  return `${size}B`;
}
</script>
