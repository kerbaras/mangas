import { findKindles, type UsbVerdict, usbKindles } from "@/lib/ipc.send";

/** The verdict that stops copying `verdicts`' books: a refusal first, else a warning. */
export const stopper = (verdicts: UsbVerdict[]) =>
  verdicts.find((v) => v.decision === "refused") ?? verdicts.find((v) => v.decision === "warn");

/** The mounted Kindle, else the `name` of one on the bus over MTP only: no copy, but the format table still applies. */
export async function reachKindle() {
  const [kindle] = await findKindles();
  if (kindle) return { kindle, name: kindle.name };
  const [mtp] = await usbKindles();
  return { kindle: undefined, name: mtp?.name };
}
