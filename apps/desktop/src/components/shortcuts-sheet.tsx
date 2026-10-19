import { Fragment } from "react";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { isMacLike } from "@/lib/platform";
import { keyLabel, SHORTCUTS, type Shortcut } from "@/lib/shortcuts";

const GROUPS: Shortcut["group"][] = ["General", "Go to", "Lists"];

function Keys({ shortcut, mac }: { shortcut: Shortcut; mac: boolean }) {
  return (
    <span className="flex items-center gap-1">
      {shortcut.bindings.map((b, i) => (
        <Fragment key={b.key}>
          {i > 0 && <span className="text-caption text-ink-3">/</span>}
          {keyLabel(b, mac).map((k) => (
            <kbd
              key={k}
              className="min-w-6 rounded-sm border border-stroke bg-surface-2 px-1.5 text-center font-mono text-caption"
            >
              {k}
            </kbd>
          ))}
        </Fragment>
      ))}
    </span>
  );
}

/** The `?` sheet, rendered from the same table the key handler reads. */
export function ShortcutsSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  const mac = isMacLike();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogTitle>Keyboard shortcuts</DialogTitle>
        <DialogDescription className="sr-only">Every shortcut in the app</DialogDescription>
        <div className="mt-3 flex flex-col gap-4">
          {GROUPS.map((group) => (
            <section key={group} aria-label={group}>
              <h3 className="mb-1 text-caption font-semibold text-ink-3">{group}</h3>
              <dl className="flex flex-col">
                {SHORTCUTS.filter((s) => s.group === group).map((s) => (
                  <div
                    key={s.label + group}
                    className="flex h-8 items-center justify-between gap-4"
                  >
                    <dt className="text-body">{s.label}</dt>
                    <dd>
                      <Keys shortcut={s} mac={mac} />
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
