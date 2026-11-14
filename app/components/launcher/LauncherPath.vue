<template>
  <span class="launcher-path" :title="path">
    <span v-if="parts.dir" class="launcher-path__dir">{{ parts.dir }}</span>
    <span>{{ parts.name }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { splitPath } from "~/common/utils";

const props = defineProps<{ path: string }>();
const parts = computed(() => splitPath(props.path));
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-path {
  @include mono;

  display: block;
  min-width: 0;
  overflow-wrap: anywhere;

  &__dir {
    display: block;
    color: var(--ak-fg-muted);
    font-size: 11px;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}
</style>
