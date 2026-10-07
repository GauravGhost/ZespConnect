/**
 * Whether this webview can decode H.264 itself. WebView2 (Windows) and WKWebView
 * (macOS) support WebCodecs; some WebKitGTK builds on Linux do not, so mirroring
 * falls back to the external scrcpy window there.
 */
export function supportsInAppMirror(): boolean {
  return typeof window !== "undefined" && "VideoDecoder" in window;
}
