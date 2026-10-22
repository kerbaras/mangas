import { type Delivery, useEmailBlock } from "@/components/delivery-picker";
import type { Async } from "@/hooks/use-async";
import type { SeriesTarget } from "@/hooks/use-series-target";
import { inFormat } from "@/lib/export-plan";
import type { VolumePlan } from "@/lib/ipc.volumes";

/**
 * The delivery the export dialog runs (Email falls back to saving while email can't take the
 * book), the format it writes for it (with the reason when the delivery chose it), and `plans`
 * named in that format. A Kindle's format follows the delivery, so a book written for one
 * delivery is written again for the other (the dialog clears the result when the format changes).
 */
export function useDeliveryFormat(
  target: SeriesTarget,
  delivery: Delivery,
  plans: Async<VolumePlan[]>,
) {
  const emailBlocked = useEmailBlock(target.formatFor("email")) !== null;
  const via: Delivery = delivery === "email" && emailBlocked ? "save" : delivery;
  const sendVia = via === "save" ? undefined : via;
  const format = target.formatFor(via);
  const planned = inFormat(plans, format);
  return { via, sendVia, email: via === "email", format, reason: target.reasonFor(via), planned };
}
