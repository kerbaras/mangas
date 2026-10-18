import { Tags } from "lucide-react";
import { FilterMenu } from "@/components/filter-menu";
import { LayoutToggle } from "@/components/layout-toggle";
import { SortMenu } from "@/components/sort-menu";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { Category, LibraryEntry } from "@/lib/ipc.library";
import { type LibraryView, sourcesOf } from "@/lib/library-view";

type Props = {
  view: LibraryView;
  entries: LibraryEntry[];
  categories: Category[];
  onChange: (patch: Partial<LibraryView>) => void;
  onEditCategories: () => void;
};

const ALL = "all";

/** Category tabs on the left; filters, sort and layout on the right. */
export function LibraryToolbar({ view, entries, categories, onChange, onEditCategories }: Props) {
  const count = (id: number) => entries.filter((e) => e.categories.includes(id)).length;
  return (
    <div className="flex h-11 shrink-0 items-center gap-2 border-b border-stroke px-4">
      <div className="min-w-0 overflow-x-auto">
        <Tabs
          value={view.category === null ? ALL : String(view.category)}
          onValueChange={(v) => onChange({ category: v === ALL ? null : Number(v) })}
        >
          <TabsList>
            <TabsTrigger value={ALL}>All</TabsTrigger>
            {categories.map((c) => (
              <TabsTrigger key={c.id} value={String(c.id)} className="gap-1.5">
                {c.name}
                <span className="text-caption text-ink-3 tabular-nums">{count(c.id)}</span>
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      </div>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Edit categories"
        title="Edit categories"
        onClick={onEditCategories}
      >
        <Tags />
      </Button>
      <div className="ml-auto flex items-center gap-1">
        <FilterMenu view={view} sources={sourcesOf(entries)} onChange={onChange} />
        <SortMenu value={view.sort} onChange={(sort) => onChange({ sort })} />
        <LayoutToggle value={view.layout} onChange={(layout) => onChange({ layout })} />
      </div>
    </div>
  );
}
