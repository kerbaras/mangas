import { Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { type PartialCopy, removePartialMove } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { formatBytes } from "@/lib/storage";

type Props = { copy: PartialCopy; onRemoved: () => void };

/** What a killed data-folder move left in its target, with a Remove action. */
export function PartialMoveAlert({ copy, onRemoved }: Props) {
  const [busy, setBusy] = useState(false);
  const remove = async () => {
    setBusy(true);
    try {
      const removed = await removePartialMove();
      toast.success("Partial copy removed", {
        id: "partial-move-remove",
        action: null,
        description: removed
          ? `${formatBytes(removed.bytes)} freed in ${removed.path}.`
          : "Nothing was left to remove.",
      });
    } catch (e: unknown) {
      notifyError("Couldn't remove the partial copy", e, { id: "partial-move-remove" });
    } finally {
      setBusy(false);
      onRemoved();
    }
  };
  return (
    <Alert>
      <AlertTitle>A data-folder move was interrupted</AlertTitle>
      <AlertDescription>
        <span>
          It left a partial copy ({formatBytes(copy.bytes)}) in{" "}
          <span className="font-mono">{copy.path}</span>. Your data folder wasn't changed. Moving to
          that folder again replaces it.
        </span>
        <Button size="sm" onClick={() => void remove()} disabled={busy} className="mt-1">
          <Trash2 /> Remove
        </Button>
      </AlertDescription>
    </Alert>
  );
}
