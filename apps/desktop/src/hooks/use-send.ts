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

/** What a failed delivery still has to send: the books from the one that failed on. */
type Pending = { books: Book[]; via: Via };

/**
 * One delivery at a time, of one or more books: email, or a USB copy. `retry` sends again only
 * what the last delivery didn't (never rewriting the books themselves).
 */
export function useSend(onSent?: () => void) {
  const [state, setState] = useState<SendState>({ kind: "idle" });
  const device = useRef<string | null>(null);
  const pending = useRef<Pending | null>(null);

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
      pending.current = { books, via };
      const sentOne = (i: number) => {
        pending.current = { books: books.slice(i + 1), via };
      };
      try {
        if (via === "usb") {
          setState({ kind: "looking" });
          const [kindle] = await findKindles();
          if (!kindle) return setState({ kind: "no-device" });
          device.current = kindle.name;
          for (const [i, { file, title }] of books.entries()) {
            setState({ kind: "copying", device: kindle.name, done: 0, total: 0 });
            const copy = await sendUsb(file, title, kindle.root);
            const verb = copy.already_there ? "Already on" : "Copied to";
            lines.push(`${verb} ${kindle.name}: ${copy.path}`);
            sentOne(i);
          }
        } else {
          for (const [i, { file, title }] of books.entries()) {
            setState({ kind: "emailing" });
            const sent = await sendEmail(file, title);
            lines.push(`Emailed ${fileName(file)} (${megabytes(sent.bytes)}) to ${sent.to}`);
            sentOne(i);
          }
        }
        pending.current = null;
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

  const retry = useCallback(async () => {
    const left = pending.current;
    if (left) await deliver(left.books, left.via);
  }, [deliver]);
  const reset = useCallback(() => {
    pending.current = null;
    setState({ kind: "idle" });
  }, []);
  return { state, deliver, usb, email, retry, reset };
}
