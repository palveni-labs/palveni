import { useQuery } from "@tanstack/react-query";
import { Disc3 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { requestBlob } from "@/api/client";
import { isApiError } from "@/api/errors";
import { queryKeys } from "@/api/query-keys";

function CoverImage({ libraryId, id }: { libraryId: string; id: string }) {
  const ref = useRef<HTMLImageElement>(null);
  const art = useQuery({
    queryKey: [...queryKeys.items.libraryLists(libraryId), "album-artwork", id],
    queryFn: async ({ signal }) => {
      try {
        return await requestBlob(`/items/${encodeURIComponent(id)}/artwork`, { signal });
      } catch (error) {
        if (isApiError(error) && error.status === 404) return null;
        throw error;
      }
    },
    retry: false, staleTime: 60_000, gcTime: 10_000,
  });
  useEffect(() => {
    const img = ref.current;
    if (!img || !art.data) return;
    const url = URL.createObjectURL(art.data);
    img.hidden = false;
    img.src = url;
    return () => { img.removeAttribute("src"); URL.revokeObjectURL(url); };
  }, [art.data]);
  return art.data ? <img ref={ref} alt="" decoding="async" className="absolute inset-0 size-full object-cover" onError={(e) => { e.currentTarget.hidden = true; }} /> : null;
}

export function AlbumCover({ libraryId, id, large = false }: { libraryId: string; id?: string; large?: boolean }) {
  const tile = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === "undefined");
  useEffect(() => {
    const element = tile.current;
    if (!element || !id || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(([entry]) => setVisible(entry?.isIntersecting ?? false));
    observer.observe(element);
    return () => observer.disconnect();
  }, [id]);
  return <span ref={tile} aria-hidden className={`relative grid shrink-0 place-items-center overflow-hidden rounded-tile bg-sunken text-icon ring-1 ring-line ring-inset ${large ? "size-16 sm:size-24" : "size-11"}`}>
    <Disc3 aria-hidden className={large ? "size-7" : "size-5"} />
    {id && visible && <CoverImage key={id} libraryId={libraryId} id={id} />}
  </span>;
}
