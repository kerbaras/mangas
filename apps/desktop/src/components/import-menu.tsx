import { FileArchive, FolderInput, Import, Loader2 } from "lucide-react";
import { HeaderAction } from "@/components/header-action";
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
        <HeaderAction
          icon={busy ? <Loader2 className="animate-spin" /> : <Import />}
          label="Import"
          disabled={busy}
        />
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
