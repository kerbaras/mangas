import { Search, Settings2 } from "lucide-react";
import { useState } from "react";
import { EmptyState } from "@/components/empty-state";
import { ScreenHeader } from "@/components/screen-header";
import { SearchResults } from "@/components/search-results";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAsync } from "@/hooks/use-async";
import { useDebounced } from "@/hooks/use-debounced";
import { useLibrary } from "@/hooks/use-library";
import { useMultiSearch } from "@/hooks/use-multi-search";
import { mangaKey } from "@/lib/format";
import { listProviders, type Manga } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";
import { searchable } from "@/lib/provider-prefs";
import { rememberInput, searchInput } from "@/lib/search-session";
import { useSettings } from "@/lib/settings";

const FILTER_KEY = "mangas.search.filter";
const readFilter = () => window.localStorage.getItem(FILTER_KEY) || "all";

export function SearchScreen() {
  const nav = useNavigation();
  const { state: library, follow } = useLibrary();
  const [settings] = useSettings();
  const listed = useAsync(listProviders, []);
  const providers = listed.state.kind === "ready" ? searchable(listed.state.data, settings) : [];
  const [query, setQuery] = useState(searchInput);
  const debounced = useDebounced(query, 300);
  const { groups, retry } = useMultiSearch(
    debounced,
    providers.map((p) => p.id),
  );
  const [stored, setStored] = useState(readFilter);
  const filter = providers.some((p) => p.id === stored) ? stored : "all";
  const pick = (id: string) => {
    window.localStorage.setItem(FILTER_KEY, id);
    setStored(id);
  };
  const [busy, setBusy] = useState<string | null>(null);

  const followed = new Set(library.kind === "ready" ? library.data.map(mangaKey) : []);
  const onFollow = async (m: Manga) => {
    setBusy(mangaKey(m));
    try {
      await follow(m);
    } catch (e: unknown) {
      notifyError(`Couldn't follow ${m.title}`, e, {
        source: m.source,
        retry: () => void onFollow(m),
      });
    } finally {
      setBusy(null);
    }
  };
  const open = (m: Manga) => nav.go({ screen: "manga", manga: m });
  const noneEnabled = listed.state.kind === "ready" && providers.length === 0;

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Search" />
      <div className="border-b border-stroke px-4 py-3">
        <div className="relative mx-auto max-w-2xl">
          <Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-ink-3" />
          <Input
            type="search"
            autoFocus
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              rememberInput(e.target.value);
            }}
            placeholder="Title, author or keyword"
            aria-label="Search manga"
            className="h-9 pl-9"
          />
        </div>
      </div>
      <div className="flex-1 overflow-auto px-4 py-3">
        <div className="mx-auto max-w-2xl">
          {noneEnabled && (
            <EmptyState
              title="No providers enabled"
              action={
                <Button onClick={() => nav.go({ screen: "settings" })}>
                  <Settings2 /> Open settings
                </Button>
              }
            >
              Turn on at least one provider in Settings to search.
            </EmptyState>
          )}
          {!noneEnabled && !debounced.trim() && (
            <EmptyState title="Search every enabled provider">
              Results are grouped by provider. Follow one to add it to your library.
            </EmptyState>
          )}
          {!noneEnabled && debounced.trim() && (
            <SearchResults
              providers={providers}
              groups={groups}
              filter={filter}
              onFilter={pick}
              onRetry={retry}
              followed={followed}
              busy={busy}
              onFollow={onFollow}
              onOpen={open}
            />
          )}
        </div>
      </div>
    </section>
  );
}
