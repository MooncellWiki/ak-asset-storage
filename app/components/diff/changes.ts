import { refDebounced } from "@vueuse/core";
import { computed, ref } from "vue";
import { parentPath, splitPath } from "~/common/utils";
import type { Ref } from "vue";
import type { ChangeKind } from "~/common/bundles";
import type { TreeNode } from "~/components/tree/model";

interface PathChange {
  path: string;
  kind: ChangeKind;
}

const DEFAULT_KINDS: Record<ChangeKind, boolean> = {
  add: true,
  chg: true,
  rem: true,
  same: false,
};

function emptyCounts(): Record<ChangeKind, number> {
  return { add: 0, chg: 0, rem: 0, same: 0 };
}

export function useChangeFilters<T extends PathChange>(changes: Ref<T[]>) {
  const kinds = ref({ ...DEFAULT_KINDS });
  const keyword = ref("");
  const q = refDebounced(keyword, 150);
  const matched = computed(() => {
    const needle = q.value.trim().toLowerCase();
    return needle
      ? changes.value.filter((c) => c.path.toLowerCase().includes(needle))
      : changes.value;
  });
  const counts = computed(() => {
    const counts = emptyCounts();
    for (const c of matched.value) counts[c.kind]++;
    return counts;
  });
  const visible = computed(() =>
    matched.value.filter((c) => kinds.value[c.kind]),
  );

  function resetFilters() {
    keyword.value = "";
    kinds.value = { ...DEFAULT_KINDS };
  }

  return { kinds, keyword, q, matched, counts, visible, resetFilters };
}

// A path can be a file in one release and a directory in another.
export const changeFileId = (path: string) => `\0${path}`;

interface ChangeDirectory {
  node: TreeNode & { counts: Record<ChangeKind, number> };
  dirs: TreeNode[];
  files: TreeNode[];
}

/** Builds FileTree nodes and aggregates the visible changes in each directory. */
export function buildChangeTree(changes: PathChange[]) {
  const directories = new Map<string, ChangeDirectory>();

  function directory(path: string): ChangeDirectory {
    const existing = directories.get(path);
    if (existing) return existing;
    const node = {
      name: path === "" ? "全部" : splitPath(path).name,
      path,
      isDir: true,
      counts: emptyCounts(),
      keep: path === "",
    };
    const dir = { node, dirs: [], files: [] };
    directories.set(path, dir);
    if (path !== "") directory(parentPath(path)).dirs.push(node);
    return dir;
  }

  const root = directory("");
  for (const { path, kind } of changes) {
    const parent = parentPath(path);
    directory(parent).files.push({
      id: changeFileId(path),
      name: splitPath(path).name,
      path,
      isDir: false,
      mark: kind,
    });
    for (let p = parent; p; p = parentPath(p)) directory(p).node.counts[kind]++;
    root.node.counts[kind]++;
  }

  const children = new Map<string, TreeNode[]>();
  for (const [path, { dirs, files }] of directories) {
    dirs.sort((a, b) => (a.name < b.name ? -1 : 1));
    children.set(path, [...dirs, ...files]);
  }
  return {
    roots: [root.node],
    childrenOf: (path: string) => children.get(path),
  };
}
