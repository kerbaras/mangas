import type { Arch, Os } from "./platform";

export type Asset = { name: string; browser_download_url: string; size: number };
export type Release = { tag_name: string; html_url: string; assets: Asset[] };
export type Target = `${Os}-${Arch}`;

/** Installer patterns per target, best first. Names come from tauri-action in release.yml:
 *  Mangas_0.3.0_aarch64.dmg, Mangas_0.3.0_x64-setup.exe, Mangas_0.3.0_x64_en-US.msi,
 *  Mangas_0.3.0_amd64.AppImage, Mangas_0.3.0_arm64.deb, Mangas-0.3.0-1.x86_64.rpm. */
const PATTERNS: Record<Target, RegExp[]> = {
  "macos-arm64": [/_(aarch64|arm64)\.dmg$/],
  "macos-x64": [/_(x64|x86_64)\.dmg$/],
  "windows-x64": [/_x64-setup\.exe$/, /_x64_[\w-]+\.msi$/],
  "windows-arm64": [/_arm64-setup\.exe$/, /_arm64_[\w-]+\.msi$/],
  "linux-x64": [/_amd64\.AppImage$/, /_amd64\.deb$/, /\.x86_64\.rpm$/],
  "linux-arm64": [/_aarch64\.AppImage$/, /_arm64\.deb$/, /\.aarch64\.rpm$/],
};

/** Every installer for a target, in the order of `PATTERNS`. */
export function installers(assets: Asset[], target: Target): Asset[] {
  return PATTERNS[target].flatMap((re) => assets.filter((a) => re.test(a.name)));
}

export function installer(assets: Asset[], target: Target): Asset | null {
  return installers(assets, target)[0] ?? null;
}

/** ".dmg", "-setup.exe", ".AppImage": the part a person recognises. */
export function kind(name: string): string {
  if (name.endsWith("-setup.exe")) return ".exe";
  return name.slice(name.lastIndexOf("."));
}

export function megabytes(bytes: number): string {
  return `${Math.max(1, Math.round(bytes / 1_000_000))} MB`;
}

export function version(tag: string): string {
  return tag.startsWith("v") ? tag.slice(1) : tag;
}
