import { Section } from "@/components/app/FeatureSection";
import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { usePhoneAudio } from "@/hooks/useDeviceLinks";
import type { SoundRoute } from "@/types";

const DESCRIPTION: Record<SoundRoute, string> = {
  phone: "Playing on the phone.",
  computer: "Playing on this computer; the phone's speaker is silent.",
  both: "Playing on the phone and this computer.",
};

/** Where the phone's sound plays; works with or without a mirror window open. */
export function PhoneSoundCard({ serial, sdk }: Readonly<{ serial: string; sdk: string | null }>) {
  const audio = usePhoneAudio(serial);

  return (
    <Section
      title="Phone sound"
      action={
        <PhoneSoundToggle
          route={audio.route}
          sdk={sdk}
          disabled={audio.pending}
          onChange={audio.set}
        />
      }
    >
      <p className="text-muted-foreground text-xs">{DESCRIPTION[audio.route]}</p>
    </Section>
  );
}
