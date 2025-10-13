<template>
  <div class="flex items-center justify-center">
    <NSelect
      v-model:value="left"
      class="w-80"
      :options="versionOpts"
      clearable
    ></NSelect>
    <NButton class="mx-2" secondary circle @click="switchVer">
      <template #icon>
        <CarbonArrowsHorizontal />
      </template>
    </NButton>
    <NSelect
      v-model:value="right"
      class="w-80"
      :options="versionOpts"
      clearable
    ></NSelect>
    <NButton class="ml-2" secondary circle @click="copyCommand">
      <template #icon>
        <CarbonCopy />
      </template>
    </NButton>
  </div>
  <div class="mt-2 flex">
    <NInput
      v-model:value="keyword"
      class="flex-grow"
      placeholder="搜索文件"
    ></NInput>
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
import { useClipboard, useUrlSearchParams, watchDebounced } from "@vueuse/core";
import CarbonArrowsHorizontal from "~icons/carbon/arrows-horizontal";
import CarbonCopy from "~icons/carbon/copy";
import CarbonSettings from "~icons/carbon/settings";
import { useMessage } from "naive-ui";
import { h, onBeforeMount, ref, watch } from "vue";
import { client } from "~/common/client";
import { useVersionSelect } from "~/common/useVersionSelect";
import { isNumber } from "~/common/utils";
import type { components } from "~/common/schema";
import FileDetailDiff from "./components/FileDetailDiff.vue";
import type { TreeOption, TreeOverrideNodeClickBehavior } from "naive-ui";
import type { TreeRenderProps } from "naive-ui/es/tree/src/interface";
import type { VNodeChild } from "vue";

const params = useUrlSearchParams("history");
const left = ref<number>();
const right = ref<number>();
const keyword = ref<string>("");
const { versions, versionOpts, load: loadVersions } = useVersionSelect();
const { copy } = useClipboard();
const message = useMessage();
watch(
  () => [left.value, right.value],
  () => {
    const l = versions.value.find((v) => v.id === left.value);
    const r = versions.value.find((v) => v.id === right.value);
    if (!l && !r) {
      delete params.diff;
      return;
    }
    params.diff = `${l?.resVersion || ""}...${r?.resVersion || ""}`;
  },
);

onBeforeMount(async () => {
  await loadVersions();
  if (params.diff) {
    const arr = (params.diff as string).split("...");
    if (arr.length === 2) {
      if (arr[0]) {
        const v = versions.value.find((v) => v.resVersion === arr[0]);
        if (v) {
          left.value = v.id;
        }
      }
      if (arr[1]) {
        const v = versions.value.find((v) => v.resVersion === arr[1]);
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
async function copyCommand() {
  const leftVersion = versions.value.find((v) => v.id === left.value);
  const rightVersion = versions.value.find((v) => v.id === right.value);

  if (!leftVersion && !rightVersion) {
    message.warning("请先选择版本");
    return;
  }

  let command = "";
  if (rightVersion && !leftVersion) {
    // Only right side selected
    command = `${rightVersion.clientVersion} ${rightVersion.resVersion}`;
  } else if (rightVersion && leftVersion) {
    // Both sides selected
    command = `${rightVersion.clientVersion} ${rightVersion.resVersion} -c ${leftVersion.clientVersion} -r ${leftVersion.resVersion}`;
  } else if (leftVersion && !rightVersion) {
    // Only left side selected
    command = `${leftVersion.clientVersion} ${leftVersion.resVersion}`;
  }

  await copy(command);
  message.success("已复制到剪贴板");
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
  pathMap: Record<string, components["schemas"]["BundleDetailsDto"]>;
  list: components["schemas"]["BundleDetailsDto"][];
}
const lData = ref<VersionFiles>({ pathMap: {}, list: [] });
const rData = ref<VersionFiles>({ pathMap: {}, list: [] });
async function loadDetail(id: number) {
  const resp = await client.GET("/api/v1/version/{id}/files", {
    params: { path: { id } },
  });
  const pathMap: Record<string, components["schemas"]["BundleDetailsDto"]> = {};
  const list = (resp.data ?? []).sort((a, b) => {
    return dirOrder(a.path, b.path);
  });
  for (const item of list) {
    pathMap[item.path] = item;
  }
  return { pathMap, list };
}
function buildTree() {
  const hasLeft = typeof left.value === "number";
  const hasRight = typeof right.value === "number";
  const top: TreeOption = { children: [] };
  function updateTree(list: components["schemas"]["BundleDetailsDto"][]) {
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
  let lList = lData.value.list.filter((v) => v.path.includes(keyword.value));
  let rList = rData.value.list.filter((v) => v.path.includes(keyword.value));
  if (hasLeft && hasRight && changeOnly.value) {
    lList = lList.filter((v) => {
      const right = rData!.value.pathMap[v.path];
      if (!right) {
        return true;
      }
      return v.fileHash !== right.fileHash;
    });
    rList = rList.filter((v) => {
      const left = lData!.value.pathMap[v.path];
      if (!left) {
        return true;
      }
      return v.fileHash !== left.fileHash;
    });
    updateTree(lList);
    updateTree(rList);
  } else {
    if (hasLeft) {
      updateTree(lList);
    }
    if (hasRight) {
      updateTree(rList);
    }
  }
  treeData.value = top.children!;
}
watchDebounced(
  () => [lData.value, rData.value, keyword.value],
  () => {
    buildTree();
  },
  { debounce: 200, maxWait: 500 },
);
watch(
  () => [left.value, right.value],
  async () => {
    lData.value = isNumber(left.value)
      ? await loadDetail(left.value!)
      : { pathMap: {}, list: [] };
    rData.value = isNumber(right.value)
      ? await loadDetail(right.value!)
      : { pathMap: {}, list: [] };
  },
);

const override: TreeOverrideNodeClickBehavior = ({ option }) => {
  if (option.children) {
    return "toggleExpand";
  }
  return "default";
};
function renderLabel(props: TreeRenderProps): VNodeChild {
  const l = lData.value.pathMap[props.option.key!];
  const r = rData.value.pathMap[props.option.key!];
  if (typeof left.value !== "number" || typeof right.value !== "number") {
    return h(
      "div",
      props.option.isLeaf
        ? {
            onClick: () => {
              if (typeof left.value === "number") {
                onLabelClick(l);
              } else {
                onLabelClick(r);
              }
            },
          }
        : undefined,
      props.option.label,
    );
  }

  if (l && r) {
    if (l.fileHash === r.fileHash) {
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
const leftDetail = ref<components["schemas"]["BundleDetailsDto"]>();
const rightDetail = ref<components["schemas"]["BundleDetailsDto"]>();
function onLabelClick(
  left?: components["schemas"]["BundleDetailsDto"],
  right?: components["schemas"]["BundleDetailsDto"],
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
