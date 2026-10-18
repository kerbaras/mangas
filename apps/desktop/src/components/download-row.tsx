import { FolderOpen, Pause, Play, RotateCcw, Trash2, X } from "lucide-react";
import { memo, type ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import {
  isMoving,
  type JobAction,
  jobActions,
  jobTitle,
  speedLabel,
  stateLabel,
} from "@/lib/download-sections";
import { languageLabel } from "@/lib/format";
import {
  cancelDownload,
  type DownloadJob,
  dismissDownload,
  isUnlisted,
  openDownloadFolder,
  pauseDownload,
  resumeDownload,
  retryDownload,
} from "@/lib/ipc.downloads";
import { notifyError } from "@/lib/notify";

type Spec = { label: string; icon: ReactNode; run: (id: number) => Promise<unknown> };

const ACTIONS: Record<JobAction, Spec> = {
  pause: { label: "Pause", icon: <Pause />, run: pauseDownload },
  resume: { label: "Resume", icon: <Play />, run: resumeDownload },
  cancel: { label: "Cancel", icon: <X />, run: cancelDownload },
  retry: { label: "Retry", icon: <RotateCcw />, run: retryDownload },
  dismiss: { label: "Clear", icon: <Trash2 />, run: dismissDownload },
  open: { label: "Open folder", icon: <FolderOpen />, run: openDownloadFolder },
};

function ActionButton({ job, action }: { job: DownloadJob; action: JobAction }) {
  const spec = ACTIONS[action];
  const run = () =>
    void spec
      .run(job.id)
      .catch((e: unknown) =>
        notifyError(`Couldn't ${spec.label.toLowerCase()} chapter ${job.number}`, e),
      );
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={spec.label} onClick={run}>
          {spec.icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{spec.label}</TooltipContent>
    </Tooltip>
  );
}

const TONE: Record<DownloadJob["state"], string> = {
  running: "text-ink",
  queued: "text-ink-3",
  paused: "text-ink-2",
  failed: "text-accent",
  cancelled: "text-ink-3",
  done: "text-ok",
};

function Detail({ job, speed }: { job: DownloadJob; speed: number | null }) {
  if (job.state === "running" || job.state === "paused") {
    const pages = job.pages_total > 0 ? `${job.pages_done}/${job.pages_total} pages` : null;
    const moving = isMoving(job);
    return (
      <div className="flex w-full items-center gap-3">
        <Progress
          value={job.pages_done}
          max={job.pages_total}
          animated={moving}
          className="flex-1"
        />
        <span className="w-24 shrink-0 text-right text-caption text-ink-3 tabular-nums">
          {pages}
        </span>
        <span className="w-20 shrink-0 text-right text-caption text-ink-3 tabular-nums">
          {moving ? speedLabel(speed) : null}
        </span>
      </div>
    );
  }
  const text = job.error && !isUnlisted(job) ? job.error : null;
  if (job.state === "done" && job.pages_total > 0)
    return <span className="text-caption text-ink-3">{job.pages_total} pages</span>;
  return text ? (
    <span title={text} className="truncate text-caption text-ink-2">
      {text}
    </span>
  ) : null;
}

type Props = { job: DownloadJob; speed: number | null };

/** One job. Memoised: a progress event re-renders only the row whose job changed. */
export const DownloadRow = memo(function DownloadRow({ job, speed }: Props) {
  const unlisted = isUnlisted(job);
  return (
    <li
      data-state={job.state}
      className="grid h-14 grid-cols-[minmax(0,2fr)_7rem_minmax(0,3fr)_auto] items-center gap-4 border-b border-stroke/60 px-4 [contain-intrinsic-size:auto_56px] [content-visibility:auto]"
    >
      <div className="min-w-0">
        <p title={jobTitle(job)} className="flex min-w-0 gap-1 text-body font-medium">
          {job.manga_title && <span className="truncate">{job.manga_title} ·</span>}
          <span className="shrink-0">Chapter {job.number}</span>
        </p>
        <p className="truncate text-caption text-ink-3">
          {[job.title, languageLabel(job.language), job.source].filter(Boolean).join(" · ")}
        </p>
      </div>
      <div className="flex flex-col items-start gap-0.5">
        <span className={cn("text-caption font-medium", TONE[job.state])}>{stateLabel(job)}</span>
        {unlisted && (
          <Badge variant="outline" title={job.error ?? undefined}>
            No longer listed
          </Badge>
        )}
      </div>
      <div className="flex min-w-0 items-center">
        <Detail job={job} speed={speed} />
      </div>
      <div className="flex justify-end gap-1">
        {jobActions(job).map((a) => (
          <ActionButton key={a} job={job} action={a} />
        ))}
      </div>
    </li>
  );
});
