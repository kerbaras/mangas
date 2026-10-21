import { ExternalLink, Lock } from "lucide-react";
import { Hint } from "@/components/hint";
import { Badge } from "@/components/ui/badge";
import { useT } from "@/i18n/use-t";
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
  const t = useT();
  if (availability.kind === "locked") {
    return (
      <Hint hint={t("details.lockedHint")}>
        <Badge>
          <Lock className="size-3" /> {t("details.locked")}
        </Badge>
      </Hint>
    );
  }
  if (availability.kind !== "external") return null;
  const { url } = availability;
  if (!url) {
    return (
      <Hint hint={t("details.anotherSiteHint")}>
        <Badge>
          <ExternalLink className="size-3" /> {t("details.anotherSite")}
        </Badge>
      </Hint>
    );
  }
  const open = () =>
    openExternal(url).catch((e: unknown) => notifyError(t("details.browserFailed"), e));
  return (
    <Hint hint={t("details.readOn", { url })} focusable={false}>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          void open();
        }}
        className="inline-flex max-w-full items-center gap-1 truncate rounded-sm text-caption font-medium text-accent-text hover:underline"
      >
        <ExternalLink className="size-3.5 shrink-0" />
        <span className="truncate">{hostOf(url)}</span>
      </button>
    </Hint>
  );
}
