import { describe, expect, it } from "vitest";
import { type Asset, installer, installers, kind, megabytes, version } from "./assets";

// Asset names of the v0.3.0 release, plus the .msi names tauri-action uses when WiX runs.
const NAMES = [
  "Mangas-0.3.0-1.aarch64.rpm",
  "Mangas-0.3.0-1.x86_64.rpm",
  "Mangas_0.3.0_aarch64.AppImage",
  "Mangas_0.3.0_aarch64.dmg",
  "Mangas_0.3.0_amd64.AppImage",
  "Mangas_0.3.0_amd64.deb",
  "Mangas_0.3.0_arm64-setup.exe",
  "Mangas_0.3.0_arm64.deb",
  "Mangas_0.3.0_x64-setup.exe",
  "Mangas_0.3.0_x64.dmg",
  "Mangas_aarch64.app.tar.gz",
  "Mangas_x64.app.tar.gz",
];
const asset = (name: string): Asset => ({
  name,
  browser_download_url: `https://x/${name}`,
  size: 1,
});
const ASSETS = NAMES.map(asset);

describe("installer", () => {
  it.each([
    ["macos-arm64", "Mangas_0.3.0_aarch64.dmg"],
    ["macos-x64", "Mangas_0.3.0_x64.dmg"],
    ["windows-x64", "Mangas_0.3.0_x64-setup.exe"],
    ["windows-arm64", "Mangas_0.3.0_arm64-setup.exe"],
    ["linux-x64", "Mangas_0.3.0_amd64.AppImage"],
    ["linux-arm64", "Mangas_0.3.0_aarch64.AppImage"],
  ] as const)("picks the %s installer", (target, name) => {
    expect(installer(ASSETS, target)?.name).toBe(name);
  });

  it("lists every Linux package, AppImage first", () => {
    expect(installers(ASSETS, "linux-arm64").map((a) => a.name)).toEqual([
      "Mangas_0.3.0_aarch64.AppImage",
      "Mangas_0.3.0_arm64.deb",
      "Mangas-0.3.0-1.aarch64.rpm",
    ]);
  });

  it("falls back to the .msi when a release has no setup .exe", () => {
    const msi = [asset("Mangas_0.4.0_x64_en-US.msi"), asset("Mangas_0.4.0_arm64_en-US.msi")];
    expect(installer(msi, "windows-x64")?.name).toBe("Mangas_0.4.0_x64_en-US.msi");
    expect(installer(msi, "windows-arm64")?.name).toBe("Mangas_0.4.0_arm64_en-US.msi");
  });

  it("returns null when the release has no build for the target", () => {
    expect(installer([asset("Mangas_x64.app.tar.gz")], "macos-x64")).toBeNull();
  });
});

describe("labels", () => {
  it("names the file type", () => {
    expect(kind("Mangas_0.3.0_x64-setup.exe")).toBe(".exe");
    expect(kind("Mangas_0.3.0_amd64.AppImage")).toBe(".AppImage");
    expect(kind("Mangas-0.3.0-1.x86_64.rpm")).toBe(".rpm");
  });

  it("formats sizes and versions", () => {
    expect(megabytes(14_412_000)).toBe("14 MB");
    expect(megabytes(10)).toBe("1 MB");
    expect(version("v0.3.0")).toBe("0.3.0");
  });
});
