import { Gauge, Layers, Scissors } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Levers } from "@/lib/ipc.volumes";

type Props = {
  levers: Levers;
  splitMb: number | null;
  onSplit: (mb: number) => void;
  onQuality: (q: number) => void;
  onEvery?: (chapters: number) => void;
};

/** The explicit ways to bring an over-budget book under its budget (`kindle::Levers`). */
export function SizeLevers({ levers, splitMb, onSplit, onQuality, onEvery }: Props) {
  const partMb = Math.floor(levers.split_bytes / 1_000_000);
  const { chapters_per_book: chapters, jpeg_quality: quality } = levers;
  return (
    <div className="flex flex-wrap gap-2">
      {chapters !== null && onEvery && (
        <Button size="sm" onClick={() => onEvery(chapters)}>
          <Layers /> {chapters} {chapters === 1 ? "chapter" : "chapters"} per book
        </Button>
      )}
      {splitMb === null && (
        <Button size="sm" onClick={() => onSplit(partMb)}>
          <Scissors /> Split into {partMb} MB parts
        </Button>
      )}
      {quality !== null ? (
        <Button size="sm" onClick={() => onQuality(quality)}>
          <Gauge /> JPEG quality {quality}
        </Button>
      ) : (
        <p className="text-caption text-ink-3">No JPEG quality down to 50 fits.</p>
      )}
    </div>
  );
}
