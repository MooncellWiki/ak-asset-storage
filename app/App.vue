<template>
  <NConfigProvider :theme="theme">
    <NModalProvider>
      <NLayout position="absolute">
        <NLayoutHeader class="mb-2 flex p-3" bordered>
          <NMenu :options="opts" mode="horizontal"></NMenu>
          <NButton class="m-auto mr-0" secondary @click="toggleDark()">
            {{ isDark ? "浅色" : "深色" }}
          </NButton>
        </NLayoutHeader>
        <NLayoutContent class="container mx-auto">
          <RouterView />
        </NLayoutContent>
      </NLayout>
    </NModalProvider>
  </NConfigProvider>
</template>
<script setup lang="ts">
import { useDark, useToggle } from "@vueuse/core";
import { darkTheme } from "naive-ui";
import { computed, h } from "vue";
import { RouterLink } from "vue-router";

const isDark = useDark();
const toggleDark = useToggle(isDark);
const theme = computed(() => {
  return isDark.value ? darkTheme : undefined;
});
const opts = [
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: "/file",
          },
        },
        () => "file",
      ),
    key: "file",
  },
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: "/diff",
          },
        },
        () => "diff",
      ),
    key: "diff",
  },
];
</script>
