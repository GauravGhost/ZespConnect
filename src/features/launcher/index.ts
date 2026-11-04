export { LauncherPopup } from "./components/LauncherPopup";

/** Matches `runtime::launcher::WINDOW` in the backend. */
const LAUNCHER_WINDOW_LABEL = "launcher";

export function isLauncherWindowLabel(label: string): boolean {
  return label === LAUNCHER_WINDOW_LABEL;
}
