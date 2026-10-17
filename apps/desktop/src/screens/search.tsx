import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { ChallengePrompt } from "@/components/challenge-prompt";
import { EmptyState } from "@/components/empty-state";
import { RetryButton } from "@/components/error-alert";
import { ProviderSelect } from "@/components/provider-select";
import { ScreenHeader } from "@/components/screen-header";
import { SearchResult } from "@/components/search-result";
import { Input } from "@/components/ui/input";
import { useDebounced } from "@/hooks/use-debounced";
import { useLibrary } from "@/hooks/use-library";
import { errorMessage, mangaKey } from "@/lib/format";
import { DEFAULT_SOURCE, isChallenge, type Manga, searchManga } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";

const SOURCE_KEY = "mangas.search.source";
const SEARCH_TOAST = "search";
const readSource = () => window.localStorage.getItem(SOURCE_KEY) || DEFAULT_SOURCE;

type Results =
  | { kind: "idle" }
  | { kind: "searching" }
  | { kind: "ready"; items: Manga[] }
  | { kind: "error"; message: string };

export function SearchScreen() {
  const nav = useNavigation();
  const { state: library, follow } = useLibrary();
  const [query, setQuery] = useState("");
  const [source, setSource] = useState(readSource);
  const [attempt, setAttempt] = useState(0);
  const debounced = useDebounced(query, 300);
  const retry = () => setAttempt((a) => a + 1);
  const [results, setResults] = useState<Results>({ kind: "idle" });
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => () => void toast.dismiss(SEARCH_TOAST), []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt reruns the search after connecting a provider
  useEffect(() => {
    const q = debounced.trim();
    if (!q) {
      setResults({ kind: "idle" });
      return;
    }
    let alive = true;
    setResults({ kind: "searching" });
    searchManga(q, source)
      .then((items) => {
        if (!alive) return;
        toast.dismiss(SEARCH_TOAST);
        setResults({ kind: "ready", items });
      })
      .catch((e: unknown) => {
        if (!alive) return;
        const message = errorMessage(e);
        setResults({ kind: "error", message });
        // Challenges get the inline Connect prompt; a toast on top would only repeat it.
        if (isChallenge(message)) toast.dismiss(SEARCH_TOAST);
        else
          notifyError("Search failed", e, {
            retry: () => setAttempt((n) => n + 1),
            id: SEARCH_TOAST,
          });
      });
    return () => {
      alive = false;
    };
  }, [debounced, source, attempt]);

  const pickSource = (id: string) => {
    window.localStorage.setItem(SOURCE_KEY, id);
    setSource(id);
  };

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

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Search">
        <ProviderSelect value={source} onChange={pickSource} />
      </ScreenHeader>
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
          {results.kind === "searching" && <p className="text-ink-3">Searching {source}…</p>}
          {results.kind === "error" && isChallenge(results.message) && (
            <ChallengePrompt source={source} onConnected={retry} />
          )}
          {results.kind === "error" && !isChallenge(results.message) && (
            <EmptyState title="Search didn't finish" action={<RetryButton onClick={retry} />}>
              Try again, or pick another source.
            </EmptyState>
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
