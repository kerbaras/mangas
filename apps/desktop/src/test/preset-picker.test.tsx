import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { PresetPicker } from "@/components/preset-picker";
import type { Preset } from "@/lib/ipc";
import { PRESETS } from "@/lib/presets";

function Harness({ onChange }: { onChange: (p: Preset) => void }) {
  const [value, setValue] = useState<Preset>("balanced");
  return (
    <PresetPicker
      value={value}
      onChange={(p) => {
        setValue(p);
        onChange(p);
      }}
    />
  );
}

describe("PresetPicker", () => {
  it("renders one radio per preset with a one-line explanation", () => {
    render(<PresetPicker value="balanced" onChange={() => {}} />);
    const radios = screen.getAllByRole("radio");
    expect(radios).toHaveLength(PRESETS.length);
    for (const p of PRESETS) expect(screen.getByText(p.description)).toBeInTheDocument();
  });

  it("marks the current preset checked and switches on click", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    expect(screen.getByRole("radio", { name: /balanced/i })).toBeChecked();

    await userEvent.click(screen.getByRole("radio", { name: /fewest cuts/i }));
    expect(onChange).toHaveBeenCalledWith("fewest-cuts");
    expect(screen.getByRole("radio", { name: /fewest cuts/i })).toBeChecked();
    expect(screen.getByRole("radio", { name: /balanced/i })).not.toBeChecked();
  });

  it("shows fewer cut lines for 'fewest cuts' than for 'no shrink'", () => {
    render(<PresetPicker value="balanced" onChange={() => {}} />);
    expect(screen.getByLabelText("3 pages, 2 cuts")).toBeInTheDocument();
    expect(screen.getByLabelText("5 pages, 4 cuts")).toBeInTheDocument();
  });
});
