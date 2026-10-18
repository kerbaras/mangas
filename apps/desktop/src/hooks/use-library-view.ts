import { useCallback, useState } from "react";
import { type LibraryView, parseView } from "@/lib/library-view";

const KEY = "mangas.library.view";

/** The library screen's category, filters, sort and layout, remembered across launches. */
export function useLibraryView() {
  const [view, setView] = useState(() => parseView(window.localStorage.getItem(KEY)));
  const update = useCallback((patch: Partial<LibraryView>) => {
    setView((v) => {
      const next = { ...v, ...patch };
      window.localStorage.setItem(KEY, JSON.stringify(next));
      return next;
    });
  }, []);
  return [view, update] as const;
}
