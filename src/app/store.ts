import { create } from "zustand";
import { persist } from "zustand/middleware";

import { DEFAULT_THEME_ID } from "@/app/theme/presets";
import type { ThemeMode } from "@/app/theme/types";

export type TabId =
  | "overview"
  | "apps"
  | "mirror"
  | "controls"
  | "wireless"
  | "files"
  | "console"
  | "emulators"
  | "settings";

interface AppStore {
  activeTab: TabId;
  selectedSerial: string | null;
  themeId: string;
  themeMode: ThemeMode;
  setActiveTab: (tab: TabId) => void;
  setSelectedSerial: (serial: string | null) => void;
  setThemeId: (id: string) => void;
  setThemeMode: (mode: ThemeMode) => void;
}

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      activeTab: "overview",
      selectedSerial: null,
      themeId: DEFAULT_THEME_ID,
      themeMode: "system",
      setActiveTab: (activeTab) => set({ activeTab }),
      setSelectedSerial: (selectedSerial) => set({ selectedSerial }),
      setThemeId: (themeId) => set({ themeId }),
      setThemeMode: (themeMode) => set({ themeMode }),
    }),
    {
      name: "zesp-connect-ui",
      partialize: (state) => ({
        activeTab: state.activeTab,
        selectedSerial: state.selectedSerial,
        themeId: state.themeId,
        themeMode: state.themeMode,
      }),
    },
  ),
);
