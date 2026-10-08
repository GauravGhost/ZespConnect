import { useEffect, useState } from "react";

/** Wall-clock time, re-rendered every 15s — close enough for a status bar. */
export function useClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 15_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

const H24 = !new Intl.DateTimeFormat([], { hour: "numeric" }).resolvedOptions().hour12;

/** "9:42" or "21:42" per the locale, without an AM/PM suffix — like Android's clock. */
export function formatClock(date: Date) {
  return date
    .toLocaleTimeString([], { hour: H24 ? "2-digit" : "numeric", minute: "2-digit" })
    .replace(/\s?[AP]M$/i, "");
}
