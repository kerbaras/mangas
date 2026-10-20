import { afterEach, describe, expect, it, vi } from "vitest";
import {
  availableUpdate,
  checkForAppUpdate,
  dismissAppUpdate,
  installAppUpdate,
  readAppUpdate,
  resetAppUpdate,
} from "@/lib/app-update";
import { progressLine, progressPercent, statusLine } from "@/lib/app-update-labels";
import { type Backend, setBackend } from "@/lib/ipc";
import type { AppUpdateBackend, AppUpdateStatus } from "@/lib/ipc.app-update";

const available: AppUpdateStatus = {
  kind: "available",
  current: "0.4.0",
  version: "0.5.0",
  notes: null,
  date: null,
  channel: "stable",
};

const pin = (b: Partial<AppUpdateBackend>) => setBackend(b as Backend);

afterEach(() => {
  resetAppUpdate({ force: true });
  setBackend(null);
  vi.restoreAllMocks();
});

describe("app update wording", () => {
  it("matches the CLI", () => {
    expect(statusLine({ kind: "up_to_date", current: "0.5.0", channel: "rc" })).toBe(
      "You're up to date (v0.5.0, release candidates channel)",
    );
    expect(statusLine(available)).toBe("mangas v0.5.0 is available (you have v0.4.0)");
    const older = { kind: "older", current: "0.3.9", version: "0.3.0", channel: "rc" } as const;
    expect(statusLine(older)).toBe(
      "The server offers v0.3.0, older than your v0.3.9: nothing to install",
    );
    expect(statusLine({ ...older, kind: "invalid_signature", version: "0.5.0" })).toBe(
      "v0.5.0 isn't signed with the key this app trusts, so it won't be installed",
    );
  });

  it("shows percent when the size is known, megabytes otherwise", () => {
    const half = { kind: "downloading", received: 5_000_000, total: 10_000_000 } as const;
    expect(progressLine(half)).toBe("Downloading… 50% of 10.0 MB");
    expect(progressPercent({ ...half, total: null })).toBeNull();
    expect(progressLine({ ...half, total: null })).toBe("Downloading… 5.0 MB");
    expect(progressPercent({ kind: "installing" })).toBe(100);
  });
});

describe("app update state", () => {
  it("asks the backend about the chosen channel", async () => {
    const checkAppUpdate = vi.fn(async () => available);
    pin({ checkAppUpdate });
    await checkForAppUpdate("rc");
    expect(checkAppUpdate).toHaveBeenCalledWith("rc");
    expect(readAppUpdate().check).toEqual({ kind: "done", status: available });
  });

  it("reports a failed check, unless it was the silent one at launch", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    pin({ checkAppUpdate: async () => Promise.reject(new Error("offline")) });
    await checkForAppUpdate("stable", { silent: true });
    expect(readAppUpdate().check).toEqual({ kind: "idle" });
    await checkForAppUpdate("stable");
    expect(readAppUpdate().check).toEqual({ kind: "failed", error: "offline" });
  });

  it("keeps only the latest check when a channel change resets it", async () => {
    let answer: ((s: AppUpdateStatus) => void) | undefined;
    pin({ checkAppUpdate: () => new Promise((r) => (answer = r)) });
    const pending = checkForAppUpdate("stable");
    await vi.waitFor(() => expect(answer).toBeDefined());
    resetAppUpdate();
    answer?.(available);
    await pending;
    expect(readAppUpdate().check).toEqual({ kind: "idle" });
  });

  it("installs with progress and brings a dismissed banner back", async () => {
    const seen: unknown[] = [];
    pin({
      checkAppUpdate: async () => available,
      installAppUpdate: async (version, onProgress) => {
        expect(version).toBe("0.5.0");
        onProgress({ kind: "downloading", received: 1, total: 2 });
        seen.push(readAppUpdate().install);
        onProgress({ kind: "installing" });
      },
    });
    await checkForAppUpdate("stable");
    dismissAppUpdate();
    await installAppUpdate();
    expect(seen).toEqual([
      { kind: "running", progress: { kind: "downloading", received: 1, total: 2 } },
    ]);
    expect(readAppUpdate().dismissed).toBe(false);
  });

  it("keeps the update offered after a failed install", async () => {
    pin({
      checkAppUpdate: async () => available,
      installAppUpdate: async () => Promise.reject(new Error("bad signature")),
    });
    await checkForAppUpdate("stable");
    await installAppUpdate();
    expect(readAppUpdate().install).toEqual({ kind: "failed", error: "bad signature" });
    expect(readAppUpdate().check).toEqual({ kind: "done", status: available });
  });

  it("offers nothing to install for an older or untrusted release", async () => {
    for (const kind of ["older", "invalid_signature"] as const) {
      const status = { kind, current: "0.4.0", version: "0.5.0", channel: "stable" } as const;
      pin({ checkAppUpdate: async () => status });
      await checkForAppUpdate("stable");
      expect(availableUpdate(readAppUpdate())).toBeNull();
    }
  });

  it("does nothing to install before a check found an update", async () => {
    const installAppUpdate_ = vi.fn();
    pin({ installAppUpdate: installAppUpdate_ });
    await installAppUpdate();
    expect(installAppUpdate_).not.toHaveBeenCalled();
  });
});
