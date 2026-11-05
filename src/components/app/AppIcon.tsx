import { convertFileSrc } from "@tauri-apps/api/core";
import { useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

interface AppIconProps {
  iconPath: string | null;
  className?: string;
  /** Shown when the app has no saved icon or it fails to load. */
  fallback: ReactNode;
}

/** A phone app's icon, read from the copy the backend saved on the computer. */
export function AppIcon({ iconPath, className, fallback }: Readonly<AppIconProps>) {
  const [failed, setFailed] = useState(false);
  if (!iconPath || failed) return fallback;

  return (
    <img
      src={convertFileSrc(iconPath)}
      alt=""
      draggable={false}
      loading="lazy"
      className={cn("shrink-0", className)}
      onError={() => setFailed(true)}
    />
  );
}
