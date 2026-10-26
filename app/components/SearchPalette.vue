<template>
  <dialog
    ref="dialog"
    class="ak-palette"
    :class="{ 'has-query': query, 'is-loading': pending > 0 }"
    aria-label="搜索"
    @close="onClose"
    @click="onDialogClick"
  >
    <div class="ak-palette__head">
      <AsIcon name="search" :size="20" class="ak-palette__icon" />
      <form
        class="ak-palette__form"
        role="search"
        @submit.prevent="open(flat[active])"
      >
        <input
          ref="input"
          v-model="query"
          type="search"
          placeholder="搜索文件、清单资源、bundle 路径或 SHA-256"
          aria-label="搜索"
          autocomplete="off"
          spellcheck="false"
          role="combobox"
          aria-autocomplete="list"
          :aria-expanded="flat.length > 0"
          :aria-controls="`${id}-list`"
          :aria-activedescendant="flat.length ? optionId(active) : undefined"
          @keydown="onKeydown"
        />
      </form>
      <button
        type="button"
        class="ak-palette__clear"
        aria-label="清空"
        @click="
          query = '';
          input?.focus();
        "
      >
        <AsIcon name="x" :size="16" />
      </button>
      <button type="button" class="ak-palette__close" @click="close">
        取消
      </button>
      <div
        class="ak-palette__loading ak-progress ak-progress--sm ak-progress--indeterminate"
        aria-hidden="true"
      ></div>
    </div>

    <div class="ak-palette__body">
      <ul
        v-show="flat.length"
        :id="`${id}-list`"
        class="ak-palette__list"
        role="listbox"
        aria-label="结果"
      >
        <li
          v-for="g in groups"
          :key="g.key"
          class="ak-palette__group"
          role="group"
          :aria-label="g.label"
        >
          <div class="ak-palette__label" aria-hidden="true">
            {{ g.label }}<small v-if="g.note">{{ g.note }}</small>
          </div>
          <ul role="none">
            <li
              v-for="item in g.items"
              :id="optionId(indexOf(item))"
              :key="item.key"
              class="ak-palette__item"
              :class="{ 'is-active': indexOf(item) === active }"
              role="option"
              :aria-selected="indexOf(item) === active"
              @mousemove="active = indexOf(item)"
            >
              <RouterLink
                class="ak-palette__link"
                :to="item.to"
                tabindex="-1"
                @click="close"
              >
                <span
                  class="ak-palette__thumb"
                  :class="{
                    'ak-palette__thumb--image': item.image,
                    'ak-palette__thumb--icon': !item.image && !item.dark,
                    'ak-palette__thumb--dark': item.dark,
                  }"
                >
                  <img
                    v-if="item.image"
                    :src="item.image"
                    alt=""
                    loading="lazy"
                  />
                  <AsIcon v-else :name="item.icon" :size="20" />
                </span>
                <span class="ak-palette__text">
                  <span class="ak-palette__title">
                    <bdi v-if="highlight(item.title)" class="has-match"
                      >{{ highlight(item.title)![0]
                      }}<mark>{{ highlight(item.title)![1] }}</mark
                      >{{ highlight(item.title)![2] }}</bdi
                    >
                    <bdi v-else>{{ item.title }}</bdi>
                  </span>
                  <span class="ak-palette__desc">{{ item.desc }}</span>
                </span>
                <span v-if="item.meta" class="ak-palette__meta">{{
                  item.meta
                }}</span>
              </RouterLink>
              <span class="ak-palette__actions">
                <button
                  v-for="a in item.actions"
                  :key="a.label"
                  type="button"
                  class="ak-palette__action"
                  :title="a.label"
                  :aria-label="a.label"
                  tabindex="-1"
                  @click="a.run()"
                >
                  <AsIcon :name="a.icon" :size="14" />
                </button>
              </span>
            </li>
          </ul>
        </li>
      </ul>
      <div v-if="notice" class="ak-palette__notice ak-palette__notice--error">
        {{ notice }}
      </div>
      <div v-if="!query.trim()" class="ak-palette__empty">
        <p class="ak-palette__empty-title">搜索文件、清单资源与 bundle</p>
        <p class="ak-palette__empty-desc">
          按名字或路径的一段来找；整串 SHA-256 按哈希找 bundle。随时按
          <kbd class="ak-kbd">{{ isMac ? "⌘K" : "Ctrl K" }}</kbd
          >或<kbd class="ak-kbd">/</kbd>打开。
        </p>
        <div class="ak-palette__shortcuts">
          <RouterLink
            v-for="s in SHORTCUTS"
            :key="s.to"
            class="ak-palette__shortcut"
            :to="s.to"
            @click="close"
          >
            {{ s.label }}
          </RouterLink>
        </div>
      </div>
      <div
        v-else-if="!pending && !flat.length && !notice"
        class="ak-palette__notice"
      >
        没有匹配「{{ query.trim() }}」的结果
      </div>
      <div class="ak-sr-only" aria-live="polite">
        {{ query.trim() && !pending ? `${flat.length} 条结果` : "" }}
      </div>
    </div>

    <div class="ak-palette__foot">
      <div class="ak-palette__hints">
        <span><kbd class="ak-kbd">↑↓</kbd> 选择</span>
        <span><kbd class="ak-kbd">↵</kbd> 打开</span>
        <span><kbd class="ak-kbd">Esc</kbd> 关闭</span>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { useEventListener } from "@vueuse/core";
