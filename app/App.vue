<template>
  <AkToastProvider :max="4">
    <a class="app-skip" href="#content">跳到内容</a>
    <AppHeader />
    <main id="content" class="app-main">
      <RouterView />
    </main>
    <AppFooter />
    <SearchPalette />
  </AkToastProvider>
</template>

<script setup lang="ts">
import { AkToastProvider } from "@mooncellwiki/prts-design-vue";
import AppFooter from "./components/AppFooter.vue";
import AppHeader from "./components/AppHeader.vue";
import SearchPalette from "./components/SearchPalette.vue";
</script>

<style lang="scss">
// The only global rules: the page canvas behind the .ak-scope body.
html {
  background: var(--ak-bg-canvas);
}

body {
  margin: 0;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--ak-bg-canvas);
}

#app {
  flex: 1;
  display: flex;
  flex-direction: column;
}
</style>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.app-main {
  flex: 1;
  width: 100%;
  max-width: 1680px;
  margin: 0 auto;
  padding: var(--ak-space-5) var(--ak-gutter) var(--ak-space-16);
  box-sizing: border-box;

  @media (max-width: $bp-phone) {
    padding-top: var(--ak-space-4);
  }
}

// Off screen until focused with Tab.
.app-skip {
  position: absolute;
  left: 8px;
  top: 8px;
  z-index: 9999;
  padding: 8px 12px;
  background: var(--ak-accent);
  color: var(--ak-accent-fg);
  font-weight: 700;
  line-height: 1.3;
  transform: translateY(calc(-100% - 12px));

  &:focus {
    transform: none;
  }
}
</style>
