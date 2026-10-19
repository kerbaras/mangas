import { RefreshCw } from "lucide-react";
import { HeaderAction } from "@/components/header-action";
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
    <HeaderAction
      icon={<RefreshCw className={cn(checking && "animate-spin")} />}
      label={checking ? "Checking…" : "Check for updates"}
      onClick={check}
      disabled={checking}
      aria-busy={checking}
    />
  );
}
