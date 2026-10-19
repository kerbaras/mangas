import { describe, expect, it } from "vitest";
import { ownNames } from "@/hooks/use-free-names";

describe("ownNames", () => {
  it("numbers repeated names like the replace export writes them", () => {
    expect(ownNames(["Book.epub", "book.epub", "Book (2).epub", "Other.epub"])).toEqual([
      "Book.epub",
      "book (2).epub",
      "Book (2) (2).epub",
      "Other.epub",
    ]);
  });
});
