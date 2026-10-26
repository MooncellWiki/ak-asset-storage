<template>
  <div class="preview">
    <section class="preview__stage" :aria-label="entry.name">
      <div class="preview__head">
        <PathBar :segments="segments" :copy="entry.path" />
      </div>
      <div class="preview__bar">
        <span class="preview__dim">{{ formatSize(entry.size) }}</span>
        <div
          v-if="tabs"
          class="ak-tabs preview__tabs"
          role="tablist"
          aria-label="查看方式"
        >
          <button
            v-for="(label, i) in tabs"
            :key="label"
            type="button"
            class="ak-tab"
            :class="{ 'is-active': pretty === (i === 0) }"
            role="tab"
            :aria-selected="pretty === (i === 0)"
            @click="pretty = i === 0"
          >
            {{ label }}
          </button>
        </div>
        <div class="preview__actions">
          <button
            v-if="text !== undefined"
            type="button"
            class="ak-btn ak-btn--sm"
            @click="copy(shown, '已复制文件内容')"
          >
            <AsIcon name="copy" :size="16" />复制
          </button>
          <a class="ak-btn ak-btn--sm" :href="url" :download="entry.name">
            <AsIcon name="download" :size="16" />下载
          </a>
        </div>
      </div>

      <div v-if="kind === 'image'" class="preview__media is-checker">
        <img :src="url" :alt="entry.name" />
      </div>
      <div v-else-if="kind === 'audio'" class="preview__media">
        <audio controls preload="metadata" :src="url"></audio>
      </div>
      <div v-else-if="kind === 'video'" class="preview__media">
        <video controls preload="metadata" :src="url"></video>
      </div>
      <AkEmpty
        v-else-if="tooLarge"
        icon="empty"
        :title="`${entry.name} 有 ${formatSize(entry.size)}，不在页面里预览`"
      >
        <template #extra>
          <button
            type="button"
            class="ak-btn ak-btn--sm ak-btn--primary"
            @click="download(url, entry.name)"
          >
            <AsIcon name="download" :size="16" />下载
          </button>
        </template>
      </AkEmpty>
      <template v-else-if="isText">
        <AkEmpty v-if="failed" icon="warn" title="没加载出来" />
        <PaneNote v-else-if="text === undefined"
          ><AkSkeleton text :repeat="12"
        /></PaneNote>
        <MarkdownView v-else-if="isMarkdown && pretty" :text="text" />
        <CodeView
          v-else
          :text="shown"
          :lang="kind === 'json' ? 'json' : lang"
          :wrap="(kind === 'text' && !lang) || (kind === 'json' && !pretty)"
        />
      </template>
      <AkEmpty v-else icon="empty" title="这种文件不在页面里预览">
        <template #extra>
          <button
            type="button"
            class="ak-btn ak-btn--sm"
            @click="download(url, entry.name)"
          >
            <AsIcon name="download" :size="16" />下载
          </button>
        </template>
      </AkEmpty>
    </section>

    <PaneBox tag="aside" label="文件信息">
      <KvList>
        <dt>路径</dt>
        <dd class="preview__path">
          <span>{{ parentPath(entry.path) }}/</span>{{ entry.name }}
        </dd>
        <dt>大小</dt>
        <dd>{{ formatSize(entry.size) }}</dd>
        <dt>修改时间</dt>
        <dd>{{ formatDateTime(entry.modified_at) }}</dd>
        <dt>直链</dt>
        <dd>
          <button
            type="button"
            class="ak-btn ak-btn--sm"
            @click="copy(absoluteUrl(url), '已复制直链')"
          >
            <AsIcon name="link" :size="16" />复制直链
          </button>
        </dd>
      </KvList>
      <h3 class="preview__sub">{{ folder }}</h3>
      <ResList>
        <ResItem
          v-for="s in siblings"
          :key="s.path"
          :to="{ query: { path: s.path } }"
          :title="s.name"
          :count="formatSize(s.size)"
          :icon="entryIcon(s)"
          :selected="s.path === entry.path"
        />
      </ResList>
      <PaneNote v-if="siblings.length === MAX_SIBLINGS"
        >只列出前 {{ MAX_SIBLINGS }} 个</PaneNote
      >
    </PaneBox>
  </div>
</template>

