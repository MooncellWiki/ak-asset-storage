<template>
  <NCard class="mb-2">
    <NForm>
      <NGrid :cols="25" :x-gap="24">
        <NFormItemGi label="开始时间" :span="5">
          <NDatePicker
            v-model:value="startTime"
            type="datetime"
            clearable
          ></NDatePicker>
        </NFormItemGi>
        <NFormItemGi label="结束时间" :span="5">
          <NDatePicker
            v-model:value="endTime"
            type="datetime"
            clearable
          ></NDatePicker>
        </NFormItemGi>
        <NFormItemGi label="创建人" :span="5">
          <NSelect v-model:value="bot" :options="bots" clearable></NSelect>
        </NFormItemGi>
        <NFormItemGi label="游戏版本" :span="5">
          <NSelect
            v-model:value="bundleVersion"
            :options="versionOpts"
            clearable
          ></NSelect>
        </NFormItemGi>
        <NFormItemGi :span="5">
          <NButton type="primary" @click="load">搜索</NButton>
        </NFormItemGi>
      </NGrid>
    </NForm>
    <NDataTable :columns="columns" :data="data"></NDataTable>
  </NCard>
</template>
<script setup lang="ts">
import { NDatePicker, type DataTableColumns } from "naive-ui";
import { onBeforeMount, ref } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import type { components } from "~/common/schema";

const columns: DataTableColumns<
  components["schemas"]["UnpackVersionDetailDto"]
> = [
  {
    title: "id",
    key: "id",
  },
  {
    title: "版本",
    key: "version",
    render: (row) => {
      return `${row.client}@${row.res}`;
    },
  },
  {
    title: "开始时间",
    key: "startTime",
  },
  {
    title: "结束时间",
    key: "endTime",
  },
  {
    title: "创建人",
    key: "tokenName",
  },
];
const data = ref<components["schemas"]["UnpackVersionDetailDto"][]>([]);
const bots = ref<{ label: string; value: number }[]>([]);
const startTime = ref();
const endTime = ref();
const bot = ref<number>();
const bundleVersion = ref<number>();
const { versionOpts, load: loadVersions } = useVersionSelect();
async function load() {
  const result = await client.GET("/api/v1/unpack-version", {
    params: {
      query: {
        startTime: startTime.value,
        endTime: endTime.value,
        token: bot.value,
        bundleVersion: bundleVersion.value,
      },
    },
  });
  data.value = result.data ?? [];
}
onBeforeMount(async () => {
  load();
  loadVersions();
  const tokens = await client.GET("/api/v1/token");
  bots.value = (tokens.data ?? []).map((v) => ({
    label: v.name,
    value: v.id,
  }));
});
</script>
