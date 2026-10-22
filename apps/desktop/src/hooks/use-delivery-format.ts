import { type Delivery, useEmailBlock } from "@/components/delivery-picker";
import type { Async } from "@/hooks/use-async";
import { useEmailRefusal } from "@/hooks/use-email-refusal";
import type { SeriesTarget } from "@/hooks/use-series-target";
import { inFormat } from "@/lib/export-plan";
import type { VolumePlan } from "@/lib/ipc.volumes";

/**
 * The delivery the export dialog runs (Email falls back to saving until mail is set up), the
 * format it writes for it (with the reason when the delivery chose it), and `plans` named in
 * that format. Email always writes a format Send to Kindle takes: a picked AZW3 or CBZ (core's
 * `email_refusal`) becomes EPUB, saying so. A book written for one delivery is written again
 * for another when its format differs (the dialog clears the result when the format changes).
 */
export function useDeliveryFormat(
  target: SeriesTarget,
  delivery: Delivery,
  plans: Async<VolumePlan[]>,
) {
  const unset = useEmailBlock() !== null;
  const via: Delivery = delivery === "email" && unset ? "save" : delivery;
  const sendVia = via === "save" ? undefined : via;
  const picked = target.formatFor(via);
  const refused = useEmailRefusal(`book.${target.formatFor("email")}`) !== null;
  const epub = via === "email" && refused;
  const format = epub ? "epub" : picked;
  const reason = epub
    ? `Email sends EPUB; Send to Kindle doesn't take ${picked.toUpperCase()} by email`
    : target.reasonFor(via);
  const planned = inFormat(plans, format);
  return { via, sendVia, email: via === "email", format, reason, planned };
}
