import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import { AppProviders } from "@/app/providers";
import { MirrorViewer } from "@/features/mirror-viewer";
import "@/styles/globals.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found");
}

// Mirror windows load the same bundle but render the viewer instead of the shell.
// They skip StrictMode so the stream is started exactly once.
const isMirrorWindow = getCurrentWebviewWindow().label.startsWith("mirror-");

ReactDOM.createRoot(rootElement).render(
  <AppProviders>
    {isMirrorWindow ? (
      <MirrorViewer />
    ) : (
      <React.StrictMode>
        <App />
      </React.StrictMode>
    )}
  </AppProviders>,
);
