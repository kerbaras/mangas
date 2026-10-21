import { FileArchive, FolderInput, Import } from "lucide-react";
import { HeaderAction } from "@/components/header-action";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ImportKind } from "@/lib/dialogs";

type Props = { busy: boolean; onPick: (kind: ImportKind) => void };

/** Library header Import menu. While importing it reads "Importing…" with a still icon: an
 * endless spin keeps WebKitGTK repainting the window without a GPU for the whole import. */
export function ImportMenu({ busy, onPick }: Props) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <HeaderAction
          icon={<Import />}
          label={busy ? "Importing…" : "Import"}
          keepLabel={busy}
          disabled={busy}
          aria-busy={busy}
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
