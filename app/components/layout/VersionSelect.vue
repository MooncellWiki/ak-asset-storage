<template>
  <AkSelect
    v-model="model"
    class="version-select"
    size="sm"
    :options="options"
    :label="label"
    :placeholder="placeholder"
  />
</template>

<script setup lang="ts">
import { AkSelect } from "@mooncellwiki/prts-design-vue";
import type {
  SelectGroupOption,
  SelectOption,
  SelectValue,
} from "@mooncellwiki/prts-design-vue";

/** A version picker: monospace, wide enough for `2.7.81 26-10-22-14-02-11-51447808`. */
defineProps<{
  options: (SelectOption | SelectGroupOption)[];
  label: string;
  placeholder?: string;
}>();
const model = defineModel<SelectValue | null>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.version-select {
  @include mono;

  width: auto;
  min-width: 300px;
  max-width: 100%;

  @media (max-width: $bp-phone) {
    width: 100%;
    min-width: 0;
  }
}
</style>
