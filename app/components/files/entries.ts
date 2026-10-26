import { fileKind } from "~/common/utils";
import type { IconName } from "../AsIcon.vue";
import type { components } from "~/common/schema";

export type AssetEntry = components["schemas"]["AssetEntry"];

export function entryIcon(e: { name: string; is_dir: boolean }): IconName {
  if (e.is_dir) return "folder";
  switch (fileKind(e.name)) {
    case "image":
      return "image";
    case "audio":
    case "video":
      return "audio";
    case "json":
      return "json";
    default:
      return "file";
  }
}

export type SortKey = "name" | "mtime" | "size";

/** Directories first, then by the chosen key (newest / largest first). */
export function sortEntries(list: AssetEntry[], key: SortKey): AssetEntry[] {
  return [...list].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    if (key === "size" && !a.is_dir) return b.size - a.size;
    if (key === "mtime") return b.modified_at.localeCompare(a.modified_at);
    return a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
  });
}
