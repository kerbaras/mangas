import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Cover } from "@/components/cover";
import { resetCovers } from "@/lib/covers";
import { type Backend, type Manga, setBackend } from "@/lib/ipc";

const manga: Manga = {
  id: "m1",
  source: "comick",
  title: "Solo Leveling",
  description: "",
  cover_url: "https://comicknew.pictures/c.jpg",
  status: "ongoing",
};

function useBackend(coverSrc: Backend["coverSrc"]) {
  setBackend({ coverSrc } as Backend);
}

const image = (container: HTMLElement) => container.querySelector("img");
const placeholder = () => screen.queryByTestId("cover-placeholder");

describe("Cover", () => {
  beforeEach(() => resetCovers());
  afterEach(() => setBackend(null));

  it("shows the title until the cached cover has loaded, then the image", async () => {
    const coverSrc = vi.fn(async () => "asset://localhost/covers/m1.jpg");
    useBackend(coverSrc);
    const { container } = render(<Cover manga={manga} />);
    expect(placeholder()).toHaveTextContent("Solo Leveling");
    await act(async () => {});
    const img = image(container);
    expect(img).toHaveAttribute("src", "asset://localhost/covers/m1.jpg");
    expect(img).toHaveClass("opacity-0");
    fireEvent.load(img as HTMLImageElement);
    expect(img).toHaveClass("opacity-100");
    expect(placeholder()).toBeNull();
    expect(coverSrc).toHaveBeenCalledWith({
      source: "comick",
      id: "m1",
      cover_url: manga.cover_url,
    });
  });

  it("keeps the placeholder when the cover cannot be fetched", async () => {
    useBackend(async () => {
      throw new Error("source comick: HTTP 403 for cover");
    });
    const { container } = render(<Cover manga={manga} />);
    await act(async () => {});
    expect(image(container)).toBeNull();
    expect(placeholder()).toHaveTextContent("Solo Leveling");
  });

  it("drops a broken image for the placeholder", async () => {
    useBackend(async () => "asset://localhost/covers/missing.jpg");
    const { container } = render(<Cover manga={manga} />);
    await act(async () => {});
    fireEvent.error(image(container) as HTMLImageElement);
    expect(image(container)).toBeNull();
    expect(placeholder()).toBeInTheDocument();
  });

  it("asks the backend once per cover across screens", async () => {
    const coverSrc = vi.fn(async () => "asset://localhost/covers/m1.jpg");
    useBackend(coverSrc);
    render(
      <>
        <Cover manga={manga} />
        <Cover manga={{ ...manga }} />
      </>,
    );
    await act(async () => {});
    expect(coverSrc).toHaveBeenCalledTimes(1);
  });

  it("does not call the backend for a manga without a cover", async () => {
    const coverSrc = vi.fn(async () => null);
    useBackend(coverSrc);
    render(<Cover manga={{ ...manga, cover_url: null }} />);
    await act(async () => {});
    expect(coverSrc).not.toHaveBeenCalled();
    expect(placeholder()).toBeInTheDocument();
  });
});
