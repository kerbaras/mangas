/** The toast closing a manual update check. */
import { toast } from "sonner";
import { summaryLine, type UpdateRun } from "./ipc.updates";
import { connectThen } from "./notify";

const ID = "update-check";

export function toastRun(run: UpdateRun) {
  const title = summaryLine(run.summary);
  if (run.failures.length === 0) {
    const show = run.summary.chapters > 0 ? toast.success : toast.info;
    show(title, { id: ID, description: run.updates.map((u) => u.title).join(", ") || undefined });
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
