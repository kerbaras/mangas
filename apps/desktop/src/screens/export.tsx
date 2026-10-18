import { Send } from "lucide-react";
import { useState } from "react";
import { DeliverySection } from "@/components/delivery-section";
import { DownloadedPicker } from "@/components/downloaded-picker";
import { EmptyState } from "@/components/empty-state";
import { ErrorAlert } from "@/components/error-alert";
import { ExportDialog } from "@/components/export-dialog";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { useDownloadedChapters } from "@/hooks/use-downloaded-chapters";
import type { ExportPick } from "@/lib/export-name";
import { useNavigation } from "@/lib/navigation";

type Props = { chapterIds?: string[] };

/** Picks downloaded chapters across the library for the export dialog; keeps the history. */
export function ExportScreen({ chapterIds }: Props) {
  const nav = useNavigation();
  const groups = useDownloadedChapters();
  const [selected, setSelected] = useState<Set<string>>(new Set(chapterIds ?? []));
  const [open, setOpen] = useState(false);
  const [exported, setExported] = useState<string | null>(null);
  const [sends, setSends] = useState(0);

  const picks: ExportPick[] =
    groups.state.kind === "ready"
      ? groups.state.data
          .map((g) => ({
            manga: g.manga,
            chapters: g.chapters.filter((c) => selected.has(c.id) && c.local_path),
          }))
          .filter((p) => p.chapters.length > 0)
      : [];
  const count = picks.reduce((n, p) => n + p.chapters.length, 0);

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Export" subtitle={`${count} chapters picked`}>
        <Button variant="primary" size="sm" disabled={count === 0} onClick={() => setOpen(true)}>
          <Send /> Export{count > 0 ? ` ${count} ${count === 1 ? "chapter" : "chapters"}` : ""}
        </Button>
      </ScreenHeader>
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(280px,2fr)_minmax(360px,3fr)]">
        <div className="overflow-auto border-r border-stroke p-4">
          {groups.state.kind === "loading" && (
            <p className="text-ink-3">Looking for downloaded chapters…</p>
          )}
          {groups.state.kind === "error" && (
            <ErrorAlert
              title="Couldn't list downloaded chapters"
              message={groups.state.message}
              onRetry={groups.reload}
            />
          )}
          {groups.state.kind === "ready" && groups.state.data.length === 0 && (
            <EmptyState
              title="Nothing downloaded yet"
              action={<Button onClick={() => nav.go({ screen: "library" })}>Go to library</Button>}
            >
              Download chapters from a manga's page, then pick them here.
            </EmptyState>
          )}
          {groups.state.kind === "ready" && groups.state.data.length > 0 && (
            <DownloadedPicker
              groups={groups.state.data}
              selected={selected}
              onChange={setSelected}
            />
          )}
        </div>
        <div className="flex flex-col gap-4 overflow-auto p-4">
          <p className="text-caption text-ink-3">
            Pick chapters, then Export opens the export dialog: bundle, format, smart cut and
            delivery. Books written before can be sent again from the list below.
          </p>
          <DeliverySection exported={exported} refresh={sends} />
        </div>
      </div>
      {open && (
        <ExportDialog
          open
          onOpenChange={setOpen}
          picks={picks}
          onExported={(results) => setExported(results.at(-1)?.output ?? null)}
          onSent={() => setSends((n) => n + 1)}
        />
      )}
    </section>
  );
}
