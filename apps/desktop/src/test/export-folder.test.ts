import { describe, expect, it } from "vitest";
import { exportFolder } from "@/components/output-folder";

const kindle = "/home/me/.local/share/mangas/kindle";

describe("exportFolder", () => {
  it("is the default export folder when nothing is typed", () => {
    expect(exportFolder(kindle, null)).toBe(kindle);
    expect(exportFolder(kindle, "  ")).toBe(kindle);
  });

  it("puts a relative folder under the default export folder", () => {
    expect(exportFolder(kindle, "books")).toBe(`${kindle}/books`);
    expect(exportFolder(kindle, "~/books")).toBe(`${kindle}/~/books`);
  });

  it("keeps an absolute folder as typed", () => {
    expect(exportFolder(kindle, "/mnt/kindle")).toBe("/mnt/kindle");
    expect(exportFolder(kindle, "C:\\Books")).toBe("C:\\Books");
    expect(exportFolder(kindle, "\\\\nas\\books")).toBe("\\\\nas\\books");
  });
});
