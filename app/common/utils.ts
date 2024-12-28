export function isNumber(v?: number): v is number {
  return typeof v === "number";
}
