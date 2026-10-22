import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { isFinalError } from "@/lib/ipc.downloads";
import {
  findKindles,
  onCopyProgress,
  sendEmail,
  sendUsb,
  type UsbVerdict,
  usbKindles,
  usbVerdict,
} from "@/lib/ipc.send";
import { fileName, megabytes } from "@/lib/send-labels";

export type Via = "email" | "usb";

/** An exported file and the title its delivery records. */
export type Book = { file: string; title: string };

export type SendState =
  | { kind: "idle" }
  | { kind: "looking" }
  | { kind: "no-device" }
  /** A Kindle on the USB bus with no mounted folder: MTP only, so no copy; email still works. */
  | { kind: "mtp"; device: string }
  /** The format table stopped the copy before it began: refused, or warned (Retry copies anyway). */
  | { kind: "format"; verdict: UsbVerdict }
  | { kind: "copying"; device: string; done: number; total: number }
  | { kind: "emailing" }
  | { kind: "done"; via: Via; message: string }
  /** `left`: the files this delivery didn't get to, the failed one first. */
  | { kind: "error"; via: Via; message: string; sent: string; left?: string[] };

/** The delivery under way in `state`, if one is. */
export const runningVia = (state: SendState): Via | undefined =>
  state.kind === "emailing"
    ? "email"
    : ["looking", "copying"].includes(state.kind)
      ? "usb"
      : undefined;

/**
 * Whether a delivery by `via` failed in a way a plain Retry can't fix: the Kindle is full, or the
 * format table stopped the copy (only the alert's Copy anyway goes on after a warning).
 */
export const refused = (state: SendState, via?: Via | "save") =>
  state.kind === "format"
    ? via === "usb"
    : state.kind === "error" && state.via === via && isFinalError(state.message);

/** What a failed delivery still has to send (from the book it failed on) and what it already did. */
type Pending = { books: Book[]; via: Via; lines: string[] };

/** The verdict that stops copying `verdicts`' books: a refusal first, else a warning. */
const stopper = (verdicts: UsbVerdict[]) =>
  verdicts.find((v) => v.decision === "refused") ?? verdicts.find((v) => v.decision === "warn");

/**
 * One delivery at a time, of one or more books: email, or a USB copy, each book's format checked
 * against the Kindle first. `retry` sends again only what the last delivery didn't (never
 * rewriting the books themselves); after a format warning it copies anyway.
 */
export function useSend(onSent?: () => void) {
  const [state, setState] = useState<SendState>({ kind: "idle" });
  const device = useRef<string | null>(null);
  const pending = useRef<Pending | null>(null);
  const warned = useRef(false);

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
    async (books: Book[], via: Via, earlier: string[] = [], force = false) => {
      warned.current = false;
      const lines = [...earlier];
      pending.current = { books, via, lines: [...lines] };
      const sentOne = (i: number) => {
        pending.current = { books: books.slice(i + 1), via, lines: [...lines] };
      };
      try {
        if (via === "usb") {
          setState({ kind: "looking" });
          const [kindle] = await findKindles();
          if (!kindle) {
            const [mtp] = await usbKindles();
            return setState(mtp ? { kind: "mtp", device: mtp.name } : { kind: "no-device" });
          }
          const verdicts = await Promise.all(books.map((b) => usbVerdict(b.file, kindle.name)));
          const stop = force ? undefined : stopper(verdicts);
          if (stop) {
            warned.current = stop.decision === "warn";
            return setState({ kind: "format", verdict: stop });
          }
          device.current = kindle.name;
          for (const [i, { file, title }] of books.entries()) {
            setState({ kind: "copying", device: kindle.name, done: 0, total: 0 });
            const copy = await sendUsb(file, title, kindle.root, force);
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
        const left = (pending.current?.books ?? []).map((b) => fileName(b.file));
        setState({ kind: "error", via, message: errorMessage(e), sent: lines.join("\n"), left });
        if (lines.length > earlier.length) onSent?.();
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

  /** Sends what's left again, by `via` when the user picked another way since; copies anyway after a format warning. */
  const retry = useCallback(
    async (via?: Via) => {
      const left = pending.current;
      if (!left) return;
      const to = via ?? left.via;
      await deliver(left.books, to, left.lines, warned.current && to === "usb");
    },
    [deliver],
  );
  /** The books the last delivery didn't get to (none once it finished). */
  const unsent = useCallback(() => pending.current?.books ?? [], []);
  const reset = useCallback(() => {
    pending.current = null;
    warned.current = false;
    setState({ kind: "idle" });
  }, []);
  return { state, deliver, usb, email, retry, reset, unsent };
}
