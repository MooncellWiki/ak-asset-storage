import { client } from "./client";
import type { components } from "./schema";

export type Bundle = components["schemas"]["BundleDetails"];
export type ChangeKind = "add" | "chg" | "rem";

export interface Change {
  path: string;
  kind: ChangeKind;
  old?: Bundle;
  new?: Bundle;
}

export const KIND_LETTER: Record<ChangeKind, string> = {
  add: "A",
  chg: "M",
  rem: "D",
};
export const KIND_LABEL: Record<ChangeKind, string> = {
  add: "新增",
  chg: "修改",
  rem: "删除",
};

/** Bundle lists of recently viewed versions (each is ~15k rows / 3 MB). */
const cache = new Map<number, Promise<Bundle[]>>();
const CACHE_SIZE = 4;

export function loadVersionBundles(id: number): Promise<Bundle[]> {
  const hit = cache.get(id);
  if (hit) {
    cache.delete(id);
    cache.set(id, hit);
    return hit;
  }
  const request = client
    .GET("/api/v1/version/{id}/files", { params: { path: { id } } })
    .then(({ data }) => {
      if (!data) throw new Error(`bundle list of version ${id} unavailable`);
      return data;
    });
  request.catch(() => cache.delete(id));
  cache.set(id, request);
  if (cache.size > CACHE_SIZE) cache.delete(cache.keys().next().value!);
  return request;
}

/** Bundles added, modified (hash differs) or removed going from `left` to `right`. */
export function diffBundles(left: Bundle[], right: Bundle[]): Change[] {
  const before = new Map(left.map((b) => [b.path, b]));
  const changes: Change[] = [];
  for (const b of right) {
    const old = before.get(b.path);
    if (!old) changes.push({ path: b.path, kind: "add", new: b });
    else if (old.fileHash !== b.fileHash)
      changes.push({ path: b.path, kind: "chg", old, new: b });
    before.delete(b.path);
  }
  for (const old of before.values())
    changes.push({ path: old.path, kind: "rem", old });
  return changes.sort((a, b) =>
    a.path < b.path ? -1 : a.path > b.path ? 1 : 0,
  );
}

/**
 * Versions in which the bundle at `path` got new content, newest first.
 * The API matches path substrings, so other paths are dropped here.
 */
export async function loadBundleHistory(path: string): Promise<Bundle[]> {
  const rows: Bundle[] = [];
  let cursor: string | undefined;
  for (let page = 0; page < 5; page++) {
    const { data } = await client.GET("/api/v1/bundle", {
      params: { query: { path, limit: 200, cursor } },
    });
    if (!data) break;
    rows.push(...data.bundles.filter((b) => b.path === path));
    if (!data.nextCursor) break;
    cursor = data.nextCursor;
  }
  // Rows come newest version first; walk from the oldest and keep each
  // version whose hash differs from the one before it.
  const changed: Bundle[] = [];
  for (let i = rows.length - 1; i >= 0; i--) {
    const b = rows[i]!;
    if (changed.at(-1)?.fileHash !== b.fileHash) changed.push(b);
  }
  return changed.reverse();
}

export interface DirNode {
  name: string;
  /** "" for the root */
  path: string;
  dirs: Map<string, DirNode>;
  files: Change[];
  counts: Record<ChangeKind, number>;
}

function dirNode(name: string, path: string): DirNode {
  return {
    name,
    path,
    dirs: new Map(),
    files: [],
    counts: { add: 0, chg: 0, rem: 0 },
  };
}

/** Directory tree of the changes, each directory counting changes below it. */
export function buildChangeTree(changes: Change[]): DirNode {
  const root = dirNode("全部", "");
  for (const change of changes) {
    const parts = change.path.split("/");
    let node = root;
    node.counts[change.kind]++;
    for (let i = 0; i < parts.length - 1; i++) {
      const name = parts[i]!;
      let next = node.dirs.get(name);
      if (!next) {
        next = dirNode(name, parts.slice(0, i + 1).join("/"));
        node.dirs.set(name, next);
      }
      next.counts[change.kind]++;
      node = next;
    }
    node.files.push(change);
  }
  return root;
}
