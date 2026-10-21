import { describe, expect, it } from "vitest";
import { crashReport, describeError } from "@/lib/errors";
import { isFinalError } from "@/lib/ipc.downloads";
import { statusBadge } from "@/lib/providers";

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
  it("names a refused certificate, without the connection hint, and makes it final (#285)", () => {
    const message =
      "network: the certificate of comick.live couldn't be verified (self-signed or from an unknown authority)";
    const info = describeError(message);
    expect(info.hint).toBe(
      "comick.live answered, but its certificate couldn't be verified: self-signed or from an unknown authority.",
    );
    expect(info.hint).not.toMatch(/connection/);
    expect(isFinalError(message)).toBe(true);
    expect(statusBadge({ state: "unreachable", reason: message }).label).toBe(
      "Certificate refused",
    );
    expect(statusBadge({ state: "unreachable", reason: "network: timed out" }).label).toBe(
      "Unreachable",
    );
    expect(isFinalError(`incomplete chapter: 1 of 3 pages failed (page 2: ${message})`)).toBe(true);
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
  it("says a proxied hostname refusal is fetched through the proxy; others stay final", () => {
    const host = "refused: cdn.example resolves to a non-public address 10.0.0.1";
    const literal = "refused: 10.0.0.1 resolves to a non-public address 10.0.0.1";
    const final = "The image host points at a local or private address, so the app won't fetch it.";
    expect(describeError(host, true).hint).toBe(
      "The image host points at a private address. Retry fetches it through the configured proxy.",
    );
    expect(describeError(host).hint).toBe(final);
    expect(describeError(literal, true).hint).toBe(final);
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
