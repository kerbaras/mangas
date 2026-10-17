import { ClipboardCopy, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAsync } from "@/hooks/use-async";
import { copyText } from "@/lib/clipboard";
import { diagnostics, openLogs } from "@/lib/ipc.diagnostics";
import { notifyError } from "@/lib/notify";

/** Settings → Diagnostics: where the logs are, and a report to paste into a bug report. */
export function DiagnosticsPanel() {
  const info = useAsync(diagnostics, []);
  const ready = info.state.kind === "ready" ? info.state.data : null;

  const openFolder = async () => {
    try {
      await openLogs();
    } catch (e: unknown) {
      notifyError("Couldn't open the logs folder", e);
    }
  };
  const copy = async () => {
    try {
      await copyText((await diagnostics()).report);
      toast.success("Diagnostics copied", { description: "Paste them into your bug report." });
    } catch (e: unknown) {
      notifyError("Couldn't copy diagnostics", e);
    }
  };

  return (
    <>
      <Field
        label="Logs"
        htmlFor="log-dir"
        hint="Activity from the last 5 days. Start the app with MANGAS_LOG=debug for more detail."
      >
        <div className="flex gap-2">
          <Input
            id="log-dir"
            readOnly
            value={ready?.log_dir ?? ""}
            placeholder={info.state.kind === "error" ? info.state.message : "Loading…"}
            className="font-mono text-caption"
          />
          <Button onClick={openFolder} className="shrink-0">
            <FolderOpen /> Open logs folder
          </Button>
        </div>
      </Field>
      <Field
        label="Report"
        hint="App version, system, folders and provider status. Nothing is sent anywhere."
      >
        <div className="flex flex-col gap-2">
          {ready && (
            <pre className="overflow-auto rounded-md border border-stroke bg-surface p-3 font-mono text-caption whitespace-pre-wrap text-ink-2">
              {ready.report}
            </pre>
          )}
          <Button onClick={copy} className="self-start">
            <ClipboardCopy /> Copy diagnostics
          </Button>
        </div>
      </Field>
    </>
  );
}
