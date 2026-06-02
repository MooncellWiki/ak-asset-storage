export function isNumber(v?: number): v is number {
  return typeof v === "number";
}

export function toReadableSize(size: number): string {
  if (size > 1024 * 1024) {
    return `${(size / 1024 / 1024).toFixed(2)}MB`;
  }
  if (size > 1024) {
    return `${(size / 1024).toFixed(2)}KB`;
  }
  return `${size}B`;
}

export function getParentPaths(path: string): string[] {
  const parts = path.split("/");
  const paths: string[] = [];
  let current = "";
  for (let i = 0; i < parts.length - 1; i++) {
    current = current ? `${current}/${parts[i]}` : parts[i]!;
    paths.push(current);
  }
  return paths;
}
