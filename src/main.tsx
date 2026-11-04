import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import { AppProviders } from "@/app/providers";
import { isLauncherWindowLabel, LauncherPopup } from "@/features/launcher";
import { isMirrorWindowLabel, MirrorViewer } from "@/features/mirror-viewer";
import "@/styles/globals.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found");
}

// Mirror and launcher windows load the same bundle but render their own view instead of the shell.
// They skip StrictMode so the stream is started, and the launcher loaded, exactly once.
const windowLabel = getCurrentWebviewWindow().label;

function windowView() {
  if (isMirrorWindowLabel(windowLabel)) return <MirrorViewer />;
  if (isLauncherWindowLabel(windowLabel)) {
    document.documentElement.classList.add("transparent-window");
    return <LauncherPopup />;
  }
  return (
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

ReactDOM.createRoot(rootElement).render(<AppProviders>{windowView()}</AppProviders>);
