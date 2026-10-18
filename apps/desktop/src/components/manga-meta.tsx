import { Badge } from "@/components/ui/badge";
import type { Manga } from "@/lib/ipc";
import { creditsLine, visibleTags } from "@/lib/manga-meta";

const MAX_TAGS = 8;

/** Credits, year and tags under the details title; renders nothing the source didn't give. */
export function MangaMeta({ manga }: { manga: Manga }) {
  const credits = creditsLine(manga);
  const { shown, hidden } = visibleTags(manga, MAX_TAGS);
  if (!credits && shown.length === 0) return null;
  return (
    <div className="mt-1 flex max-w-2xl flex-col gap-2">
      {credits && <p className="truncate text-caption text-ink-2">{credits}</p>}
      {shown.length > 0 && (
        <ul aria-label="Tags" className="flex flex-wrap gap-1">
          {shown.map((t) => (
            <li key={t}>
              <Badge variant="outline">{t}</Badge>
            </li>
          ))}
          {hidden > 0 && (
            <li>
              <Badge variant="outline" title={manga.tags?.slice(MAX_TAGS).join(", ")}>
                +{hidden}
              </Badge>
            </li>
          )}
        </ul>
      )}
    </div>
  );
}
