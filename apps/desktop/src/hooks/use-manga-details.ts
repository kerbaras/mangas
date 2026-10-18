import { getManga, type Manga } from "@/lib/ipc";
import { useAsync } from "./use-async";

/**
 * The full record for a manga opened from a card. Search results can be sparse (no synopsis,
 * status unknown), so the card is only a placeholder until the library or source answers; when
 * that fails the card stays.
 */
export function useMangaDetails(card: Manga) {
  const { source, id } = card;
  const { state } = useAsync(() => getManga(source, id), [source, id]);
  return {
    manga: state.kind === "ready" ? state.data : card,
    loading: state.kind === "loading",
  };
}
