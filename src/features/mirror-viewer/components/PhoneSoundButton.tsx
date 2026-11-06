import { Loader2, Volume2, VolumeOff } from "lucide-react";

import { usePhoneAudio } from "@/hooks/useDeviceLinks";
import { MirrorIconButton } from "./MirrorIconButton";

/** Title-bar switch between hearing the phone here and on the phone itself. */
export function PhoneSoundButton({ serial }: Readonly<{ serial: string }>) {
  const audio = usePhoneAudio(serial);

  return (
    <MirrorIconButton
      label={audio.on ? "Play the sound on the phone instead" : "Play the phone's sound here"}
      icon={audio.pending ? Loader2 : audio.on ? Volume2 : VolumeOff}
      iconClassName={audio.pending ? "animate-spin" : undefined}
      pressed={audio.on}
      disabled={audio.pending}
      onClick={() => audio.set(audio.on ? "phone" : "computer")}
    />
  );
}
