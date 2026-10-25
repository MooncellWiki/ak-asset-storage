<template>
  <PageHeader title="没有这个页面" en="Not Found" />
  <AkEmpty code="404" title="这个地址没有对应的页面">
    <template #extra>
      <RouterLink class="ak-btn ak-btn--sm" to="/">回到版本列表</RouterLink>
    </template>
  </AkEmpty>
</template>

<script setup lang="ts">
import { AkEmpty } from "@mooncellwiki/prts-design-vue";
import PageHeader from "~/components/PageHeader.vue";
</script>
