<template>
  <div class="flex items-center justify-center">
    <audio
      v-if="['mp3', 'wav'].includes(type)"
      controls
      :src="escapedPath"
    ></audio>
    <img
      v-if="['webp', 'png'].includes(type)"
      :src="escapedPath"
      class="preview-img max-w-full"
    />
    <TextPreview
      v-if="['atlas'].includes(type)"
      :path="escapedPath"
    ></TextPreview>
  </div>
</template>
<script setup lang="ts">
import { computed } from "vue";
import TextPreview from "./TextPreview.vue";

const props = defineProps<{
  path: string;
}>();

const type = computed(() => {
  return props.path.slice(props.path?.lastIndexOf(".") + 1).toLowerCase();
});

const escapedPath = computed(() => {
  const lastSlashIndex = props.path.lastIndexOf("/");
  const directory = props.path.slice(0, lastSlashIndex);
  const filename = props.path.slice(lastSlashIndex + 1);
  return `${directory}/${encodeURIComponent(filename)}`;
});
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
</style>