import {
  computed,
  nextTick,
  ref,
  shallowRef,
  useId,
  useTemplateRef,
  watch,
} from "vue";
import { useRouter } from "vue-router";
import { client } from "~/common/client";
import { useCopy } from "~/common/clipboard";
import { paletteOpen } from "~/common/palette";
import {
  absoluteUrl,
  assetUrl,
  bundleFilename,
  download,
  fileKind,
  formatSize,
  parentPath,
  storageUrl,
} from "~/common/utils";
import { diffLink, useVersions } from "~/common/versions";
import AsIcon from "./AsIcon.vue";
import { entryIcon } from "./files/entries";
import type { IconName } from "./AsIcon.vue";
import type { RouteLocationRaw } from "vue-router";

interface Action {
  label: string;
  icon: IconName;
  run: () => void;
}
interface Item {
  key: string;
  title: string;
  desc: string;
  meta?: string;
  icon: IconName;
  /** Image thumbnail instead of the icon */
  image?: string;
  dark?: boolean;
  to: RouteLocationRaw;
  actions: Action[];
}
interface Group {
  key: string;
  label: string;
  items: Item[];
  /** e.g. "前 6 条" when the source has more */
  note?: string;
}

const router = useRouter();
const copy = useCopy();
const { latest, latestWithManifest, byId, previous } = useVersions();
const id = useId();

const dialog = useTemplateRef<HTMLDialogElement>("dialog");
const input = useTemplateRef<HTMLInputElement>("input");
const query = ref("");
const active = ref(0);

// ── open / close ──
let returnFocus: HTMLElement | null = null;
watch(paletteOpen, async (open) => {
  const d = dialog.value;
  if (!d) return;
  if (open && !d.open) {
    returnFocus = document.activeElement as HTMLElement | null;
    d.showModal();
    await nextTick();
    input.value?.select();
  } else if (!open && d.open) {
    d.close();
  }
});
function onClose() {
  paletteOpen.value = false;
  returnFocus?.focus();
  returnFocus = null;
}
function close() {
  paletteOpen.value = false;
}
/** Clicks on the ::backdrop land on the dialog itself, outside its box. */
function onDialogClick(e: MouseEvent) {
  const r = dialog.value!.getBoundingClientRect();
  if (
    e.target === dialog.value &&
    (e.clientX < r.left ||
      e.clientX > r.right ||
      e.clientY < r.top ||
      e.clientY > r.bottom)
  )
    close();
}
useEventListener(document, "keydown", (e: KeyboardEvent) => {
  if (paletteOpen.value || e.defaultPrevented) return;
  const target = e.target as HTMLElement | null;
  const typing =
    target &&
    (target.isContentEditable ||
      /^(?:INPUT|TEXTAREA|SELECT)$/u.test(target.tagName));
  if (
    (e.key === "k" && (e.metaKey || e.ctrlKey)) ||
    (e.key === "/" && !typing)
  ) {
    e.preventDefault();
    paletteOpen.value = true;
  }
});

