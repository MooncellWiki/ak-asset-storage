<template>
  <NCard class="mb-2">
    <NForm :label-width="80" label-placement="left">
      <NGrid :cols="24" :x-gap="24">
        <NFormItemGi label="path" :span="6">
          <NInput v-model:value="model.path"></NInput>
        </NFormItemGi>
        <NFormItemGi label="hash" :span="6">
          <NInput v-model:value="model.hash"></NInput>
        </NFormItemGi>
        <NFormItemGi label="version" :span="6">
          <NSelect
            v-model:value="model.version"
            :options="versionOpts"
            clearable
          ></NSelect>
        </NFormItemGi>
        <NFormItemGi :span="6">
          <NButton type="primary" @click="search">搜索</NButton>
        </NFormItemGi>
      </NGrid>
    </NForm>
    <NDataTable
      virtual-scroll
      max-height="500px"
      size="small"
      :data="list"
      :columns="columns"
    ></NDataTable>
  </NCard>
  <NModal v-model:show="detailVisible" display-directive="if">
    <NCard class="w-fit">
      <FileDetailVue :detail="detail!" />
    </NCard>
  </NModal>
</template>
<script lang="ts" setup>
import { NButton } from "naive-ui";
import { h, onBeforeMount, ref } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import type { components } from "~/common/schema";
import FileDetailVue from "../components/FileDetail.vue";
import type { TableColumns } from "naive-ui/es/data-table/src/interface";

const model = ref<{
  path?: string;
  hash?: string;
  version?: number;
}>({});
const { versionOpts, load } = useVersionSelect();
onBeforeMount(() => {
  load();
});
const list = ref<components["schemas"]["BundleDetailsDto"][]>([]);
async function search() {
  if (model.value.path || model.value.hash || model.value.version) {
    const { data } = await client.GET("/bundle", {
      params: { query: model.value },
    });
    list.value = data ?? [];
  }
}
const columns: TableColumns<components["schemas"]["BundleDetailsDto"]> = [
  { title: "path", key: "path" },
  { title: "hash", key: "hash", width: 550 },
  { title: "clientVersion", key: "client", width: 140 },
  { title: "resVersion", key: "res", width: 250 },
  {
    key: "action",
    title: "操作",
    width: 140,
    render(row) {
      return h(NButton, { onClick: () => showDetail(row) }, () => "详情");
    },
  },
];
const detail = ref<components["schemas"]["BundleDetailsDto"]>();
const detailVisible = ref(false);
function showDetail(data: components["schemas"]["BundleDetailsDto"]) {
  detail.value = data;
  detailVisible.value = true;
}
</script>
