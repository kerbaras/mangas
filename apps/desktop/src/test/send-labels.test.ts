import { describe, expect, it } from "vitest";
import type { ExportRecord } from "@/lib/ipc.send";
import { deliveryLabel, emailReady, emailRefusal, titleOf } from "@/lib/send-labels";

const record = (patch: Partial<ExportRecord>): ExportRecord => ({
  id: 1,
  file: "/b/x.epub",
  title: "x",
  device: "",
  exported_at: "2026-10-18 09:00:00",
  sent_via: null,
  sent_to: null,
  sent_at: null,
  exists: true,
  ...patch,
});

describe("send labels", () => {
  it("derives the title from the file name", () => {
    expect(titleOf("/a/b/Vol 1.epub")).toBe("Vol 1");
    expect(titleOf("C:\\Books\\Vol 2.EPUB")).toBe("Vol 2");
  });

  it("describes where a book went", () => {
    expect(deliveryLabel(record({}))).toBe("Not sent");
    expect(
      deliveryLabel(
        record({ sent_via: "email", sent_to: "r@kindle.com", sent_at: "2026-10-18 09:12:33" }),
      ),
    ).toBe("Emailed to r@kindle.com · 2026-10-18 09:12 UTC");
  });

  it("needs an address, a server and a sender before email is offered", () => {
    const base = { to: "r@kindle.com", from: "", username: "", host: "smtp.x" };
    expect(emailReady(base)).toBe(false);
    expect(emailReady({ ...base, username: "me@x" })).toBe(true);
    expect(emailReady({ ...base, host: " ", username: "me@x" })).toBe(false);
  });

  it("lets EPUB and PDF go by email and explains why a CBZ can't", () => {
    expect(emailRefusal("/b/Vol 1.epub")).toBeNull();
    expect(emailRefusal("C:\\b\\Vol 1.PDF")).toBeNull();
    expect(emailRefusal("/b/Vol 1.cbz")).toBe(
      "Send to Kindle by email takes EPUB or PDF, not a CBZ. Copy it over USB, or export it as EPUB.",
    );
    expect(emailRefusal("/b/v1.0/book")).toContain("not this file");
  });
});
