import { describe, expect, it } from "vitest";
import { crashReport, describeError } from "@/lib/errors";
import { isFinalError } from "@/lib/ipc.downloads";

describe("describeError", () => {
  it("explains network errors and keeps the raw message", () => {
    const info = describeError("network: error sending request for url (https://x)");
    expect(info.hint).toMatch(/Couldn't reach the site/);
    expect(info.detail).toBe("network: error sending request for url (https://x)");
    expect(info.challenge).toBe(false);
  });
  it("says to install ca-certificates instead of checking the connection", () => {
    const message =
      "network: no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package (e.g. `sudo apt install ca-certificates`)";
    const info = describeError(message);
    expect(info.hint).toMatch(/Install the ca-certificates package/);
    expect(info.detail).toBe(message);
    expect(isFinalError(message)).toBe(true);
    expect(isFinalError("network: connection reset by peer")).toBe(false);
  });
  it("flags challenge errors", () => {
    const info = describeError("challenge: mangadot needs a browser session for https://m");
    expect(info.challenge).toBe(true);
    expect(info.hint).toMatch(/browser check/);
  });
  it("has no hint for unknown errors", () => {
    expect(describeError("something odd")).toEqual({
      hint: null,
      detail: "something odd",
      challenge: false,
    });
  });
  it("never returns an empty detail", () => {
    expect(describeError("  ").detail).toBe("Unknown error");
  });
});

describe("crashReport", () => {
  it("joins message, stack and component stack", () => {
    const error = new TypeError("boom");
    error.stack = "TypeError: boom\n    at Screen";
    expect(crashReport(error, "\n    in Screen")).toBe(
      "TypeError: boom\n\nTypeError: boom\n    at Screen\n\nComponent stack:\n    in Screen",
    );
  });
});