// ── search ──
const groups = shallowRef<Group[]>([]);
const pending = ref(0);
const notice = ref("");
let seq = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

watch(query, (text) => {
  clearTimeout(timer);
  const q = text.trim();
  seq++;
  active.value = 0;
  notice.value = "";
  if (!q) {
    groups.value = [];
    pending.value = 0;
    return;
  }
  timer = setTimeout(search, 250, q, seq);
});

function fileItem(e: {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}): Item {
  const url = assetUrl(e.path);
  return {
    key: `f:${e.path}`,
    title: e.name,
    desc: parentPath(e.path),
    meta: e.is_dir ? undefined : formatSize(e.size),
    icon: entryIcon(e),
    image: !e.is_dir && fileKind(e.name) === "image" ? url : undefined,
    to: { path: "/asset", query: { path: e.path } },
    actions: e.is_dir
      ? []
      : [
          {
            label: "复制直链",
            icon: "link",
            run: () => copy(absoluteUrl(url), "已复制直链"),
          },
          { label: "下载", icon: "download", run: () => download(url, e.name) },
        ],
  };
}

async function searchFiles(q: string, prefix: string, limit: number) {
  const { data, response } = await client.GET("/api/v1/files", {
    params: { query: { path: q, prefix, limit } },
  });
  if (!data) {
    if (response.status === 503)
      notice.value = "文件搜索繁忙或索引还在准备，稍后再试";
    return { items: [] as Item[], truncated: false };
  }
  return { items: data.results.map(fileItem), truncated: data.truncated };
}

async function searchManifest(q: string): Promise<Item[]> {
  const v = latestWithManifest.value;
  if (!v) return [];
  const { data } = await client.GET("/api/v1/manifest/{version_id}/search", {
    params: { path: { version_id: v.id }, query: { q } },
  });
  return (data ?? []).slice(0, 5).map((n) => ({
    key: `m:${n.path}`,
    title: n.name,
    desc: parentPath(n.path) || "/",
    icon: n.nodeType === "file" ? "file" : "folder",
    to: { path: "/manifest", query: { version: v.resVersion, path: n.path } },
    actions: [],
  }));
}

async function searchBundles(q: string): Promise<Item[]> {
  const byHash = /^[0-9a-f]{64}$/iu.test(q);
  const v = latest.value;
  if (!byHash && !v) return [];
  const { data } = await client.GET("/api/v1/bundle", {
    params: {
      query: byHash
        ? { hash: q.toLowerCase(), limit: 5 }
        : { path: q, version: v!.id, limit: 5 },
    },
  });
  return (data?.bundles ?? []).map((b) => {
    const version = byId(b.versionId);
    return {
      key: `b:${b.id}`,
      title: b.path,
      desc: `${b.versionClient} ${b.versionRes}`,
      meta: formatSize(b.fileSize),
      icon: "bundle",
      dark: true,
      to: diffLink(previous(version), version, { bundle: b.path }),
      actions: [
        {
          label: "下载",
          icon: "download",
          run: () => download(storageUrl(b.fileHash), bundleFilename(b)),
        },
      ],
    };
  });
}

