import { Badge } from "@/components/ui/badge";
import { statusLabel } from "@/lib/format";
import type { Manga } from "@/lib/ipc";

export function StatusBadge({ status }: { status: Manga["status"] }) {
  return <Badge variant={status === "ongoing" ? "accent" : "neutral"}>{statusLabel[status]}</Badge>;
}
