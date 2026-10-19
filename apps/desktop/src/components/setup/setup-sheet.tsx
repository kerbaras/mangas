import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";
import { loadSettings, useSettings, writeSettings } from "@/lib/settings";
import { closeSetup, needsSetup, useSetupOpen, useSetupReopened } from "@/lib/setup";
import { FolderStep, ProvidersStep, ReaderStep } from "./setup-steps";

const STEPS = [
  {
    title: "Where should chapters go?",
    hint: "Downloads and detector models are kept in this folder. You can move it later in Settings.",
    Body: FolderStep,
  },
  {
    title: "Which e-reader do you use?",
    hint: "Exports are sized for this device. You can pick another one for each export.",
    Body: ReaderStep,
  },
  {
    title: "Connect your sources",
    hint: "Turn on the sites you read from. Sites behind a browser check need a one-time Connect.",
    Body: ProvidersStep,
  },
];

/**
 * First-run setup: finishing, skipping or closing it stores the flag so it doesn't come back.
 * Reopened from Settings, closing it (Esc) just goes back to Settings.
 */
export function SetupSheet() {
  // Set while the "done" flag failed to save: the sheet stays open and closing it retries.
  const [unsaved, setUnsaved] = useState(false);
  const open = useSetupOpen() || unsaved;
  const [settings] = useSettings();
  // Reopened from Settings with first-run setup already done: closing it changes nothing.
  const rerun = useSetupReopened() && !unsaved && !needsSetup(settings.setupComplete);
  const nav = useNavigation();
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const { title, hint, Body } = STEPS[step];
  const last = step === STEPS.length - 1;

  const done = async () => {
    if (busy) return;
    setBusy(true);
    const saved = await writeSettings({ setupComplete: true });
    setBusy(false);
    if (!saved) {
      notifyError("Couldn't save the setup", "settings not saved; try again");
      setUnsaved(true);
      await loadSettings();
      return;
    }
    setUnsaved(false);
    closeSetup();
    setStep(0);
    nav.go({ screen: "library" });
  };

  const dismiss = () => {
    if (busy) return;
    if (!rerun) return void done();
    closeSetup();
    setStep(0);
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && dismiss()}>
      <DialogContent className="max-w-lg" onInteractOutside={(e) => e.preventDefault()}>
        <p className="text-caption text-ink-3">
          Step {step + 1} of {STEPS.length}
        </p>
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>{hint}</DialogDescription>
        <div className="mt-4">
          <Body onBusy={setBusy} />
        </div>
        <DialogFooter className="items-center">
          <Button variant="ghost" onClick={done} disabled={busy} className="mr-auto">
            Skip setup
          </Button>
          {step > 0 && (
            <Button onClick={() => setStep(step - 1)} disabled={busy}>
              Back
            </Button>
          )}
          <Button variant="primary" onClick={last ? done : () => setStep(step + 1)} disabled={busy}>
            {last ? "Finish" : "Next"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
