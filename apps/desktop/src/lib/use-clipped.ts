import { type RefObject, useCallback, useEffect, useState } from "react";

const cuts = (el: Element) => el.scrollWidth > el.clientWidth || el.scrollHeight > el.clientHeight;

/** Whether `el`, or one of its children, hides part of its text (an ellipsis or a height clamp). */
export const isClipped = (el: HTMLElement | null): boolean =>
  el !== null && (cuts(el) || Array.from(el.children).some(cuts));

/**
 * A live `isClipped` check for `ref`. While `watch` is on, a resize re-renders the caller so a
 * trigger can become (or stop being) a tab stop as its text starts or stops fitting.
 */
export function useClipped(ref: RefObject<HTMLElement | null>, watch: boolean): () => boolean {
  const [, setClipped] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!watch || !el || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => setClipped(isClipped(el)));
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref, watch]);
  return useCallback(() => isClipped(ref.current), [ref]);
}
