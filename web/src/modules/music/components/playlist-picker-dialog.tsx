import { Button, Input, Modal } from "@heroui/react";
import { ListMusic, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { LoadingRows } from "@/ui/states";
import { useAddTracksToPlaylist, useCreatePlaylist, usePlaylists } from "../queries";

/** Modal for creating or renaming a playlist (or saving the current queue as a playlist). */
export function PlaylistNameDialog({
  isOpen,
  onOpenChange,
  title,
  initialName = "",
  submitLabel,
  isPending = false,
  onSubmit,
}: {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  initialName?: string;
  submitLabel: string;
  isPending?: boolean;
  onSubmit: (name: string) => void | Promise<void>;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(initialName);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) {
      setError(t("musicApp.playlists.nameRequired"));
      return;
    }
    setError(null);
    await onSubmit(trimmed);
    onOpenChange(false);
  };

  return (
    <Modal.Backdrop isOpen={isOpen} onOpenChange={onOpenChange}>
      <Modal.Container placement="center" size="sm">
        <Modal.Dialog>
          <form onSubmit={(event) => void handleSubmit(event)}>
            <Modal.Header>
              <Modal.Heading>{title}</Modal.Heading>
            </Modal.Header>
            <Modal.Body className="grid gap-2">
              <label htmlFor="playlist-name-input" className="text-sm font-mid text-fg">
                {t("musicApp.playlists.nameLabel")}
              </label>
              <Input
                id="playlist-name-input"
                autoFocus
                value={name}
                onChange={(event) => {
                  setName(event.target.value);
                  if (error) setError(null);
                }}
                placeholder={t("musicApp.playlists.namePlaceholder")}
              />
              {error ? <p role="alert" className="text-xs text-failed">{error}</p> : null}
            </Modal.Body>
            <Modal.Footer>
              <Button variant="ghost" onPress={() => onOpenChange(false)}>
                {t("common.cancel")}
              </Button>
              <Button type="submit" variant="primary" isDisabled={isPending}>
                {submitLabel}
              </Button>
            </Modal.Footer>
          </form>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

/** Modal for picking an existing playlist (or creating a new one) to add 1+ tracks into. */
export function PlaylistPickerDialog({
  trackIds,
  onClose,
}: {
  trackIds: string[] | null;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const open = Boolean(trackIds && trackIds.length > 0);
  const playlists = usePlaylists(open);
  const addTracks = useAddTracksToPlaylist();
  const createPlaylist = useCreatePlaylist();
  const [newName, setNewName] = useState("");

  const handleCreateAndAdd = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = newName.trim();
    if (!trimmed || !trackIds?.length) return;
    await createPlaylist.mutateAsync({ name: trimmed, trackIds });
    setNewName("");
    onClose();
  };

  return (
    <Modal.Backdrop isOpen={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Modal.Container placement="center" size="sm">
        <Modal.Dialog>
          <Modal.Header>
            <Modal.Heading>{t("musicApp.playlists.addToPlaylist")}</Modal.Heading>
          </Modal.Header>
          <Modal.Body className="grid gap-3">
            <form onSubmit={(event) => void handleCreateAndAdd(event)} className="flex gap-2">
              <Input
                aria-label={t("musicApp.playlists.nameLabel")}
                value={newName}
                onChange={(event) => setNewName(event.target.value)}
                placeholder={t("musicApp.playlists.namePlaceholder")}
                className="min-w-0 flex-1"
              />
              <Button
                type="submit"
                variant="outline"
                size="sm"
                isDisabled={!newName.trim() || createPlaylist.isPending}
              >
                <Plus aria-hidden />
                {t("musicApp.playlists.createAndAdd")}
              </Button>
            </form>
            <div className="max-h-72 overflow-y-auto">
              {playlists.isPending ? (
                <LoadingRows rows={3} />
              ) : (playlists.data?.playlists.length ?? 0) === 0 ? (
                <p className="py-4 text-center text-sm text-fg-muted">{t("musicApp.playlists.empty")}</p>
              ) : (
                <ul className="grid gap-1">
                  {playlists.data!.playlists.map((playlist) => (
                    <li key={playlist.id}>
                      <button
                        type="button"
                        disabled={addTracks.isPending}
                        onClick={() => {
                          if (!trackIds?.length) return;
                          addTracks.mutate({ playlistId: playlist.id, trackIds }, { onSuccess: () => onClose() });
                        }}
                        className="flex w-full items-center gap-3 rounded-control px-3 py-2 text-start text-sm text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                      >
                        <ListMusic className="size-4 shrink-0 text-icon" aria-hidden />
                        <span className="min-w-0 flex-1 truncate font-mid">{playlist.name}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </Modal.Body>
          <Modal.Footer>
            <Button variant="ghost" onPress={onClose}>
              {t("common.cancel")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}
