import { Music, Pause, Play, SkipBack, SkipForward } from "lucide-react";

import { FeatureRow } from "@/components/app/FeatureSection";
import { Button } from "@/components/ui/button";
import { useNowPlaying } from "@/hooks/useNowPlaying";
import type { NowPlaying } from "@/types";

/** The phone's current song with previous, play/pause and next buttons. */
export function NowPlayingRow({ serial }: { serial: string }) {
  const { nowPlaying, pressing, press } = useNowPlaying(serial);
  const idle = !nowPlaying || pressing;

  return (
    <FeatureRow
      icon={Music}
      label={nowPlaying?.title ?? "Nothing playing"}
      hint={describe(nowPlaying)}
      busy={nowPlaying === undefined || pressing}
    >
      <div className="flex shrink-0 items-center">
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          aria-label="Previous"
          title="Previous"
          disabled={idle || !nowPlaying.canPrevious}
          onClick={() => press("previous")}
        >
          <SkipBack />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          aria-label={nowPlaying?.playing ? "Pause" : "Play"}
          title={nowPlaying?.playing ? "Pause" : "Play"}
          disabled={idle}
          onClick={() => press("playPause")}
        >
          {nowPlaying?.playing ? <Pause /> : <Play />}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          aria-label="Next"
          title="Next"
          disabled={idle || !nowPlaying.canNext}
          onClick={() => press("next")}
        >
          <SkipForward />
        </Button>
      </div>
    </FeatureRow>
  );
}

function describe(nowPlaying: NowPlaying | null | undefined): string {
  if (nowPlaying === undefined) return "Checking the phone";
  if (nowPlaying === null) return "Play music or a video on the phone";
  if (nowPlaying.error) return nowPlaying.error;
  return [nowPlaying.artist, nowPlaying.app].filter(Boolean).join(" · ");
}
