import { describe, expect, it } from "vitest";
import { type Asset, installer, installers, kind, megabytes, version } from "./assets";

// Asset names of a release. Up to 0.5.x tauri-action used the product name "Mangas"; from 0.6.0
// it is "mangas". Older releases keep their names, so both must resolve.
const names = (p: string, v: string) => [
  `${p}-${v}-1.aarch64.rpm`,
  `${p}-${v}-1.x86_64.rpm`,
  `${p}_${v}_aarch64.AppImage`,
  `${p}_${v}_aarch64.dmg`,
  `${p}_${v}_amd64.AppImage`,
  `${p}_${v}_amd64.deb`,
  `${p}_${v}_arm64-setup.exe`,
  `${p}_${v}_arm64.deb`,
  `${p}_${v}_x64-setup.exe`,
  `${p}_${v}_x64.dmg`,
  `${p}_aarch64.app.tar.gz`,
  `${p}_x64.app.tar.gz`,
];
const asset = (name: string): Asset => ({
  name,
  browser_download_url: `https://x/${name}`,
  size: 1,
});
const RELEASES = [
  ["old (Mangas_…)", "Mangas", "0.3.0"],
  ["new (mangas_…)", "mangas", "0.6.0"],
] as const;

describe.each(RELEASES)("installer, %s names", (_, p, v) => {
  const assets = names(p, v).map(asset);

  it.each([
    ["macos-arm64", `${p}_${v}_aarch64.dmg`],
    ["macos-x64", `${p}_${v}_x64.dmg`],
    ["windows-x64", `${p}_${v}_x64-setup.exe`],
    ["windows-arm64", `${p}_${v}_arm64-setup.exe`],
    ["linux-x64", `${p}_${v}_amd64.AppImage`],
    ["linux-arm64", `${p}_${v}_aarch64.AppImage`],
  ] as const)("picks the %s installer", (target, name) => {
    expect(installer(assets, target)?.name).toBe(name);
  });

  it("lists every Linux package, AppImage first", () => {
    expect(installers(assets, "linux-arm64").map((a) => a.name)).toEqual([
      `${p}_${v}_aarch64.AppImage`,
      `${p}_${v}_arm64.deb`,
      `${p}-${v}-1.aarch64.rpm`,
    ]);
    expect(installers(assets, "linux-x64").map((a) => a.name)).toEqual([
      `${p}_${v}_amd64.AppImage`,
      `${p}_${v}_amd64.deb`,
      `${p}-${v}-1.x86_64.rpm`,
    ]);
  });

  it("falls back to the .msi when a release has no setup .exe", () => {
    const msi = [asset(`${p}_${v}_x64_en-US.msi`), asset(`${p}_${v}_arm64_en-US.msi`)];
    expect(installer(msi, "windows-x64")?.name).toBe(`${p}_${v}_x64_en-US.msi`);
    expect(installer(msi, "windows-arm64")?.name).toBe(`${p}_${v}_arm64_en-US.msi`);
  });

  it("returns null when the release has no build for the target", () => {
    expect(installer([asset(`${p}_x64.app.tar.gz`)], "macos-x64")).toBeNull();
  });

  it("names the file type", () => {
    expect(kind(`${p}_${v}_x64-setup.exe`)).toBe(".exe");
    expect(kind(`${p}_${v}_amd64.AppImage`)).toBe(".AppImage");
    expect(kind(`${p}-${v}-1.x86_64.rpm`)).toBe(".rpm");
  });
});

describe("labels", () => {
  it("formats sizes and versions", () => {
    expect(megabytes(14_412_000)).toBe("14 MB");
    expect(megabytes(10)).toBe("1 MB");
    expect(version("v0.3.0")).toBe("0.3.0");
  });
});
