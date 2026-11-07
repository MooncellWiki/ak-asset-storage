<template>
  <pre
    class="code"
    :class="{ 'is-wrap': wrap }"
    tabindex="0"
    v-html="html"
  ></pre>
</template>

<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { highlightLines } from "~/common/highlight";
import type { BundledLanguage } from "shiki";
import type { CodeToken } from "~/common/highlight";

/**
 * Source text with line numbers (a CSS counter) and Shiki highlighting.
 * Plain text shows first; colours arrive once the grammar has loaded.
 */
const props = defineProps<{
  text: string;
  /** Shiki language; plain text when absent */
  lang?: BundledLanguage;
  /** Soft-wrap long lines (story scripts, minified JSON) */
  wrap?: boolean;
}>();

/** Above this, tokenising would block the page for seconds: show plain text. */
const MAX_HIGHLIGHT = 512 * 1024;

const ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
};
const escape = (s: string) => s.replace(/[&<>"]/gu, (c) => ESCAPES[c]!);

const tokens = shallowRef<CodeToken[][]>();
let request = 0;
watch(
  [() => props.text, () => props.lang],
  async ([text, lang]) => {
    const seq = ++request;
    tokens.value = undefined;
    if (!lang || text.length > MAX_HIGHLIGHT) return;
    try {
      const lines = await highlightLines(text, lang);
      if (seq === request) tokens.value = lines;
    } catch {
      // Unknown grammar or tokenizer failure: stay plain.
    }
  },
  { immediate: true },
);

// One <div> per line; the inner <span> keeps the line a single flex item,
// or leading indentation would be dropped next to the line number.
const html = computed(() => {
  const lines = tokens.value
    ? tokens.value.map((line) =>
        line
          .map((t) =>
            t.cls
              ? `<span class="${t.cls}">${escape(t.text)}</span>`
              : escape(t.text),
          )
          .join(""),
      )
    : props.text
        .replace(/\r\n?/gu, "\n")
        .replace(/\n$/u, "")
        .split("\n")
        .map(escape);
  return lines.map((line) => `<div><span>${line || " "}</span></div>`).join("");
});
</script>

<style scoped lang="scss">
.code {
  margin: 0;
  padding: 0;
  max-height: 72vh;
  overflow: auto;
  font: 13px/1.6 var(--ak-font-mono);
  tab-size: 2;
  background: var(--ak-bg-inset);
  color: var(--ak-fg);
  counter-reset: ln;

  > :deep(div) {
    display: flex;
    white-space: pre;

    // Line number, pinned while scrolling sideways.
    &::before {
      position: sticky;
      left: 0;
      flex: none;
      width: 3.5em;
      margin-right: 1em;
      padding-right: 1em;
      border-right: 1px solid var(--ak-border);
      background: var(--ak-bg-inset);
      color: var(--ak-fg-subtle);
      text-align: right;
      content: counter(ln);
      counter-increment: ln;
      user-select: none;
    }

    &:hover {
      background: var(--ak-bg-hover);
    }

    > span {
      flex: 1 0 auto;
    }
  }

  &.is-wrap > :deep(div) {
    white-space: pre-wrap;

    > span {
      flex: 1 1 auto;
      min-width: 0;
      overflow-wrap: anywhere;
    }
  }

  // Shiki token classes (common/highlight.ts), coloured with design tokens.
  :deep(.k) {
    color: var(--ak-fg);
    font-weight: 600;
  }

  :deep(.s) {
    color: var(--ak-link);
  }

  :deep(.n) {
    color: var(--ak-warning);
  }

  :deep(.p) {
    color: var(--ak-fg-muted);
  }

  :deep(.c) {
    color: var(--ak-fg-subtle);
    font-style: italic;
  }

  :deep(.f) {
    color: var(--ak-rt-kw, var(--ak-accent));
  }

  :deep(.v) {
    color: var(--ak-fg-secondary);
  }

  :deep(.l) {
    color: var(--ak-link);
    text-decoration: underline;
  }

  :deep(.i) {
    color: var(--ak-success);
  }

  :deep(.d) {
    color: var(--ak-danger);
  }

  :deep(.it) {
    font-style: italic;
  }

  :deep(.bd) {
    font-weight: 700;
  }
}
</style>
