import { Hint } from "@/components/hint";
import { Badge } from "@/components/ui/badge";
import { useT } from "@/i18n/use-t";
import type { Manga } from "@/lib/ipc";
import { creditsLine, visibleTags } from "@/lib/manga-meta";

const MAX_TAGS = 8;

/** Credits, year and tags under the details title; renders nothing the source didn't give. */
export function MangaMeta({ manga }: { manga: Manga }) {
  const t = useT();
  const credits = creditsLine(manga);
  const { shown, hidden } = visibleTags(manga, MAX_TAGS);
  if (!credits && shown.length === 0) return null;
  return (
    <div className="mt-1 flex max-w-2xl flex-col gap-2">
      {credits && <p className="truncate text-caption text-ink-2">{credits}</p>}
      {shown.length > 0 && (
        <ul aria-label={t("details.tags")} className="flex flex-wrap gap-1">
          {shown.map((t) => (
            <li key={t}>
              <Badge variant="outline">{t}</Badge>
            </li>
          ))}
          {hidden > 0 && (
            <li>
              <Hint hint={manga.tags?.slice(MAX_TAGS).join(", ")}>
                <Badge variant="outline">+{hidden}</Badge>
              </Hint>
            </li>
          )}
        </ul>
      )}
    </div>
  );
}
