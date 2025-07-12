<template>
  <NModal v-model:show="show">
    <NCard class="w-fit">
      <div class="flex gap-2">
        <FileDetail v-if="left" :detail="left"></FileDetail>
        <FileDetail v-if="right" :detail="right"></FileDetail>
      </div>
    </NCard>
  </NModal>
</template>
<script lang="ts" setup>
import { useVModel } from "@vueuse/core";
import type { components } from "~/common/schema";
import FileDetail from "./FileDetail.vue";

const props = defineProps<{
  visible: boolean;
  left?: components["schemas"]["BundleDetailsDto"];
  right?: components["schemas"]["BundleDetailsDto"];
}>();
const emit = defineEmits<{ "update:visible": [boolean] }>();
const show = useVModel(props, "visible", emit);
</script>
