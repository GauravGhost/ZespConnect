import { Section } from "@/components/app/FeatureSection";
import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { usePhoneAudio } from "@/hooks/useDeviceLinks";

/** Where the phone's sound plays; works with or without a mirror window open. */
export function PhoneSoundCard({ serial }: Readonly<{ serial: string }>) {
  const audio = usePhoneAudio(serial);

  return (
    <Section
      title="Phone sound"
      action={
        <PhoneSoundToggle onComputer={audio.on} disabled={audio.pending} onChange={audio.set} />
      }
    >
      <p className="text-muted-foreground text-xs">
        {audio.on
          ? "Playing on this computer; the phone's speaker is silent."
          : "Playing on the phone."}
      </p>
    </Section>
  );
}
