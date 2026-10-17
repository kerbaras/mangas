import { FileArchive, FolderInput, Import, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ImportKind } from "@/lib/dialogs";

type Props = { busy: boolean; onPick: (kind: ImportKind) => void };

export function ImportMenu({ busy, onPick }: Props) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button size="sm" disabled={busy}>
          {busy ? <Loader2 className="animate-spin" /> : <Import />} Import
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => onPick("folder")}>
          <FolderInput /> Series folder…
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onPick("cbz")}>
          <FileArchive /> CBZ files…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
