import { Loader2, Volume2, VolumeOff } from "lucide-react";

import { usePhoneAudio } from "@/hooks/useDeviceLinks";
import { cn } from "@/lib/utils";

/** Title-bar switch between hearing the phone here and on the phone itself. */
export function PhoneSoundButton({
  serial,
  className,
}: Readonly<{ serial: string; className?: string }>) {
  const audio = usePhoneAudio(serial);
  const Icon = audio.pending ? Loader2 : audio.on ? Volume2 : VolumeOff;

  return (
    <button
      type="button"
      onClick={() => audio.set(!audio.on)}
      disabled={audio.pending}
      aria-pressed={audio.on}
      title={audio.on ? "Play the sound on the phone instead" : "Play the phone's sound here"}
      className={cn(
        "flex items-center justify-center rounded-full transition-colors",
        audio.on
          ? "text-emerald-300 hover:bg-emerald-500/25"
          : "text-neutral-300 hover:bg-white/10",
        className,
      )}
    >
      <Icon className={cn("size-3.5", audio.pending && "animate-spin")} />
    </button>
  );
}
