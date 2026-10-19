import { AlertCircle, Ban, Check, Clock, EyeOff, Pause, Send } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { AvailabilityCell } from "@/components/availability-cell";
import { Hint } from "@/components/hint";
import { Checkbox } from "@/components/ui/checkbox";
import { Progress } from "@/components/ui/progress";
import { sourceRef, splitGroup } from "@/lib/chapter-groups";
import { isUnavailable } from "@/lib/chapters";
import { cn } from "@/lib/cn";
import { describeFailure } from "@/lib/errors";
import { languageLabel } from "@/lib/format";
import type { Chapter, DownloadProgress } from "@/lib/ipc";
import { isUnlisted } from "@/lib/ipc.downloads";

type Props = {
  chapter: Chapter;
  selected: boolean;
  /** Another row has the same number, language and group: show the source's id to tell them apart. */
  showSourceId?: boolean;
  progress: DownloadProgress | undefined;
  onToggle: (id: string, shift: boolean) => void;
};

/** The state label; `hint` (an error or retry reason) shows in an edge-aware tooltip. */
function StateLabel(props: { hint?: string; className: string; children: ReactNode }) {
  return (
    <Hint hint={props.hint}>
      <span className={cn("flex w-fit items-center gap-1 text-caption", props.className)}>
        {props.children}
      </span>
    </Hint>
  );
}

function Waiting({ icon, label, hint }: { icon: ReactNode; label: string; hint?: string }) {
  return (
    <StateLabel hint={hint} className="text-ink-3">
      {icon} {label}
    </StateLabel>
  );
}

function StateCell({ chapter, progress }: Pick<Props, "chapter" | "progress">) {
  // Kept only for its download job; cleared from the Downloads screen.
  if (progress && !chapter.local_path && isUnlisted({ error: progress.error ?? progress.retrying }))
    return (
      <Waiting
        icon={<EyeOff className="size-3.5" />}
        label="No longer listed"
        hint="The source stopped listing this chapter. Clear it from Downloads."
      />
    );
  if (progress?.state === "queued") {
    const label = progress.retrying ? "Retrying" : "Queued";
    const hint = progress.retrying ?? undefined;
    return <Waiting icon={<Clock className="size-3.5" />} label={label} hint={hint} />;
  }
  if (progress?.state === "paused") {
    return <Waiting icon={<Pause className="size-3.5" />} label="Paused" />;
  }
  if (progress?.state === "running") {
    return (
      <div className="flex items-center gap-2">
        <Progress value={progress.page} max={progress.pages} className="w-24" />
        <span className="text-caption text-ink-3 tabular-nums">
          {progress.pages > 0 ? `${progress.page}/${progress.pages}` : "Starting"}
        </span>
      </div>
    );
  }
  if (progress?.error) {
    return (
      <StateLabel hint={describeFailure(progress.error).tooltip} className="text-accent">
        <AlertCircle className="size-3.5" /> Failed
      </StateLabel>
    );
  }
  if (progress?.state === "cancelled" && !chapter.local_path) {
    return <Waiting icon={<Ban className="size-3.5" />} label="Cancelled" />;
  }
  if (chapter.availability && isUnavailable(chapter)) {
    return <AvailabilityCell availability={chapter.availability} />;
  }
  if (chapter.local_path && chapter.exported_at) {
    return (
      <StateLabel
        hint={`Exported ${new Date(chapter.exported_at).toLocaleString()}`}
        className="text-ok"
      >
        <Send className="size-3.5" /> Exported
      </StateLabel>
    );
  }
  if (chapter.local_path || progress?.state === "done") {
    return (
      <span className="flex items-center gap-1 text-caption text-ink-2">
        <Check className="size-3.5" /> On disk
      </span>
    );
  }
  return <span className="text-caption text-ink-3">—</span>;
}

/** Clips the middle of a long name: the start ellipsizes, the end always shows. */
function GroupName({ name }: { name: string }) {
  const [head, tail] = splitGroup(name);
  return (
    <>
      <span className="min-w-0 truncate">{head}</span>
      {tail && <span className="shrink-0 whitespace-pre">{tail}</span>}
    </>
  );
}

export function ChapterRow({ chapter, selected, showSourceId, progress, onToggle }: Props) {
  const off = isUnavailable(chapter);
  const handle = (e: MouseEvent) => {
    if (!off) onToggle(chapter.id, e.shiftKey);
  };
  return (
    <tr
      aria-selected={selected}
      aria-disabled={off || undefined}
      data-nav-item
      tabIndex={-1}
      onClick={handle}
      className={cn(
        "h-9 border-b border-stroke/60 text-body transition-colors outline-none focus-visible:bg-surface-2 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent",
        off ? "text-ink-3" : "hover:bg-surface-2/60",
        selected && "bg-accent-soft/60 hover:bg-accent-soft",
      )}
    >
      <td className="w-10 pl-4">
        <Checkbox
          checked={selected && !off}
          disabled={off}
          aria-label={`Select chapter ${chapter.number}`}
          onClick={(e) => e.stopPropagation()}
          onChange={() => onToggle(chapter.id, false)}
        />
      </td>
      <td className="w-16 text-ink-2 tabular-nums">{chapter.volume ?? "—"}</td>
      <td className="w-16 font-medium tabular-nums">{chapter.number}</td>
      <td className={cn("truncate pr-4", off ? "text-ink-3" : "text-ink")}>
        {chapter.title ?? <span className="text-ink-3">Untitled</span>}
      </td>
      <td className="w-40 max-w-40 pr-4 text-ink-2">
        <Hint hint={chapter.group} focusable={false} truncated>
          <div className="flex min-w-0 items-baseline overflow-hidden">
            {chapter.group ? (
              <GroupName name={chapter.group} />
            ) : (
              <span className="text-ink-3">—</span>
            )}
            {showSourceId && (
              <span className="ml-1 shrink-0 text-caption text-ink-3">{sourceRef(chapter.id)}</span>
            )}
          </div>
        </Hint>
      </td>
      <td className="w-28 text-ink-2">{languageLabel(chapter.language)}</td>
      <td className="w-44 pr-4">
        <StateCell chapter={chapter} progress={progress} />
      </td>
    </tr>
  );
}
