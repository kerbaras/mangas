import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { EmptyState } from "@/components/empty-state";
import { ScreenHeader } from "@/components/screen-header";
import { SearchResult } from "@/components/search-result";
import { Input } from "@/components/ui/input";
import { useDebounced } from "@/hooks/use-debounced";
import { useLibrary } from "@/hooks/use-library";
import { mangaKey } from "@/lib/format";
import { type Manga, searchManga } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";

const SOURCE = "mangadex";

type Results =
  | { kind: "idle" }
  | { kind: "searching" }
  | { kind: "ready"; items: Manga[] }
  | { kind: "error"; message: string };

export function SearchScreen() {
  const nav = useNavigation();
  const { state: library, follow } = useLibrary();
  const [query, setQuery] = useState("");
  const debounced = useDebounced(query, 300);
  const [results, setResults] = useState<Results>({ kind: "idle" });
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    const q = debounced.trim();
    if (!q) {
      setResults({ kind: "idle" });
      return;
    }
    let alive = true;
    setResults({ kind: "searching" });
    searchManga(q, SOURCE)
      .then((items) => alive && setResults({ kind: "ready", items }))
      .catch((e: unknown) => alive && setResults({ kind: "error", message: String(e) }));
    return () => {
      alive = false;
    };
  }, [debounced]);

  const followed = new Set(library.kind === "ready" ? library.data.map(mangaKey) : []);
  const onFollow = async (m: Manga) => {
    setBusy(mangaKey(m));
    try {
      await follow(m);
    } finally {
      setBusy(null);
    }
  };
  const open = (m: Manga) => nav.go({ screen: "manga", source: m.source, id: m.id });

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Search" subtitle={SOURCE} />
      <div className="border-b border-stroke px-4 py-3">
        <div className="relative mx-auto max-w-2xl">
          <Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-ink-3" />
          <Input
            type="search"
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Title, author or keyword"
            aria-label="Search manga"
            className="h-9 pl-9"
          />
        </div>
      </div>
      <div className="flex-1 overflow-auto px-4 py-3">
        <div className="mx-auto max-w-2xl">
          {results.kind === "idle" && (
            <EmptyState title="Search a source">
              Results show cover, title and status. Follow one to add it to your library.
            </EmptyState>
          )}
          {results.kind === "searching" && <p className="text-ink-3">Searching {SOURCE}…</p>}
          {results.kind === "error" && (
            <p className="text-accent">Search failed: {results.message}</p>
          )}
          {results.kind === "ready" && results.items.length === 0 && (
            <EmptyState title={`No results for “${debounced.trim()}”`}>
              Check the spelling or try the original title.
            </EmptyState>
          )}
          {results.kind === "ready" && results.items.length > 0 && (
            <ul className="flex flex-col gap-1">
              {results.items.map((m) => (
                <SearchResult
                  key={mangaKey(m)}
                  manga={m}
                  followed={followed.has(mangaKey(m))}
                  busy={busy === mangaKey(m)}
                  onFollow={onFollow}
                  onOpen={open}
                />
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}
