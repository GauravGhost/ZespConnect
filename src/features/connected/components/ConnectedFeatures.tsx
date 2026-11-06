import { ArrowLeftRight, Gamepad2, Volume2 } from "lucide-react";

import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { Switch } from "@/components/ui/switch";
import { useClipboardSync, useControllerPassthrough, usePhoneAudio } from "@/hooks/useDeviceLinks";
import type { Device, SoundRoute } from "@/types";

const SOUND_HINT: Record<SoundRoute, string> = {
  phone: "Where the phone's sound plays",
  computer: "The phone's speaker is silent",
  both: "Playing on the phone and this computer",
};

/** Ongoing links with the phone, each switched on or off per device. */
export function ConnectedFeatures({ device }: { device: Device }) {
  const clipboard = useClipboardSync(device.serial);
  const audio = usePhoneAudio(device.serial);
  const controllers = useControllerPassthrough(device.serial);

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
          hint={SOUND_HINT[audio.route]}
          busy={audio.pending}
        >
          <PhoneSoundToggle
            route={audio.route}
            sdk={device.sdk}
            disabled={audio.pending}
            onChange={audio.set}
          />
        </FeatureRow>
        <FeatureRow
          icon={Gamepad2}
          label="Controller passthrough"
          hint="Controllers on this PC work in this phone's mirror windows"
          busy={controllers.pending}
          htmlFor="feature-controller-passthrough"
        >
          <Switch
            id="feature-controller-passthrough"
            checked={controllers.on}
            disabled={controllers.pending}
            onCheckedChange={controllers.set}
          />
        </FeatureRow>
      </div>
    </Section>
  );
}
