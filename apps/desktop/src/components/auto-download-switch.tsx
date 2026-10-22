import { useState } from "react";
import { Switch } from "@/components/ui/switch";
import { useT } from "@/i18n/use-t";
import type { LibraryEntry } from "@/lib/ipc.library";
import { setAutoDownload } from "@/lib/ipc.updates";
import { notifyError } from "@/lib/notify";

/** Details option: update checks queue this title's new chapters for download. */
export function AutoDownloadSwitch({ entry }: { entry: LibraryEntry }) {
  const t = useT();
  const [on, setOn] = useState(entry.auto_download);
  const [saving, setSaving] = useState(false);
  // One write at a time, so a late reply can't overwrite a newer choice.
  const change = async (next: boolean) => {
    setOn(next);
    setSaving(true);
    try {
      await setAutoDownload(entry.source, entry.id, next);
    } catch (e: unknown) {
      setOn(!next);
      notifyError(t("details.autoDownloadFailed"), e);
    } finally {
      setSaving(false);
    }
  };
  const id = `auto-download-${entry.source}-${entry.id}`;
  return (
    <label htmlFor={id} className="flex items-center gap-2 text-body text-ink-2">
      <Switch id={id} checked={on} disabled={saving} onCheckedChange={change} />
      {t("details.autoDownload")}
    </label>
  );
}
