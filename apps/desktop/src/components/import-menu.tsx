import { FileArchive, FolderInput, Import } from "lucide-react";
import { HeaderAction } from "@/components/header-action";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useT } from "@/i18n/use-t";
import type { ImportKind } from "@/lib/dialogs";

type Props = { busy: boolean; onPick: (kind: ImportKind) => void };

/** Library header Import menu. While importing it reads "Importing…" with a still icon: an
 * endless spin keeps WebKitGTK repainting the window without a GPU for the whole import. */
export function ImportMenu({ busy, onPick }: Props) {
  const t = useT();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <HeaderAction
          icon={<Import />}
          label={busy ? t("library.importing") : t("library.import")}
          keepLabel={busy}
          disabled={busy}
          aria-busy={busy}
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => onPick("folder")}>
          <FolderInput /> {t("library.importFolder")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onPick("cbz")}>
          <FileArchive /> {t("library.importCbz")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
