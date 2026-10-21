import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { ChallengePrompt } from "@/components/challenge-prompt";
import { ChapterActions } from "@/components/chapter-actions";
import { ChaptersTable } from "@/components/chapters-table";
import { Cover } from "@/components/cover";
import { DescriptionSkeleton } from "@/components/description-skeleton";
import { ErrorAlert } from "@/components/error-alert";
import { GroupSelect } from "@/components/group-select";
import { LanguageTabs } from "@/components/language-tabs";
import { LibraryControls } from "@/components/library-controls";
import { MangaMeta } from "@/components/manga-meta";
import { NoChapters } from "@/components/no-chapters";
import { ScreenHeader } from "@/components/screen-header";
import { StatusBadge } from "@/components/status-badge";
import { useAsync } from "@/hooks/use-async";
import { useChapterDownload } from "@/hooks/use-chapter-download";
import { useChapterGroup } from "@/hooks/use-chapter-group";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { useDownloads } from "@/hooks/use-downloads";
import { useLibraryEntry } from "@/hooks/use-library-entry";
import { useMangaDetails } from "@/hooks/use-manga-details";
import { filterByGroup, groupsOf } from "@/lib/chapter-groups";
import {
  chapterSummary,
  filterByLanguage,
  languagesOf,
  selectableIds,
  selectionSummary,
  sortChapters,
} from "@/lib/chapters";
import { isChallenge, type Manga } from "@/lib/ipc";
import { loadChapters } from "@/lib/load-chapters";
import { notifyError } from "@/lib/notify";
import { useRefreshShortcut } from "@/lib/refresh-shortcut";
import { useSettingsKnown } from "@/lib/settings";

type Props = { manga: Manga };

export function MangaDetailsScreen({ manga: card }: Props) {
  const { source, id } = card;
  const { manga, loading, reload: reloadManga } = useMangaDetails(card);
  const known = useSettingsKnown();
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
    [source, id, known],
  );
  // Refreshing a followed title's chapters also saves its source record; show what that filled.
  const refreshed = chapters.state.kind === "ready" ? chapters.state.data : null;
  const shelf = useLibraryEntry(source, id);
  const reloadShelf = shelf.reload;
  useEffect(() => {
    if (refreshed) {
      reloadManga();
      reloadShelf();
    }
  }, [refreshed, reloadManga, reloadShelf]);
  useRefreshShortcut(chapters.reload);
  const downloads = useDownloads();
  const [language, setLanguage] = useState<string | null>(null);

  const all = chapters.state.kind === "ready" ? chapters.state.data : [];
  const landed = all.filter((c) => {
    const p = downloads.get(c.id);
    return !c.local_path && p?.state === "done";
  }).length;
  // A finished download changes `local_path` on disk; refetch so export/download buttons follow.
  useEffect(() => {
    if (landed > 0) chapters.reload();
  }, [landed, chapters.reload]);
  const groups = useMemo(() => groupsOf(all), [all]);
  const { group, setGroup } = useChapterGroup(shelf.entry, reloadShelf);
  // Other groups' rows are out of the picture: no selection, download or export reaches them.
  const pool = useMemo(() => filterByGroup(all, group), [all, group]);
  const languages = useMemo(() => languagesOf(pool), [pool]);
  // A tab the new group doesn't have would hide every row (and its tabs may be gone).
  const lang = language && languages.includes(language) ? language : null;
  const visible = useMemo(() => sortChapters(filterByLanguage(pool, lang)), [pool, lang]);
  const visibleIds = useMemo(() => selectableIds(visible), [visible]);
  const { selected, onToggle, onToggleAll, clear } = useChapterSelection(visibleIds);

  const pane = useRef<HTMLDivElement>(null);
  const download = useChapterDownload({ source, mangaId: id, title: manga.title }, clear);
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
          <p className="mt-2 text-caption text-ink-3">{chapterSummary(all)}</p>
          {shelf.entry && (
            <LibraryControls
              entry={shelf.entry}
              categories={shelf.categories}
              onStatus={shelf.setStatus}
              onToggle={shelf.toggle}
            />
          )}
        </div>
      </div>
      <div className="flex h-11 shrink-0 items-center gap-3 border-b border-stroke px-4">
        {(groups.length > 1 || group) && (
          <GroupSelect
            groups={groups}
            value={group}
            onChange={(g) => {
              clear();
              setLanguage(null);
              setGroup(g);
            }}
          />
        )}
        {languages.length > 1 && (
          <LanguageTabs languages={languages} value={lang} onChange={setLanguage} />
        )}
        <span className="text-caption text-ink-3">{selectionSummary(pool, selected, visible)}</span>
        <ChapterActions
          manga={manga}
          pool={pool}
          visible={visible}
          selected={selected}
          onExported={chapters.reload}
          download={download}
        />
      </div>
      <div ref={pane} className="relative min-h-0 flex-1 overflow-auto overscroll-contain">
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
        {chapters.state.kind === "ready" && all.length === 0 && <NoChapters source={source} />}
        {chapters.state.kind === "ready" && all.length > 0 && (
          <ChaptersTable
            chapters={visible}
            selected={selected}
            downloads={downloads}
            onToggle={onToggle}
            onToggleAll={onToggleAll}
            scrollRef={pane}
          />
        )}
      </div>
    </section>
  );
}
