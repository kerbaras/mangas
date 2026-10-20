import { Badge } from "@/components/ui/badge";
import { statusLabel } from "@/lib/format";
import type { Manga } from "@/lib/ipc";

/** The publication status; nothing when the source doesn't say. */
export function StatusBadge({ status }: { status: Manga["status"] }) {
  if (status === "unknown") return null;
  return <Badge variant={status === "ongoing" ? "accent" : "neutral"}>{statusLabel[status]}</Badge>;
}
