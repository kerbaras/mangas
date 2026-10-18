import { AlertCircle, Check } from "lucide-react";
import type { MouseEvent } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/cn";
import { languageLabel } from "@/lib/format";
import type { Chapter, DownloadProgress } from "@/lib/ipc";

type Props = {
  chapter: Chapter;
  selected: boolean;
  progress: DownloadProgress | undefined;
  onToggle: (id: string, shift: boolean) => void;
};

function DownloadedCell({ chapter, progress }: Pick<Props, "chapter" | "progress">) {
  if (progress && !progress.done) {
    return (
      <div className="flex items-center gap-2">
        <Progress value={progress.page} max={progress.pages} className="w-24" />
        <span className="text-caption text-ink-3 tabular-nums">
          {progress.page}/{progress.pages}
        </span>
      </div>
    );
  }
  if (progress?.error) {
    return (
      <span className="flex items-center gap-1 text-caption text-accent" title={progress.error}>
        <AlertCircle className="size-3.5" /> Failed
      </span>
    );
  }
  if (chapter.local_path || progress?.done) {
    return (
      <span className="flex items-center gap-1 text-caption text-ink-2">
        <Check className="size-3.5" /> On disk
      </span>
    );
  }
  return <span className="text-caption text-ink-3">—</span>;
}

export function ChapterRow({ chapter, selected, progress, onToggle }: Props) {
  const handle = (e: MouseEvent) => onToggle(chapter.id, e.shiftKey);
  return (
    <tr
      aria-selected={selected}
      onClick={handle}
      className={cn(
        "h-9 border-b border-stroke/60 text-body transition-colors hover:bg-surface-2/60",
        selected && "bg-accent-soft/60 hover:bg-accent-soft",
      )}
    >
      <td className="w-10 pl-4">
        <Checkbox
          checked={selected}
          aria-label={`Select chapter ${chapter.number}`}
          onClick={(e) => e.stopPropagation()}
          onChange={() => onToggle(chapter.id, false)}
        />
      </td>
      <td className="w-16 text-ink-2 tabular-nums">{chapter.volume ?? "—"}</td>
      <td className="w-16 font-medium tabular-nums">{chapter.number}</td>
      <td className="truncate pr-4 text-ink">
        {chapter.title ?? <span className="text-ink-3">Untitled</span>}
      </td>
      <td className="w-40 truncate pr-4 text-ink-2" title={chapter.group ?? undefined}>
        {chapter.group ?? <span className="text-ink-3">—</span>}
      </td>
      <td className="w-28 text-ink-2">{languageLabel(chapter.language)}</td>
      <td className="w-44 pr-4">
        <DownloadedCell chapter={chapter} progress={progress} />
      </td>
    </tr>
  );
}
