import type { components } from "~/common/schema";
import type { TreeOption } from "naive-ui";

type ManifestNodeDto = components["schemas"]["ManifestNodeDto"];

export function isManifestDirectory(nodeType: string): boolean {
  return nodeType === "directory" || nodeType === "both";
}

export function toManifestTreeOption(node: ManifestNodeDto): TreeOption {
  return {
    key: node.path,
    label: node.name,
    isLeaf: !isManifestDirectory(node.nodeType),
    nodeType: node.nodeType,
  };
}
