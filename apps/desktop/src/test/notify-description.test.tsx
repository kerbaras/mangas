import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Description } from "@/lib/notify";

const refused = (address: string) =>
  `refused: webtoon-phinf.pstatic.net resolves to a non-public address ${address}`;

function detail(text: string) {
  render(<Description hint="The image host points at a local or private address." detail={text} />);
  const body = document.querySelector<HTMLElement>(".max-h-12");
  if (!body) throw new Error("no detail line");
  expect(body).not.toHaveAttribute("title");
  return body;
}

describe("toast description", () => {
  it.each(["10.0.0.1", "::ffff:10.0.0.1", "fd00::1"])(
    "wraps on word boundaries and keeps %s on one line",
    (address) => {
      const body = detail(refused(address));
      const toastBody = body.parentElement;
      expect(toastBody).toHaveClass("break-words");
      for (const el of [toastBody, body]) expect(el?.className).not.toMatch(/break-all|line-clamp/);
      const ip = screen.getByText(address);
      expect(ip).toHaveClass("whitespace-nowrap");
      expect(ip.tagName).toBe("SPAN");
      expect(body).toHaveTextContent(refused(address));
    },
  );

  it("leaves other errors as plain text", () => {
    const body = detail("network: connection reset");
    expect(body.parentElement).toHaveClass("break-words");
    expect(body.querySelector(".whitespace-nowrap")).toBeNull();
  });

  it("shows the whole error when full", () => {
    const text = "can't eject KINDLE: it's in use by tail. The books copied to it are still there";
    render(<Description hint={null} detail={text} full />);
    const body = screen.getByText(text);
    expect(body).not.toHaveClass("max-h-12");
    expect(body).not.toHaveClass("overflow-hidden");
  });
});
