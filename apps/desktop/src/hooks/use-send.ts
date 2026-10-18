import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { findKindles, onCopyProgress, sendEmail, sendUsb } from "@/lib/ipc.send";
import { fileName, megabytes } from "@/lib/send-labels";

export type Via = "email" | "usb";

/** An exported file and the title its delivery records. */
export type Book = { file: string; title: string };

export type SendState =
  | { kind: "idle" }
  | { kind: "looking" }
  | { kind: "no-device" }
  | { kind: "copying"; device: string; done: number; total: number }
  | { kind: "emailing" }
  | { kind: "done"; via: Via; message: string }
  | { kind: "error"; via: Via; message: string; sent: string };

/** One delivery at a time, of one or more books: email, or a USB copy. */
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

  /** Sends the books one after another; a USB copy first looks for one mounted Kindle. */
  const deliver = useCallback(
    async (books: Book[], via: Via) => {
      const lines: string[] = [];
      try {
        if (via === "usb") {
          setState({ kind: "looking" });
          const [kindle] = await findKindles();
          if (!kindle) return setState({ kind: "no-device" });
          device.current = kindle.name;
          for (const { file, title } of books) {
            setState({ kind: "copying", device: kindle.name, done: 0, total: 0 });
            const copy = await sendUsb(file, title, kindle.root);
            const verb = copy.already_there ? "Already on" : "Copied to";
            lines.push(`${verb} ${kindle.name}: ${copy.path}`);
          }
        } else {
          for (const { file, title } of books) {
            setState({ kind: "emailing" });
            const sent = await sendEmail(file, title);
            lines.push(`Emailed ${fileName(file)} (${megabytes(sent.bytes)}) to ${sent.to}`);
          }
        }
        setState({ kind: "done", via, message: lines.join("\n") });
        onSent?.();
      } catch (e) {
        setState({ kind: "error", via, message: errorMessage(e), sent: lines.join("\n") });
        if (lines.length > 0) onSent?.();
      } finally {
        device.current = null;
      }
    },
    [onSent],
  );

  const usb = useCallback(
    (file: string, title: string) => deliver([{ file, title }], "usb"),
    [deliver],
  );
  const email = useCallback(
    (file: string, title: string) => deliver([{ file, title }], "email"),
    [deliver],
  );

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, deliver, usb, email, reset };
}
