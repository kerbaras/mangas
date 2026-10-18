export type Os = "macos" | "windows" | "linux";
export type Arch = "arm64" | "x64";
export type Platform = { os: Os; arch: Arch; archKnown: boolean };

/** What the browser tells us. `hints` comes from `navigator.userAgentData` (Chromium only). */
export type BrowserInfo = {
  userAgent: string;
  maxTouchPoints?: number;
  hints?: { platform?: string; architecture?: string; bitness?: string };
};

const MOBILE = /android|iphone|ipad|ipod|mobile|cros/i;

function osFrom(info: BrowserInfo): Os | null {
  const platform = info.hints?.platform?.toLowerCase() ?? "";
  const ua = info.userAgent;
  if (MOBILE.test(ua) || platform === "android" || platform === "chrome os") return null;
  if (platform === "macos" || /macintosh|mac os x/i.test(ua)) {
    // iPadOS Safari reports a Mac user agent; touch support gives it away.
    return (info.maxTouchPoints ?? 0) > 1 ? null : "macos";
  }
  if (platform === "windows" || /windows/i.test(ua)) return "windows";
  if (platform === "linux" || /linux|x11/i.test(ua)) return "linux";
  return null;
}

function archFrom(info: BrowserInfo): Arch | null {
  const hint = info.hints?.architecture?.toLowerCase();
  if (hint === "arm") return "arm64";
  if (hint === "x86") return info.hints?.bitness === "32" ? null : "x64";
  const ua = info.userAgent;
  if (/aarch64|arm64|armv8/i.test(ua)) return "arm64";
  // Mac browsers say "Intel Mac OS X" on Apple silicon too, so only Linux and Windows UAs count.
  if (!/macintosh/i.test(ua) && /x86_64|x64|win64|amd64|wow64/i.test(ua)) return "x64";
  return null;
}

/** Most Macs sold since 2020 are Apple silicon; most Windows and Linux desktops are x64. */
const DEFAULT_ARCH: Record<Os, Arch> = { macos: "arm64", windows: "x64", linux: "x64" };

export function detectPlatform(info: BrowserInfo): Platform | null {
  const os = osFrom(info);
  if (!os) return null;
  const arch = archFrom(info);
  return { os, arch: arch ?? DEFAULT_ARCH[os], archKnown: arch !== null };
}
