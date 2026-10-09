/** Mirror windows are labelled `mirror-<session id>` by the backend (`open_mirror_window`). */
export const MIRROR_WINDOW_PREFIX = "mirror-";

export function isMirrorWindowLabel(label: string): boolean {
  return label.startsWith(MIRROR_WINDOW_PREFIX);
}

/** The session id encoded in a mirror window label, or null for any other window. */
export function mirrorSessionId(label: string): number | null {
  if (!isMirrorWindowLabel(label)) return null;
  const id = Number.parseInt(label.slice(MIRROR_WINDOW_PREFIX.length), 10);
  return Number.isFinite(id) ? id : null;
}
