import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProviderIcon } from "@/components/provider-icon";

const provider = { name: "MangaDot", icon_url: "https://mangadot.net/icon-192.png" };

describe("ProviderIcon", () => {
  it("loads the icon without a Referer, which hotlink-protected hosts refuse", () => {
    const { container } = render(<ProviderIcon provider={provider} />);
    const img = container.querySelector("img");
    expect(img?.getAttribute("src")).toBe(provider.icon_url);
    expect(img?.getAttribute("referrerpolicy")).toBe("no-referrer");
  });

  it("falls back to the initial when the icon fails to load", () => {
    const { container, getByText } = render(<ProviderIcon provider={provider} />);
    fireEvent.error(container.querySelector("img") as HTMLImageElement);
    expect(container.querySelector("img")).toBeNull();
    expect(getByText("M")).toBeTruthy();
  });
});
