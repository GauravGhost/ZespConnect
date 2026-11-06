import type { LucideIcon } from "lucide-react";
import {
  CircleOff,
  Crosshair,
  Gauge,
  Hand,
  Layers,
  Rabbit,
  Snail,
  SquareDashed,
  Timer,
  Turtle,
} from "lucide-react";

import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { QuickTile } from "@/components/ui/quick-tile";
import { SegmentedChoice, type SegmentOption } from "@/components/ui/segmented-choice";
import { errorMessage } from "@/lib/ipc";
import type { DeveloperToggleKey } from "@/types";
import { useDeveloperState, useSetAnimationScale, useSetDeveloperToggle } from "../hooks";

const TOGGLES: { key: DeveloperToggleKey; label: string; icon: LucideIcon }[] = [
  { key: "show_taps", label: "Show taps", icon: Hand },
  { key: "pointer_location", label: "Pointer location", icon: Crosshair },
  { key: "layout_bounds", label: "Layout bounds", icon: SquareDashed },
  { key: "gpu_overdraw", label: "GPU overdraw", icon: Layers },
];

const SCALES: readonly SegmentOption<string>[] = [
  { value: "0", label: "Off", icon: CircleOff },
  { value: "0.5", label: "0.5x", icon: Rabbit },
  { value: "1", label: "1x", icon: Gauge },
  { value: "2", label: "2x", icon: Turtle },
  { value: "5", label: "5x", icon: Snail },
];

/** Common Android developer options, applied straight to the device. */
export function DeveloperView({ serial }: { serial: string }) {
  const { data, isLoading, isError, error } = useDeveloperState(serial);
  const setToggle = useSetDeveloperToggle(serial);
  const setScale = useSetAnimationScale(serial);

  return (
    <div className="mx-auto w-full max-w-3xl p-4">
      <Section title="Developer options">
        {isLoading ? (
          <p className="text-muted-foreground text-xs">Reading developer options…</p>
        ) : isError || !data ? (
          <p className="text-muted-foreground text-xs">
            Couldn&apos;t read developer options: {errorMessage(error)}
          </p>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2">
              {TOGGLES.map(({ key, label, icon }) => (
                <QuickTile
                  key={key}
                  layout="wide"
                  icon={icon}
                  label={label}
                  checked={data[key]}
                  disabled={setToggle.isPending}
                  onChange={(on) => setToggle.mutate({ key, on })}
                />
              ))}
            </div>
            <FeatureRow
              icon={Timer}
              label="Animation speed"
              hint="Window, transition and animator scale"
              busy={setScale.isPending}
            >
              <SegmentedChoice
                label="Animation scale"
                options={SCALES}
                value={data.animation_scale === null ? "" : String(data.animation_scale)}
                disabled={setScale.isPending}
                onChange={(value) => setScale.mutate(Number(value))}
              />
            </FeatureRow>
          </>
        )}
      </Section>
    </div>
  );
}
