import type { SoundRoute } from "@/types";

/** Keeping the sound on the phone while it also plays here needs Android 13 (API 33). */
const SOUND_ON_BOTH_MIN_SDK = 33;

export function supportsSoundOnBoth(sdk: string | null): boolean {
  const level = Number.parseInt(sdk ?? "", 10);
  return Number.isFinite(level) && level >= SOUND_ON_BOTH_MIN_SDK;
}

/** The route choices a phone can use; "both" stays while it is live, e.g. started by Game Mode. */
export function soundRouteChoices<T extends { value: SoundRoute }>(
  choices: readonly T[],
  sdk: string | null,
  current: SoundRoute,
): readonly T[] {
  if (supportsSoundOnBoth(sdk) || current === "both") return choices;
  return choices.filter((choice) => choice.value !== "both");
}
