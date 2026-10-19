import { useCover } from "@/hooks/use-cover";
import { cn } from "@/lib/cn";
import type { Manga } from "@/lib/ipc";

type Props = { manga: Manga; className?: string };

/**
 * 2:3 cover. The title, set like a spine, holds the box until the image has loaded and stays
 * when there is no image or it fails, so a cover never shows as broken or shifts the layout.
 * One column, ellipsised at the bottom: a long title stays inside the box instead of clipping.
 */
export function Cover({ manga, className }: Props) {
  const { src, loaded, onLoad, onError } = useCover(manga);
  return (
    <div
      className={cn(
        "relative aspect-[2/3] overflow-hidden rounded-sm border border-stroke bg-surface-2 shadow-[inset_1px_0_0_rgba(0,0,0,0.08)]",
        className,
      )}
    >
      {!loaded && (
        <p
          data-testid="cover-placeholder"
          className="absolute inset-0 truncate p-2 text-caption font-medium text-ink-3 [writing-mode:vertical-rl]"
        >
          {manga.title}
        </p>
      )}
      {src && (
        <img
          src={src}
          alt=""
          loading="lazy"
          onLoad={onLoad}
          onError={onError}
          className={cn(
            "absolute inset-0 size-full object-cover transition-opacity duration-200",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      )}
    </div>
  );
}
