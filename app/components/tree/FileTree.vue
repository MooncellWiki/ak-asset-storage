<template>
  <div
    ref="scroller"
    class="file-tree"
    role="tree"
    tabindex="0"
    :aria-label="label"
    :aria-activedescendant="activeIndex >= 0 ? rowId(activeIndex) : undefined"
    @scroll="scrollTop = ($event.target as HTMLElement).scrollTop"
    @keydown="onKey"
    @focus="onFocus"
  >
    <div v-if="sticky.length" class="file-tree__sticky" aria-hidden="true">
      <div
        v-for="s in sticky"
        :key="s.index"
        class="file-tree__row is-sticky"
        :class="{ 'is-selected': s.row.node.path === selected }"
        :style="{ '--_d': s.row.depth, top: `${s.top}px`, zIndex: s.z }"
        @click="jumpTo(s.index)"
      >
        <FileTreeRow
          :row="s.row"
          :highlight="highlight"
          @toggle="emit('toggle', s.row.node.path, !s.row.open)"
        />
      </div>
    </div>
    <div class="file-tree__space" :style="{ height: `${rows.length * ROW}px` }">
      <template
        v-for="{ index, row } in slice"
        :key="row.kind === 'node' ? row.node.path : `note-${index}`"
      >
        <div
          v-if="row.kind === 'node'"
          :id="rowId(index)"
          class="file-tree__row"
          :class="{
            'is-selected': row.node.path === selected,
            'is-active': index === activeIndex,
          }"
          role="treeitem"
          :aria-level="row.depth + 1"
          :aria-posinset="row.posinset"
          :aria-setsize="row.setsize"
          :aria-expanded="row.node.isDir ? row.open : undefined"
          :aria-selected="row.node.path === selected"
          :title="row.node.path || row.label"
          :style="{
            '--_d': row.depth,
            transform: `translateY(${index * ROW}px)`,
          }"
          @click="activate(row)"
        >
          <FileTreeRow
            :row="row"
            :highlight="highlight"
            @toggle="emit('toggle', row.node.path, !row.open)"
          />
        </div>
        <div
          v-else
          class="file-tree__note"
          :style="{
            '--_d': row.depth,
            transform: `translateY(${index * ROW}px)`,
          }"
        >
          {{ row.text }}
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, useId, useTemplateRef, watch } from "vue";
import FileTreeRow from "./FileTreeRow.vue";
import { stickyRows, visibleRows } from "./model";
import type { Row, TreeNode } from "./model";

/**
 * A file tree after the interaction model of @pierre/trees
 * (https://trees.software): always virtualised, directories stick to the top
 * while their contents scroll past, single-child directory chains collapse
 * into one row, long names truncate in the middle, and the tree is one tab
 * stop driven by the keyboard (aria-activedescendant).
 *
 * Expansion is controlled by the parent: `toggle` asks to open / close a
 * directory, `select` reports a click / Enter on a row. Clicking a
 * directory selects and opens it; clicking the selected open one closes it.
 */
const props = withDefaults(
  defineProps<{
    label: string;
    roots: TreeNode[];
    childrenOf: (path: string) => TreeNode[] | undefined;
    expanded: Set<string>;
    selected?: string;
    /** Every directory open (search results) */
    expandAll?: boolean;
    /** Text to mark in names */
    highlight?: string;
    /** Merge chains of single-child directories into one row */
    flatten?: boolean;
    emptyNote?: (path: string) => string;
  }>(),
  {
    selected: undefined,
    highlight: undefined,
    flatten: true,
    emptyNote: undefined,
  },
);
const emit = defineEmits<{
  select: [node: TreeNode];
  toggle: [path: string, open: boolean];
}>();

const ROW = 30;
const OVERSCAN = 10;

const rows = computed(() =>
  visibleRows({
    roots: props.roots,
    childrenOf: props.childrenOf,
    isOpen: (path) => props.expandAll || props.expanded.has(path),
    emptyNote: props.emptyNote,
    flatten: props.flatten,
  }),
);
type NodeRow = Row & { kind: "node" };
const isNode = (r: Row | undefined): r is NodeRow => r?.kind === "node";
function indexOf(path: string | undefined) {
  return path === undefined
    ? -1
    : rows.value.findIndex((r) => isNode(r) && r.node.path === path);
}

// ── virtual scrolling ──
const scroller = useTemplateRef<HTMLElement>("scroller");
const { height: viewport } = useElementSize(scroller);
const scrollTop = ref(0);
const range = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN);
  const end = Math.min(
    rows.value.length,
    Math.ceil((scrollTop.value + viewport.value) / ROW) + OVERSCAN,
  );
  return Array.from({ length: Math.max(0, end - start) }, (_, i) => start + i);
});
/** Rendered rows with their index (the template narrows on `row.kind`). */
const slice = computed(() =>
  range.value.map((index) => ({ index, row: rows.value[index]! })),
);
const maxSticky = computed(() =>
  Math.max(1, Math.min(5, Math.floor(viewport.value / ROW / 3))),
);
const sticky = computed(() => {
  const { stack, shift } = stickyRows(
    rows.value,
    scrollTop.value,
    ROW,
    maxSticky.value,
  );
  return stack.map((index, k) => ({
    index,
    row: rows.value[index] as NodeRow,
    top: k * ROW + (k === stack.length - 1 ? shift : 0),
    z: stack.length - k,
  }));
});

function scrollToRow(index: number, center = false) {
  const el = scroller.value;
  const row = rows.value[index];
  if (!el || !row) return;
  const top = index * ROW;
  const covered = Math.min(row.depth, maxSticky.value) * ROW;
  if (center) el.scrollTop = Math.max(0, top - (viewport.value - ROW) / 2);
  else if (top < el.scrollTop + covered) el.scrollTop = top - covered;
  else if (top + ROW > el.scrollTop + viewport.value)
    el.scrollTop = top + ROW - viewport.value;
  scrollTop.value = el.scrollTop;
}

