import { Switch } from "@/components/ui/switch";

/** Export option: write over a book already in the folder instead of a "Title (2)" copy. */
export function ReplaceSwitch({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  return (
    <label htmlFor="export-replace" className="flex items-center gap-2 text-body text-ink-2">
      <Switch
        id="export-replace"
        aria-label="Replace existing files"
        checked={on}
        onCheckedChange={onChange}
      />
      Replace existing files
    </label>
  );
}
