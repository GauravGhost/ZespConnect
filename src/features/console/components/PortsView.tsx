import {
  ArrowRight,
  Laptop,
  Loader2,
  Plus,
  RefreshCw,
  Smartphone,
  Trash2,
  Unplug,
  X,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SegmentedChoice, type SegmentOption } from "@/components/ui/segmented-choice";
import type { PortDirection, PortRule } from "@/types";
import { useChangePortRules, usePortRules } from "../hooks";

const DIRECTIONS: readonly SegmentOption<PortDirection>[] = [
  { value: "reverse", label: "Phone to computer", icon: Smartphone },
  { value: "forward", label: "Computer to phone", icon: Laptop },
];

const HINTS: Record<PortDirection, string> = {
  reverse: "Apps on the phone reach a server on this computer at localhost, e.g. a dev server.",
  forward: "Tools on this computer reach a server on the phone at localhost.",
};

const MAX_PORT = 65535;

/** A port typed by the user, or null when it is not a whole number in 1-65535. */
function parsePort(text: string): number | null {
  if (!/^\d{1,5}$/.test(text.trim())) return null;
  const port = Number(text);
  return port >= 1 && port <= MAX_PORT ? port : null;
}

function PortField({
  id,
  label,
  value,
  placeholder,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id} className="text-muted-foreground text-xs">
        {label}
      </Label>
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        inputMode="numeric"
        autoComplete="off"
        className="h-9 w-32 font-mono"
      />
    </div>
  );
}

function AddRuleForm({ serial }: { serial: string }) {
  const change = useChangePortRules(serial);
  const [direction, setDirection] = useState<PortDirection>("reverse");
  const [hostText, setHostText] = useState("");
  const [deviceText, setDeviceText] = useState("");

  // A blank second port means "the same port", the usual dev server setup.
  const hostPort = parsePort(hostText || deviceText);
  const devicePort = parsePort(deviceText || hostText);
  const valid = hostPort !== null && devicePort !== null;

  function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!valid || change.isPending) return;
    change.mutate(
      { kind: "add", direction, hostPort, devicePort },
      {
        onSuccess: () => {
          setHostText("");
          setDeviceText("");
        },
      },
    );
  }

  const hostField = (
    <PortField
      id="port-host"
      label="Computer port"
      value={hostText}
      placeholder={deviceText || "3000"}
      onChange={setHostText}
    />
  );
  const deviceField = (
    <PortField
      id="port-device"
      label="Phone port"
      value={deviceText}
      placeholder={hostText || "3000"}
      onChange={setDeviceText}
    />
  );
  const [from, to] = direction === "forward" ? [hostField, deviceField] : [deviceField, hostField];

  return (
    <form onSubmit={submit} className="shrink-0 space-y-3 border-b p-4">
      <SegmentedChoice
        label="Direction"
        options={DIRECTIONS}
        value={direction}
        onChange={setDirection}
      />
      <div className="flex flex-wrap items-end gap-3">
        {from}
        <ArrowRight className="text-muted-foreground mb-2.5 size-4" />
        {to}
        <Button type="submit" size="sm" className="mb-0.5" disabled={!valid || change.isPending}>
          {change.isPending ? <Loader2 className="animate-spin" /> : <Plus />}
          Add
        </Button>
      </div>
      <p className="text-muted-foreground text-xs">{HINTS[direction]}</p>
    </form>
  );
}

function RuleRow({ rule, serial }: { rule: PortRule; serial: string }) {
  const change = useChangePortRules(serial);
  const [from, to] =
    rule.direction === "forward"
      ? [
          { icon: Laptop, spec: rule.host },
          { icon: Smartphone, spec: rule.device },
        ]
      : [
          { icon: Smartphone, spec: rule.device },
          { icon: Laptop, spec: rule.host },
        ];

  return (
    <li className="flex items-center gap-3 px-4 py-2.5">
      <from.icon className="text-muted-foreground size-4" />
      <span className="font-mono text-sm">{from.spec}</span>
      <ArrowRight className="text-muted-foreground size-3.5" />
      <to.icon className="text-muted-foreground size-4" />
      <span className="flex-1 font-mono text-sm">{to.spec}</span>
      <Button
        variant="ghost"
        size="icon"
        className="size-8"
        aria-label={`Remove ${from.spec} to ${to.spec}`}
        disabled={change.isPending}
        onClick={() => change.mutate({ kind: "remove", rule })}
      >
        {change.isPending ? <Loader2 className="animate-spin" /> : <X />}
      </Button>
    </li>
  );
}

export function PortsView({ serial }: { serial: string }) {
  const rules = usePortRules(serial);
  const clear = useChangePortRules(serial);
  const list = rules.data ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AddRuleForm serial={serial} />

      <div className="flex shrink-0 items-center gap-2 px-4 pt-3 pb-1">
        <p className="text-muted-foreground flex-1 text-xs font-medium">
          {list.length === 1 ? "1 rule" : `${list.length} rules`}
        </p>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void rules.refetch()}
          disabled={rules.isFetching}
        >
          <RefreshCw className={rules.isFetching ? "animate-spin" : undefined} />
          Refresh
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => clear.mutate({ kind: "clear" })}
          disabled={list.length === 0 || clear.isPending}
        >
          {clear.isPending ? <Loader2 className="animate-spin" /> : <Trash2 />}
          Remove all
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {rules.isError ? (
          <p className="text-destructive px-4 py-6 text-center text-sm">
            Could not read port rules. Try Refresh.
          </p>
        ) : list.length === 0 && !rules.isPending ? (
          <div className="text-muted-foreground flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
            <Unplug className="size-6" />
            <p className="max-w-sm text-sm">
              No port rules on this device. Add one above to share a port over adb.
            </p>
          </div>
        ) : (
          <ul className="divide-y">
            {list.map((rule) => (
              <RuleRow
                key={`${rule.direction}-${rule.host}-${rule.device}`}
                rule={rule}
                serial={serial}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
