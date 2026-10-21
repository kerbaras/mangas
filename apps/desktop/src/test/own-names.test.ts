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

describe("ownNames around kept books (Try again, #337)", () => {
  it("never names a book as a kept file, like volume::retry_plans", () => {
    expect(ownNames(["X ch 1.epub", "X ch 1.epub"], ["X ch 1.epub"])).toEqual([
      "X ch 1 (2).epub",
      "X ch 1 (3).epub",
    ]);
  });

  it("skips a name whose first part a split kept book holds", () => {
    expect(ownNames(["X.epub"], ["X (part 1).epub"])).toEqual(["X (2).epub"]);
  });
});
