<template>
  <div class="h-[calc(100vh-4.5rem)] flex flex-col overflow-hidden">
    <div class="border-b border-gray-200 p-3">
      <div class="flex items-center gap-2">
        <NSelect
          v-model:value="selectedVersion"
          class="w-80"
          :options="versionOpts"
          clearable
          placeholder="选择版本"
        />
      </div>
    </div>

    <div
      v-if="selectedVersion == null"
      class="flex flex-1 items-center justify-center text-gray-400"
    >
      <p>请先选择一个版本</p>
    </div>

    <template v-else>
      <NSpin v-if="loadingRoot" class="mt-8" />
      <div v-else class="flex-1 overflow-hidden">
        <NSplit
          direction="horizontal"
          :default-size="0.25"
          :min="0.15"
          :max="0.4"
        >
          <template #1>
            <div class="h-full overflow-hidden">
              <ManifestTree
                v-model="selectedPath"
                :version-id="selectedVersion"
                :tree-data="treeData"
                @select="selectedNodeType = $event"
              />
            </div>
          </template>
          <template #2>
            <div class="relative h-full overflow-auto px-4 py-2">
              <ManifestContent
                v-model="selectedPath"
                :version-id="selectedVersion"
                :node-type="selectedNodeType"
              />
            </div>
          </template>
        </NSplit>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { onBeforeMount, ref, watch } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import ManifestContent from "./components/ManifestContent.vue";
import ManifestTree from "./components/ManifestTree.vue";
import { toManifestTreeOption } from "./utils";
import type { TreeOption } from "naive-ui";

const { versionOpts, load: loadVersions } = useVersionSelect();
const selectedVersion = ref<number>();
const selectedPath = ref("");
const selectedNodeType = ref<string>();
const treeData = ref<TreeOption[]>([]);
const loadingRoot = ref(false);

onBeforeMount(loadVersions);

watch(selectedVersion, async (versionId) => {
  selectedPath.value = "";
  selectedNodeType.value = undefined;
  if (versionId == null) {
    treeData.value = [];
    return;
  }
  loadingRoot.value = true;
  const { data } = await client.GET("/api/v1/manifest/{version_id}/children", {
    params: { path: { version_id: versionId }, query: {} },
  });
  treeData.value = (data ?? []).map(toManifestTreeOption);
  loadingRoot.value = false;
});
</script>
