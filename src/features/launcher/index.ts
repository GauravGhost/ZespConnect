export { LauncherPopup } from "./components/LauncherPopup";
export { LauncherSettings } from "./components/LauncherSettings";

/** Matches `runtime::launcher::WINDOW` in the backend. */
const LAUNCHER_WINDOW_LABEL = "launcher";

export function isLauncherWindowLabel(label: string): boolean {
  return label === LAUNCHER_WINDOW_LABEL;
}
