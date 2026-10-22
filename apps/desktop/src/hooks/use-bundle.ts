import { useState } from "react";
import type { ExportPick } from "@/lib/export-name";
import { type BundleMode, initialMode } from "@/lib/export-plan";

/** How the picked chapters are bundled into books, and the "fewer chapters per book" lever. */
export function useBundle(picks: ExportPick[]) {
  const [mode, setMode] = useState<BundleMode>(() =>
    picks.length > 1 ? "single" : initialMode(picks.flatMap((p) => p.chapters)),
  );
  const [size, setSize] = useState(10);
  const every = (n: number) => {
    setMode("every");
    setSize(n);
  };
  return { mode, setMode, size, setSize, every };
}
