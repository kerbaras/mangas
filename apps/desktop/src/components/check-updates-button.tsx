import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { notifyError } from "@/lib/notify";
import { toastRun } from "@/lib/update-toast";
import { runCheck, useUpdates } from "@/lib/updates";

/** Library header action: checks every followed title now, spinning while any check runs. */
export function CheckUpdatesButton() {
  const { checking } = useUpdates();
  const check = () =>
    runCheck(false)
      .then(toastRun)
      .catch((e: unknown) => notifyError("Couldn't check for updates", e, { retry: check }));
  return (
    <Button size="sm" onClick={check} disabled={checking} aria-busy={checking}>
      <RefreshCw className={cn(checking && "animate-spin")} />
      {checking ? "Checking…" : "Check for updates"}
    </Button>
  );
}
