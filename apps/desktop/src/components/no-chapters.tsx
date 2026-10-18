import { BookOpen } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { languageList } from "@/lib/format";
import { DEFAULT_LANGUAGE } from "@/lib/ipc";
import { languagesFor } from "@/lib/provider-prefs";
import { useSettings } from "@/lib/settings";

/** A source with no chapters in the languages it was asked for (its own pick or the app's). */
export function NoChapters({ source }: { source: string }) {
  const [settings] = useSettings();
  const wanted = languagesFor(settings, source);
  const names = languageList(wanted.length ? wanted : [DEFAULT_LANGUAGE]);
  return (
    <EmptyState icon={<BookOpen />} title={`No chapters in ${names}`}>
      {source} lists no chapters in {names} for this title. Pick other chapter languages for this
      source, or try the same title on another one.
    </EmptyState>
  );
}
