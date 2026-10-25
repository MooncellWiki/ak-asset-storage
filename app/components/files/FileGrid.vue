<template>
  <div class="grid">
    <RouterLink
      v-for="e in entries"
      :key="e.path"
      class="thumb"
      :to="linkFor(e)"
      :title="e.name"
    >
      <span
        class="thumb__img"
        :class="{ 'is-image': fileKind(e.name) === 'image' }"
      >
        <img
          v-if="fileKind(e.name) === 'image' && !failed.has(e.path)"
          :src="assetUrl(e.path)"
          alt=""
          loading="lazy"
          decoding="async"
          @error="onError(e.path)"
        />
        <AsIcon v-else :name="entryIcon(e)" :size="40" />
      </span>
      <span class="thumb__name">{{ e.name }}</span>
      <span class="thumb__meta">{{
        e.is_dir ? "目录" : formatSize(e.size)
      }}</span>
    </RouterLink>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { assetUrl, fileKind, formatSize } from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import { entryIcon } from "./entries";
import type { AssetEntry } from "./entries";

/** Directory as a thumbnail grid: images on a checkerboard, icons for the rest. */
defineProps<{ entries: AssetEntry[]; linkFor: (e: AssetEntry) => object }>();

const failed = ref(new Set<string>());
function onError(path: string) {
  failed.value = new Set(failed.value).add(path);
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
  gap: var(--ak-space-3);
  padding: var(--ak-space-4);
}

.thumb {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--ak-bg-surface);
  border: 1px solid var(--ak-border);
  color: inherit;
  text-decoration: none;
  transition: border-color var(--ak-dur-fast);

  &:visited {
    color: inherit;
  }

  &:hover {
    border-color: var(--ak-fg-muted);
    color: inherit;
    text-decoration: none;
  }

  &__img {
    aspect-ratio: 1;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-bottom: 1px solid var(--ak-border-subtle);
    color: var(--ak-fg-subtle);

    &.is-image {
      @include checker;
    }

    > img {
      display: block;
      max-width: 100%;
      max-height: 100%;
      object-fit: contain;
    }
  }

  &__name {
    padding: 6px 8px 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--ak-fs-xs);
    font-weight: 600;
  }

  &__meta {
    padding: 0 8px 6px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--ak-fg-subtle);
  }
}
</style>
