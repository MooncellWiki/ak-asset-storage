<template>
  <section class="ak-card ak-card--accent-left latest" aria-label="最新版本">
    <div>
      <div class="latest__client">{{ version.clientVersion }}</div>
      <div class="latest__res">
        <span>{{ version.resVersion }}</span>
        <CopyButton :text="version.resVersion" label="复制版本号" />
      </div>
    </div>
    <AkTag :variant="versionStatus(version).variant">{{
      versionStatus(version).label
    }}</AkTag>
    <div class="latest__actions">
      <RouterLink
        v-if="previous(version)"
        class="ak-btn ak-btn--primary"
        :to="diffLink(previous(version), version)"
      >
        对比上一版
      </RouterLink>
      <RouterLink
        v-if="hasManifest(version)"
        class="ak-btn"
        :to="{ path: '/manifest', query: { version: version.resVersion } }"
      >
        清单
      </RouterLink>
      <RouterLink
        v-if="hasGamedata"
        class="ak-btn"
        :to="{
          path: '/asset',
          query: { path: `gamedata/${version.resVersion}` },
        }"
      >
        数据表
      </RouterLink>
      <AkDropdown :options="MORE" placement="bottom-end" @select="onMore">
        <AkButton variant="ghost" icon="more" label="更多" />
      </AkDropdown>
    </div>
  </section>
  <RelaunchDialog v-model="relaunchOpen" :version="version" />
</template>

<script setup lang="ts">
import {
  AkButton,
  AkDropdown,
  AkTag,
  useToast,
} from "@mooncellwiki/prts-design-vue";
import { ref } from "vue";
import { client } from "~/common/client";
import { useCopy } from "~/common/clipboard";
import { downloadText } from "~/common/utils";
import {
  diffLink,
  hasManifest,
  torappuCommand,
  useVersions,
  versionStatus,
} from "~/common/versions";
import CopyButton from "../CopyButton.vue";
import RelaunchDialog from "../RelaunchDialog.vue";
import type { DropdownKey } from "@mooncellwiki/prts-design-vue";
import type { Version } from "~/common/versions";

/** The newest version: client version, res version, status and its actions. */
const props = defineProps<{ version: Version; hasGamedata: boolean }>();

const { previous } = useVersions();
const copy = useCopy();
const toast = useToast();

const relaunchOpen = ref(false);
const MORE = [
  { key: "command", label: "复制 torappu 命令", icon: "source" as const },
  { key: "hot-update", label: "下载热更新列表", icon: "download" as const },
  { type: "divider" as const },
  { key: "relaunch", label: "重新解包…", icon: "refresh" as const },
];

async function onMore(key: DropdownKey) {
  const v = props.version;
  if (key === "command") {
    await copy(torappuCommand(v, previous(v)), "已复制 torappu 命令");
  } else if (key === "hot-update") {
    const { data } = await client.GET("/api/v1/version/{id}", {
      params: { path: { id: v.id } },
    });
    if (data)
      downloadText(data.hotUpdateList, `hot_update_list_${v.resVersion}.json`);
    else toast.error("没拿到热更新列表");
  } else if (key === "relaunch") {
    relaunchOpen.value = true;
  }
}
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

.latest {
  display: flex;
  flex-flow: row wrap;
  align-items: center;
  gap: var(--ak-space-4) var(--ak-space-8);
  margin-bottom: var(--ak-space-4);
  padding: var(--ak-space-5) var(--ak-space-6);

  &__client {
    font: 800 var(--ak-fs-display) / 1 var(--ak-font-display);
    font-variant-numeric: lining-nums;
    letter-spacing: var(--ak-tracking-display);
  }

  &__res {
    @include mono(15px);

    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    font-weight: 600;
  }

  &__actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ak-space-2);
    margin-left: auto;

    @media (max-width: $bp-phone) {
      margin-left: 0;
    }
  }
}
</style>
