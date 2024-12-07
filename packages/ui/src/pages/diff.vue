<template>
  <div class="flex items-center justify-center">
    <NSelect v-model:value="left" class="w-80" :options="versionOpts"></NSelect>
    <NButton class="mx-2" secondary circle @click="switchVer">
      <template #icon>
        <CarbonArrowsHorizontal />
      </template>
    </NButton>
    <NSelect
      v-model:value="right"
      class="w-80"
      :options="versionOpts"
    ></NSelect>
  </div>
  <div class="mt-2 flex">
    <NInput class="flex-grow" placeholder="搜索文件"></NInput>
    <NPopover trigger="click">
      <template #trigger>
        <NButton circle secondary class="ml-2">
          <CarbonSettings />
        </NButton>
      </template>
      <div class="flex">
        只显示有变更的文件
        <NSwitch v-model:value="changeOnly" class="ml-2" />
      </div>
    </NPopover>
  </div>
  <NTree
    class="mt-2"
    :override-default-node-click-behavior="override"
    :selectable="false"
    :data="treeData"
    :render-label="renderLabel"
    :checkable="false"
    block-line
    block-node
  ></NTree>
  <FileDetailDiff
    v-model:visible="detailVisible"
    :left="leftDetail"
    :right="rightDetail"
  ></FileDetailDiff>
</template>
<script setup lang="ts">
import { useUrlSearchParams } from "@vueuse/core";
import CarbonArrowsHorizontal from "~icons/carbon/arrows-horizontal";
import CarbonSettings from "~icons/carbon/settings";
import { NButton, NInput, NPopover, NSelect, NSwitch, NTree } from "naive-ui";
import { h, onBeforeMount, ref, watch } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import type { components } from "~/common/schema";
import FileDetailDiff from "./components/FileDetailDiff.vue";
import type { TreeOption, TreeOverrideNodeClickBehavior } from "naive-ui";
import type { TreeRenderProps } from "naive-ui/es/tree/src/interface";
import type { VNodeChild } from "vue";

const params = useUrlSearchParams("history");
const left = ref<number>();
const right = ref<number>();
const { versions, versionOpts, load: loadVersions } = useVersionSelect();
watch(
  () => [left.value, right.value],
  () => {
    const l = versions.value.find((v) => v.id === left.value);
    const r = versions.value.find((v) => v.id === right.value);
    params.diff = `${l?.res || ""}...${r?.res || ""}`;
  },
);

onBeforeMount(async () => {
  await loadVersions();
  if (params.diff) {
    const arr = (params.diff as string).split("...");
    if (arr.length === 2) {
      if (arr[0]) {
        const v = versions.value.find((v) => v.res === arr[0]);
        if (v) {
          left.value = v.id;
        }
      }
      if (arr[1]) {
        const v = versions.value.find((v) => v.res === arr[1]);
        if (v) {
          right.value = v.id;
        }
      }
    }
  }
});
function switchVer() {
  const leftVal = left.value;
  left.value = right.value;
  right.value = leftVal;
}
function dirOrder(a: string, b: string) {
  if (a === b) {
    return 0;
  } else if (a < b) {
    return -1;
  } else {
    return 1;
  }
}
const treeData = ref<TreeOption[]>([]);
const changeOnly = ref(true);
interface VersionFiles {
  pathMap: Record<string, components["schemas"]["FileDetail"]>;
  list: components["schemas"]["FileDetail"][];
}
let lData: VersionFiles = { pathMap: {}, list: [] };
let rData: VersionFiles = { pathMap: {}, list: [] };
async function loadTree(id: number) {
  const resp = await client.GET("/api/v1/version/{id}/files", {
    params: { path: { id } },
  });
  const pathMap: Record<string, components["schemas"]["FileDetail"]> = {};
  const list = (resp.data ?? []).sort((a, b) => {
    return dirOrder(a.path, b.path);
  });
  for (const item of list) {
    pathMap[item.path] = item;
  }
  return { pathMap, list };
}
watch(
  () => [left.value, right.value],
  async () => {
    const hasLeft = typeof left.value === "number";
    const hasRight = typeof right.value === "number";
    if (hasLeft) {
      lData = await loadTree(left.value!);
    }
    if (hasRight) {
      rData = await loadTree(right.value!);
    }
    const top: TreeOption = { children: [] };
    function updateTree(list: components["schemas"]["FileDetail"][]) {
      for (const item of list) {
        const paths = item.path.split("/");
        let cur = top;
        for (let i = 0; i < paths.length; i++) {
          const curPath = paths.slice(0, i + 1).join("/");
          const child = cur.children!.find((v) => v.key === curPath);
          if (!child) {
            const next = {
              key: curPath,
              children: i === paths.length - 1 ? void 0 : [],
              label: paths[i],
              isLeaf: i === paths.length - 1,
            };
            cur.children!.push(next);
            cur = next;
          } else {
            cur = child;
          }
        }
      }
    }
    if (hasLeft && hasRight && changeOnly.value) {
      const lList = lData!.list.filter((v) => {
        const right = rData!.pathMap[v.path];
        if (!right) {
          return true;
        }
        return v.hash !== right.hash;
      });
      const rList = rData!.list.filter((v) => {
        const left = lData!.pathMap[v.path];
        if (!left) {
          return true;
        }
        return v.hash !== left.hash;
      });
      updateTree(lList);
      updateTree(rList);
    } else {
      if (hasLeft) {
        updateTree(lData.list);
      }
      if (hasRight) {
        updateTree(rData.list);
      }
    }
    treeData.value = top.children!;
  },
);

const override: TreeOverrideNodeClickBehavior = ({ option }) => {
  if (option.children) {
    return "toggleExpand";
  }
  return "default";
};
function renderLabel(props: TreeRenderProps): VNodeChild {
  if (typeof left.value !== "number" || typeof right.value !== "number") {
    return h("div", undefined, props.option.label);
  }
  const l = lData.pathMap[props.option.key!];
  const r = rData.pathMap[props.option.key!];
  if (l && r) {
    if (l.hash === r.hash) {
      return h("div", { onClick: () => onLabelClick(r) }, props.option.label);
    } else {
      return h(
        "div",
        { class: "label_file_changed", onClick: () => onLabelClick(l, r) },
        props.option.label,
      );
    }
  } else if (l && !r) {
    return h(
      "div",
      { class: "label_file_remove", onClick: () => onLabelClick(l, r) },
      props.option.label,
    );
  } else if (!l && r) {
    return h(
      "div",
      { class: "label_file_add", onClick: () => onLabelClick(l, r) },
      props.option.label,
    );
  } else {
    return h("div", undefined, props.option.label);
  }
}
const detailVisible = ref(false);
const leftDetail = ref<components["schemas"]["FileDetail"]>();
const rightDetail = ref<components["schemas"]["FileDetail"]>();
function onLabelClick(
  left?: components["schemas"]["FileDetail"],
  right?: components["schemas"]["FileDetail"],
) {
  leftDetail.value = left;
  rightDetail.value = right;
  detailVisible.value = true;
}
</script>
<style lang="css">
.label_file_changed {
  --at-apply: bg-yellow-500;
}
.label_file_remove {
  --at-apply: bg-red-500;
}
.label_file_add {
  --at-apply: bg-green-500;
}
</style>
