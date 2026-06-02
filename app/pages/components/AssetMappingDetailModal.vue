<template>
  <NModal
    :show="visible"
    preset="card"
    title="资源详情"
    style="width: 600px"
    @update:show="$emit('update:visible', $event)"
  >
    <NSpin v-if="loading" class="w-full" />
    <template v-else-if="detail">
      <NDescriptions bordered :column="1" label-placement="left">
        <NDescriptionsItem label="Asset Name">
          {{ detail.assetName }}
        </NDescriptionsItem>
        <NDescriptionsItem label="Bundle Path">
          {{ detail.bundlePath }}
        </NDescriptionsItem>
        <NDescriptionsItem label="Asset Path">
          {{ detail.assetPath ?? "-" }}
        </NDescriptionsItem>
        <NDescriptionsItem label="Short Name">
          {{ detail.shortName ?? "-" }}
        </NDescriptionsItem>
      </NDescriptions>
    </template>
  </NModal>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { client } from "~/common/client";
import type { components } from "~/common/schema";

const props = defineProps<{
  visible: boolean;
  versionId?: number;
  assetName?: string;
}>();
defineEmits<{
  (e: "update:visible", v: boolean): void;
}>();

const loading = ref(false);
const detail = ref<components["schemas"]["AssetMappingDetailDto"]>();

watch(
  () => [props.visible, props.versionId, props.assetName],
  async () => {
    if (!props.visible || !props.versionId || !props.assetName) {
      detail.value = undefined;
      return;
    }
    loading.value = true;
    const { data } = await client.GET("/api/v1/manifest/{version_id}/detail", {
      params: {
        path: { version_id: props.versionId },
        query: { asset_name: props.assetName },
      },
    });
    detail.value = data;
    loading.value = false;
  },
);
</script>
