import { afterEach, describe, expect, it, vi } from "vitest";
import { chapterFailed } from "@/hooks/use-downloads";
import { failureLog, rememberDownloads } from "@/lib/download-labels";
import { type Backend, setBackend } from "@/lib/ipc";
import { UNLISTED } from "@/lib/ipc.downloads";

const notifyError = vi.hoisted(() => vi.fn());
vi.mock("@/lib/notify", () => ({ notifyError }));

afterEach(() => {
  setBackend(null);
  vi.clearAllMocks();
});

const challenge = "challenge: MangaDot needs a browser session for https://mangadot.net";

describe("chapterFailed", () => {
  it("logs a warning with the provider, manga and chapter ids", async () => {
    const reportError = vi.fn(async () => {});
    setBackend({ reportError } as unknown as Backend);
    rememberDownloads([
      ["ch-7", { label: "Chapter 7 of Kaiju", source: "mangadot", mangaId: "m-1" }],
    ]);
    chapterFailed("ch-7", challenge);
    await vi.waitFor(() =>
      expect(reportError).toHaveBeenCalledWith(
        `chapter download failed: provider=mangadot manga=m-1 chapter=ch-7: ${challenge}`,
        "warn",
      ),
    );
  });

  it("passes the provider to the toast so a challenge offers Connect", () => {
    setBackend({ reportError: async () => {} } as unknown as Backend);
    rememberDownloads([
      ["ch-8", { label: "Chapter 8 of Kaiju", source: "mangadot", mangaId: "m-1" }],
    ]);
    chapterFailed("ch-8", challenge);
    expect(notifyError).toHaveBeenCalledWith(
      "Chapter 8 of Kaiju didn't download",
      challenge,
      expect.objectContaining({
        source: "mangadot",
        id: "download-ch-8",
        retry: expect.any(Function),
      }),
    );
  });

  it("still logs a chapter it knows nothing about", () => {
    expect(failureLog("ch-9", "HTTP 429")).toBe(
      "chapter download failed: provider=unknown manga=unknown chapter=ch-9: HTTP 429",
    );
  });

  it("offers no retry for a chapter the source no longer lists", () => {
    setBackend({ reportError: async () => {} } as unknown as Backend);
    rememberDownloads([
      ["ch-10", { label: "Chapter 10 of Kaiju", source: "mangadex", mangaId: "m-1" }],
    ]);
    chapterFailed("ch-10", UNLISTED);
    chapterFailed("ch-10", "not found: the source no longer lists this chapter");
    for (const [, , options] of notifyError.mock.calls)
      expect(options).toMatchObject({ retry: undefined });
  });
});
