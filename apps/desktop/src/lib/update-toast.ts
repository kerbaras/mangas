/** The toast closing a manual update check. */
import { toast } from "sonner";
import { describeFailure } from "./errors";
import { summaryLine, type UpdateRun } from "./ipc.updates";
import { connectThen } from "./notify";

/** The update check's one toast: a result replaces an earlier one, failure included. */
export const UPDATE_TOAST = "update-check";

export function toastRun(run: UpdateRun) {
  const title = summaryLine(run.summary);
  const queued = run.queued > 0 ? ` · ${run.queued} queued for download` : "";
  if (run.failures.length === 0) {
    const show = run.summary.chapters > 0 ? toast.success : toast.info;
    const titles = run.updates.map((u) => u.title).join(", ");
    // Same id as a previous warning: reset the fields Sonner would otherwise keep.
    show(title, {
      id: UPDATE_TOAST,
      duration: 5_000,
      description: titles ? `${titles}${queued}` : null,
      action: null,
    });
    return;
  }
  const providers = [...new Set(run.failures.map((f) => f.source))];
  const challenged = run.failures.find((f) => f.challenge);
  toast.warning(title, {
    id: UPDATE_TOAST,
    duration: 10_000,
    description: [
      `Not checked: ${run.failures.map((f) => f.title).join(", ")} (${providers.join(", ")})`,
      ...reasons(run),
    ].join(". "),
    action: challenged
      ? { label: "Connect", onClick: () => void connectThen(challenged.source) }
      : null,
  });
}

/** Why the titles weren't checked: each distinct reason once, readable hint first. */
const reasons = (run: UpdateRun) => [
  ...new Set(run.failures.map((f) => describeFailure(f.error).label).filter(Boolean)),
];
