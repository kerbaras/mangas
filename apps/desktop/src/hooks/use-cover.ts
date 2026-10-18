import { useEffect, useState } from "react";
import { loadCover } from "@/lib/covers";
import type { Manga } from "@/lib/ipc";

/** The cover's displayable URL plus whether the image has loaded or failed to. */
export function useCover({ source, id, cover_url }: Manga) {
  const [src, setSrc] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);

  // Keyed by value, not by object: a reloaded list with the same covers keeps them on screen.
  useEffect(() => {
    let alive = true;
    setSrc(null);
    setLoaded(false);
    setFailed(false);
    loadCover({ source, id, cover_url }).then((url) => {
      if (!alive) return;
      setSrc(url);
      setFailed(url === null);
    });
    return () => {
      alive = false;
    };
  }, [source, id, cover_url]);

  return {
    src: failed ? null : src,
    loaded: loaded && !failed,
    onLoad: () => setLoaded(true),
    onError: () => setFailed(true),
  };
}
