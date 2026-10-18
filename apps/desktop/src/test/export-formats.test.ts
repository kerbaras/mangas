import { describe, expect, it } from "vitest";
import { effectiveFormat, FORMATS, formatLabel, isFormat } from "@/lib/export-formats";
import { sampleDevices } from "@/lib/mock-devices";

describe("export formats", () => {
  it("lists epub, cbz and pdf", () => {
    expect(FORMATS.map((f) => f.id)).toEqual(["epub", "cbz", "pdf"]);
    expect(isFormat("cbz")).toBe(true);
    expect(isFormat("mobi")).toBe(false);
    expect(isFormat(null)).toBe(false);
    expect(formatLabel("pdf")).toBe("PDF");
  });

  it("follows the device until a format is picked", () => {
    expect(effectiveFormat(null, sampleDevices, "kobo-clara")).toBe("cbz");
    expect(effectiveFormat(null, sampleDevices, "kindle-scribe")).toBe("epub");
    expect(effectiveFormat(null, sampleDevices, "remarkable2")).toBe("pdf");
    expect(effectiveFormat("pdf", sampleDevices, "kobo-clara")).toBe("pdf");
    expect(effectiveFormat(null, [], "kobo-clara")).toBe("epub");
  });

  it("mock profiles cover the required devices", () => {
    const ids = sampleDevices.map((d) => d.id);
    for (const id of ["kindle-paperwhite12", "kindle-scribe", "kobo-clara", "kobo-libra", "tablet"])
      expect(ids).toContain(id);
  });
});
