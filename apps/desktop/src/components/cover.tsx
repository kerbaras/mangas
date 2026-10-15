import { cn } from "@/lib/cn";
import type { Manga } from "@/lib/ipc";

type Props = { manga: Manga; className?: string };

/** 2:3 cover. Without an image, the title stands in, set like a spine. */
export function Cover({ manga, className }: Props) {
  return (
    <div
      className={cn(
        "aspect-[2/3] overflow-hidden rounded-sm border border-stroke bg-surface-2 shadow-[inset_1px_0_0_rgba(0,0,0,0.08)]",
        className,
      )}
    >
      {manga.cover_url ? (
        <img src={manga.cover_url} alt="" className="size-full object-cover" loading="lazy" />
      ) : (
        <p className="p-2 text-caption font-medium text-ink-3 [writing-mode:vertical-rl]">
          {manga.title}
        </p>
      )}
    </div>
  );
}
