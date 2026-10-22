import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MangaMeta } from "@/components/manga-meta";
import type { Manga } from "@/lib/ipc";
import { bookCreators, creditsLine, visibleTags } from "@/lib/manga-meta";

const base: Manga = {
  id: "solo",
  source: "comick",
  title: "Solo Leveling",
  description: "",
  cover_url: null,
  status: "completed",
};

describe("creditsLine", () => {
  it("joins the year, authors and art-only credits", () => {
    const m = { ...base, authors: ["Chugong"], artists: ["Jang Sung-Rak"], year: 2018 };
    expect(creditsLine(m)).toBe("2018 · Chugong · Art by Jang Sung-Rak");
  });
  it("doesn't repeat a mangaka who is author and artist", () => {
    const m = { ...base, authors: ["Oda Eiichiro"], artists: ["Oda Eiichiro"] };
    expect(creditsLine(m)).toBe("Oda Eiichiro");
  });
  it("is empty when the source gave nothing", () => {
    expect(creditsLine(base)).toBe("");
    expect(creditsLine({ ...base, authors: [], year: null })).toBe("");
  });
});

describe("bookCreators", () => {
  it("lists authors, then artists not already named, trimmed and once each", () => {
    const m = { ...base, authors: ["Oda Eiichiro", " "], artists: [" Oda Eiichiro ", "Studio"] };
    expect(bookCreators(m)).toEqual(["Oda Eiichiro", "Studio"]);
  });
  it("is empty, not a placeholder, when the source names nobody", () => {
    expect(bookCreators(base)).toEqual([]);
  });
});

describe("visibleTags", () => {
  it("caps the list and counts the rest", () => {
    const m = { ...base, tags: ["a", "b", "c"] };
    expect(visibleTags(m, 2)).toEqual({ shown: ["a", "b"], hidden: 1 });
    expect(visibleTags(base, 2)).toEqual({ shown: [], hidden: 0 });
  });
});

describe("MangaMeta", () => {
  it("renders credits and tags", () => {
    render(<MangaMeta manga={{ ...base, authors: ["Chugong"], tags: ["Action", "Fantasy"] }} />);
    expect(screen.getByText("Chugong")).toBeTruthy();
    expect(screen.getByRole("list", { name: "Tags" }).textContent).toBe("ActionFantasy");
  });
  it("renders nothing without metadata", () => {
    const { container } = render(<MangaMeta manga={base} />);
    expect(container.innerHTML).toBe("");
  });
});
