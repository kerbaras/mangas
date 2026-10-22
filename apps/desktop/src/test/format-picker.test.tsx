import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FormatPicker } from "@/components/format-picker";
import { TooltipProvider } from "@/components/ui/tooltip";

const picker = (onChange = vi.fn()) => (
  <TooltipProvider>
    <FormatPicker value="epub" onChange={onChange} />
  </TooltipProvider>
);

describe("FormatPicker", () => {
  it("checks the current format and reports a new pick", () => {
    const onChange = vi.fn();
    render(picker(onChange));
    expect(screen.getByRole("radio", { name: "EPUB" })).toBeChecked();
    fireEvent.click(screen.getByRole("radio", { name: "CBZ" }));
    expect(onChange).toHaveBeenCalledWith("cbz");
  });

  it("says what a format is for on hover", async () => {
    render(picker());
    await userEvent.setup().hover(screen.getByText("AZW3 (Kindle)"));
    expect((await screen.findByRole("tooltip")).textContent).toBe("Kindle book to copy over USB.");
  });
});
