import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ErrorAlert } from "@/components/error-alert";

const TOKEN = "y".repeat(200);

describe("ErrorAlert with an unbroken token", () => {
  it("lets the text column shrink and wrap anywhere instead of running off the card", () => {
    render(
      <ErrorAlert title="Detector support" message={`detector status check failed: ${TOKEN}`} />,
    );
    const alert = screen.getByRole("alert");
    expect(alert.className).toContain("has-[>svg]:grid-cols-[16px_minmax(0,1fr)]");
    expect(alert.className).not.toMatch(/grid-cols-\[16px_1fr\]/);
    expect(alert.className).toContain("[overflow-wrap:anywhere]");
    expect(screen.getByText(new RegExp(TOKEN)).closest("[role=alert]")).toBe(alert);
  });
});
