import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { FormatPicker } from "@/components/format-picker";

describe("FormatPicker", () => {
  it("checks the current format and reports a new pick", () => {
    const onChange = vi.fn();
    render(<FormatPicker value="epub" onChange={onChange} />);
    expect(screen.getByRole("radio", { name: /EPUB/ })).toBeChecked();
    fireEvent.click(screen.getByRole("radio", { name: /CBZ/ }));
    expect(onChange).toHaveBeenCalledWith("cbz");
  });
});
