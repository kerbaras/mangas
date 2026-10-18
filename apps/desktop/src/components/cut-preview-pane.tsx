import { ErrorAlert } from "@/components/error-alert";
import type { CutPreview } from "@/lib/ipc.cut-preview";

export type PreviewState =
  | { kind: "loading" }
  | { kind: "ready"; preview: CutPreview }
  | { kind: "error"; message: string };

/** The preview pages of the smart-cut options panel, with their count and cut time. */
export function CutPreviewPane({ state }: { state: PreviewState }) {
  if (state.kind === "error")
    return <ErrorAlert title="Couldn't cut this chapter" message={state.message} />;
  const preview = state.kind === "ready" ? state.preview : null;
  return (
    <div className="flex max-h-[60vh] min-h-0 flex-col gap-2">
      <p className="text-caption text-ink-3" aria-live="polite">
        {preview
          ? `First ${preview.pages.length} pages${preview.more ? " (the chapter has more)" : ""} · cut in ${(preview.elapsed_ms / 1000).toFixed(1)} s`
          : "Cutting pages…"}
      </p>
      <div
        className={`grid grid-cols-4 gap-2 overflow-auto ${preview ? "" : "opacity-50"}`}
        aria-busy={!preview}
      >
        {preview?.pages.map((p, i) => (
          <img
            key={p.src}
            src={p.src}
            alt={`Page ${i + 1}`}
            width={p.width}
            height={p.height}
            className="h-auto w-full border border-stroke bg-white"
          />
        ))}
      </div>
    </div>
  );
}
