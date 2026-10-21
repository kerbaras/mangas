import { ListFilter } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useT } from "@/i18n/use-t";
import { cn } from "@/lib/cn";
import { READING_STATUSES, type ReadingStatus } from "@/lib/ipc.library";
import type { LibraryView } from "@/lib/library-view";

const ANY = "any";

type Props = {
  view: Pick<LibraryView, "source" | "status">;
  sources: string[];
  onChange: (patch: Partial<Pick<LibraryView, "source" | "status">>) => void;
};

/** Source and reading-status filters; the button reads "Filter (n)" while any is on. */
export function FilterMenu({ view, sources, onChange }: Props) {
  const t = useT();
  const active = (view.source ? 1 : 0) + (view.status ? 1 : 0);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className={cn(active > 0 && "text-accent-text")}>
          <ListFilter />{" "}
          {active > 0 ? t("library.filterActive", { count: active }) : t("library.filter")}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-48">
        <DropdownMenuLabel>{t("library.readingStatus")}</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={view.status ?? ANY}
          onValueChange={(v) => onChange({ status: v === ANY ? null : (v as ReadingStatus) })}
        >
          <DropdownMenuRadioItem value={ANY}>{t("library.anyStatus")}</DropdownMenuRadioItem>
          {READING_STATUSES.map((s) => (
            <DropdownMenuRadioItem key={s.id} value={s.id}>
              {s.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        {sources.length > 1 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuLabel>{t("library.source")}</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={view.source ?? ANY}
              onValueChange={(v) => onChange({ source: v === ANY ? null : v })}
            >
              <DropdownMenuRadioItem value={ANY}>{t("library.allSources")}</DropdownMenuRadioItem>
              {sources.map((s) => (
                <DropdownMenuRadioItem key={s} value={s}>
                  {s}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </>
        )}
        {active > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={() => onChange({ source: null, status: null })}>
              {t("library.clearFilters")}
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