function search(q: string, mine: number) {
  const result: Record<string, Group> = {};
  const order = ["files", "manifest", "bundle"];
  const publish = () => {
    if (mine === seq)
      groups.value = order.flatMap((k) =>
        result[k]?.items.length ? [result[k]!] : [],
      );
  };
  const run = (promise: Promise<void>) => {
    pending.value++;
    void promise
      .catch(() => {
        if (mine === seq) notice.value = "有一部分没搜出来，稍后再试";
      })
      .finally(() => {
        if (mine === seq) pending.value--;
      });
  };
  // The previous query's results stay until the first source answers.
  pending.value = 0;
  run(
    Promise.all([
      searchFiles(q, "raw/", 6),
      searchFiles(q, "gamedata/latest/", 4),
    ]).then(([raw, gamedata]) => {
      result.files = {
        key: "files",
        label: "文件",
        items: [...raw.items, ...gamedata.items],
        note: raw.truncated || gamedata.truncated ? "只列出前几条" : undefined,
      };
      publish();
    }),
  );
  run(
    searchManifest(q).then((items) => {
      result.manifest = {
        key: "manifest",
        label: "清单资源",
        items,
        note: latestWithManifest.value?.resVersion,
      };
      publish();
    }),
  );
  run(
    searchBundles(q).then((items) => {
      result.bundle = {
        key: "bundle",
        label: "Bundle",
        items,
        note: latest.value?.resVersion,
      };
      publish();
    }),
  );
}

const flat = computed(() => groups.value.flatMap((g) => g.items));
const optionId = (i: number) => `${id}-opt-${i}`;
const indexOf = (item: Item) => flat.value.indexOf(item);

/** Title split around the first case-insensitive match of the query. */
function highlight(title: string): [string, string, string] | undefined {
  const q = query.value.trim();
  const at = q ? title.toLowerCase().indexOf(q.toLowerCase()) : -1;
  return at < 0
    ? undefined
    : [
        title.slice(0, at),
        title.slice(at, at + q.length),
        title.slice(at + q.length),
      ];
}

function open(item: Item | undefined) {
  if (!item) return;
  close();
  void router.push(item.to);
}

function onKeydown(e: KeyboardEvent) {
  const n = flat.value.length;
  if (e.key === "ArrowDown" && n) {
    e.preventDefault();
    active.value = (active.value + 1) % n;
  } else if (e.key === "ArrowUp" && n) {
    e.preventDefault();
    active.value = (active.value - 1 + n) % n;
  } else if (e.key === "Enter" && !e.isComposing) {
    e.preventDefault();
    open(flat.value[active.value]);
  }
}
watch(active, async (i) => {
  await nextTick();
  document.getElementById(optionId(i))?.scrollIntoView({ block: "nearest" });
});

const SHORTCUTS = [
  { to: "/", label: "版本" },
  { to: "/diff", label: "变更" },
  { to: "/manifest", label: "清单" },
  { to: "/asset", label: "文件" },
  { to: "/story", label: "剧情资源" },
];
const isMac = /Mac|iPhone|iPad/u.test(navigator.platform);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

