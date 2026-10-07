import { create } from "zustand";
import { persist } from "zustand/middleware";

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
  setActiveTab: (tab: TabId) => void;
  setSelectedSerial: (serial: string | null) => void;
}

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      activeTab: "overview",
      selectedSerial: null,
      setActiveTab: (activeTab) => set({ activeTab }),
      setSelectedSerial: (selectedSerial) => set({ selectedSerial }),
    }),
    {
      name: "zesp-connect-ui",
      partialize: (state) => ({
        activeTab: state.activeTab,
        selectedSerial: state.selectedSerial,
      }),
    },
  ),
);
