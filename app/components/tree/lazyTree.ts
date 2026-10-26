import { ref, shallowReactive } from "vue";
import type { TreeNode } from "./model";

/**
 * State of a FileTree whose directories load their children on first open
 * (manifest, files). Opening a directory that holds a single directory keeps
 * opening down the chain, so FileTree can show it as one `a/b/c` row.
 */
export function useLazyTree(fetch: (path: string) => Promise<TreeNode[]>) {
  const cache = shallowReactive(new Map<string, TreeNode[]>());
  const errors = shallowReactive(new Set<string>());
  const pending = new Map<string, Promise<TreeNode[]>>();
  const expanded = ref(new Set<string>());
  let generation = 0;

  function load(path: string): Promise<TreeNode[]> {
    const hit = cache.get(path);
    // A failed directory shows as empty until it is opened again, which retries.
    if (hit && !errors.has(path)) return Promise.resolve(hit);
    const inFlight = pending.get(path);
    if (inFlight) return inFlight;
    const gen = generation;
    errors.delete(path);
    cache.delete(path);
    const request = fetch(path)
      .catch(() => {
        if (gen === generation) errors.add(path);
        return [];
      })
      .then((list) => {
        if (gen === generation) cache.set(path, list);
        pending.delete(path);
        return list;
      });
    pending.set(path, request);
    return request;
  }

  function setOpen(paths: string[], open: boolean) {
    const next = new Set(expanded.value);
    for (const p of paths) {
      if (open) next.add(p);
      else next.delete(p);
    }
    expanded.value = next;
  }

  async function toggle(path: string, open = !expanded.value.has(path)) {
    setOpen([path], open);
    if (!open) return;
    const gen = generation;
    let children = await load(path);
    // Open single-directory chains all the way down (until a reset).
    for (
      let only = children[0];
      children.length === 1 && only?.isDir;
      only = children[0]
    ) {
      if (gen !== generation) return;
      setOpen([only.path], true);
      children = await load(only.path);
    }
  }

  /** Loads and opens every ancestor of `path` below `root`. */
  async function reveal(path: string, root = "") {
    const gen = generation;
    await load(root);
    const rest =
      root && path.startsWith(`${root}/`) ? path.slice(root.length + 1) : path;
    const ancestors: string[] = [];
    let current = root;
    for (const part of rest.split("/").slice(0, -1)) {
      current = current ? `${current}/${part}` : part;
      ancestors.push(current);
    }
    for (const p of ancestors) {
      if (gen !== generation) return;
      await load(p);
    }
    if (gen === generation) setOpen(ancestors, true);
  }

  /** Drops everything loaded (e.g. when switching versions). */
  function reset() {
    generation++;
    cache.clear();
    errors.clear();
    pending.clear();
    expanded.value = new Set();
  }

  return {
    childrenOf: (path: string) => cache.get(path),
    failed: (path: string) => errors.has(path),
    expanded,
    load,
    toggle,
    reveal,
    reset,
  };
}
