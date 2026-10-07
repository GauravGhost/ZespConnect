import type * as React from "react";
import { Toaster as SonnerToaster } from "sonner";

import { useResolvedTheme } from "@/app/theme/useResolvedTheme";

export function Toaster() {
  const theme = useResolvedTheme();

  return (
    <SonnerToaster
      position="bottom-right"
      closeButton
      theme={theme}
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
        } as React.CSSProperties
      }
      toastOptions={{
        classNames: {
          toast: "group toast",
          description: "text-muted-foreground",
          actionButton: "bg-primary-container text-on-primary-container",
          cancelButton: "bg-muted text-muted-foreground",
        },
      }}
    />
  );
}
