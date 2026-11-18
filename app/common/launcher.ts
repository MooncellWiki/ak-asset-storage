import { client } from "./client";
import type { components, operations } from "./schema";

export type LauncherRelease = components["schemas"]["LauncherRelease"];

export async function fetchLauncherReleases(
  query: operations["list_releases"]["parameters"]["query"],
) {
  const { data, response } = await client.GET("/api/v1/launcher/releases", {
    params: { query },
  });
  if (!data) throw new Error(`加载发布列表失败（${response.status}）`);
  return data;
}

export function archivedFilePercent(release: LauncherRelease) {
  const { archivedFiles, totalFiles, readyAt } = release;
  if (!totalFiles) return readyAt ? 100 : 0;
  return Math.floor((archivedFiles / totalFiles) * 10_000) / 100;
}
