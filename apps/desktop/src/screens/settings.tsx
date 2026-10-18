import { useState } from "react";
import { ErrorAlert } from "@/components/error-alert";
import { ScreenHeader } from "@/components/screen-header";
import { cn } from "@/lib/cn";
import { loadSettings, useSettingsLoad } from "@/lib/settings";
import { GROUPS, type GroupId } from "./settings-groups";

/** The form only once the stored settings are in; a failed load shows why, with a retry. */
export function SettingsScreen() {
  const load = useSettingsLoad();
  if (load.kind === "ready") return <SettingsForm />;
  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Settings" />
      <div className="p-4">
        {load.kind === "failed" && (
          <ErrorAlert
            title="Couldn't load your settings"
            message={load.error}
            onRetry={() => void loadSettings()}
            className="mx-auto max-w-2xl"
          />
        )}
      </div>
    </section>
  );
}

/** The group picked last, so coming back to Settings opens it again. */
let lastGroup: GroupId = "general";

function SettingsForm() {
  const [current, setCurrent] = useState<GroupId>(lastGroup);
  const pick = (id: GroupId) => {
    lastGroup = id;
    setCurrent(id);
  };
  const group = GROUPS.find((g) => g.id === current) ?? GROUPS[0];
  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Settings" />
      <div className="flex min-h-0 flex-1">
        <nav
          aria-label="Settings groups"
          className="flex w-48 shrink-0 flex-col gap-0.5 overflow-y-auto overscroll-contain border-r border-stroke p-2"
        >
          {GROUPS.map((g) => (
            <button
              key={g.id}
              type="button"
              aria-current={g.id === group.id ? "page" : undefined}
              onClick={() => pick(g.id)}
              className={cn(
                "flex h-8 items-center gap-2.5 rounded-md px-2.5 text-left text-ink-2 transition-colors [&_svg]:size-4",
                "hover:bg-surface-2 hover:text-ink aria-[current=page]:bg-accent-soft aria-[current=page]:text-ink",
              )}
            >
              {g.icon}
              {g.label}
            </button>
          ))}
        </nav>
        <div className="relative min-w-0 flex-1 overflow-y-auto overscroll-contain px-6 py-5">
          <div className="mx-auto max-w-2xl">
            <group.Body />
          </div>
        </div>
      </div>
    </section>
  );
}