// ── active row (keyboard focus) and selection ──
const id = useId();
const activePath = ref<string>();
const activeIndex = computed(() => indexOf(activePath.value));
const rowId = (index: number) => `${id}-row-${index}`;

/** Reveal the selected row once it exists (lazy trees load ancestors first). */
let revealed: string | undefined;
watch(
  [() => props.selected, rows, viewport],
  async ([selected]) => {
    if (!selected || selected === revealed || !viewport.value) return;
    const index = indexOf(selected);
    if (index < 0) return;
    revealed = selected;
    activePath.value = selected;
    await nextTick();
    const top = index * ROW - scrollTop.value;
    scrollToRow(index, top < 0 || top > viewport.value - ROW);
  },
  { immediate: true },
);

function activate(row: NodeRow) {
  activePath.value = row.node.path;
  const { node, open } = row;
  if (node.isDir && open && props.selected === node.path) {
    emit("toggle", node.path, false);
    return;
  }
  emit("select", node);
  if (node.isDir && !open) emit("toggle", node.path, true);
}

function moveTo(index: number) {
  const row = rows.value[index];
  if (!isNode(row)) return;
  activePath.value = row.node.path;
  scrollToRow(index);
}
/** Next node row from `index` in direction `step`, skipping notes. */
function step(index: number, by: number) {
  for (let i = index + by; i >= 0 && i < rows.value.length; i += by)
    if (isNode(rows.value[i])) return i;
  return index;
}

let typed = "";
let typedAt = 0;
function onKey(e: KeyboardEvent) {
  const i = activeIndex.value < 0 ? step(-1, 1) : activeIndex.value;
  const row = rows.value[i];
  if (!isNode(row)) return;
  const page = Math.max(1, Math.floor(viewport.value / ROW) - 1);
  let handled = true;
  switch (e.key) {
    case "ArrowDown":
      moveTo(activeIndex.value < 0 ? i : step(i, 1));
      break;
    case "ArrowUp":
      moveTo(step(i, -1));
      break;
    case "PageDown":
      moveTo(step(Math.min(rows.value.length, i + page), -1));
      break;
    case "PageUp":
      moveTo(step(Math.max(-1, i - page), 1));
      break;
    case "Home":
      moveTo(step(-1, 1));
      break;
    case "End":
      moveTo(step(rows.value.length, -1));
      break;
    case "ArrowRight":
      if (row.node.isDir && !row.open) emit("toggle", row.node.path, true);
      else if (row.open && rows.value[i + 1]?.parent === i) moveTo(i + 1);
      break;
    case "ArrowLeft":
      if (row.open) emit("toggle", row.node.path, false);
      else if (row.parent >= 0) moveTo(row.parent);
      break;
    case "Enter":
    case " ":
      activate(row);
      break;
    default:
      handled = false;
  }
  if (handled) {
    e.preventDefault();
    return;
  }
  // Type-ahead: jump to the next row whose name starts with what was typed.
  if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
    const now = Date.now();
    typed = (now - typedAt > 600 ? "" : typed) + e.key.toLowerCase();
    typedAt = now;
    const n = rows.value.length;
    for (let k = typed.length === 1 ? 1 : 0; k <= n; k++) {
      const j = (i + k) % n;
      const r = rows.value[j];
      if (isNode(r) && r.label.toLowerCase().startsWith(typed)) {
        moveTo(j);
        break;
      }
    }
  }
}
function onFocus() {
  if (activeIndex.value < 0) {
    const selected = indexOf(props.selected);
    activePath.value = selected >= 0 ? props.selected : undefined;
    if (activeIndex.value < 0) moveTo(step(-1, 1));
  }
}

/** A click on a sticky row brings that directory back into view. */
function jumpTo(index: number) {
  const row = rows.value[index];
  if (!isNode(row)) return;
  scrollToRow(index);
  activate(row);
}
</script>

<style scoped lang="scss">
$row: 30px;

.file-tree {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
  outline: 0;

  &__space {
    position: relative;
  }

  &__sticky {
    position: sticky;
    top: 0;
    z-index: 2;
    height: 0;
  }

  &__row,
  &__note {
    position: absolute;
    left: 0;
    right: 0;
    height: $row;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px 0 calc(12px + var(--_d, 0) * 16px);
  }

  &__row {
    border-left: 2px solid transparent;
    font-size: var(--ak-fs-sm);
    color: var(--ak-fg-secondary);
    cursor: pointer;
    user-select: none;

    &:hover {
      background: var(--ak-bg-hover);
      color: var(--ak-fg);
    }

    &.is-selected {
      background: var(--ak-accent-subtle);
      border-left-color: var(--ak-accent);
      color: var(--ak-fg);
      font-weight: 600;
    }

    // Opaque, so the rows scrolling underneath do not show through.
    &.is-sticky {
      background: var(--ak-bg-surface);
      box-shadow: 0 1px 0 var(--ak-border-subtle);

      &:hover {
        background: linear-gradient(var(--ak-bg-hover), var(--ak-bg-hover))
          var(--ak-bg-surface);
      }

      &.is-selected {
        background: linear-gradient(
            var(--ak-accent-subtle),
            var(--ak-accent-subtle)
          )
          var(--ak-bg-surface);
      }
    }
  }

  &:focus-visible &__row.is-active {
    outline: 2px solid var(--ak-focus);
    outline-offset: -2px;
  }

  &__note {
    padding-left: calc(34px + var(--_d, 0) * 16px);
    font-size: var(--ak-fs-xs);
    color: var(--ak-fg-subtle);
  }
}
</style>
