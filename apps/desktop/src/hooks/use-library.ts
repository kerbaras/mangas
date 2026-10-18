import { useCallback } from "react";
import { useAppFocus } from "@/lib/app-focus";
import { followManga, listLibrary, type Manga, unfollowManga } from "@/lib/ipc";
import { useAsync } from "./use-async";

export function useLibrary() {
  const { state, reload, patch } = useAsync(listLibrary, []);
  useAppFocus(reload);

  const unfollow = useCallback(
    async (m: Manga) => {
      await unfollowManga(m.source, m.id);
      patch((items) => items.filter((x) => x.id !== m.id || x.source !== m.source));
    },
    [patch],
  );

  const follow = useCallback(
    async (m: Manga) => {
      const saved = await followManga(m.source, m.id);
      patch((items) => (items.some((x) => x.id === saved.id) ? items : [...items, saved]));
      return saved;
    },
    [patch],
  );

  return { state, reload, follow, unfollow };
}
