import { useState } from "react";

import { Button } from "@/components/ui/button";
import { ComboboxInput, type ComboboxOption } from "@/components/ui/combobox-input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { MirrorOptions } from "@/types";

const toOptions = (values: number[]): ComboboxOption[] =>
  values.map((value) => ({ value: String(value), label: String(value) }));

const RESOLUTION_PRESETS: ComboboxOption[] = [
  { value: "", label: "Native" },
  ...toOptions([720, 1080, 1440, 2160]),
];
const FPS_PRESETS = toOptions([30, 60, 90, 120]);
const BITRATE_PRESETS = toOptions([2, 4, 8, 16, 24]);
const ZOOM_PRESETS = toOptions([0.5, 1, 1.5, 2, 3, 5]);

interface MirrorOptionsFormProps {
  value: MirrorOptions;
  onChange: (next: MirrorOptions) => void;
  variant: "screen" | "camera";
}

/** Numeric field with a preset dropdown. Keeps the raw text locally so partial
 * decimal input ("1.") survives while typing, committing only valid numbers. */
function PresetNumberField({
  id,
  label,
  placeholder,
  value,
  presets,
  onChange,
}: {
  id: string;
  label: string;
  placeholder: string;
  value: number | null;
  presets: ComboboxOption[];
  onChange: (next: number | null) => void;
}) {
  const [text, setText] = useState(value === null ? "" : String(value));

  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <ComboboxInput
        id={id}
        inputMode="decimal"
        placeholder={placeholder}
        value={text}
        options={presets}
        onValueChange={(next) => {
          setText(next);
          const trimmed = next.trim();
          if (trimmed === "") {
            onChange(null);
            return;
          }
          const parsed = Number(trimmed);
          if (Number.isFinite(parsed)) onChange(parsed);
        }}
      />
    </div>
  );
}

function ToggleField({
  id,
  label,
  checked,
  onChange,
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <Label htmlFor={id} className="cursor-pointer font-normal">
        {label}
      </Label>
      <Switch id={id} checked={checked} onCheckedChange={onChange} />
    </div>
  );
}

export function MirrorOptionsForm({ value, onChange, variant }: MirrorOptionsFormProps) {
  function update<K extends keyof MirrorOptions>(key: K, next: MirrorOptions[K]) {
    onChange({ ...value, [key]: next });
  }

  if (variant === "camera") {
    return (
      <div className="space-y-3">
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label>Facing</Label>
            <div className="flex items-center gap-1">
              {(["back", "front"] as const).map((facing) => (
                <Button
                  key={facing}
                  type="button"
                  size="sm"
                  className="flex-1"
                  variant={value.camera_facing === facing ? "secondary" : "outline"}
                  onClick={() => update("camera_facing", facing)}
                >
                  {facing === "back" ? "Back" : "Front"}
                </Button>
              ))}
            </div>
          </div>
          <PresetNumberField
            id="camera-zoom"
            label="Zoom"
            placeholder="1.0"
            value={value.camera_zoom}
            presets={ZOOM_PRESETS}
            onChange={(next) => update("camera_zoom", next)}
          />
        </div>

        <div className="grid grid-cols-2 gap-x-6 gap-y-2">
          <ToggleField
            id="camera-torch"
            label="Torch"
            checked={value.camera_torch}
            onChange={(next) => update("camera_torch", next)}
          />
          <ToggleField
            id="camera-audio"
            label="Audio"
            checked={value.audio}
            onChange={(next) => update("audio", next)}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-3">
        <PresetNumberField
          id="mirror-resolution"
          label="Max size"
          placeholder="Native"
          value={value.resolution}
          presets={RESOLUTION_PRESETS}
          onChange={(next) => update("resolution", next)}
        />
        <PresetNumberField
          id="mirror-fps"
          label="FPS"
          placeholder="60"
          value={value.fps}
          presets={FPS_PRESETS}
          onChange={(next) => update("fps", next)}
        />
        <PresetNumberField
          id="mirror-bitrate"
          label="Bitrate (Mbps)"
          placeholder="8"
          value={value.bitrate}
          presets={BITRATE_PRESETS}
          onChange={(next) => update("bitrate", next)}
        />
      </div>

      <div className="grid grid-cols-2 gap-x-6 gap-y-2">
        <ToggleField
          id="mirror-audio"
          label="Audio"
          checked={value.audio}
          onChange={(next) => update("audio", next)}
        />
        <ToggleField
          id="mirror-borderless"
          label="Borderless"
          checked={value.borderless}
          onChange={(next) => update("borderless", next)}
        />
        <ToggleField
          id="mirror-stay-awake"
          label="Stay awake"
          checked={value.stay_awake}
          onChange={(next) => update("stay_awake", next)}
        />
        <ToggleField
          id="mirror-always-on-top"
          label="Always on top"
          checked={value.always_on_top}
          onChange={(next) => update("always_on_top", next)}
        />
      </div>
    </div>
  );
}
