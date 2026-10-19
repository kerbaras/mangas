import { ArrowLeft } from "lucide-react";
import { type ReactNode, type RefObject, useEffect, useRef, useState } from "react";
import { HeaderCompact } from "@/components/header-action";
import { Hint } from "@/components/hint";
import { Button } from "@/components/ui/button";
import { WindowControls } from "@/components/window-controls";
import { useNavigation } from "@/lib/navigation";

type Props = { title: ReactNode; subtitle?: ReactNode; back?: boolean; children?: ReactNode };

/** Below this header width `HeaderAction`s drop their labels (a 960 px window leaves 904). */
export const COMPACT_BELOW = 1040;

/** Whether `el` is narrower than `COMPACT_BELOW`, kept live as the window resizes. */
function useCompact(el: RefObject<HTMLElement | null>): boolean {
  const [compact, setCompact] = useState(false);
  useEffect(() => {
    const node = el.current;
    if (!node || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => setCompact(node.clientWidth < COMPACT_BELOW));
    observer.observe(node);
    return () => observer.disconnect();
  }, [el]);
  return compact;
}

/**
 * 48px bar shared by every screen that doubles as the window's title bar: title left, actions
 * right, optional back arrow, and window controls where the OS does not draw them. Empty space
 * drags the window (`data-tauri-drag-region` must sit on the element that receives the press).
 * Below `COMPACT_BELOW` its `HeaderAction`s go icon-only, so the title and actions stay on one row.
 */
export function ScreenHeader({ title, subtitle, back, children }: Props) {
  const nav = useNavigation();
  const ref = useRef<HTMLElement>(null);
  const compact = useCompact(ref);
  return (
    <header
      ref={ref}
      data-tauri-drag-region
      className="flex h-12 shrink-0 select-none items-center gap-3 border-b border-stroke px-4"
    >
      {back && (
        <Button variant="ghost" size="icon" aria-label="Back" onClick={nav.back} className="-ml-2">
          <ArrowLeft />
        </Button>
      )}
      <div data-tauri-drag-region className="flex min-w-0 items-baseline gap-2">
        <Hint hint={typeof title === "string" ? title : null} truncated>
          <h1 data-tauri-drag-region className="truncate text-body-lg font-semibold">
            {title}
          </h1>
        </Hint>
        {subtitle && (
          <span
            data-tauri-drag-region
            className="shrink-0 whitespace-nowrap text-caption text-ink-3"
          >
            {subtitle}
          </span>
        )}
      </div>
      <div data-tauri-drag-region className="ml-auto flex shrink-0 items-center gap-2">
        <HeaderCompact.Provider value={compact}>{children}</HeaderCompact.Provider>
      </div>
      <WindowControls />
    </header>
  );
}
