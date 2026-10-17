import { describe, expect, it } from "vitest";
import { crashReport, describeError } from "@/lib/errors";

describe("describeError", () => {
  it("explains network errors and keeps the raw message", () => {
    const info = describeError("network: error sending request for url (https://x)");
    expect(info.hint).toMatch(/Couldn't reach the site/);
    expect(info.detail).toBe("network: error sending request for url (https://x)");
    expect(info.challenge).toBe(false);
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
