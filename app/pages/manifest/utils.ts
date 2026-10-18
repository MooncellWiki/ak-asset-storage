import type { TreeOption } from "naive-ui";
import type { components } from "~/common/schema";

type ManifestNode = components["schemas"]["ManifestNode"];

export function isManifestDirectory(nodeType: string): boolean {
  return nodeType === "directory" || nodeType === "both";
}

export function toManifestTreeOption(node: ManifestNode): TreeOption {
  return {
    key: node.path,
    label: node.name,
    isLeaf: !isManifestDirectory(node.nodeType),
    nodeType: node.nodeType,
  };
}
