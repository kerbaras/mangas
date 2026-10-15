import { Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { listLibrary, type Manga } from "@/lib/ipc";

type State =
  | { kind: "loading" }
  | { kind: "ready"; items: Manga[] }
  | { kind: "error"; message: string };

export function LibraryScreen() {
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    listLibrary()
      .then((items) => setState({ kind: "ready", items }))
      .catch((e: unknown) => setState({ kind: "error", message: String(e) }));
  }, []);

  return (
    <section className="flex h-full flex-col">
      <header className="flex h-12 items-center justify-between border-b border-stroke px-4">
        <h1 className="text-body-lg font-semibold">Library</h1>
        <Button variant="primary" size="sm" disabled>
          <Plus /> Follow a manga
        </Button>
      </header>
      <div className="flex-1 overflow-auto p-4">
        {state.kind === "loading" && <p className="text-ink-3">Loading…</p>}
        {state.kind === "error" && (
          <p className="text-accent">Couldn't open the library: {state.message}</p>
        )}
        {state.kind === "ready" && state.items.length === 0 && <EmptyLibrary />}
        {state.kind === "ready" && state.items.length > 0 && <Grid items={state.items} />}
      </div>
    </section>
  );
}

function EmptyLibrary() {
  return (
    <div className="mx-auto mt-24 max-w-sm text-center">
      <p className="text-subtitle font-semibold">Nothing on the shelf yet</p>
      <p className="mt-2 text-ink-2">
        Search a source and follow a title. Chapters you download show up here, ready to send to
        your Kindle.
      </p>
    </div>
  );
}

function Grid({ items }: { items: Manga[] }) {
  return (
    <ul className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-4">
      {items.map((m) => (
        <li key={`${m.source}:${m.id}`} className="group">
          <div className="aspect-[2/3] overflow-hidden rounded-md border border-stroke bg-surface-2">
            {m.cover_url && (
              <img src={m.cover_url} alt="" className="size-full object-cover" loading="lazy" />
            )}
          </div>
          <p className="mt-2 truncate text-body font-medium" title={m.title}>
            {m.title}
          </p>
          <p className="text-caption text-ink-3">{m.source}</p>
        </li>
      ))}
    </ul>
  );
}
