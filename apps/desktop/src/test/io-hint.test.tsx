import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ExportProgressPanel } from "@/components/export-progress";
import { ExportStoppedPanel } from "@/components/export-stopped-panel";
import { describeError } from "@/lib/errors";
import { ExportStopped } from "@/lib/export-stopped";
import { ioHint, ioRest } from "@/lib/io-hint";

const DENIED = "Permission denied (os error 13)";
const folders = { data: "/home/u/.local/share/mangas", export: "/media/kindle/books" };

describe("ioHint", () => {
  it("names the export folder when the export folder failed", () => {
    const hint = describeError(`io: /media/kindle/books: ${DENIED}`, folders).hint;
    expect(hint).toBe(
      "The export folder /media/kindle/books couldn't be written. Check its permissions or choose another folder.",
    );
  });
  it("names the data folder for a file inside it", () => {
    const message = `io: /home/u/.local/share/mangas/chapters/x/1: ${DENIED}`;
    expect(describeError(message, folders).hint).toMatch(
      /^A file in the data folder \/home\/u\/.local\/share\/mangas couldn't/,
    );
  });
  it("prefers the export folder when it sits in the data folder", () => {
    const inData = { data: "/d", export: "/d/kindle/" };
    expect(describeError(`io: /d/kindle/a.epub: ${DENIED}`, inData).hint).toMatch(
      /^The export folder \/d\/kindle couldn't/,
    );
  });
  it("drops the hint outside both folders and for a sibling with the same prefix", () => {
    expect(describeError(`io: /tmp/ro: ${DENIED}`, folders).hint).toBeNull();
    expect(describeError(`io: /media/kindle/books2: ${DENIED}`, folders).hint).toBeNull();
  });
  it("names a root export folder", () => {
    const hint = ioHint("/b.epub: Permission denied (os error 13)", { export: "/" });
    expect(hint).toContain("The export folder / couldn't be written");
  });

  it("finds the io error inside a wrapped message", () => {
    expect(ioRest(`Book failed: io: /a: ${DENIED}`)).toBe(`/a: ${DENIED}`);
    expect(ioRest("network: io: nope")).toBe("nope");
    expect(ioRest("library: no such table")).toBeNull();
    expect(ioHint(`/a: ${DENIED}`, {})).toBeNull();
  });
});

const reason = `io: /media/kindle/books: ${DENIED}`;
const exportHint = /The export folder \/media\/kindle\/books couldn't be written/;

describe("export panels", () => {
  it("show the folder hint for a retryable io failure, with Try again", () => {
    const state = { kind: "error", message: reason, folder: "/media/kindle/books" } as const;
    render(<ExportProgressPanel state={state} onRetry={() => {}} />);
    expect(screen.getByText(exportHint)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });
  it("show it when a volume export stopped at a book", () => {
    const failed = { title: "Vol 1", output: "/media/kindle/books/Vol 1.epub", reason };
    const stopped = new ExportStopped({ written: [], not_written: [], failed });
    expect(stopped.message).toBe(`Vol 1 failed: ${reason}`);
    expect(describeError(stopped.message, { export: "/media/kindle/books" }).hint).toMatch(
      exportHint,
    );
    render(<ExportStoppedPanel stopped={stopped} onReset={() => {}} />);
    expect(screen.getByText(exportHint)).toBeTruthy();
  });
});
