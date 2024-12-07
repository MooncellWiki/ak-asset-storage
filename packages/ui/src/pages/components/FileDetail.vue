<template>
  <div class="w-[600px]">
    <div>path: {{ detail?.path }}</div>
    <div>sha256: {{ detail?.hash }}</div>
    <div>size: {{ fmtSize(detail?.size) }}</div>
    <div>version: {{ detail?.client }} - {{ detail?.res }}</div>
    <div class="mt-2">
      <NButton @click="download">下载</NButton>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { NButton } from "naive-ui";
import type { components } from "~/common/schema";

const props = defineProps<{
  detail: components["schemas"]["FileDetail"];
}>();

function download() {
  const path = `/storage/${props.detail.hash.slice(0, 2)}/${props.detail.hash.slice(2, 4)}/${props.detail.hash.slice(4)}`;
  const a = document.createElement("a");
  a.href = path;
  a.download = `${props.detail.client}_${props.detail.res}_${props.detail.path}.zip`;
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
