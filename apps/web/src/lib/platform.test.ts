import { describe, expect, it } from "vitest";
import { detectPlatform } from "./platform";

const MAC_SAFARI =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
const WIN_CHROME =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36";
const LINUX_X64 = "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0";
const LINUX_ARM = "Mozilla/5.0 (X11; Linux aarch64; rv:131.0) Gecko/20100101 Firefox/131.0";
const ANDROID =
  "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Mobile Safari/537.36";
const IPHONE =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

describe("detectPlatform", () => {
  it("defaults a Mac with no hints to Apple silicon, marked as a guess", () => {
    expect(detectPlatform({ userAgent: MAC_SAFARI })).toEqual({
      os: "macos",
      arch: "arm64",
      archKnown: false,
    });
  });

  it("reads Intel and Apple silicon Macs from client hints", () => {
    const x86 = { platform: "macOS", architecture: "x86", bitness: "64" };
    expect(detectPlatform({ userAgent: MAC_SAFARI, hints: x86 })?.arch).toBe("x64");
    const arm = { platform: "macOS", architecture: "arm", bitness: "64" };
    expect(detectPlatform({ userAgent: MAC_SAFARI, hints: arm })).toEqual({
      os: "macos",
      arch: "arm64",
      archKnown: true,
    });
  });

  it("guesses Windows x64 from the user agent and trusts arch hints", () => {
    // Windows on Arm browsers send "Win64; x64" too, so the user agent alone is a guess.
    expect(detectPlatform({ userAgent: WIN_CHROME })).toEqual({
      os: "windows",
      arch: "x64",
      archKnown: false,
    });
    const x64 = { platform: "Windows", architecture: "x86", bitness: "64" };
    expect(detectPlatform({ userAgent: WIN_CHROME, hints: x64 })?.archKnown).toBe(true);
    const arm = { platform: "Windows", architecture: "arm", bitness: "64" };
    expect(detectPlatform({ userAgent: WIN_CHROME, hints: arm })).toEqual({
      os: "windows",
      arch: "arm64",
      archKnown: true,
    });
  });

  it("detects Linux x64 and arm64", () => {
    expect(detectPlatform({ userAgent: LINUX_X64 })?.arch).toBe("x64");
    expect(detectPlatform({ userAgent: LINUX_ARM })).toEqual({
      os: "linux",
      arch: "arm64",
      archKnown: true,
    });
  });

  it("offers nothing to phones and iPads", () => {
    expect(detectPlatform({ userAgent: ANDROID })).toBeNull();
    expect(detectPlatform({ userAgent: IPHONE })).toBeNull();
    expect(detectPlatform({ userAgent: MAC_SAFARI, maxTouchPoints: 5 })).toBeNull();
  });

  it("returns null for an unknown system", () => {
    expect(detectPlatform({ userAgent: "curl/8.5.0" })).toBeNull();
  });
});
