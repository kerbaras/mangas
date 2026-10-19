import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SmartCutButton } from "@/components/smart-cut-preview";

describe("smart-cut options button", () => {
  it("keeps its label on one line and lets the caption wrap instead", () => {
    render(
      <SmartCutButton
        series={null}
        chapters={[]}
        device={undefined}
        format="epub"
        cutPreset="balanced"
        detectors={false}
        saved={null}
        onSaved={() => {}}
      />,
    );
    const button = screen.getByRole("button", { name: /Smart-cut options/ });
    expect(button.className).toContain("whitespace-nowrap");
    expect(button.className).toContain("shrink-0");
    expect(screen.getByText(/Pick chapters of one series/).className).toContain("min-w-0");
  });
});
