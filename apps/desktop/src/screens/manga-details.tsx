import { Download, Send } from "lucide-react";
import { useMemo, useState } from "react";
import { ChaptersTable } from "@/components/chapters-table";
import { Cover } from "@/components/cover";
import { ScreenHeader } from "@/components/screen-header";
import { StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsync } from "@/hooks/use-async";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { useDownloads } from "@/hooks/use-downloads";
import {
  downloadable,
  exportable,
  filterByLanguage,
  languagesOf,
  selectionSummary,
  sortChapters,
} from "@/lib/chapters";
import { languageLabel } from "@/lib/format";
import { downloadChapters, listChapters, listLibrary, searchManga } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";

type Props = { source: string; id: string };

async function loadManga(source: string, id: string) {
  const inLibrary = (await listLibrary()).find((m) => m.source === source && m.id === id);
  if (inLibrary) return inLibrary;
  return (await searchManga(id, source)).find((m) => m.id === id) ?? null;
}

export function MangaDetailsScreen({ source, id }: Props) {
  const nav = useNavigation();
  const manga = useAsync(() => loadManga(source, id), [source, id]);
  const chapters = useAsync(() => listChapters(source, id), [source, id]);
  const downloads = useDownloads();
  const [language, setLanguage] = useState<string | null>(null);

  const all = chapters.state.kind === "ready" ? chapters.state.data : [];
  const languages = useMemo(() => languagesOf(all), [all]);
  const visible = useMemo(() => sortChapters(filterByLanguage(all, language)), [all, language]);
  const visibleIds = useMemo(() => visible.map((c) => c.id), [visible]);
  const { selected, onToggle, onToggleAll, clear } = useChapterSelection(visibleIds);

  const toDownload = downloadable(all, selected);
  const toExport = exportable(all, selected);
  const title = manga.state.kind === "ready" ? manga.state.data?.title : "…";

  const download = async () => {
    await downloadChapters(
      source,
      toDownload.map((c) => c.id),
    );
    clear();
  };

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader back title={title} subtitle={source} />
      {manga.state.kind === "ready" && manga.state.data && (
        <div className="flex gap-5 border-b border-stroke px-4 py-4">
          <Cover manga={manga.state.data} className="w-24 shrink-0" />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h2 className="text-subtitle font-semibold">{manga.state.data.title}</h2>
              <StatusBadge status={manga.state.data.status} />
            </div>
            <p className="mt-2 line-clamp-3 max-w-2xl text-body text-ink-2">
              {manga.state.data.description}
            </p>
            <p className="mt-2 text-caption text-ink-3">
              {all.length} chapters · {all.filter((c) => c.local_path).length} on disk
            </p>
          </div>
        </div>
      )}
      <div className="flex h-11 shrink-0 items-center gap-3 border-b border-stroke px-4">
        {languages.length > 1 && (
          <Tabs
            value={language ?? "all"}
            onValueChange={(v) => setLanguage(v === "all" ? null : v)}
          >
            <TabsList>
              <TabsTrigger value="all">All</TabsTrigger>
              {languages.map((l) => (
                <TabsTrigger key={l} value={l}>
                  {languageLabel(l)}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        )}
        <span className="text-caption text-ink-3">{selectionSummary(all, selected)}</span>
        <div className="ml-auto flex gap-2">
          <Button
            size="sm"
            disabled={toExport.length === 0}
            onClick={() => nav.go({ screen: "kindle", chapterIds: toExport.map((c) => c.id) })}
          >
            <Send /> Send to Kindle
          </Button>
          <Button size="sm" variant="primary" disabled={toDownload.length === 0} onClick={download}>
            <Download /> Download selected
          </Button>
        </div>
      </div>
      <div className="flex-1 overflow-auto">
        {chapters.state.kind === "loading" && <p className="p-4 text-ink-3">Loading chapters…</p>}
        {chapters.state.kind === "error" && (
          <p className="p-4 text-accent">Couldn't list chapters: {chapters.state.message}</p>
        )}
        {chapters.state.kind === "ready" && (
          <ChaptersTable
            chapters={visible}
            selected={selected}
            downloads={downloads}
            onToggle={onToggle}
            onToggleAll={onToggleAll}
          />
        )}
      </div>
    </section>
  );
}
