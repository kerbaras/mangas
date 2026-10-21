import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, exportBackend, solo } from "./export-backend";

const blank = { ...readSettings().kindleEmail, to: "", host: "", username: "" };
const ready = { ...blank, to: "reader@kindle.com", host: "smtp.example.test", username: "me" };

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: blank, defaultFormat: null });
});

/** The axe rules a tablist breaks when a non-tab sits in it, run on the delivery picker. */
async function tablistViolations() {
  const list = await screen.findByRole("tablist", { name: "Delivery" });
  const rules = ["aria-required-children", "aria-required-parent", "nested-interactive"];
  const result = await axe.run(list, { runOnly: { type: "rule", values: rules } });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.html).join(" | ")}`);
}

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

describe("delivery picker accessibility (#283)", () => {
  it.each([
    ["enabled", ready],
    ["disabled (no mail setup)", blank],
  ])("axe passes with Email %s and every child of the tablist is a tab", async (_, email) => {
    exportBackend();
    writeSettings({ kindleEmail: email });
    render(dialog());
    expect(await tablistViolations()).toEqual([]);
    const list = screen.getByRole("tablist", { name: "Delivery" });
    expect([...list.children].map((c) => c.getAttribute("role"))).toEqual(["tab", "tab", "tab"]);
  });

  const blockedEmail = async () => {
    exportBackend();
    writeSettings({ kindleEmail: blank });
    render(dialog());
    const email = await screen.findByRole("tab", { name: "Email to Kindle" });
    expect(email.getAttribute("aria-disabled")).toBe("true");
    return email;
  };

  it("the disabled Email tab shows why on hover", async () => {
    const email = await blockedEmail();
    fireEvent.pointerMove(email, { pointerType: "mouse" });
    expect((await screen.findByRole("tooltip")).textContent).toMatch(/Kindle address/);
    fireEvent.mouseDown(email);
    expect(email.getAttribute("aria-selected")).toBe("false");
  });

  it("the disabled Email tab takes keyboard focus, shows why and can't be picked", async () => {
    const email = await blockedEmail();
    const usb = screen.getByRole("tab", { name: "Kindle over USB" });
    usb.focus();
    fireEvent.keyDown(usb, { key: "ArrowRight" });
    await waitFor(() => expect(document.activeElement).toBe(email));
    expect((await screen.findByRole("tooltip")).textContent).toMatch(/Kindle address/);
    fireEvent.keyDown(email, { key: "Enter" });
    expect(email.getAttribute("aria-selected")).toBe("false");
  });
});
