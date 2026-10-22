import type { Device, ExportFormat } from "./ipc";

type Row = [string, string, number, number, number, boolean, ExportFormat];

const kindleDelivery: Device["delivery"] = {
  usb: "azw3",
  email: "epub",
  reason: "Kindle over USB: AZW3; Email: EPUB",
};

/** Same table as `mangas_core::export::PROFILES`. */
const rows: Row[] = [
  [
    "kindle-paperwhite12",
    "Kindle Paperwhite (12th gen) / Signature",
    1264,
    1680,
    300,
    true,
    "epub",
  ],
  ["kindle-paperwhite5", "Kindle Paperwhite (11th gen) / Signature", 1236, 1648, 300, true, "epub"],
  ["kindle-paperwhite3", "Kindle Paperwhite 3/4", 1072, 1448, 300, true, "epub"],
  ["kindle-scribe", "Kindle Scribe", 1860, 2480, 300, true, "epub"],
  ["kindle-oasis3", "Kindle Oasis 3", 1264, 1680, 300, true, "epub"],
  ["kobo-clara", "Kobo Clara BW / 2E / HD", 1072, 1448, 300, true, "cbz"],
  ["kobo-clara-colour", "Kobo Clara Colour", 1072, 1448, 300, false, "cbz"],
  ["kobo-libra", "Kobo Libra 2 / H2O", 1264, 1680, 300, true, "cbz"],
  ["kobo-libra-colour", "Kobo Libra Colour", 1264, 1680, 300, false, "cbz"],
  ["kobo-elipsa", "Kobo Elipsa 2E", 1404, 1872, 227, true, "cbz"],
  ["remarkable2", "reMarkable 2", 1404, 1872, 226, true, "pdf"],
  ["tablet", "Generic tablet (colour)", 1536, 2048, 264, false, "cbz"],
];

export const sampleDevices: Device[] = rows.map(
  ([id, name, width, height, dpi, grayscale, format]) => ({
    id,
    name,
    width,
    height,
    dpi,
    grayscale,
    format,
    // Same rule as `export::delivery_formats`.
    delivery: id.startsWith("kindle-") ? kindleDelivery : null,
  }),
);
