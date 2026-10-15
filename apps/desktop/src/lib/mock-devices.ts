import type { Device } from "./ipc";

/** Same list as `mangas_core::export::kindle::DEVICES`. */
export const sampleDevices: Device[] = [
  {
    id: "kindle-paperwhite12",
    name: "Kindle Paperwhite (12th gen) / Signature",
    width: 1264,
    height: 1680,
    dpi: 300,
    grayscale: true,
  },
  {
    id: "kindle-paperwhite5",
    name: "Kindle Paperwhite (11th gen) / Signature",
    width: 1236,
    height: 1648,
    dpi: 300,
    grayscale: true,
  },
  {
    id: "kindle-paperwhite3",
    name: "Kindle Paperwhite 3/4",
    width: 1072,
    height: 1448,
    dpi: 300,
    grayscale: true,
  },
  {
    id: "kindle-scribe",
    name: "Kindle Scribe",
    width: 1860,
    height: 2480,
    dpi: 300,
    grayscale: true,
  },
  {
    id: "kindle-oasis3",
    name: "Kindle Oasis 3",
    width: 1264,
    height: 1680,
    dpi: 300,
    grayscale: true,
  },
];
