import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import { PAIR_TIMEOUT_SECS } from "@/lib/wireless";
import type { QrPayload } from "@/types";

export type QrPairStatus =
  | { kind: "idle" }
  | { kind: "waiting" }
  | { kind: "success"; address: string }
  | { kind: "error"; message: string };

export interface QrPairing {
  qr: QrPayload | null;
  status: QrPairStatus;
  generating: boolean;
  secondsLeft: number;
  timer: string;
  generate: () => Promise<void>;
  cancel: () => void;
}

/**
 * Generate a pairing QR code and wait for the phone to scan it. Each attempt
 * gets a session id so results from a superseded attempt are ignored.
 */
export function useQrPairing(): QrPairing {
  const queryClient = useQueryClient();
  const [qr, setQr] = useState<QrPayload | null>(null);
  const [status, setStatus] = useState<QrPairStatus>({ kind: "idle" });
  const [generating, setGenerating] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const sessionRef = useRef(0);

  const generate = useCallback(async () => {
    const session = ++sessionRef.current;
    setQr(null);
    setStatus({ kind: "idle" });
    setGenerating(true);
    try {
      const payload = await ipc.wireless.qrPayload();
      if (session !== sessionRef.current) return;
      setQr(payload);
      setNow(Date.now());
      setStatus({ kind: "waiting" });
      setGenerating(false);

      const address = await ipc.wireless.qrPair(payload.payload, PAIR_TIMEOUT_SECS);
      queryClient.invalidateQueries({ queryKey: ["devices"] });
      if (session !== sessionRef.current) return;
      setStatus({ kind: "success", address });
      toast.success(`Connected to ${address}`);
    } catch (error) {
      if (session !== sessionRef.current) return;
      setGenerating(false);
      const message = errorMessage(error);
      // We cancel the request ourselves when the user closes the pairing UI.
      if (message === "Pairing cancelled") return;
      setStatus({ kind: "error", message });
    }
  }, [queryClient]);

  const cancel = useCallback(() => {
    sessionRef.current++;
    void ipc.wireless.cancelQr().catch(() => {});
    setQr(null);
    setStatus({ kind: "idle" });
    setGenerating(false);
  }, []);

  useEffect(() => {
    if (!qr) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [qr]);

  const secondsLeft = qr ? Math.max(0, qr.created + PAIR_TIMEOUT_SECS - Math.floor(now / 1000)) : 0;
  const timer = `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, "0")}`;

  return { qr, status, generating, secondsLeft, timer, generate, cancel };
}
