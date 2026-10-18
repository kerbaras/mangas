import { Lock, Pause } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { QueueStatus } from "@/lib/ipc.downloads";

type Props = { gate: QueueStatus; onResume: () => void };

/** Why nothing new starts: pause all (with Resume) or a data-folder move (read-only). */
export function QueueBanner({ gate, onResume }: Props) {
  if (gate.held)
    return (
      <Alert className="mx-4 mt-4 w-auto">
        <Lock />
        <AlertTitle>Queue held while the data folder moves</AlertTitle>
        <AlertDescription>Downloads continue on their own once the move finishes.</AlertDescription>
      </Alert>
    );
  if (!gate.paused) return null;
  return (
    <Alert className="mx-4 mt-4 w-auto">
      <Pause />
      <AlertTitle>Queue paused</AlertTitle>
      <AlertDescription>
        Nothing new starts until you resume, here or with <code>mangas queue resume --all</code>.
      </AlertDescription>
      <Button size="sm" variant="primary" onClick={onResume} className="absolute top-3 right-4">
        Resume
      </Button>
    </Alert>
  );
}
