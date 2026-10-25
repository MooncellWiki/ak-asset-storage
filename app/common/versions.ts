import { computed, shallowRef } from "vue";
import { client } from "./client";
import type { components } from "./schema";
import type { SelectGroupOption } from "@mooncellwiki/prts-design-vue";

export type Version = components["schemas"]["VersionSummary"];

/** All versions, newest first; loaded once and shared by every page. */
const versions = shallowRef<Version[]>([]);
const state = shallowRef<"loading" | "ready" | "error">("loading");
let request: Promise<void> | undefined;

async function fetchVersions() {
  state.value = "loading";
  try {
    const { data } = await client.GET("/api/v1/version");
    if (!data) throw new Error("version list unavailable");
    versions.value = [...data].sort((a, b) => b.id - a.id);
    state.value = "ready";
  } catch {
    state.value = "error";
    request = undefined;
  }
}

export function loadVersions(): Promise<void> {
  request ??= fetchVersions();
  return request;
}

const latest = computed(() => versions.value[0]);
const latestWithManifest = computed(() =>
  versions.value.find((v) => v.assetMappingStatus === "ready"),
);

function byRes(res: string | undefined): Version | undefined {
  return res ? versions.value.find((v) => v.resVersion === res) : undefined;
}

function byId(id: number | null | undefined): Version | undefined {
  return id == null ? undefined : versions.value.find((v) => v.id === id);
}

/** The version released right before `version`. */
function previous(version: Version | undefined): Version | undefined {
  if (!version) return undefined;
  const i = versions.value.indexOf(version);
  return i < 0 ? undefined : versions.value[i + 1];
}

export function useVersions() {
  void loadVersions();
  return {
    versions,
    state,
    latest,
    latestWithManifest,
    byRes,
    byId,
    previous,
    reload: () => {
      request = undefined;
      return loadVersions();
    },
  };
}

/** Select options grouped by client version (newest first). */
export function versionOptions(list: Version[]): SelectGroupOption[] {
  const groups: SelectGroupOption[] = [];
  for (const v of list) {
    const option = { label: `${v.clientVersion} ${v.resVersion}`, value: v.id };
    const last = groups.at(-1);
    if (last?.label === v.clientVersion) last.children.push(option);
    else
      groups.push({
        type: "group",
        label: v.clientVersion,
        children: [option],
      });
  }
  return groups;
}

export function hasManifest(v: Version | undefined): boolean {
  return v?.assetMappingStatus === "ready";
}

/** Status tag of a version: ready, still downloading, or importing the manifest. */
export function versionStatus(v: Version): {
  label: string;
  variant: "success" | "warning";
} {
  if (!v.isReady) return { label: "下载中", variant: "warning" };
  if (v.assetMappingStatus === "importing")
    return { label: "正在导入清单", variant: "warning" };
  return { label: "已就绪", variant: "success" };
}

/** Arguments for a torappu run that unpacks `v` against `prev`. */
export function torappuCommand(v: Version, prev: Version | undefined): string {
  return prev
    ? `${v.clientVersion} ${v.resVersion} -c ${prev.clientVersion} -r ${prev.resVersion}`
    : `${v.clientVersion} ${v.resVersion}`;
}

export function diffLink(
  left: Version | undefined,
  right: Version | undefined,
  extra: Record<string, string> = {},
) {
  return {
    path: "/diff",
    query: {
      diff: `${left?.resVersion ?? ""}...${right?.resVersion ?? ""}`,
      ...extra,
    },
  };
}
