<template>
  <div class="markdown" v-html="html"></div>
</template>

<script setup lang="ts">
import MarkdownIt from "markdown-it";
import { computed } from "vue";

/** Rendered Markdown (raw HTML disabled) with a small prose style from design tokens. */
const props = defineProps<{ text: string }>();

const md = new MarkdownIt({ linkify: true });
const html = computed(() => md.render(props.text));
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.markdown {
  max-height: 72vh;
  overflow: auto;
  padding: var(--ak-space-4) var(--ak-space-5);
  font-size: var(--ak-fs-body);
  line-height: var(--ak-lh-body);
  overflow-wrap: anywhere;

  > :deep(:first-child) {
    margin-top: 0;
  }

  :deep(:is(h1, h2, h3, h4, h5, h6)) {
    margin: 1.4em 0 0.6em;
    line-height: 1.3;
    font-weight: 700;
  }

  :deep(:is(h1, h2)) {
    padding-left: var(--ak-space-3);
    border-left: var(--ak-bar-w-lg, 4px) solid var(--ak-accent);
  }

  :deep(h1) {
    font-size: var(--ak-fs-h2);
  }

  :deep(h2) {
    font-size: var(--ak-fs-h3);
  }

  :deep(:is(p, ul, ol, blockquote, table, pre)) {
    margin: 0 0 1em;
  }

  :deep(a) {
    color: var(--ak-link);
  }

  :deep(blockquote) {
    padding: 0 var(--ak-space-4);
    border-left: 3px solid var(--ak-border-strong);
    color: var(--ak-fg-muted);
  }

  :deep(code) {
    @include mono(0.9em);

    padding: 0.1em 0.35em;
    background: var(--ak-bg-inset);
  }

  :deep(pre) {
    padding: var(--ak-space-3) var(--ak-space-4);
    overflow: auto;
    background: var(--ak-bg-inset);

    code {
      padding: 0;
      background: none;
    }
  }

  :deep(table) {
    border-collapse: collapse;
  }

  :deep(:is(th, td)) {
    padding: 6px 10px;
    border: 1px solid var(--ak-border);
  }

  :deep(th) {
    background: var(--ak-bg-surface-2);
  }

  :deep(img) {
    max-width: 100%;
  }

  :deep(hr) {
    border: 0;
    border-top: 1px solid var(--ak-border);
  }
}
</style>
