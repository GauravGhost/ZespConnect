import {
  Laptop,
  Loader2,
  MonitorSmartphone,
  Music,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  Smartphone,
} from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { usePhoneAudio } from "@/hooks/useDeviceLinks";
import { useNowPlaying } from "@/hooks/useNowPlaying";
import { soundRouteChoices } from "@/lib/phoneSound";
import { cn } from "@/lib/utils";

const OUTPUTS = [
  { value: "phone", label: "This phone", icon: Smartphone },
  { value: "computer", label: "Computer", icon: Laptop },
  { value: "both", label: "Both", icon: MonitorSmartphone },
] as const;

/** The shade's media player, like Android's: what plays, its buttons and where it plays. */
export function MediaCard({ serial, sdk }: { serial: string; sdk: string | null }) {
  const { nowPlaying, pressing, press } = useNowPlaying(serial);
  // Android hides the player when nothing plays.
  if (!nowPlaying) return null;

  const subtitle = nowPlaying.error ?? nowPlaying.artist ?? nowPlaying.app;
  const playLabel = nowPlaying.playing ? "Pause" : "Play";

  return (
    <section
      aria-label="Now playing"
      className="bg-primary-container text-on-primary-container flex flex-col gap-2 rounded-[1.5rem] p-3"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="flex min-w-0 items-center gap-1.5 text-[11px] font-medium opacity-80">
          <Music className="size-3.5 shrink-0" />
          <span className="truncate">{nowPlaying.app}</span>
        </span>
        <OutputSwitcher serial={serial} sdk={sdk} />
      </div>

      <div className="flex items-center gap-1">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold" title={nowPlaying.title}>
            {nowPlaying.title}
          </p>
          <p className="truncate text-xs opacity-75" title={subtitle}>
            {subtitle}
          </p>
        </div>
        <MediaButton
          label="Previous"
          disabled={pressing || !nowPlaying.canPrevious}
          onClick={() => press("previous")}
        >
          <SkipBack />
        </MediaButton>
        <button
          type="button"
          aria-label={playLabel}
          title={playLabel}
          disabled={pressing}
          onClick={() => press("playPause")}
          className={cn(
            "bg-on-primary-container text-primary-container flex size-10 shrink-0 items-center justify-center outline-none",
            "transition-all duration-200 ease-emphasized active:scale-[0.94] disabled:opacity-60",
            "focus-visible:ring-ring/50 focus-visible:ring-[3px]",
            nowPlaying.playing ? "rounded-[0.9rem]" : "rounded-full",
          )}
        >
          {nowPlaying.playing ? (
            <Pause className="size-4 fill-current" />
          ) : (
            <Play className="size-4 fill-current" />
          )}
        </button>
        <MediaButton
          label="Next"
          disabled={pressing || !nowPlaying.canNext}
          onClick={() => press("next")}
        >
          <SkipForward />
        </MediaButton>
      </div>
    </section>
  );
}

function MediaButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="hover:bg-on-primary-container/15 hover:text-on-primary-container size-8 [&_svg]:fill-current"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}

/** The output chip: plays the phone's sound on the phone, on this computer, or on both. */
function OutputSwitcher({ serial, sdk }: { serial: string; sdk: string | null }) {
  const audio = usePhoneAudio(serial);
  const current = OUTPUTS.find((output) => output.value === audio.route) ?? OUTPUTS[0];
  const Icon = audio.pending ? Loader2 : current.icon;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={`Playing on ${current.label}. Choose where to play`}
          disabled={audio.pending}
          className="bg-on-primary-container/15 hover:bg-on-primary-container/25 focus-visible:ring-ring/50 flex h-7 shrink-0 items-center gap-1.5 rounded-full px-2.5 text-[11px] font-semibold outline-none transition-colors focus-visible:ring-[3px] disabled:opacity-70"
        >
          <Icon className={cn("size-3.5", audio.pending && "animate-spin")} />
          {current.label}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuLabel>Play on</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {soundRouteChoices(OUTPUTS, sdk, audio.route).map(
          ({ value: route, label, icon: OutputIcon }) => (
            <DropdownMenuCheckboxItem
              key={route}
              checked={audio.route === route}
              onCheckedChange={() => audio.set(route)}
            >
              <OutputIcon />
              {label}
            </DropdownMenuCheckboxItem>
          ),
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
