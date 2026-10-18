import { Send } from "lucide-react";
import { useState } from "react";
import { ExportDialog } from "@/components/export-dialog";
import { Button } from "@/components/ui/button";
import type { Chapter, Manga } from "@/lib/ipc";

type Props = { manga: Manga; chapters: Chapter[]; onExported?: () => void };

/** The one "Export" in Details: opens the export dialog on the given downloaded chapters. */
export function ExportButton({ manga, chapters, onExported }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button size="sm" disabled={chapters.length === 0} onClick={() => setOpen(true)}>
        <Send /> Export
      </Button>
      {open && (
        <ExportDialog
          open
          onOpenChange={setOpen}
          picks={[{ manga, chapters }]}
          onExported={() => onExported?.()}
        />
      )}
    </>
  );
}
