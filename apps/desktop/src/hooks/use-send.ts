import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { findKindles, onCopyProgress, sendEmail, sendUsb } from "@/lib/ipc.send";
import { fileName, megabytes } from "@/lib/send-labels";

export type Via = "email" | "usb";

export type SendState =
  | { kind: "idle" }
  | { kind: "looking" }
  | { kind: "no-device" }
  | { kind: "copying"; device: string; done: number; total: number }
  | { kind: "emailing" }
  | { kind: "done"; via: Via; message: string }
  | { kind: "error"; via: Via; message: string };

/** One delivery at a time: email, or a USB copy that first looks for a mounted Kindle. */
export function useSend(onSent?: () => void) {
  const [state, setState] = useState<SendState>({ kind: "idle" });
  const device = useRef<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    onCopyProgress(({ done, total }) => {
      const name = device.current;
      if (name) setState({ kind: "copying", device: name, done, total });
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  const usb = useCallback(
    async (file: string, title: string) => {
      setState({ kind: "looking" });
      try {
        const [kindle] = await findKindles();
        if (!kindle) return setState({ kind: "no-device" });
        device.current = kindle.name;
        setState({ kind: "copying", device: kindle.name, done: 0, total: 0 });
        const copy = await sendUsb(file, title, kindle.root);
        const verb = copy.already_there ? "Already on" : "Copied to";
        setState({ kind: "done", via: "usb", message: `${verb} ${kindle.name}: ${copy.path}` });
        onSent?.();
      } catch (e) {
        setState({ kind: "error", via: "usb", message: errorMessage(e) });
      } finally {
        device.current = null;
      }
    },
    [onSent],
  );

  const email = useCallback(
    async (file: string, title: string) => {
      setState({ kind: "emailing" });
      try {
        const sent = await sendEmail(file, title);
        const what = `${fileName(file)} (${megabytes(sent.bytes)})`;
        setState({ kind: "done", via: "email", message: `Emailed ${what} to ${sent.to}` });
        onSent?.();
      } catch (e) {
        setState({ kind: "error", via: "email", message: errorMessage(e) });
      }
    },
    [onSent],
  );

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, usb, email, reset };
}
