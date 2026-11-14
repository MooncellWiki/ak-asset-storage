<template>
  <table class="ak-table ak-table--compact launcher-files">
    <thead>
      <tr>
        <th scope="col">路径</th>
        <th scope="col" class="launcher-files__size">大小</th>
        <th scope="col" class="launcher-files__hash">MD5</th>
        <th scope="col" class="launcher-files__archive">归档</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="file in files" :key="file.path">
        <td>
          <LauncherPath :path="file.path" />
          <div class="launcher-files__meta">
            {{ formatSize(file.size) }} · MD5 <code>{{ file.md5 }}</code>
          </div>
        </td>
        <td class="launcher-files__size">{{ formatSize(file.size) }}</td>
        <td class="launcher-files__hash">{{ file.md5 }}</td>
        <td class="launcher-files__archive">
          <a
            v-if="file.sha256"
            class="ak-btn ak-btn--ghost ak-btn--sm"
            :href="storageUrl(file.sha256, '/storage/launcher/blobs')"
            :download="splitPath(file.path).name"
            :aria-label="`下载 ${splitPath(file.path).name}`"
          >
            <AsIcon name="download" :size="16" />下载
          </a>
          <AkTag
            v-else
            size="sm"
            :variant="
              file.retriesExhausted
                ? 'danger'
                : file.lastError
                  ? 'warning'
                  : 'default'
            "
            :title="file.lastError ?? undefined"
          >
            {{
              file.retriesExhausted
                ? `停止重试 (${file.attempts})`
                : file.lastError
                  ? `等待重试 (${file.attempts})`
                  : "待归档"
            }}
          </AkTag>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<script setup lang="ts">
import { AkTag } from "@mooncellwiki/prts-design-vue";
import { formatSize, splitPath, storageUrl } from "~/common/utils";
import AsIcon from "../AsIcon.vue";
import LauncherPath from "./LauncherPath.vue";
import type { components } from "~/common/schema";

defineProps<{ files: components["schemas"]["LauncherFile"][] }>();
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.launcher-files {
  table-layout: fixed;

  &__size {
    @include num;

    width: 7rem;
  }

  &__hash {
    width: 15rem;
    overflow-wrap: anywhere;
  }

  td#{&}__hash {
    @include mono(12px);

    color: var(--ak-fg-muted);
  }

  &__archive {
    width: 8rem;
    text-align: center;
  }

  &__meta {
    display: none;
    overflow-wrap: anywhere;
    color: var(--ak-fg-muted);
    font-size: var(--ak-fs-xs);
  }

  @media (max-width: 759px) {
    &__size,
    &__hash {
      display: none;
    }

    &__archive {
      width: 6.5rem;
    }

    &__meta {
      display: block;
      line-height: 1.35;

      code {
        @include mono(11px);

        display: block;
      }
    }
  }
}
</style>
