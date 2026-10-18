import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useMangaDetails } from "@/hooks/use-manga-details";
import { type Backend, type Manga, setBackend } from "@/lib/ipc";

const card: Manga = {
  id: "en-webtoon-95",
  source: "webtoons",
  title: "Tower of God",
  description: "By SIU",
  cover_url: null,
  status: "unknown",
};
const stored: Manga = { ...card, description: "Reach the top of the tower.", status: "ongoing" };

function backendWith(getManga: Backend["getManga"]) {
  setBackend({ getManga } as Backend);
}

afterEach(() => setBackend(null));

describe("useMangaDetails", () => {
  it("shows the card while loading, then the stored record", async () => {
    let resolve: (m: Manga) => void = () => {};
    const pending = new Promise<Manga>((r) => (resolve = r));
    backendWith(() => pending);
    const { result } = renderHook(() => useMangaDetails(card));
    expect(result.current).toEqual({ manga: card, loading: true });
    resolve(stored);
    await waitFor(() => expect(result.current).toEqual({ manga: stored, loading: false }));
  });
  it("asks for the card's source and id", async () => {
    const calls: string[] = [];
    backendWith(async (source, id) => {
      calls.push(`${source}:${id}`);
      return stored;
    });
    renderHook(() => useMangaDetails(card));
    await waitFor(() => expect(calls).toEqual(["webtoons:en-webtoon-95"]));
  });
  it("keeps the card when the details can't be loaded", async () => {
    backendWith(async () => {
      throw new Error("network: down");
    });
    const { result } = renderHook(() => useMangaDetails(card));
    await waitFor(() => expect(result.current).toEqual({ manga: card, loading: false }));
  });
});
