import { useEffect } from "react";

import { tabForShortcut } from "@/app/navigation";
import { useAppStore } from "@/app/store";
import { IS_MAC } from "@/lib/platform";

/** Switches tabs on Cmd/Ctrl + the key shown next to each sidebar item. */
export function useTabShortcuts() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const tab = tabForShortcut(event, IS_MAC);
      if (!tab) return;
      event.preventDefault();
      setActiveTab(tab);
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setActiveTab]);
}
