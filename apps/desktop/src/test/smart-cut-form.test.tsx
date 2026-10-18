import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { SmartCutForm } from "@/components/smart-cut-form";
import { compact, type ExportPreset, isEmptyPreset } from "@/lib/ipc.cut-preview";

function Harness({
  onChange,
  initial = {},
}: {
  onChange: (v: ExportPreset) => void;
  initial?: ExportPreset;
}) {
  const [value, setValue] = useState<ExportPreset>(initial);
  return (
    <>
      <SmartCutForm
        value={value}
        onChange={(v) => {
          setValue(v);
          onChange(v);
        }}
        basePreset="balanced"
        grayscale
      />
      <button type="button" onClick={() => setValue({})}>
        Reset
      </button>
    </>
  );
}

describe("SmartCutForm", () => {
  it("shows defaults for unset options", () => {
    render(<Harness onChange={() => {}} />);
    expect(screen.getByLabelText("Target page height")).toHaveValue("1");
    expect(screen.getByText("100% of the screen (default)")).toBeInTheDocument();
    expect(screen.getByText("up to 10% (default)")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Trim margins" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "Sharpen" })).toBeChecked();
  });

  it("reports each tweak as a preset field", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("Target page height"), { target: { value: "0.7" } });
    expect(onChange).toHaveBeenLastCalledWith({ page_height: 0.7 });
    expect(screen.getByText("70% of the screen")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Overlap after a forced cut"), {
      target: { value: "0.1" },
    });
    await userEvent.click(screen.getByRole("switch", { name: "Trim margins" }));
    expect(onChange).toHaveBeenLastCalledWith({
      page_height: 0.7,
      overlap: 0.1,
      trim_margins: true,
    });
  });

  it("starts from a saved preset and resets to defaults", async () => {
    render(<Harness onChange={() => {}} initial={{ page_height: 0.6, gamma: 1.2 }} />);
    expect(screen.getByLabelText("Target page height")).toHaveValue("0.6");
    expect(screen.getByText("1.20")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(screen.getByText("100% of the screen (default)")).toBeInTheDocument();
  });

  it("treats unset and null fields as an empty preset", () => {
    expect(isEmptyPreset({})).toBe(true);
    expect(isEmptyPreset({ gamma: undefined, contrast: null as unknown as number })).toBe(true);
    expect(compact({ page_height: 0.8, gamma: undefined })).toEqual({ page_height: 0.8 });
  });
});
