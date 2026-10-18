import { BookOpen } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { VolumeExportDialog } from "@/components/volume-export-dialog";
import type { Chapter, Manga } from "@/lib/ipc";

type Props = { manga: Pick<Manga, "source" | "id" | "title">; chapters: Chapter[] };

/** "Export as volume" in Details: bundles the selected downloaded chapters into books. */
export function VolumeExportButton({ manga, chapters }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button size="sm" disabled={chapters.length === 0} onClick={() => setOpen(true)}>
        <BookOpen /> Export as volume
      </Button>
      {open && <VolumeExportDialog open onOpenChange={setOpen} manga={manga} chapters={chapters} />}
    </>
  );
}
