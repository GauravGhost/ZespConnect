import { open } from "@tauri-apps/plugin-dialog";

/** Ask the user for an APK to install, or null if they cancelled. */
export function chooseApk(): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    filters: [{ name: "Android package", extensions: ["apk", "apks", "xapk"] }],
    title: "Select an APK to install",
  }).then((selected) => (typeof selected === "string" ? selected : null));
}
