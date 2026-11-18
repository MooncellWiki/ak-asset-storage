<template>
  <a
    v-if="file?.sha256"
    class="ak-btn ak-btn--sm"
    :class="{ 'ak-btn--primary': primary }"
    :href="storageUrl(file.sha256, '/storage/launcher/blobs')"
    :download="splitPath(path).name"
  >
    <AsIcon name="download" :size="16" />下载
  </a>
  <span
    v-else
    class="download-status"
    aria-live="polite"
    :title="file?.lastError ?? undefined"
  >
    {{ status }}
  </span>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { client } from "~/common/client";
import { splitPath, storageUrl } from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import type { components } from "~/common/schema";

const props = defineProps<{
  releaseId: number;
  path: string;
  primary?: boolean;
}>();
const file = shallowRef<components["schemas"]["LauncherFile"]>();
const loading = ref(false);

watch(
  () => [props.releaseId, props.path] as const,
  async ([id, path], _, onCleanup) => {
    const request = new AbortController();
    onCleanup(() => request.abort());
    file.value = undefined;
    loading.value = true;
    try {
      // The endpoint matches substrings, so use only the exact manifest path.
      let after: string | undefined;
      do {
        const { data } = await client.GET(
          "/api/v1/launcher/releases/{id}/files",
          {
            params: { path: { id }, query: { q: path, after, limit: 200 } },
            signal: request.signal,
          },
        );
        if (!data || request.signal.aborted) return;
        const match = data.files.find((entry) => entry.path === path);
        if (match) {
          file.value = match;
          return;
        }
        after = data.nextAfter ?? undefined;
      } while (after != null);
    } catch {
      // The comparison metadata stays visible if archive status is unavailable.
    } finally {
      if (!request.signal.aborted) loading.value = false;
    }
  },
  { immediate: true },
);

const status = computed(() => {
  if (loading.value) return "正在读取归档状态…";
  if (!file.value) return "无法获取归档状态";
  if (file.value.retriesExhausted) return "归档失败";
  return file.value.lastError ? "等待重试" : "待归档";
});
</script>

<style scoped lang="scss">
.download-status {
  color: var(--ak-fg-muted);
  font-size: var(--ak-fs-xs);
}
</style>
