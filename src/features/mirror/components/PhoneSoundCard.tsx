import { Loader2, Volume2 } from "lucide-react";

import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { Card, CardContent } from "@/components/ui/card";
import { usePhoneAudio } from "@/hooks/useDeviceLinks";

/** Where the phone's sound plays; works with or without a mirror window open. */
export function PhoneSoundCard({ serial }: Readonly<{ serial: string }>) {
  const audio = usePhoneAudio(serial);

  return (
    <Card className="gap-0 py-0">
      <CardContent className="flex items-center gap-3 p-4">
        <span className="bg-muted text-primary flex size-10 shrink-0 items-center justify-center rounded-2xl">
          {audio.pending ? (
            <Loader2 className="size-5 animate-spin" />
          ) : (
            <Volume2 className="size-5" />
          )}
        </span>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">Phone sound</p>
          <p className="text-muted-foreground truncate text-xs">
            {audio.on
              ? "Playing on this computer; the phone's speaker is silent"
              : "Playing on the phone"}
          </p>
        </div>
        <PhoneSoundToggle onComputer={audio.on} disabled={audio.pending} onChange={audio.set} />
      </CardContent>
    </Card>
  );
}
