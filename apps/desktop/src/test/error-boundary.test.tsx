import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "@/components/error-boundary";
import { type Backend, setBackend } from "@/lib/ipc";

function Boom(): never {
  throw new Error("render exploded");
}

afterEach(() => {
  setBackend(null);
  vi.restoreAllMocks();
});

describe("ErrorBoundary", () => {
  it("shows the crash, logs it and copies the error", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const reportError = vi.fn(async () => {});
    setBackend({ reportError } as unknown as Backend);
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("render exploded")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /copy error/i }));
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("Error: render exploded"));
    expect(await screen.findByRole("button", { name: /copied/i })).toBeInTheDocument();
    expect(reportError).toHaveBeenCalledWith(expect.stringContaining("render exploded"));
  });
});
