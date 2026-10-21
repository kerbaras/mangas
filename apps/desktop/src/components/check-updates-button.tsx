import { RefreshCw } from "lucide-react";
import { HeaderAction } from "@/components/header-action";
import { notifyError } from "@/lib/notify";
import { toastRun, UPDATE_TOAST } from "@/lib/update-toast";
import { runCheck, useUpdates } from "@/lib/updates";

/**
 * Library header action: checks every followed title now. While any check runs it reads
 * "Checking…" with a still icon: a run takes a minute with a big library, and an endless spin
 * keeps the webview repainting (~100 % CPU without a GPU) the whole time. The label stays in a
 * compact header too: the button is disabled, so its hint can't open.
 */
export function CheckUpdatesButton() {
  const { checking } = useUpdates();
  const check = () =>
    runCheck(false)
      .then(toastRun)
      .catch((e: unknown) =>
        notifyError("Couldn't check for updates", e, { retry: check, id: UPDATE_TOAST }),
      );
  return (
    <HeaderAction
      icon={<RefreshCw />}
      label={checking ? "Checking…" : "Check for updates"}
      keepLabel={checking}
      onClick={check}
      disabled={checking}
      aria-busy={checking}
    />
  );
}
