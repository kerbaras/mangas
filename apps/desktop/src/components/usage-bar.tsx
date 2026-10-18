import { cn } from "@/lib/cn";
import type { StorageUsage } from "@/lib/ipc.storage";
import { formatBytes, share, usageSegments } from "@/lib/storage";

/** One stacked bar of what each app folder takes, with a legend of sizes below. */
export function UsageBar({ usage }: { usage: StorageUsage }) {
  const segments = usageSegments(usage);
  const total = segments.reduce((sum, s) => sum + s.bytes, 0);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <span className="text-body font-medium">{formatBytes(total)} used</span>
      </div>
      <div
        role="img"
        aria-label={segments.map((s) => `${s.label} ${formatBytes(s.bytes)}`).join(", ")}
        className="flex h-2 w-full overflow-hidden rounded-full bg-surface-2"
      >
        {segments.map((s) => (
          <div
            key={s.key}
            className={cn("h-full", s.color)}
            style={{ width: `${share(s.bytes, total)}%` }}
          />
        ))}
      </div>
      <ul className="flex flex-wrap gap-x-5 gap-y-1 text-caption">
        {segments.map((s) => (
          <li key={s.key} className="flex items-center gap-1.5 whitespace-nowrap">
            <span className={cn("size-2 shrink-0 rounded-full", s.color)} />
            <span className="text-ink-2">{s.label}</span>
            <span className="tabular-nums text-ink-3">{formatBytes(s.bytes)}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
