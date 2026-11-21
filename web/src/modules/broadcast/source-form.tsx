import { Button } from "@heroui/react";
import { useMutation } from "@tanstack/react-query";
import { useRef, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/ui/section";
import { ErrorState } from "@/ui/states";
import { broadcastApi, newSourceId, type BroadcastKind, type BroadcastSource } from "./api";

export const fieldClass =
  "w-full rounded-control border border-line bg-surface px-3 py-2 text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)";
const sizeLimit = 1024 * 1024;
function ContentField({
  label,
  value,
  onChange,
  accept,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  accept: string;
}) {
  const { t } = useTranslation();
  const [error, setError] = useState("");
  const sequence = useRef(0);
  async function read(file?: File) {
    const current = ++sequence.current;
    if (!file) return;
    setError("");
    try {
      if (file.size > sizeLimit) throw new RangeError();
      const text = new TextDecoder("utf-8", { fatal: true }).decode(await file.arrayBuffer());
      if (current === sequence.current) onChange(text);
    } catch (err) {
      if (current === sequence.current)
        setError(t(err instanceof RangeError ? "broadcast.tooLarge" : "broadcast.readFailed"));
    }
  }
  return (
    <div className="grid gap-2">
      <label className="grid gap-1 text-sm text-fg">
        {label}
        <textarea
          value={value}
          onChange={(event) => {
            sequence.current++;
            onChange(event.target.value);
          }}
          className={fieldClass}
          rows={3}
        />
      </label>
      <input
        type="file"
        accept={accept}
        aria-label={label + " · " + t("broadcast.upload")}
        className="max-w-full text-sm text-fg-muted"
        onChange={(event) => {
          void read(event.target.files?.[0]);
        }}
      />
      {error && (
        <span role="alert" className="text-sm text-danger">
          {error}
        </span>
      )}
    </div>
  );
}

export function SourceForm({
  kind,
  source,
  close,
  saved,
}: {
  kind: BroadcastKind;
  source?: BroadcastSource;
  close: () => void;
  saved: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [id] = useState(() => source?.id ?? newSourceId());
  const [name, setName] = useState(source?.name ?? "");
  const [feedUrl, setFeedUrl] = useState(source?.feedUrl ?? "");
  const [guideUrl, setGuideUrl] = useState(source?.guideUrl ?? "");
  const [content, setContent] = useState("");
  const [guide, setGuide] = useState("");
  const [invalid, setInvalid] = useState(false);
  const save = useMutation({
    mutationFn: () =>
      broadcastApi.save(id, {
        kind,
        name,
        expectedVersion: source?.version ?? 0,
        content: content.trim() || null,
        guide: guide.trim() || null,
        feedUrl: feedUrl.trim() || null,
        guideUrl: guideUrl.trim() || null,
      }),
    onSuccess: () => saved(id),
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!feedUrl.trim() && !content.trim()) {
      setInvalid(true);
      return;
    }
    setInvalid(false);
    save.mutate();
  }
  return (
    <Section title={t(source ? "broadcast.edit" : "broadcast.add")}>
      <form onSubmit={submit} className="grid max-w-3xl gap-4">
        <label className="grid gap-1 text-sm text-fg">
          {t("broadcast.name")}
          <input
            className={fieldClass}
            required
            maxLength={128}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label className="grid gap-1 text-sm text-fg">
          {t("broadcast.feedUrl")}
          <input
            className={fieldClass}
            type="url"
            value={feedUrl}
            onChange={(e) => setFeedUrl(e.target.value)}
          />
        </label>
        <ContentField
          label={t("broadcast.content")}
          value={content}
          onChange={setContent}
          accept={kind === "podcast" ? ".rss,.atom,.xml,.txt" : ".m3u,.m3u8,.txt"}
        />
        {kind !== "podcast" && (
          <details>
            <summary className="cursor-pointer text-sm text-fg">{t("broadcast.guide")}</summary>
            <div className="mt-3 grid gap-3">
              <label className="grid gap-1 text-sm text-fg">
                {t("broadcast.guideUrl")}
                <input
                  className={fieldClass}
                  type="url"
                  value={guideUrl}
                  onChange={(e) => setGuideUrl(e.target.value)}
                />
              </label>
              <ContentField
                label={t("broadcast.guide")}
                value={guide}
                onChange={setGuide}
                accept=".xml,.xmltv,.txt"
              />
            </div>
          </details>
        )}
        {invalid && (
          <span role="alert" className="text-sm text-danger">
            {t("broadcast.subRequired")}
          </span>
        )}
        {save.isError && <ErrorState error={save.error} compact />}
        <div className="flex gap-2">
          <Button type="submit" size="sm" isDisabled={save.isPending}>
            {t(save.isPending ? "broadcast.saving" : "broadcast.save")}
          </Button>
          <Button size="sm" variant="ghost" isDisabled={save.isPending} onPress={close}>
            {t("broadcast.cancel")}
          </Button>
        </div>
      </form>
    </Section>
  );
}
