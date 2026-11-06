import {
  Activity,
  BellOff,
  Check,
  ChevronDown,
  Coffee,
  Gamepad2,
  Headphones,
  Joystick,
  Loader2,
  MonitorOff,
  MonitorSmartphone,
  Pin,
  PlugZap,
  RefreshCw,
  RotateCcw,
  Smartphone,
  Speaker,
  Square,
  Timer,
  Video,
  type LucideIcon,
} from "lucide-react";
import { useId } from "react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SegmentedChoice } from "@/components/ui/segmented-choice";
import { Switch } from "@/components/ui/switch";
import { PresetNumberField } from "@/features/mirror";
import { useActiveDevice } from "@/hooks/useDevices";
import { useToolsStatus } from "@/hooks/useTools";
import { ipc } from "@/lib/ipc";
import { BITRATE_PRESETS, FPS_PRESETS, RESOLUTION_PRESETS } from "@/lib/mirror";
import { supportsSoundOnBoth } from "@/lib/phoneSound";
import { supportsInAppMirror } from "@/lib/webcodecs";
import type { GameOptions, GameOrientation } from "@/types";
import {
  useAudioOutputs,
  useControllers,
  useGameSession,
  useStartGame,
  useStopGame,
} from "../hooks";
import { DEFAULT_GAME_BITRATE_MBPS, DEFAULT_GAME_FPS, useGameModeStore } from "../store";

const ORIENTATIONS = [
  { value: "auto", label: "Auto", icon: RotateCcw },
  { value: "portrait", label: "Portrait", icon: Smartphone },
  { value: "landscape", label: "Landscape", icon: MonitorSmartphone },
] as const;

export function GameModePanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;
  const { data: tools } = useToolsStatus();
  const { video, options, setVideo, setOptions } = useGameModeStore();
  const session = useGameSession(serial);
  const start = useStartGame();
  const stop = useStopGame();

  if (!device || !serial) {
    return (
      <NoDeviceState
        icon={PlugZap}
        description="Connect an Android device to play its games here."
      />
    );
  }

  const scrcpyReady = Boolean(tools?.scrcpy.found);
  const inApp = supportsInAppMirror();
  const running = session !== null;
  const soundOnPhoneSupported = supportsSoundOnBoth(device.sdk);

  /** Save a change and, while a session runs, apply it to the phone and window right away. */
  function change<K extends keyof GameOptions>(key: K, value: GameOptions[K]) {
    const next = { ...options, [key]: value };
    setOptions(next);
    // Failures arrive as `game:warning` toasts; a session that just ended needs nothing.
    if (session) void ipc.game.update(session.id, next).catch(() => undefined);
  }

  const toggle = (key: keyof GameOptions) => ({
    checked: Boolean(options[key]),
    onCheckedChange: (value: boolean) => change(key, value),
  });

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-3 p-4">
      {!scrcpyReady ? (
        <Alert variant="warning">
          <AlertTitle>scrcpy is required</AlertTitle>
          <AlertDescription>Install it from the Settings screen to use Game Mode.</AlertDescription>
        </Alert>
      ) : null}
      {!inApp ? (
        <Alert variant="warning">
          <AlertTitle>Game Mode is not available here</AlertTitle>
          <AlertDescription>
            It needs in-app mirroring, which this system's web view cannot decode.
          </AlertDescription>
        </Alert>
      ) : null}

      <Section
        title="Game Mode"
        action={
          running ? (
            <Button
              size="sm"
              variant="destructive"
              disabled={stop.isPending}
              onClick={() => stop.mutate(session.id)}
            >
              {stop.isPending ? <Loader2 className="animate-spin" /> : <Square />}
              Stop
            </Button>
          ) : (
            <Button
              size="sm"
              disabled={!scrcpyReady || !inApp || start.isPending}
              onClick={() => start.mutate({ serial, video, game: options })}
            >
              {start.isPending ? <Loader2 className="animate-spin" /> : <Joystick />}
              Start Game Mode
            </Button>
          )
        }
      >
        <p className="text-muted-foreground text-xs">
          {running
            ? "Playing. Changes below apply right away, except video settings, which apply on the next start."
            : "Opens a low-latency window for playing and streaming. Everything it changes on the phone is put back when you stop."}
          {device.transport === "wifi" ? " USB gives the lowest latency." : null}
        </p>
      </Section>

      <Section title="Video">
        <div className="grid grid-cols-3 gap-3">
          <PresetNumberField
            id="game-resolution"
            label="Max size"
            placeholder="Native"
            value={video.resolution}
            presets={RESOLUTION_PRESETS}
            onChange={(resolution) => setVideo({ ...video, resolution })}
          />
          <PresetNumberField
            id="game-fps"
            label="FPS"
            placeholder={String(DEFAULT_GAME_FPS)}
            value={video.fps}
            presets={FPS_PRESETS}
            onChange={(fps) => setVideo({ ...video, fps })}
          />
          <PresetNumberField
            id="game-bitrate"
            label="Bitrate (Mbps)"
            placeholder={String(DEFAULT_GAME_BITRATE_MBPS)}
            value={video.bitrate}
            presets={BITRATE_PRESETS}
            onChange={(bitrate) => setVideo({ ...video, bitrate })}
          />
        </div>
        <SwitchRow
          icon={Coffee}
          label="Keep phone awake"
          hint="The screen never times out while playing"
          checked={video.stayAwake}
          onCheckedChange={(stayAwake) => setVideo({ ...video, stayAwake })}
        />
      </Section>

      <Section
        title="Orientation"
        action={
          <SegmentedChoice
            label="Orientation"
            options={ORIENTATIONS}
            value={options.orientation}
            onChange={(value: GameOrientation) => change("orientation", value)}
          />
        }
      >
        <p className="text-muted-foreground text-xs">
          {options.orientation === "auto"
            ? "The phone follows the game, as it normally does."
            : `The phone is held in ${options.orientation} while playing, then unlocked again.`}
        </p>
      </Section>

      <Section title="Streaming">
        <SwitchRow
          icon={Video}
          label="Clean capture"
          hint="No bars or cursor, so OBS captures only the game"
          {...toggle("cleanCapture")}
        />
        <SwitchRow
          icon={Pin}
          label="Always on top"
          hint="Keep the game window above other windows"
          {...toggle("alwaysOnTop")}
        />
        <SwitchRow
          icon={Activity}
          label="Stats overlay"
          hint="FPS, bitrate and dropped frames; turn off before streaming"
          {...toggle("stats")}
        />
      </Section>

      <Section title="Phone">
        <SwitchRow
          icon={BellOff}
          label="Do Not Disturb"
          hint="Keep notifications off screen and off stream"
          {...toggle("doNotDisturb")}
        />
        <SwitchRow
          icon={MonitorOff}
          label="Turn phone screen off"
          hint="Saves battery; the game keeps running"
          {...toggle("screenOff")}
        />
      </Section>

      <SoundSection
        options={options}
        soundOnPhoneSupported={soundOnPhoneSupported}
        toggle={toggle}
        onDevice={(name) => change("audioDevice", name)}
      />

      <ControllersSection toggle={toggle("controllers")} />
    </div>
  );
}