<script setup lang="ts">
import { AkEmpty, AkSkeleton } from "@mooncellwiki/prts-design-vue";
import { computed, ref, watch } from "vue";
import { useCopy } from "~/common/clipboard";
import { languageOf } from "~/common/highlight";
import {
  absoluteUrl,
  assetUrl,
  download,
  fileKind,
  formatDateTime,
  formatSize,
  parentPath,
} from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import CodeView from "../CodeView.vue";
import KvList from "../KvList.vue";
import PaneBox from "../layout/PaneBox.vue";
import PaneNote from "../layout/PaneNote.vue";
import MarkdownView from "../MarkdownView.vue";
import PathBar from "../PathBar.vue";
import ResItem from "../ResItem.vue";
import ResList from "../ResList.vue";
import { entryIcon } from "./entries";
import type { PathSegment } from "../PathBar.vue";
import type { AssetEntry } from "./entries";

/**
 * One file: a preview by type (image, audio, video, highlighted text,
 * Markdown) with copy / download, and its details and siblings on the side.
 */
const props = defineProps<{
  entry: AssetEntry;
  siblings: AssetEntry[];
  segments: PathSegment[];
}>();

/** Text files larger than this are offered as a download only. */
const MAX_PREVIEW = 1024 * 1024;
const MAX_SIBLINGS = 200;

const copy = useCopy();
const url = computed(() => assetUrl(props.entry.path));
const kind = computed(() => fileKind(props.entry.name));
const lang = computed(() => languageOf(props.entry.name));
const isMarkdown = computed(() =>
  /\.(?:md|markdown)$/iu.test(props.entry.name),
);
const isText = computed(
  () => kind.value === "json" || kind.value === "text" || !!lang.value,
);
const tooLarge = computed(() => isText.value && props.entry.size > MAX_PREVIEW);

const text = ref<string>();
const failed = ref(false);
/** Formatted JSON / rendered Markdown, or the raw source */
const pretty = ref(true);
let request = 0;
watch(
  () => props.entry.path,
  async () => {
    const seq = ++request;
    text.value = undefined;
    failed.value = false;
    pretty.value = true;
    if (!isText.value || tooLarge.value) return;
    try {
      const response = await fetch(url.value);
      if (!response.ok) throw new Error(String(response.status));
      const body = await response.text();
      if (seq === request) text.value = body;
    } catch {
      if (seq === request) failed.value = true;
    }
  },
  { immediate: true },
);

/** Pretty-printed JSON, or undefined when the text is not valid JSON. */
const formatted = computed(() => {
  if (kind.value !== "json" || text.value === undefined) return undefined;
  try {
    return JSON.stringify(JSON.parse(text.value), undefined, 2);
  } catch {
    return undefined;
  }
});
const tabs = computed(() => {
  if (formatted.value !== undefined) return ["格式化", "原始"];
  if (isMarkdown.value && text.value !== undefined) return ["预览", "源码"];
  return undefined;
});
const shown = computed(
  () => (pretty.value && formatted.value) || text.value || "",
);

const folder = computed(
  () => parentPath(props.entry.path).split("/").at(-1) ?? "",
);
const siblings = computed(() =>
  props.siblings.filter((e) => !e.is_dir).slice(0, MAX_SIBLINGS),
);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.preview {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: var(--ak-space-4);
  align-items: start;

  @media (max-width: $bp-narrow) {
    grid-template-columns: minmax(0, 1fr);
  }

  &__stage {
    min-width: 0;
    background: var(--ak-bg-surface);
    border: 1px solid var(--ak-border);
  }

  &__head {
    padding: var(--ak-space-3) var(--ak-space-4);
    border-bottom: 1px solid var(--ak-border);
  }

  &__bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ak-space-3) var(--ak-space-4);
    min-height: 44px;
    padding: 0 var(--ak-space-4);
    border-bottom: 1px solid var(--ak-border);
  }

  &__tabs {
    border-bottom: 0;
  }

  &__actions {
    display: inline-flex;
    gap: 4px;
    margin-left: auto;
  }

  &__dim {
    color: var(--ak-fg-muted);
  }

  &__media {
    display: grid;
    place-items: center;
    min-height: 240px;
    padding: var(--ak-space-4);

    &.is-checker {
      @include checker;
    }

    > img,
    > video {
      display: block;
      max-width: 100%;
      max-height: 72vh;
      object-fit: contain;
    }

    > audio {
      width: min(100%, 560px);
    }
  }

  &__path {
    @include mono;

    display: block;
    overflow-wrap: anywhere;

    > span {
      color: var(--ak-fg-muted);
    }
  }

  &__sub {
    margin: 0;
    padding: var(--ak-space-3) var(--ak-space-4) var(--ak-space-2);
    border-top: 1px solid var(--ak-border);
    font-size: var(--ak-fs-sm);
    font-weight: 700;
  }
}
</style>
