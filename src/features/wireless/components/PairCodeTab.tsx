import { KeyRound, Loader2, Plug } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { MdnsDevice } from "@/types";

import type { useConnectWireless, usePairCode } from "../hooks";

const PAIRING_SERVICE = "_adb-tls-pairing._tcp";
const CONNECT_SERVICE = "_adb-tls-connect._tcp";

/** The form values and mutations live in the panel so they survive switching tabs. */
export function PairCodeTab({
  addr,
  onAddrChange,
  code,
  onCodeChange,
  services,
  isConnected,
  pairCode,
  connect,
}: {
  addr: string;
  onAddrChange: (value: string) => void;
  code: string;
  onCodeChange: (value: string) => void;
  services: MdnsDevice[];
  isConnected: (address: string) => boolean;
  pairCode: ReturnType<typeof usePairCode>;
  connect: ReturnType<typeof useConnectWireless>;
}) {
  const discovered = services.filter(
    (service) =>
      service.service_type === CONNECT_SERVICE || service.service_type === PAIRING_SERVICE,
  );
  const pending = pairCode.isPending || connect.isPending;
  const codeValid = code.trim() === "" || /^\d{6,16}$/.test(code.trim());
  const canSubmit = addr.trim().length > 0 && codeValid && !pending;

  function submit(event: React.FormEvent) {
    event.preventDefault();
    const target = addr.trim();
    const secret = code.trim();
    if (!target || !codeValid) return;
    // Address + code pairs first; a bare address just connects.
    if (secret) pairCode.mutate({ addr: target, code: secret });
    else connect.mutate(target);
  }

  return (
    <>
      <form className="space-y-3" onSubmit={submit}>
        <div className="space-y-1.5">
          <Label htmlFor="wireless-addr" className="text-muted-foreground text-xs">
            Address
          </Label>
          <Input
            id="wireless-addr"
            value={addr}
            onChange={(event) => onAddrChange(event.target.value)}
            placeholder="192.168.1.42:5555"
            className="h-9 font-mono"
            autoComplete="off"
            spellCheck={false}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="wireless-code" className="text-muted-foreground text-xs">
            Pairing code <span className="font-normal">(optional)</span>
          </Label>
          <Input
            id="wireless-code"
            value={code}
            onChange={(event) => onCodeChange(event.target.value.replace(/\D/g, "").slice(0, 16))}
            placeholder="6-digit code"
            inputMode="numeric"
            className="h-9 font-mono"
            autoComplete="off"
          />
        </div>
        <Button type="submit" size="sm" className="w-full" disabled={!canSubmit}>
          {pending ? (
            <Loader2 className="size-4 animate-spin" />
          ) : code.trim() ? (
            <KeyRound className="size-4" />
          ) : (
            <Plug className="size-4" />
          )}
          {code.trim() ? "Pair & connect" : "Connect"}
        </Button>
      </form>
      <p className="text-muted-foreground text-xs">
        Enter the phone's address to connect, or add its code to pair first.
      </p>

      {discovered.length > 0 ? (
        <div className="pt-1">
          <p className="text-muted-foreground mb-1 text-xs font-medium tracking-wide uppercase">
            Discovered
          </p>
          <ul className="divide-y divide-border/60">
            {discovered.map((service) => (
              <ServiceRow
                key={`${service.name}-${service.address}`}
                service={service}
                connected={isConnected(service.address)}
                onUse={onAddrChange}
                onConnect={(value) => connect.mutate(value)}
                connecting={connect.isPending && connect.variables === service.address}
              />
            ))}
          </ul>
        </div>
      ) : null}
    </>
  );
}

function ServiceRow({
  service,
  connected,
  connecting,
  onUse,
  onConnect,
}: {
  service: MdnsDevice;
  connected: boolean;
  connecting: boolean;
  onUse: (address: string) => void;
  onConnect: (address: string) => void;
}) {
  const pairing = service.service_type === PAIRING_SERVICE;
  return (
    <li className="flex items-center gap-3 py-2">
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium">{service.name}</p>
        <p className="text-muted-foreground truncate font-mono text-[11px]">{service.address}</p>
      </div>
      {pairing ? (
        <Button variant="ghost" size="sm" onClick={() => onUse(service.address)}>
          Use address
        </Button>
      ) : (
        <Button
          variant="outline"
          size="sm"
          disabled={connected || connecting}
          onClick={() => onConnect(service.address)}
        >
          {connecting ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <Plug className="size-3.5" />
          )}
          {connected ? "Connected" : "Connect"}
        </Button>
      )}
    </li>
  );
}
