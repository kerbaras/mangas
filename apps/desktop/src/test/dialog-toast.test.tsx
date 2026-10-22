import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { toast } from "sonner";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { Toaster } from "@/components/ui/sonner";

describe("a toast over a dialog", () => {
  beforeAll(() => {
    // sonner's swipe handler captures the pointer, which jsdom lacks.
    Element.prototype.setPointerCapture ??= () => {};
    // jsdom has no matchMedia; the Toaster follows the OS colour scheme through it.
    const query = { matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() };
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => query),
    );
  });
  afterAll(() => vi.unstubAllGlobals());

  it("runs its action without closing the dialog", async () => {
    const retry = vi.fn();
    const closed = vi.fn();
    render(
      <>
        <Toaster />
        <Dialog open onOpenChange={closed}>
          <DialogContent>
            <DialogTitle>Export</DialogTitle>
          </DialogContent>
        </Dialog>
      </>,
    );
    act(() => {
      toast.error("Couldn't eject Kindle", { action: { label: "Retry", onClick: retry } });
    });
    await userEvent.click(await screen.findByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
    expect(closed).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});
