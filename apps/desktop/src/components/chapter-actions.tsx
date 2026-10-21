import { Download, FolderOpen, RotateCcw } from "lucide-react";
import { ExportButton } from "@/components/export-button";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/use-t";
import { chapterFolder } from "@/lib/chapter-folder";
import {
  exportable,
  exportTargets,
  isUnavailable,
  pickSelected,
  type Selection,
} from "@/lib/chapters";
import { revealFolder } from "@/lib/dialogs";
import { type Chapter, isLocal, type Manga } from "@/lib/ipc";

type Props = {
  manga: Manga;
  /** The chapters of the chosen group; `visible` are the ones on screen. */
  pool: Chapter[];
  visible: Chapter[];
  selected: Selection;
  onExported: () => void;
  download: (chapters: Chapter[], force?: boolean) => Promise<void>;
};

/**
 * Details' toolbar actions. Imported titles have no source to download from, so instead of
 * "Download again" / "Download selected" they get "Open folder" (the one selected chapter's
 * folder, else the title's).
 */
export function ChapterActions({ manga, pool, visible, selected, onExported, download }: Props) {
  const t = useT();
  const toExport = exportable(pool, selected);
  const toDownload = pickSelected(pool, selected).filter((c) => !isUnavailable(c));
  const exportButton = (
    <ExportButton
      manga={manga}
      chapters={exportTargets(pool, visible, selected)}
      onExported={onExported}
    />
  );
  if (isLocal(manga)) {
    const folder = chapterFolder(toExport.length === 1 ? toExport : pool, toExport.length !== 1);
    return (
      <div className="ml-auto flex gap-2">
        {exportButton}
        <Button size="sm" disabled={!folder} onClick={() => folder && void revealFolder(folder)}>
          <FolderOpen /> {t("details.openFolder")}
        </Button>
      </div>
    );
  }
  return (
    <div className="ml-auto flex gap-2">
      {exportButton}
      {toExport.length > 0 && (
        <Button size="sm" onClick={() => void download(toExport, true)}>
          <RotateCcw /> {t("details.downloadAgain")}
        </Button>
      )}
      <Button
        size="sm"
        variant="primary"
        disabled={toDownload.length === 0}
        onClick={() => void download(toDownload)}
      >
        <Download /> {t("details.downloadSelected")}
      </Button>
    </div>
  );
}
