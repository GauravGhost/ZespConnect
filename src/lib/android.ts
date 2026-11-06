/** The Android API level reported by a device, or `null` when unknown. */
export function apiLevel(sdk: string | null | undefined): number | null {
  const level = Number.parseInt(sdk ?? "", 10);
  return Number.isFinite(level) ? level : null;
}

/** Whether the device runs at least API `min`; unknown versions do not. */
export function atLeastApi(sdk: string | null | undefined, min: number): boolean {
  const level = apiLevel(sdk);
  return level !== null && level >= min;
}
