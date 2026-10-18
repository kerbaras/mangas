// The page's only script: resolve the latest release's installer for this visitor. Without it
// (or when GitHub's API fails or rate-limits) every link keeps pointing at the releases page.
import {
  installer,
  installers,
  kind,
  megabytes,
  type Release,
  type Target,
  version,
} from "./assets";
import { type BrowserInfo, detectPlatform, type Platform } from "./platform";

const API = "https://api.github.com/repos/kerbaras/mangas/releases/latest";
const CACHE_KEY = "mangas:latest-release";
const CACHE_MS = 10 * 60 * 1000;

const OS_NAME = { macos: "macOS", windows: "Windows", linux: "Linux" } as const;
const ARCH_NAME: Record<Target, string> = {
  "macos-arm64": "Apple silicon",
  "macos-x64": "Intel",
  "windows-x64": "x64",
  "windows-arm64": "Arm",
  "linux-x64": "x64",
  "linux-arm64": "Arm",
};

async function browserInfo(): Promise<BrowserInfo> {
  type UaData = { getHighEntropyValues(keys: string[]): Promise<BrowserInfo["hints"]> };
  const uaData = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  const hints = await uaData
    ?.getHighEntropyValues(["platform", "architecture", "bitness"])
    .catch(() => undefined);
  return { userAgent: navigator.userAgent, maxTouchPoints: navigator.maxTouchPoints, hints };
}

function cached(): Release | null {
  try {
    const hit = JSON.parse(sessionStorage.getItem(CACHE_KEY) ?? "null");
    return hit && Date.now() - hit.at < CACHE_MS ? hit.release : null;
  } catch {
    return null;
  }
}

async function latestRelease(): Promise<Release | null> {
  const hit = cached();
  if (hit) return hit;
  try {
    const res = await fetch(API, {
      headers: { Accept: "application/vnd.github+json" },
      signal: AbortSignal.timeout(8000),
    });
    if (!res.ok) return null;
    const release: Release = await res.json();
    sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), release }));
    return release;
  } catch {
    return null;
  }
}

function setText(selector: string, text: string) {
  for (const el of document.querySelectorAll<HTMLElement>(selector)) el.textContent = text;
}

function fillAllDownloads(release: Release) {
  for (const link of document.querySelectorAll<HTMLAnchorElement>("a[data-target]")) {
    const target = link.dataset.target as Target;
    const kindFilter = link.dataset.kind;
    const match = installers(release.assets, target).find(
      (a) => !kindFilter || kind(a.name) === kindFilter,
    );
    if (match) link.href = match.browser_download_url;
  }
}

function fillPrimary(release: Release, platform: Platform) {
  const target: Target = `${platform.os}-${platform.arch}`;
  const asset = installer(release.assets, target);
  const button = document.querySelector<HTMLAnchorElement>("#download");
  if (!asset || !button) return;
  button.href = asset.browser_download_url;
  setText("#download-label", `Download for ${OS_NAME[platform.os]}`);
  setText(
    "#download-meta",
    `${ARCH_NAME[target]} · ${kind(asset.name)} · ${megabytes(asset.size)}`,
  );
  // Browsers can't tell an Intel Mac from Apple silicon (or Windows x64 from Arm) without
  // client hints, so point at the other build when we guessed.
  const other: Target = `${platform.os}-${platform.arch === "arm64" ? "x64" : "arm64"}`;
  const alt = document.querySelector<HTMLAnchorElement>("#download-alt");
  const otherAsset = installer(release.assets, other);
  if (!platform.archKnown && alt && otherAsset) {
    alt.href = otherAsset.browser_download_url;
    alt.textContent = `${OS_NAME[platform.os]} ${ARCH_NAME[other]} build`;
    alt.hidden = false;
  }
}

async function main() {
  const [release, info] = await Promise.all([latestRelease(), browserInfo()]);
  if (!release) return;
  setText("[data-version]", `Version ${version(release.tag_name)}`);
  fillAllDownloads(release);
  const platform = detectPlatform(info);
  if (platform) fillPrimary(release, platform);
}

main();
