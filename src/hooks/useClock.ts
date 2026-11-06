import { useNow } from "./useNow";

const CLOCK_TICK_MS = 15_000;

/** Wall-clock time, re-rendered every 15s - close enough for a status bar. */
export function useClock() {
  return new Date(useNow(CLOCK_TICK_MS));
}

const uses24Hour = !new Intl.DateTimeFormat([], { hour: "numeric" }).resolvedOptions().hour12;

/** "9:42" or "21:42" per the locale, without an AM/PM suffix - like Android's clock. */
export function formatClock(date: Date) {
  return date
    .toLocaleTimeString([], { hour: uses24Hour ? "2-digit" : "numeric", minute: "2-digit" })
    .replace(/\s?[AP]M$/i, "");
}
