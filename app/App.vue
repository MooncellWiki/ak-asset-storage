<template>
  <NConfigProvider :theme="theme" class="h-100vh">
    <NModalProvider>
      <NMessageProvider>
        <NLayout class="h-full">
          <NLayoutHeader class="h-4rem flex p-0.75rem" bordered>
            <NMenu :options="opts" mode="horizontal"></NMenu>
            <NButton class="m-auto mr-0" secondary @click="toggleDark()">
              {{ isDark ? "浅色" : "深色" }}
            </NButton>
          </NLayoutHeader>
          <NLayout
            position="absolute"
            class="top-4rem!"
            content-class="mx-auto h-[calc(100vh-4rem)]"
          >
            <div class="container mx-auto mt-2">
              <RouterView />
            </div>
          </NLayout>
        </NLayout>
      </NMessageProvider>
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
  {
    label: () =>
      h(
        RouterLink,
        {
          to: {
            path: "/asset",
          },
        },
        () => "asset",
      ),
    key: "asset",
  },
];
</script>
