import { Button, toast } from "@heroui/react";
import { Upload, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Library } from "@/api/types";
import { errorText } from "@/lib/errors";
import { formatBytesCompact } from "../format";
import { useUploadPhoto } from "../queries";

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const res = typeof reader.result === "string" ? reader.result : "";
      resolve(res);
    };
    reader.onerror = () => reject(reader.error ?? new Error("Failed to read image"));
    reader.readAsDataURL(file);
  });
}

export function UploadPhotoModal({
  open,
  onClose,
  photoLibraries,
  defaultLibraryId,
}: {
  open: boolean;
  onClose: () => void;
  photoLibraries: readonly Library[];
  defaultLibraryId?: string;
}) {
  if (!open) return null;
  return (
    <UploadPhotoDialog
      onClose={onClose}
      photoLibraries={photoLibraries}
      defaultLibraryId={defaultLibraryId}
    />
  );
}

function UploadPhotoDialog({
  onClose,
  photoLibraries,
  defaultLibraryId,
}: {
  onClose: () => void;
  photoLibraries: readonly Library[];
  defaultLibraryId?: string;
}) {
  const { t } = useTranslation();
  const upload = useUploadPhoto();
  const [libraryId, setLibraryId] = useState<string>(() => defaultLibraryId ?? photoLibraries[0]?.id ?? "");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedFile || !libraryId) return;
    try {
      const dataBase64 = await fileToBase64(selectedFile);
      upload.mutate(
        { libraryId, body: { fileName: selectedFile.name, dataBase64 } },
        {
          onSuccess: (info) => {
            toast.success(t("photosApp.upload.success", { name: info.fileName }));
            onClose();
          },
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    } catch (err) {
      toast.danger(errorText(t, err));
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("photosApp.upload.title")}
        className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
      >
        <div className="flex items-center justify-between gap-2">
          <h2 className="font-heading text-lg font-strong text-heading">
            {t("photosApp.upload.title")}
          </h2>
          <button
            type="button"
            aria-label={t("photosApp.upload.cancel")}
            onClick={onClose}
            className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        {photoLibraries.length === 0 ? (
          <p className="text-sm text-fg-muted">{t("photosApp.upload.noLibrary")}</p>
        ) : (
          <form onSubmit={(e) => void handleSubmit(e)} className="grid gap-3.5">
            <div className="grid gap-1.5">
              <label htmlFor="photo-upload-library" className="text-sm font-mid text-fg">
                {t("photosApp.upload.libraryLabel")}
              </label>
              <select
                id="photo-upload-library"
                value={libraryId}
                onChange={(e) => setLibraryId(e.target.value)}
                className="h-9 rounded-control border border-line-strong bg-surface px-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                {photoLibraries.map((lib) => (
                  <option key={lib.id} value={lib.id}>
                    {lib.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="grid gap-1.5">
              <label htmlFor="photo-upload-file" className="text-sm font-mid text-fg">
                {t("photosApp.upload.chooseFile")}
              </label>
              <input
                id="photo-upload-file"
                type="file"
                accept="image/*,.heic,.heif,.dng,.cr2,.cr3,.nef,.arw,.orf,.rw2,.raf,.jxl"
                onChange={(e) => setSelectedFile(e.target.files?.[0] ?? null)}
                className="block w-full rounded-control border border-line bg-sunken px-3 py-2 text-sm text-fg file:me-3 file:rounded-control file:border-0 file:bg-surface file:px-2.5 file:py-1 file:text-xs file:font-mid file:text-fg"
              />
              {selectedFile ? (
                <p className="text-xs text-fg-muted">
                  {t("photosApp.upload.selectedFile", {
                    name: selectedFile.name,
                    size: formatBytesCompact(selectedFile.size),
                  })}
                </p>
              ) : null}
            </div>

            <div className="flex justify-end gap-2 pt-1">
              <Button type="button" variant="outline" size="sm" onPress={onClose}>
                {t("photosApp.upload.cancel")}
              </Button>
              <Button
                type="submit"
                variant="primary"
                size="sm"
                isDisabled={!selectedFile || !libraryId || upload.isPending}
              >
                <Upload className="size-4" aria-hidden />
                {upload.isPending ? t("photosApp.upload.uploading") : t("photosApp.upload.submit")}
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
