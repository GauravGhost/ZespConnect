import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { GameOptions } from "@/types";

export const DEFAULT_GAME_FPS = 60;
export const DEFAULT_GAME_BITRATE_MBPS = 16;

/** Game Mode's video settings; they apply when a session starts. */
export interface GameVideo {
  resolution: number | null;
  fps: number | null;
  bitrate: number | null;
  stayAwake: boolean;
}

export const DEFAULT_GAME_OPTIONS: GameOptions = {
  orientation: "auto",
  doNotDisturb: true,
  screenOff: false,
  cleanCapture: false,
  alwaysOnTop: false,
  stats: false,
  soundOnComputer: true,
  soundOnPhone: false,
  lowLatencyAudio: true,
  audioDevice: null,
  controllers: true,
};

const DEFAULT_GAME_VIDEO: GameVideo = {
  resolution: null,
  fps: DEFAULT_GAME_FPS,
  bitrate: DEFAULT_GAME_BITRATE_MBPS,
  stayAwake: true,
};

interface GameModeStore {
  video: GameVideo;
  options: GameOptions;
  setVideo: (video: GameVideo) => void;
  setOptions: (options: GameOptions) => void;
}

export const useGameModeStore = create<GameModeStore>()(
  persist(
    (set) => ({
      video: DEFAULT_GAME_VIDEO,
      options: DEFAULT_GAME_OPTIONS,
      setVideo: (video) => set({ video }),
      setOptions: (options) => set({ options }),
    }),
    {
      name: "zesp-connect-game-mode",
      version: 1,
      // Settings added in later versions take their defaults.
      merge: (persisted, current) => {
        const saved = persisted as Partial<GameModeStore> | undefined;
        return {
          ...current,
          video: { ...current.video, ...saved?.video },
          options: { ...current.options, ...saved?.options },
        };
      },
      partialize: (state) => ({ video: state.video, options: state.options }),
    },
  ),
);
