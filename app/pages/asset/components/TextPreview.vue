<template>
  <NCard>
    <pre>
    {{ content }}
    </pre>
  </NCard>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  path: string;
}>();

const content = ref("");

function load() {
  fetch(props.path)
    .then((resp) => resp.text())
    .then((text) => {
      content.value = text;
    });
}

watch(
  () => props.path,
  () => {
    load();
  },
  {
    immediate: true,
  },
);
</script>
