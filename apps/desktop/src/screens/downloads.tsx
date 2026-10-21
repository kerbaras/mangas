import { Download, Pause, Play, Trash2 } from "lucide-react";
import { useMemo } from "react";
import { DownloadSection } from "@/components/download-section";
import { EmptyState } from "@/components/empty-state";
import { QueueBanner } from "@/components/queue-banner";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { useQueue } from "@/hooks/use-downloads";
import { groupJobs, SECTIONS, summary } from "@/lib/download-sections";
import { clearDownloads, pauseDownload, resumeDownload } from "@/lib/ipc.downloads";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";

const attempt = (title: string, run: () => Promise<unknown>) => () =>
  void run().catch((e: unknown) => notifyError(title, e));

const pauseAll = attempt("Couldn't pause the queue", () => pauseDownload(null));
const resumeAll = attempt("Couldn't resume the queue", () => resumeDownload(null));
const clearCompleted = attempt("Couldn't clear completed downloads", clearDownloads);

/** Finished jobs pile up (500+): a page of them keeps a completion from relaying every row. */
const COMPLETED_PAGE = 50;

export function DownloadsScreen() {
  const nav = useNavigation();
  const { jobs, gate, samples } = useQueue();
  const groups = useMemo(() => groupJobs(jobs.values()), [jobs]);
  const empty = jobs.size === 0;

  const clear = (
    <Button variant="ghost" size="sm" onClick={clearCompleted}>
      <Trash2 /> Clear completed
    </Button>
  );

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Downloads" subtitle={summary(groups)}>
        {gate.paused ? (
          <Button variant="primary" size="sm" onClick={resumeAll}>
            <Play /> Resume all
          </Button>
        ) : (
          <Button size="sm" onClick={pauseAll} disabled={empty}>
            <Pause /> Pause all
          </Button>
        )}
      </ScreenHeader>
      <QueueBanner gate={gate} onResume={resumeAll} />
      <div className="relative min-h-0 flex-1 overflow-auto overscroll-contain pt-4">
        {empty ? (
          <EmptyState
            title="No downloads"
            icon={<Download />}
            action={<Button onClick={() => nav.go({ screen: "library" })}>Go to library</Button>}
          >
            Chapters you download, here or from the command line, show up in this list.
          </EmptyState>
        ) : (
          SECTIONS.map(({ key, title }) => (
            <DownloadSection
              key={key}
              title={title}
              jobs={groups[key]}
              samples={samples}
              action={key === "completed" ? clear : undefined}
              pageSize={key === "completed" ? COMPLETED_PAGE : undefined}
            />
          ))
        )}
      </div>
    </section>
  );
}
