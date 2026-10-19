import { RotateCcw } from "lucide-react";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { defaultsOf, type Settings, writeSettings } from "@/lib/settings";

type Props = {
  title: string;
  /** Settings "Reset to defaults" puts back; none hides the button. */
  resets?: readonly (keyof Settings)[];
  /** What a reset changes, for the confirmation. */
  resetHint?: string;
  /** What a reset does besides restoring the settings in `resets`; false: it failed (and said
   * so), so the reset isn't reported as done. */
  onReset?: () => Promise<boolean>;
  children: ReactNode;
};

/** One Settings group: title, an optional reset with confirmation, then its fields. */
export function SettingsGroup({ title, resets = [], resetHint, onReset, children }: Props) {
  const [asking, setAsking] = useState(false);
  const reset = async () => {
    setAsking(false);
    writeSettings(defaultsOf(resets));
    if (onReset && !(await onReset())) return;
    toast.success(`${title} settings reset`);
  };
  return (
    <section aria-label={title} className="flex flex-col gap-6">
      <div className="flex min-h-8 items-center justify-between gap-4">
        <h2 className="text-subtitle font-semibold">{title}</h2>
        {resets.length > 0 && (
          <Button variant="ghost" size="sm" onClick={() => setAsking(true)}>
            <RotateCcw /> Reset to defaults
          </Button>
        )}
      </div>
      {children}
      <ConfirmDialog
        open={asking}
        title={`Reset ${title.toLowerCase()} settings?`}
        description={resetHint ?? "The settings in this group go back to their defaults."}
        confirm="Reset"
        onConfirm={() => void reset()}
        onCancel={() => setAsking(false)}
      />
    </section>
  );
}
