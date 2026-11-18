<template>
  <ChangeDetail
    :path="change.path"
    :kind="change.kind"
    :before="{
      version: versionLabel(before),
      size: change.oldSize,
      hash: change.oldMd5,
    }"
    :after="{
      version: versionLabel(after),
      size: change.newSize,
      hash: change.newMd5,
    }"
    @close="emit('close')"
  >
    <template #before-action>
      <LauncherFileDownload :release-id="before.id" :path="change.path" />
    </template>
    <template #after-action>
      <LauncherFileDownload
        :release-id="after.id"
        :path="change.path"
        primary
      />
    </template>
  </ChangeDetail>
</template>

<script setup lang="ts">
import ChangeDetail from "../diff/ChangeDetail.vue";
import LauncherFileDownload from "./LauncherFileDownload.vue";
import type { LauncherChange } from "./LauncherChangeTable.vue";
import type { components } from "~/common/schema";

type Release = components["schemas"]["LauncherRelease"];
defineProps<{ change: LauncherChange; before: Release; after: Release }>();
const emit = defineEmits<{ close: [] }>();

function versionLabel(release: Release) {
  return `#${release.id} · ${release.packageVersion} / ${release.gameVersion}`;
}
</script>
