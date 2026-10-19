import { useEffect, useRef } from "react";
import { rememberScroll, savedScroll } from "@/lib/screen-memory";

const USER_SCROLL = ["wheel", "pointerdown", "keydown", "touchstart"] as const;

/**
 * Ref for a scroll pane whose offset is kept under `key` for the session. On mount (and when
 * `key` changes) the saved offset is put back; while the content is still too short to reach it
 * (results or covers loading), it is put back again as the content grows, until it is reached or
 * the user scrolls. Only offsets the user lands on are saved.
 */
export function useScrollMemory<T extends HTMLElement>(key: string) {
  const ref = useRef<T>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const target = savedScroll(key);
    let pending = target > 0;
    const apply = () => {
      el.scrollTop = target;
      if (el.scrollTop >= target - 1) stop();
    };
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(apply);
    const stop = () => {
      pending = false;
      observer?.disconnect();
      for (const ev of USER_SCROLL) el.removeEventListener(ev, stop);
    };
    const save = () => {
      if (!pending) rememberScroll(key, el.scrollTop);
    };
    if (pending) {
      for (const child of el.children) observer?.observe(child);
      observer?.observe(el);
      for (const ev of USER_SCROLL) el.addEventListener(ev, stop, { passive: true });
      apply();
    } else {
      el.scrollTop = 0;
    }
    el.addEventListener("scroll", save, { passive: true });
    return () => {
      stop();
      el.removeEventListener("scroll", save);
    };
  }, [key]);
  return ref;
}
