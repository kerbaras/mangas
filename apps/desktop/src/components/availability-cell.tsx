import { ExternalLink, Lock } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { openExternal } from "@/lib/dialogs";
import type { Availability } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

const hostOf = (url: string) => {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
};

/** Badge for a chapter the source lists but does not serve: locked, or read on another site. */
export function AvailabilityCell({ availability }: { availability: Availability }) {
  if (availability.kind === "locked") {
    return (
      <Badge title="Paid, wait-until-free or early access on the site">
        <Lock className="size-3" /> Locked
      </Badge>
    );
  }
  if (availability.kind !== "external") return null;
  const { url } = availability;
  if (!url) {
    return (
      <Badge title="Published on another site">
        <ExternalLink className="size-3" /> Another site
      </Badge>
    );
  }
  const open = () =>
    openExternal(url).catch((e: unknown) => notifyError("Couldn't open the browser", e));
  return (
    <button
      type="button"
      title={`Read on ${url}`}
      onClick={(e) => {
        e.stopPropagation();
        void open();
      }}
      className="inline-flex max-w-full items-center gap-1 truncate rounded-sm text-caption font-medium text-accent hover:underline"
    >
      <ExternalLink className="size-3.5 shrink-0" />
      <span className="truncate">{hostOf(url)}</span>
    </button>
  );
}
