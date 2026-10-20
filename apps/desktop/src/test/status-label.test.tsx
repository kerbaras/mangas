import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SearchResult } from "@/components/search-result";
import type { Manga } from "@/lib/ipc";

const manga = (status: Manga["status"]): Manga => ({
  id: "100269",
  source: "mangaplus",
  title: "Boruto: Two Blue Vortex",
  description: "",
  cover_url: null,
  status,
});

function result(status: Manga["status"]) {
  render(
    <SearchResult
      manga={manga(status)}
      followed={false}
      busy={false}
      onFollow={vi.fn()}
      onOpen={vi.fn()}
    />,
  );
}

describe("search result status", () => {
  it("shows the status the source gives", () => {
    result("ongoing");
    expect(screen.getByText("Ongoing")).toBeInTheDocument();
  });

  it("shows no label when the source doesn't say", () => {
    result("unknown");
    expect(screen.queryByText("Unknown")).not.toBeInTheDocument();
  });
});
