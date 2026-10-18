import { BookOpen, Download, Send } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { ChallengePrompt } from "@/components/challenge-prompt";
import { ChaptersTable } from "@/components/chapters-table";
import { Cover } from "@/components/cover";
import { EmptyState } from "@/components/empty-state";
import { ErrorAlert } from "@/components/error-alert";
import { MangaMeta } from "@/components/manga-meta";
import { ScreenHeader } from "@/components/screen-header";
import { StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsync } from "@/hooks/use-async";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { useDownloads } from "@/hooks/use-downloads";
import { useMangaDetails } from "@/hooks/use-manga-details";
import {
  downloadable,
  exportable,
  filterByLanguage,
  languagesOf,
  selectableIds,
  selectionSummary,
  sortChapters,
} from "@/lib/chapters";
import { rememberLabels } from "@/lib/download-labels";
import { languageLabel } from "@/lib/format";
import { DEFAULT_LANGUAGE, downloadChapters, isChallenge, type Manga } from "@/lib/ipc";
import { loadChapters } from "@/lib/load-chapters";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";

type Props = { manga: Manga };

export function MangaDetailsScreen({ manga: card }: Props) {
  const { source, id } = card;
  const { manga, loading } = useMangaDetails(card);
  const nav = useNavigation();
  const refreshToast = `refresh-${source}:${id}`;
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      toast.dismiss(refreshToast);
    };
  }, [refreshToast]);
  const chapters = useAsync(
    () =>
      loadChapters(source, id, (e) => {
        if (!mounted.current) return;
        notifyError("Couldn't refresh chapters; showing the saved list", e, {
          source,
          retry: chapters.reload,
          id: refreshToast,
        });
      }),
    [source, id],
  );
  const downloads = useDownloads();
  const [language, setLanguage] = useState<string | null>(null);

  const all = chapters.state.kind === "ready" ? chapters.state.data : [];
  const landed = all.filter((c) => {
    const p = downloads.get(c.id);
    return !c.local_path && p?.done && !p.error;
  }).length;
  // A finished download changes `local_path` on disk; refetch so export/download buttons follow.
  useEffect(() => {
    if (landed > 0) chapters.reload();
  }, [landed, chapters.reload]);
  const languages = useMemo(() => languagesOf(all), [all]);
  const visible = useMemo(() => sortChapters(filterByLanguage(all, language)), [all, language]);
  const visibleIds = useMemo(() => selectableIds(visible), [visible]);
  const { selected, onToggle, onToggleAll, clear } = useChapterSelection(visibleIds);

  const toDownload = downloadable(all, selected);
  const toExport = exportable(all, selected);

  const download = async () => {
    rememberLabels(toDownload.map((c) => [c.id, `Chapter ${c.number} of ${manga.title}`]));
    try {
      // Per-chapter failures, missing chapters included, arrive as progress errors.
      await downloadChapters(
        source,
        id,
        toDownload.map((c) => c.id),
      );
      clear();
    } catch (e: unknown) {
      notifyError("Download failed", e, { source, retry: download, id: `download-${source}` });
    }
  };

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader back title={manga.title} subtitle={source} />
      <div className="flex gap-5 border-b border-stroke px-4 py-4">
        <Cover manga={manga} className="w-24 shrink-0" />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h2 className="text-subtitle font-semibold">{manga.title}</h2>
            {!loading && <StatusBadge status={manga.status} />}
          </div>
          {!loading && <MangaMeta manga={manga} />}
          {loading ? (
            <DescriptionSkeleton />
          ) : (
            <p className="mt-2 line-clamp-3 max-w-2xl text-body text-ink-2">{manga.description}</p>
          )}
          <p className="mt-2 text-caption text-ink-3">
            {all.length} chapters · {all.filter((c) => c.local_path).length} on disk
          </p>
        </div>
      </div>
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
        <span className="text-caption text-ink-3">{selectionSummary(all, selected, visible)}</span>
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
        {chapters.state.kind === "error" &&
          (isChallenge(chapters.state.message) ? (
            <ChallengePrompt source={source} onConnected={chapters.reload} />
          ) : (
            <div className="p-4">
              <ErrorAlert
                title="Couldn't list chapters"
                message={chapters.state.message}
                onRetry={chapters.reload}
              />
            </div>
          ))}
        {chapters.state.kind === "ready" && all.length === 0 && (
          <EmptyState icon={<BookOpen />} title={`No ${languageLabel(DEFAULT_LANGUAGE)} chapters`}>
            {source} lists no chapters in {languageLabel(DEFAULT_LANGUAGE)} for this title. Other
            languages aren't fetched yet; try the same title on another source.
          </EmptyState>
        )}
        {chapters.state.kind === "ready" && all.length > 0 && (
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

function DescriptionSkeleton() {
  return (
    <div role="status" aria-label="Loading details" className="mt-2 flex max-w-2xl flex-col gap-2">
      <span className="h-3 w-[28rem] max-w-full animate-pulse rounded-sm bg-surface-2" />
      <span className="h-3 w-[22rem] max-w-full animate-pulse rounded-sm bg-surface-2" />
    </div>
  );
}
