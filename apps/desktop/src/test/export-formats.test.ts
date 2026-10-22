import { describe, expect, it } from "vitest";
import {
  deliveryFormat,
  effectiveFormat,
  FORMATS,
  formatLabel,
  isFormat,
} from "@/lib/export-formats";
import { sampleDevices } from "@/lib/mock-devices";

describe("export formats", () => {
  it("lists epub, azw3, cbz and pdf", () => {
    expect(FORMATS.map((f) => f.id)).toEqual(["epub", "azw3", "cbz", "pdf"]);
    expect(isFormat("azw3")).toBe(true);
    expect(formatLabel("azw3")).toBe("AZW3 (Kindle)");
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

  it("gives a Kindle AZW3 over USB and EPUB by email, other devices nothing", () => {
    const kindle = sampleDevices.find((d) => d.id === "kindle-scribe");
    const kobo = sampleDevices.find((d) => d.id === "kobo-clara");
    expect(deliveryFormat(kindle, "usb")).toBe("azw3");
    expect(deliveryFormat(kindle, "email")).toBe("epub");
    expect(deliveryFormat(kindle, "save")).toBeUndefined();
    expect(deliveryFormat(kobo, "usb")).toBeUndefined();
    expect(kindle?.delivery?.reason).toBe("Kindle over USB: AZW3; Email: EPUB");
  });

  it("mock profiles cover the required devices", () => {
    const ids = sampleDevices.map((d) => d.id);
    for (const id of ["kindle-paperwhite12", "kindle-scribe", "kobo-clara", "kobo-libra", "tablet"])
      expect(ids).toContain(id);
  });
});
