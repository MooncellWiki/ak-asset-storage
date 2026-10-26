<template>
  <footer class="footer">
    <div class="footer__inner">
      <span>Arknights Asset Storage {{ version }}</span>
      <a href="https://github.com/MooncellWiki/ak-asset-storage">GitHub</a>
      <a href="https://github.com/MooncellWiki/torappu">torappu</a>
      <a href="https://mooncellwiki.github.io/prts-design/">PRTS Design</a>
      <span>游戏素材版权归鹰角网络所有</span>
    </div>
  </footer>
</template>

<script setup lang="ts">
const version = __APP_VERSION__;
</script>

<style scoped lang="scss">
// Black in both themes like the header, with the accent hazard stripe on top.
.footer {
  position: relative;
  overflow: hidden;
  margin-top: auto;
  background: var(--ak-chrome-bg-solid);
  color: var(--ak-chrome-fg);

  &::before {
    content: "";
    position: absolute;
    inset: 0 0 auto;
    height: 4px;
    background: repeating-linear-gradient(
      -45deg,
      var(--ak-theme-accent) 0 12px,
      transparent 12px 24px
    );
    opacity: 0.9;
  }

  &__inner {
    max-width: 1680px;
    margin: 0 auto;
    padding: var(--ak-space-4) var(--ak-gutter);
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px var(--ak-space-5);
    font-size: var(--ak-fs-xs);
    opacity: 0.6;
  }

  a {
    color: inherit;
    text-decoration: none;

    &:hover {
      color: var(--ak-theme-accent);
      text-decoration: underline;
      text-underline-offset: 2px;
    }
  }
}
</style>
