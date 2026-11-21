import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useFeature } from "@/modules/access";
import { TrackLoudness } from "./music-loudness";
import { OriginalPlayer } from "./original-player";

export function MusicPlayer({ id, title, libraryId }: { id: string; title: string; libraryId: string }) {
  const { t } = useTranslation();
  const enabled = useFeature("musicLoudness");
  const [expanded, setExpanded] = useState(false);
  return <>
    <OriginalPlayer id={id} title={title} />
    {enabled && <details className="px-2 text-sm" onToggle={(event) => setExpanded(event.currentTarget.open)}>
      <summary className="cursor-pointer text-fg-muted">{t("music.loudness")}</summary>
      {expanded && <TrackLoudness id={id} libraryId={libraryId} />}
    </details>}
  </>;
}
