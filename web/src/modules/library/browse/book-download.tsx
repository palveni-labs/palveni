import { Button, Spinner, toast } from "@heroui/react";
import { useMutation } from "@tanstack/react-query";
import { Download } from "lucide-react";
import { useTranslation } from "react-i18next";
import { API_BASE, request } from "@/api/client";
import type { components } from "@/api/generated";
import { errorText } from "@/lib/errors";

export function BookDownloadButton({ id }: { id: string }) {
  const { t } = useTranslation();
  const path = `/items/${encodeURIComponent(id)}/book-download`;
  const download = useMutation({
    mutationFn: async () => {
      const info = await request<components["schemas"]["BookDownload"]>(path, { method: "POST" });
      const url = new URL(info.url, window.location.origin);
      if (url.origin !== window.location.origin || url.pathname !== API_BASE + path) {
        throw new Error("Invalid book download URL");
      }
      const link = document.createElement("a");
      link.href = url.href;
      link.download = "";
      link.referrerPolicy = "no-referrer";
      document.body.append(link);
      link.click();
      link.remove();
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });
  return <Button variant="outline" isDisabled={download.isPending} onPress={() => download.mutate()}>
    {download.isPending ? <Spinner size="sm" color="current" /> : <Download aria-hidden />}
    {t("browse.downloadBook")}
  </Button>;
}
