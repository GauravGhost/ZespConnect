/** Whether this webview can decode H.264; WebKitGTK on Linux may not, so mirroring falls back to scrcpy. */
export function supportsInAppMirror(): boolean {
  return typeof window !== "undefined" && "VideoDecoder" in window;
}
