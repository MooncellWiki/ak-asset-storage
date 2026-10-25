import type { IconName } from "../AsIcon.vue";
import type { ChangeKind } from "~/common/bundles";

/** One node of a FileTree. Paths are unique; directories have children. */
export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  /** Defaults to a folder / file icon */
  icon?: IconName;
  /** Change mark shown instead of the icon (diff leaves) */
  mark?: ChangeKind;
  /** +~− counts at the right end (diff directories) */
  counts?: Record<ChangeKind, number>;
  /** Never merged with its only child (e.g. the diff's "全部" root) */
  keep?: boolean;
}

export type Row =
  | {
      kind: "node";
      node: TreeNode;
      /** Name, or `a/b/c` for a flattened chain of single-child directories */
      label: string;
      depth: number;
      open: boolean;
      /** Index of the parent row, -1 at the top level */
      parent: number;
      /** Index of the last row inside this row's subtree */
      end: number;
      posinset: number;
      setsize: number;
    }
  | { kind: "note"; text: string; depth: number; parent: number; end: number };

export interface TreeSource {
  roots: TreeNode[];
  /** Children of a directory; undefined while not loaded */
  childrenOf: (path: string) => TreeNode[] | undefined;
  isOpen: (path: string) => boolean;
  /** Note shown inside an open directory that has no rows ("" for none) */
  emptyNote?: (path: string) => string;
  /** Merge chains of single-child directories into one row */
  flatten: boolean;
}

/** The visible rows of a tree, in display order. */
export function visibleRows(source: TreeSource): Row[] {
  const rows: Row[] = [];

  function walk(nodes: TreeNode[], depth: number, parent: number) {
    for (const [i, first] of nodes.entries()) {
      let node = first;
      let label = first.name;
      // Follow single-directory chains whose children are already known.
      while (source.flatten && node.isDir && !node.keep) {
        const kids = source.childrenOf(node.path);
        if (kids?.length !== 1 || !kids[0]!.isDir) break;
        node = kids[0]!;
        label = `${label}/${node.name}`;
      }
      const open = node.isDir && source.isOpen(node.path);
      const index = rows.length;
      const row: Row = {
        kind: "node",
        node,
        label,
        depth,
        open,
        parent,
        end: index,
        posinset: i + 1,
        setsize: nodes.length,
      };
      rows.push(row);
      if (open) {
        const kids = source.childrenOf(node.path);
        const text = kids
          ? (source.emptyNote?.(node.path) ?? "空目录")
          : "加载中…";
        if (kids?.length) walk(kids, depth + 1, index);
        else if (text)
          rows.push({
            kind: "note",
            text,
            depth: depth + 1,
            parent: index,
            end: index + 1,
          });
      }
      row.end = rows.length - 1;
    }
  }

  walk(source.roots, 0, -1);
  return rows;
}

/** Ancestor row indices of `index`, outermost first. */
export function ancestorsOf(rows: Row[], index: number): number[] {
  const chain: number[] = [];
  for (let p = rows[index]?.parent ?? -1; p >= 0; p = rows[p]!.parent)
    chain.unshift(p);
  return chain;
}

/**
 * Directory rows that stick to the top while their subtree scrolls past,
 * with how far the innermost one is pushed up as its subtree ends.
 */
export function stickyRows(
  rows: Row[],
  scrollTop: number,
  rowHeight: number,
  max: number,
): { stack: number[]; shift: number } {
  const stack: number[] = [];
  for (;;) {
    if (stack.length >= max) break;
    const top = Math.floor((scrollTop + stack.length * rowHeight) / rowHeight);
    if (top >= rows.length) break;
    const next = ancestorsOf(rows, top)[stack.length];
    if (next === undefined) break;
    stack.push(next);
  }
  let shift = 0;
  const last = stack.at(-1);
  if (last !== undefined) {
    // The row after the subtree pushes the innermost sticky row out.
    const bottom = (rows[last]!.end + 1) * rowHeight - scrollTop;
    shift = Math.min(0, bottom - stack.length * rowHeight);
  }
  return { stack, shift };
}

/** A static tree (directories implied) from a flat list of paths. */
export function treeFromPaths(
  entries: { path: string; isDir: boolean; icon?: IconName }[],
): {
  roots: TreeNode[];
  childrenOf: (path: string) => TreeNode[] | undefined;
  dirs: string[];
} {
  const children = new Map<string, Map<string, TreeNode>>([["", new Map()]]);
  const add = (parent: string, node: TreeNode) => {
    const siblings = children.get(parent)!;
    const existing = siblings.get(node.name);
    if (!existing) siblings.set(node.name, node);
    else if (node.isDir) existing.isDir = true;
    if (node.isDir && !children.has(node.path))
      children.set(node.path, new Map());
  };
  for (const e of entries) {
    const parts = e.path.split("/");
    for (let i = 0; i < parts.length; i++) {
      const path = parts.slice(0, i + 1).join("/");
      const last = i === parts.length - 1;
      add(parts.slice(0, i).join("/"), {
        name: parts[i]!,
        path,
        isDir: last ? e.isDir : true,
        icon: last ? e.icon : undefined,
      });
    }
  }
  const sorted = (m: Map<string, TreeNode>) =>
    [...m.values()].sort((a, b) =>
      a.isDir === b.isDir ? (a.name < b.name ? -1 : 1) : a.isDir ? -1 : 1,
    );
  const cache = new Map<string, TreeNode[]>();
  const childrenOf = (path: string) => {
    const m = children.get(path);
    if (!m) return undefined;
    let list = cache.get(path);
    if (!list) cache.set(path, (list = sorted(m)));
    return list;
  };
  return {
    roots: childrenOf("")!,
    childrenOf,
    dirs: [...children.keys()].filter(Boolean),
  };
}

export interface NamePart {
  text: string;
  mark: boolean;
}

/**
 * A row label split for middle truncation: the head shrinks with an
 * ellipsis while the tail (end of the name, keeping the extension) stays,
 * so `illust_char_002_amiya_epoque#4b_material` reads `illust_char_00…aterial`.
 * The first case-insensitive match of `query` is marked in both halves.
 */
export function nameParts(
  label: string,
  isDir: boolean,
  query?: string,
): { head: NamePart[]; tail: NamePart[] } {
  const dot = label.lastIndexOf(".");
  const ext = !isDir && dot > 0 ? label.length - dot : 0;
  const tailLength = label.length > 16 ? Math.min(12, ext + 5) : 0;
  const cut = label.length - tailLength;
  const q = query?.trim().toLowerCase();
  const at = q ? label.toLowerCase().indexOf(q) : -1;
  const range: [number, number] = at < 0 ? [0, 0] : [at, at + q!.length];

  const piece = (from: number, to: number): NamePart[] => {
    const [s, e] = [Math.max(from, range[0]), Math.min(to, range[1])];
    if (s >= e)
      return from < to ? [{ text: label.slice(from, to), mark: false }] : [];
    return [
      { text: label.slice(from, s), mark: false },
      { text: label.slice(s, e), mark: true },
      { text: label.slice(e, to), mark: false },
    ].filter((p) => p.text);
  };
  return { head: piece(0, cut), tail: piece(cut, label.length) };
}