// A modal <dialog> centred near the top: the skin's search palette, adapted
// from prts-design packages/css/src/chrome/search-palette.css.
.ak-palette {
  --_top: clamp(48px, 10vh, 120px);
  --_head-h: 56px;
  --_foot-h: 40px;

  position: fixed;
  z-index: calc(var(--ak-z-modal) + 1);
  inset: var(--_top) 0 auto;
  width: min(720px, calc(100vw - var(--ak-gutter) * 2));
  max-width: none;
  max-height: calc(100vh - var(--_top) * 2);
  height: auto;
  margin: 0 auto;
  padding: 0;
  overflow: hidden;
  flex-direction: column;
  background: var(--ak-bg-surface);
  color: var(--ak-fg);
  border: 1px solid var(--ak-border);
  border-top-width: 3px;
  border-image: linear-gradient(var(--ak-accent) 3px, var(--ak-border) 3px) 3 1
    1 1;
  box-shadow: var(--ak-shadow-lg);
  font-size: var(--ak-fs-sm);
  line-height: 1.4;

  &[open] {
    display: flex;
    animation: palette-in var(--ak-dur-normal) var(--ak-ease-out);
  }

  &::backdrop {
    background: var(--ak-bg-overlay);
    backdrop-filter: blur(2px);
  }

  &__head {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--_head-h);
    padding-right: 10px;
    border-bottom: 1px solid var(--ak-border);
  }

  &__icon {
    margin: 0 14px 0 18px;
    color: var(--ak-fg-muted);
  }

  &__form {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    margin: 0;

    input {
      flex: 1;
      min-width: 0;
      height: var(--_head-h);
      margin: 0;
      padding: 0;
      border: 0;
      border-radius: 0;
      background: transparent;
      color: inherit;
      font: inherit;
      font-size: var(--ak-fs-body);
      outline: 0;
      box-shadow: none;
      appearance: none;

      &::placeholder {
        color: var(--ak-fg-subtle);
      }

      &::-webkit-search-cancel-button,
      &::-webkit-search-decoration {
        display: none;
      }

      // iOS Safari zooms into inputs under 16px.
      @supports (-webkit-touch-callout: none) {
        font-size: 16px;
      }
    }
  }

  &__clear,
  &__close {
    flex: none;
    display: none;
    align-items: center;
    justify-content: center;
    height: 28px;
    min-width: 28px;
    padding: 0 4px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--ak-fg-muted);
    font: inherit;
    font-size: var(--ak-fs-xs);
    cursor: pointer;

    &:hover {
      color: var(--ak-fg);
      border-color: var(--ak-border-strong);
    }

    &:focus-visible {
      outline: 2px solid var(--ak-focus);
      outline-offset: -2px;
    }
  }

  &.has-query &__clear {
    display: inline-flex;
  }

  &__loading {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    background: transparent;
    visibility: hidden;
  }

  &.is-loading &__loading {
    visibility: visible;
  }

  &__body {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  &__list {
    max-height: calc(
      100vh - var(--_top) * 2 - var(--_head-h) - var(--_foot-h) - 4px
    );
    overflow-y: auto;
    overscroll-behavior: contain;
    margin: 0;
    padding: 6px 0;
    list-style: none;

    ul {
      list-style: none;
      margin: 0;
      padding: 0;
    }
  }

  &__group {
    margin: 0;

    & + & {
      margin-top: 4px;
      padding-top: 4px;
      border-top: 1px solid var(--ak-border-subtle);
    }
  }

  &__label {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 8px 16px 4px;
    font: 700 var(--ak-fs-overline) / 1 var(--ak-font-label);
    letter-spacing: var(--ak-tracking-overline);
    text-transform: uppercase;
    color: var(--ak-fg-muted);

    small {
      @include mono(10px);

      margin-left: auto;
      font-weight: 500;
      text-transform: none;
      color: var(--ak-fg-subtle);
    }
  }

  &__item {
    position: relative;
    display: flex;
    align-items: stretch;
    margin: 0;

    &.is-active {
      background: var(--ak-accent-subtle);
    }
  }

  &__link {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 16px 7px 14px;
    border-left: 2px solid transparent;
    color: inherit;
    text-decoration: none;
    cursor: pointer;

    &:hover,
    &:visited {
      color: inherit;
      text-decoration: none;
    }

    .is-active > & {
      border-left-color: var(--ak-accent);
    }
  }

  &__thumb {
    flex: none;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    overflow: hidden;
    background: var(--ak-bg-surface-3);
    border: 1px solid var(--ak-border);
    color: var(--ak-fg-muted);

    > img {
      width: 100%;
      height: 100%;
      object-fit: contain;
      display: block;
    }

    &--image {
      @include checker;
    }

    &--icon {
      background: var(--ak-bg-surface-2);
    }

    // Dark box, white glyph in both themes (.ak-glyph-box)
    &--dark {
      background: var(--ak-gray-900);
      border-color: var(--ak-gray-900);
      color: var(--ak-gray-0);
    }
  }

  &__text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  &__title {
    display: flex;
    min-width: 0;
    font-size: var(--ak-fs-body);
    font-weight: 600;
    color: var(--ak-fg);

    > bdi {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    // Titles with a match: the match bold, the rest regular.
    > .has-match {
      font-weight: 400;
    }

    mark {
      padding: 0;
      background: none;
      color: inherit;
      font-weight: 700;
    }
  }

  &__desc {
    font-size: var(--ak-fs-xs);
    color: var(--ak-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__meta {
    flex: none;
    margin-left: 4px;
    color: var(--ak-fg-muted);
    font-size: var(--ak-fs-xs);
    white-space: nowrap;
  }

  // Always laid out so rows keep their width; visible on the active row.
  &__actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 8px;
    visibility: hidden;

    .is-active > &,
    .ak-palette__item:focus-within > & {
      visibility: visible;
    }
  }

  &__action {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid transparent;
    background: transparent;
    color: var(--ak-fg-muted);
    cursor: pointer;

    &:hover,
    &:focus-visible {
      border-color: var(--ak-border-strong);
      color: var(--ak-fg);
      outline: 0;
    }
  }

  &__empty {
    padding: 20px 0 14px;
  }

  &__empty-title {
    margin: 0 20px 6px;
    font-size: var(--ak-fs-body);
    font-weight: 700;
  }

  &__empty-desc {
    margin: 0 20px 10px;
    color: var(--ak-fg-muted);

    .ak-kbd {
      margin: 0 2px;
    }
  }

  &__shortcuts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 2px 16px 0;
  }

  &__shortcut {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--ak-border-strong);
    color: var(--ak-fg-secondary);
    font-size: var(--ak-fs-xs);
    font-weight: 500;
    text-decoration: none;

    &:hover {
      border-color: var(--ak-accent);
      color: var(--ak-accent);
      text-decoration: none;
    }
  }

  &__notice {
    padding: 14px 16px;
    color: var(--ak-fg-muted);

    &--error {
      color: var(--ak-danger);
    }
  }

  &__foot {
    flex: none;
    display: flex;
    align-items: center;
    height: var(--_foot-h);
    padding: 0 12px 0 16px;
    border-top: 1px solid var(--ak-border);
    background: var(--ak-bg-surface-2);
    color: var(--ak-fg-muted);
    font-size: var(--ak-fs-xs);
  }

  &__hints {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-left: auto;
    white-space: nowrap;

    > span {
      display: inline-flex;
      align-items: center;
      gap: 4px;
    }
  }

  // Touch screens: no keyboard hints, row actions always shown, a cancel button.
  @media (hover: none) and (pointer: coarse) {
    &__hints {
      display: none;
    }

    &__actions {
      visibility: visible;
    }

    &__action {
      width: 36px;
      height: 36px;
    }

    &__close {
      display: inline-flex;
    }
  }

  @media (max-width: $bp-phone) {
    --_top: 8px;

    inset: var(--_top) 8px auto;
    width: auto;
    max-height: calc(100dvh - 16px);

    &__list {
      max-height: calc(100dvh - 16px - var(--_head-h) - var(--_foot-h) - 4px);
    }

    &__icon {
      margin: 0 10px 0 14px;
    }

    &__close {
      display: inline-flex;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    &[open] {
      animation: none;
    }
  }
}

@keyframes palette-in {
  from {
    opacity: 0;
    transform: translateY(-8px);
    clip-path: inset(0 0 100% 0);
  }

  to {
    opacity: 1;
    transform: none;
    clip-path: inset(0 0 0 0);
  }
}
</style>