function SoundSection({
  options,
  soundOnPhoneSupported,
  toggle,
  onDevice,
}: Readonly<{
  options: GameOptions;
  soundOnPhoneSupported: boolean;
  toggle: (key: keyof GameOptions) => { checked: boolean; onCheckedChange: (on: boolean) => void };
  onDevice: (name: string | null) => void;
}>) {
  const outputs = useAudioOutputs();
  const selectId = useId();
  const soundOff = !options.soundOnComputer;
  const saved = options.audioDevice ?? null;
  const available = outputs.data ?? [];
  const choices: { value: string | null; label: string }[] = [
    { value: null, label: "System default" },
    ...available.map((name) => ({ value: name, label: name })),
    // A saved device that is unplugged stays selected; the default plays meanwhile.
    ...(saved && !available.includes(saved)
      ? [{ value: saved, label: `${saved} (not found)` }]
      : []),
  ];
  const selectedLabel = choices.find((choice) => choice.value === saved)?.label ?? "System default";

  return (
    <Section title="Sound">
      <SwitchRow
        icon={Speaker}
        label="Play phone sound on PC"
        hint="OBS can capture it with Application Audio Capture"
        {...toggle("soundOnComputer")}
      />
      <SwitchRow
        icon={Smartphone}
        label="Keep sound on the phone too"
        hint={soundOnPhoneSupported ? "Plays on both" : "Needs Android 13 or newer"}
        disabled={soundOff || !soundOnPhoneSupported}
        {...toggle("soundOnPhone")}
      />
      <SwitchRow
        icon={Timer}
        label="Low-latency sound"
        hint="Less delay; turn off if the sound crackles"
        disabled={soundOff}
        {...toggle("lowLatencyAudio")}
      />
      <FeatureRow
        icon={Headphones}
        label="Output"
        hint="Pick a virtual cable to give OBS its own track"
        busy={outputs.isFetching}
        htmlFor={selectId}
      >
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              id={selectId}
              variant="outline"
              size="sm"
              className="max-w-48"
              disabled={soundOff}
            >
              <span className="truncate">{selectedLabel}</span>
              <ChevronDown className="size-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {choices.map((choice) => (
              <DropdownMenuItem key={choice.value ?? ""} onSelect={() => onDevice(choice.value)}>
                <span className="flex-1">{choice.label}</span>
                {choice.value === saved ? <Check className="size-4" /> : null}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </FeatureRow>
    </Section>
  );
}

function ControllersSection({
  toggle,
}: Readonly<{ toggle: { checked: boolean; onCheckedChange: (on: boolean) => void } }>) {
  const controllers = useControllers();
  const list = controllers.data ?? [];

  return (
    <Section
      title="Controllers"
      action={
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label="Look for controllers again"
          disabled={controllers.isFetching}
          onClick={() => void controllers.refetch()}
        >
          <RefreshCw className={controllers.isFetching ? "animate-spin" : undefined} />
        </Button>
      }
    >
      <SwitchRow
        icon={Gamepad2}
        label="Controller passthrough"
        hint="Controllers on this PC work as real controllers on the phone"
        {...toggle}
      />
      {list.length === 0 ? (
        <p className="text-muted-foreground text-xs">
          No controllers found. Plug one in or pair it with this PC.
        </p>
      ) : (
        <ul className="space-y-1">
          {list.map((controller) => (
            <li key={controller.id} className="flex items-center gap-2 text-sm">
              <Gamepad2 className="text-muted-foreground size-4" />
              <span className="truncate">{controller.name}</span>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

function SwitchRow({
  icon,
  label,
  hint,
  checked,
  disabled,
  onCheckedChange,
}: Readonly<{
  icon: LucideIcon;
  label: string;
  hint: string;
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: (on: boolean) => void;
}>) {
  const id = useId();
  return (
    <FeatureRow icon={icon} label={label} hint={hint} busy={false} htmlFor={id}>
      <Switch id={id} checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
    </FeatureRow>
  );
}
