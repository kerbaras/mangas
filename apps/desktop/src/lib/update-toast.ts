/** The toast closing a manual update check. */
import { toast } from "sonner";
import { summaryLine, type UpdateRun } from "./ipc.updates";
import { connectThen } from "./notify";

const ID = "update-check";

export function toastRun(run: UpdateRun) {
  const title = summaryLine(run.summary);
  const queued = run.queued > 0 ? ` · ${run.queued} queued for download` : "";
  if (run.failures.length === 0) {
    const show = run.summary.chapters > 0 ? toast.success : toast.info;
    const titles = run.updates.map((u) => u.title).join(", ");
    show(title, { id: ID, description: titles ? `${titles}${queued}` : undefined });
    return;
  }
  const providers = [...new Set(run.failures.map((f) => f.source))];
  const challenged = run.failures.find((f) => f.challenge);
  toast.warning(title, {
    id: ID,
    duration: 10_000,
    description: `Not checked: ${run.failures.map((f) => f.title).join(", ")} (${providers.join(", ")})`,
    action: challenged
      ? { label: "Connect", onClick: () => void connectThen(challenged.source) }
      : undefined,
  });
}
