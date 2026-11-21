import { Button } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { ArrowUp, FolderOpen } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];

export function MountPicker({ onPick }: { onPick: (path: string) => void }) {
  const { t } = useTranslation();
  const [mount, setMount] = useState("");
  const [folder, setFolder] = useState("");
  const mounts = useQuery({
    queryKey: ["storage", "mounts"],
    queryFn: ({ signal }) => request<Schemas["MountList"]>("/storage/mounts", { signal }),
  });
  const dirs = useQuery({
    queryKey: ["storage", "folders", mount, folder],
    queryFn: ({ signal }) =>
      request<Schemas["StorageFolders"]>("/storage/folders", { query: { mount, folder }, signal }),
    enabled: !!mount,
  });
  if (!mounts.data?.mounts.length) return null;
  const path = folder ? `${mount}/${folder}` : mount;
  return (
    <fieldset className="min-w-0 space-y-2">
      <legend className="text-sm font-mid text-fg">{t("mounts.choose")}</legend>
      <div className="flex flex-wrap items-center gap-2">
        <select
          aria-label={t("mounts.mount")}
          className="max-w-full rounded-control border border-line bg-surface px-2 py-1.5 text-sm text-fg"
          value={mount}
          onChange={(event) => {
            setMount(event.target.value);
            setFolder("");
          }}
        >
          <option value="">{t("mounts.mount")}</option>
          {mounts.data.mounts.map((m) => (
            <option key={m.path} value={m.path}>
              {m.protocol.toUpperCase()} · {m.path}
            </option>
          ))}
        </select>
        {mount && (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            isDisabled={!dirs.data || dirs.isError}
            onPress={() => onPick(path)}
          >
            <FolderOpen aria-hidden />
            {t("mounts.use")}
          </Button>
        )}
      </div>
      {mount && (
        <div className="flex flex-wrap items-center gap-2">
          {folder && (
            <Button
              type="button"
              size="sm"
              variant="ghost"
              aria-label={t("mounts.up")}
              onPress={() => setFolder(folder.split("/").slice(0, -1).join("/"))}
            >
              <ArrowUp aria-hidden />
            </Button>
          )}
          <span className="break-all text-sm text-fg-muted">{path}</span>
          {!!dirs.data?.directories.length && (
            <select
              aria-label={t("mounts.directory")}
              value=""
              className="max-w-full rounded-control border border-line bg-surface px-2 py-1.5 text-sm text-fg"
              onChange={(event) => {
                const chosen = event.target.value;
                if (chosen) setFolder(folder ? `${folder}/${chosen}` : chosen);
              }}
            >
              <option value="">{t("mounts.chooseFolder")}</option>
              {dirs.data.directories.map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          )}
        </div>
      )}
      {dirs.isError && mount && (
        <span role="alert" className="text-sm text-failed">
          {t("mounts.loadError")}
        </span>
      )}
      {dirs.data?.truncated && <p className="text-sm text-fg-muted">{t("mounts.limited")}</p>}
    </fieldset>
  );
}
