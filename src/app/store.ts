import { create } from "zustand";
import { persist } from "zustand/middleware";

import { DEFAULT_THEME_ID } from "@/app/theme/presets";
import type { ThemeMode } from "@/app/theme/types";

export type TabId =
  | "overview"
  | "features"
  | "apps"
  | "mirror"
  | "wireless"
  | "files"
  | "console"
  | "performance"
  | "processes"
  | "emulators"
  | "settings";

export type AppsView = "list" | "grid";

interface AppStore {
  activeTab: TabId;
  selectedSerial: string | null;
  themeId: string;
  themeMode: ThemeMode;
  /** Latest scrcpy version whose update banner the user dismissed. */
  dismissedScrcpyUpdate: string | null;
  appsView: AppsView;
  setActiveTab: (tab: TabId) => void;
  setSelectedSerial: (serial: string | null) => void;
  setThemeId: (id: string) => void;
  setThemeMode: (mode: ThemeMode) => void;
  dismissScrcpyUpdate: (version: string) => void;
  setAppsView: (view: AppsView) => void;
}

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      activeTab: "overview",
      selectedSerial: null,
      themeId: DEFAULT_THEME_ID,
      themeMode: "system",
      dismissedScrcpyUpdate: null,
      appsView: "list",
      setActiveTab: (activeTab) => set({ activeTab }),
      setSelectedSerial: (selectedSerial) => set({ selectedSerial }),
      setThemeId: (themeId) => set({ themeId }),
      setThemeMode: (themeMode) => set({ themeMode }),
      dismissScrcpyUpdate: (dismissedScrcpyUpdate) => set({ dismissedScrcpyUpdate }),
      setAppsView: (appsView) => set({ appsView }),
    }),
    {
      name: "zesp-connect-ui",
      version: 1,
      // v0 had a separate "controls" tab; its contents now live on Overview.
      migrate: (persisted) => {
        const state = persisted as { activeTab?: string };
        if (state.activeTab === "controls") state.activeTab = "overview";
        return state as AppStore;
      },
      partialize: (state) => ({
        activeTab: state.activeTab,
        selectedSerial: state.selectedSerial,
        themeId: state.themeId,
        themeMode: state.themeMode,
        dismissedScrcpyUpdate: state.dismissedScrcpyUpdate,
        appsView: state.appsView,
      }),
    },
  ),
);
