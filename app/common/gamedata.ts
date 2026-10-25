import { shallowRef } from "vue";
import { client } from "./client";

/** Extracted gamedata version directories, newest first (without `latest`). */
const dirs = shallowRef<string[]>([]);
let request: Promise<void> | undefined;

async function fetchDirs() {
  try {
    const { data } = await client.GET("/api/v1/files/{path}", {
      params: { path: { path: "gamedata" } },
    });
    dirs.value = (data?.children ?? [])
      .filter((c) => c.is_dir && c.name !== "latest")
      .map((c) => c.name)
      .sort()
      .reverse();
  } catch {
    request = undefined;
  }
}

export function useGamedataDirs() {
  request ??= fetchDirs();
  return dirs;
}
