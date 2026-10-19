import { Download } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import {
  availableUpdate,
  dismissAppUpdate,
  installAppUpdate,
  useAppUpdate,
} from "@/lib/app-update";
import { progressLine, progressPercent } from "@/lib/app-update-labels";

/** "Mangas vX is available": dismissible until Restart to update, then download progress. */
export function AppUpdateBanner() {
  const state = useAppUpdate();
  const update = availableUpdate(state);
  const { install } = state;
  if (!update || (state.dismissed && install.kind === "idle")) return null;
  const running = install.kind === "running";
  return (
    <Alert className="mx-4 mt-4 w-auto pr-64" role="status">
      <Download />
      <AlertTitle>Mangas v{update.version} is available</AlertTitle>
      <AlertDescription>
        {running ? (
          <div className="flex w-full max-w-md flex-col gap-1.5">
            <Progress
              value={progressPercent(install.progress) ?? 0}
              max={progressPercent(install.progress) === null ? 0 : 100}
              aria-label="Update download"
            />
            <span>{progressLine(install.progress)}</span>
          </div>
        ) : install.kind === "failed" ? (
          <span className="text-bad">Update failed: {install.error}</span>
        ) : (
          "Restart to update. Downloads in progress pick up where they left off."
        )}
      </AlertDescription>
      <div className="absolute top-3 right-4 flex gap-2">
        {!running && (
          <Button size="sm" variant="ghost" onClick={dismissAppUpdate}>
            Not now
          </Button>
        )}
        <Button
          size="sm"
          variant="primary"
          disabled={running}
          onClick={() => void installAppUpdate()}
        >
          {install.kind === "failed" ? "Try again" : "Restart to update"}
        </Button>
      </div>
    </Alert>
  );
}
