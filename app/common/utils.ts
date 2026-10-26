import { format, isThisYear, isValid, parse, parseISO } from "date-fns";

const MINUS = "−";

/** 1,369 */
export function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

/** 20 B · 2.47 KiB · 419.7 KiB · 11.79 MiB */
export function formatSize(bytes: number): string {
  const abs = Math.abs(bytes);
  if (abs < 1024) return `${bytes} B`;
  if (abs < 1024 * 1024) {
    const kib = bytes / 1024;
    return `${kib.toFixed(Math.abs(kib) < 10 ? 2 : 1)} KiB`;
  }
  if (abs < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(2)} MiB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GiB`;
}

/** +2.61 MiB · −5.98 KiB · ±0 B */
export function formatDelta(bytes: number): string {
  if (bytes === 0) return "±0 B";
  const sign = bytes > 0 ? "+" : MINUS;
  return sign + formatSize(Math.abs(bytes));
}

/** 2026-10-22 14:31 in local time */
export function formatDateTime(iso: string): string {
  const d = parseISO(iso);
  return isValid(d) ? format(d, "yyyy-MM-dd HH:mm") : iso;
}

/**
 * Release time encoded in a res version (`26-10-22-14-02-11-…`), formatted
 * as `10-22 14:02` in the current year and `2025-03-14 09:20` otherwise.
 */
export function formatResTime(res: string): string {
  const d = parse(res.slice(0, 14), "yy-MM-dd-HH-mm", new Date());
  if (!isValid(d)) return "";
  return format(d, isThisYear(d) ? "MM-dd HH:mm" : "yyyy-MM-dd HH:mm");
}

/** `a/b/c.ab` → `{ dir: "a/b/", name: "c.ab" }` */
export function splitPath(path: string): { dir: string; name: string } {
  const i = path.lastIndexOf("/");
  return { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
}

export function parentPath(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

/** Bundle objects are stored by content hash. */
export function storageUrl(hash: string): string {
  return `/storage/${hash.slice(0, 2)}/${hash.slice(2, 4)}/${hash.slice(4)}`;
}

/**
 * Public URL of an extracted file: `raw/…` is served under `/assets`,
 * `gamedata/…` under `/gamedata`.
 */
export function assetUrl(path: string): string {
  const parts = path.split("/").map(encodeURIComponent);
  if (parts[0] === "raw") parts[0] = "assets";
  return `/${parts.join("/")}`;
}

export function absoluteUrl(path: string): string {
  return new URL(path, location.origin).href;
}

/** Starts a download through a temporary link. */
export function download(href: string, filename?: string) {
  const a = document.createElement("a");
  a.href = href;
  if (filename) a.download = filename;
  a.rel = "noopener";
  document.body.append(a);
  a.click();
  a.remove();
}

/** Saves text as a file. */
export function downloadText(
  text: string,
  filename: string,
  type = "application/json",
) {
  const url = URL.createObjectURL(new Blob([text], { type }));
  download(url, filename);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function bundleFilename(bundle: {
  versionClient: string;
  versionRes: string;
  path: string;
}): string {
  return `${bundle.versionClient}_${bundle.versionRes}_${bundle.path}.zip`;
}

export type FileKind = "image" | "audio" | "video" | "json" | "text" | "other";

const KINDS: Record<string, FileKind> = {
  png: "image",
  jpg: "image",
  jpeg: "image",
  webp: "image",
  gif: "image",
  svg: "image",
  mp3: "audio",
  wav: "audio",
  ogg: "audio",
  mp4: "video",
  webm: "video",
  json: "json",
  txt: "text",
  log: "text",
  atlas: "text",
  md: "text",
  csv: "text",
  xml: "text",
  yaml: "text",
  yml: "text",
  toml: "text",
  lua: "text",
  js: "text",
  ts: "text",
  html: "text",
  css: "text",
};

export function fileKind(name: string): FileKind {
  const dot = name.lastIndexOf(".");
  return dot < 0
    ? "other"
    : (KINDS[name.slice(dot + 1).toLowerCase()] ?? "other");
}
