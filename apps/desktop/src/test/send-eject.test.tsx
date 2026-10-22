import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { COPYING_HINT } from "@/components/kindle-row";
import { SendPanel } from "@/components/send-panel";
import { SendStatus } from "@/components/send-status";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type Backend, setBackend } from "@/lib/ipc";
import type { CopyProgress, UsbCopy } from "@/lib/ipc.send";
import { emailRefusal } from "./email-refusal-stub";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
  }),
}));

const kindle = {
  name: "Kindle",
  root: "/media/me/Kindle",
  documents: "/media/me/Kindle/documents",
  ejectable: true,
};
const FILE = "/books/Solo Leveling 1-3.epub";
const COPIED = "/media/me/Kindle/documents/Solo Leveling 1-3.epub";
const BUSY =
  "can't eject Kindle at /media/me/Kindle: it's in use by less. Close it and eject again. The books copied to it are still there";

afterEach(() => {
  setBackend(null);
  vi.clearAllMocks();
});

function stub(overrides: Partial<Backend>) {
  setBackend({
    onCopyProgress: async (_: (p: CopyProgress) => void) => () => {},
    usbKindles: vi.fn().mockResolvedValue([]),
    findKindles: vi.fn().mockResolvedValue([kindle]),
    usbVerdict: async (_: string, device: string) => ({ decision: "supported", device }),
    emailRefusal,
    ...overrides,
  } as unknown as Backend);
}

const panel = () =>
  render(
    <TooltipProvider>
      <SendPanel file={FILE} />
    </TooltipProvider>,
  );

describe("Eject (#330)", () => {
  it("is disabled with a hint while copying, then ejects what was copied", async () => {
    let finish: (copy: UsbCopy) => void = () => {};
    const sendUsb = vi.fn(() => new Promise<UsbCopy>((r) => (finish = r)));
    const ejectKindle = vi.fn().mockResolvedValue({ device: "Kindle", powered_off: true });
    stub({ sendUsb, ejectKindle });
    panel();
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    await waitFor(() => expect(sendUsb).toHaveBeenCalled());
    expect((screen.getByRole("button", { name: "Eject" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(screen.getByText(COPYING_HINT)).toBeTruthy();
    finish({ path: COPIED, already_there: false });
    await screen.findByText(/Copied to Kindle:/);
    const eject = await screen.findByRole("button", { name: "Eject" });
    await waitFor(() => expect((eject as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(eject);
    await waitFor(() => expect(ejectKindle).toHaveBeenCalledWith(kindle.root, [COPIED]));
    await waitFor(() => expect(toast.success).toHaveBeenCalledOnce());
    const [, loading] = vi.mocked(toast.loading).mock.calls[0] ?? [];
    const [title, done] = vi.mocked(toast.success).mock.calls[0] ?? [];
    expect(title).toBe("Kindle ejected");
    expect(done).toMatchObject({ id: loading?.id, action: null });
  });

  it("a busy volume replaces the toast with the message and a Retry", async () => {
    const ejectKindle = vi
      .fn()
      .mockRejectedValueOnce(BUSY)
      .mockRejectedValueOnce(BUSY)
      .mockResolvedValue({ device: "Kindle" });
    const sendUsb = vi.fn().mockResolvedValue({ path: COPIED, already_there: false });
    stub({ ejectKindle, sendUsb });
    panel();
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    await screen.findByText(/Copied to Kindle:/);
    fireEvent.click(await screen.findByRole("button", { name: "Eject" }));
    await waitFor(() => expect(toast.error).toHaveBeenCalledOnce());
    const [, loading] = vi.mocked(toast.loading).mock.calls[0] ?? [];
    const [title, failed] = vi.mocked(toast.error).mock.calls[0] ?? [];
    expect(title).toBe("Couldn't eject Kindle");
    expect(failed?.id).toBe(loading?.id);
    // The whole message shows: its end says the books are still on the Kindle.
    const description = failed?.description as { props: { full?: boolean } } | undefined;
    expect(description?.props.full).toBe(true);
    type Retry = { label: string; onClick: (e: { preventDefault(): void }) => void };
    const retry = failed?.action as Retry;
    expect(retry.label).toBe("Retry");
    // Still busy: Retry keeps the toast (sonner would dismiss it) and the new failure replaces it.
    const kept = vi.fn();
    retry.onClick({ preventDefault: kept });
    expect(kept).toHaveBeenCalledOnce();
    await waitFor(() => expect(toast.error).toHaveBeenCalledTimes(2));
    expect(vi.mocked(toast.error).mock.calls[1]?.[1]?.id).toBe(loading?.id);
    const again = vi.mocked(toast.error).mock.calls[1]?.[1]?.action as Retry;
    again.onClick({ preventDefault: vi.fn() });
    await waitFor(() => expect(toast.success).toHaveBeenCalledOnce());
    expect(vi.mocked(toast.success).mock.calls[0]?.[1]?.id).toBe(loading?.id);
  });

  it("a new copy after an eject offers Eject again", async () => {
    const ejectKindle = vi.fn().mockResolvedValue({ device: "Kindle", powered_off: true });
    const sendUsb = vi.fn().mockResolvedValue({ path: COPIED, already_there: false });
    stub({ ejectKindle, sendUsb });
    panel();
    const copy = screen.getByRole("button", { name: /Copy over USB/ });
    fireEvent.click(copy);
    fireEvent.click(await screen.findByRole("button", { name: "Eject" }));
    await screen.findByRole("button", { name: "Ejected" });
    fireEvent.click(copy);
    await waitFor(() => expect(sendUsb).toHaveBeenCalledTimes(2));
    const again = await screen.findByRole("button", { name: "Eject" });
    await waitFor(() => expect((again as HTMLButtonElement).disabled).toBe(false));
  });

  it("an MTP Kindle has no Eject and says why", async () => {
    stub({
      findKindles: vi.fn().mockResolvedValue([]),
      usbKindles: vi.fn().mockResolvedValue([{ name: "Kindle" }]),
    });
    panel();
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(/connected over MTP, so there's nothing to eject/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Eject" })).toBeNull();
  });

  it("the completion panel offers Eject after a USB copy, never after an email", () => {
    stub({});
    const usb = { kind: "done", via: "usb", message: `Copied to Kindle: ${COPIED}` } as const;
    const { rerender } = render(<SendStatus state={{ ...usb, kindle, copied: [COPIED] }} eject />);
    expect(screen.getByRole("button", { name: "Eject" })).toBeTruthy();
    rerender(<SendStatus state={{ kind: "done", via: "email", message: "Emailed" }} eject />);
    expect(screen.queryByRole("button", { name: "Eject" })).toBeNull();
    const mtp = { ...kindle, ejectable: false };
    rerender(<SendStatus state={{ ...usb, kindle: mtp, copied: [COPIED] }} eject />);
    expect(screen.queryByRole("button", { name: "Eject" })).toBeNull();
  });
});
