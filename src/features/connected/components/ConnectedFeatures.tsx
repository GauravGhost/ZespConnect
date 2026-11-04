import { ArrowLeftRight, Volume2 } from "lucide-react";

import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { Switch } from "@/components/ui/switch";
import { useClipboardSync, usePhoneAudio } from "@/hooks/useDeviceLinks";
import type { Device } from "@/types";
import { NowPlayingRow } from "./NowPlayingRow";

/** Ongoing links with the phone, each switched on or off per device. */
export function ConnectedFeatures({ device }: { device: Device }) {
  const clipboard = useClipboardSync(device.serial);
  const audio = usePhoneAudio(device.serial);

  return (
    <Section title="This phone">
      <div className="flex flex-col gap-2">
        <FeatureRow
          icon={ArrowLeftRight}
          label="Clipboard sync"
          hint="Copy on one, paste on the other (text)"
          busy={clipboard.pending}
          htmlFor="feature-clipboard-sync"
        >
          <Switch
            id="feature-clipboard-sync"
            checked={clipboard.on}
            disabled={clipboard.pending}
            onCheckedChange={clipboard.set}
          />
        </FeatureRow>
        <FeatureRow
          icon={Volume2}
          label="Phone sound"
          hint={audio.on ? "The phone's speaker is silent" : "Where the phone's sound plays"}
          busy={audio.pending}
        >
          <PhoneSoundToggle onComputer={audio.on} disabled={audio.pending} onChange={audio.set} />
        </FeatureRow>
        <NowPlayingRow serial={device.serial} />
      </div>
    </Section>
  );
}
