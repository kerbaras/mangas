/** Send to Kindle part of the IPC contract (`mangas_core::send`, `library::export_history`). */
import type { Unlisten } from "./ipc";
import { backend } from "./ipc";

/** Mirrors `mangas_core::send::Security`. */
export type Security = "starttls" | "tls" | "none";

/** Mirrors `mangas_core::send::EmailSettings`; empty strings mean "not set". */
export type EmailSettings = {
  to: string;
  from: string;
  host: string;
  /** null: the usual port for `security`. */
  port: number | null;
  security: Security;
  username: string;
  password: string;
};

export const EMPTY_EMAIL: EmailSettings = {
  to: "",
  from: "",
  host: "",
  port: null,
  security: "starttls",
  username: "",
  password: "",
};

/** Mirrors `mangas_core::send::password::Storage`: where the SMTP password is kept. */
/** `locked`: a keychain that is locked or won't be read, so the password may be in it. */
export type PasswordStorage = "keychain" | "file" | "locked";
export type EmailPassword = { password: string; storage: PasswordStorage };

export const DEFAULT_PORT: Record<Security, number> = { starttls: 587, tls: 465, none: 25 };

/** Mirrors `mangas_core::send::KindleVolume`. */
export type KindleVolume = { name: string; root: string; documents: string };

/** Mirrors `mangas_core::send::email::Sent`. */
export type SentEmail = { to: string; bytes: number };

export type CopyProgress = { done: number; total: number };

/** Mirrors `mangas_core::library::ExportRecord`. */
export type ExportRecord = {
  id: number;
  file: string;
  title: string;
  device: string;
  exported_at: string | null;
  sent_via: "email" | "usb" | null;
  sent_to: string | null;
  sent_at: string | null;
  exists: boolean;
};

/** Where the book is on the Kindle; `already_there` when nothing had to be copied. */
export type UsbCopy = { path: string; already_there: boolean };

export type SendBackend = {
  findKindles(): Promise<KindleVolume[]>;
  /** Copies the EPUB to the Kindle at `root` (the first found when omitted); resolves to where it is. */
  sendUsb(file: string, title: string, root?: string): Promise<UsbCopy>;
  onCopyProgress(cb: (p: CopyProgress) => void): Promise<Unlisten>;
  /** Emails the EPUB to the Kindle address in the settings. */
  sendEmail(file: string, title: string): Promise<SentEmail>;
  /**
   * Sends a test message with `settings` (saved or not); resolves to the address it went to.
   * Unless a new password was `typed`, the saved one is used, which may prompt to unlock.
   */
  sendTestEmail(settings: EmailSettings, typed: boolean): Promise<string>;
  /**
   * The saved password and where it is kept. May bring up the keychain's unlock prompt, so
   * only for something the user did; `retry` prompts again after a dismissed one.
   */
  emailPassword(retry: boolean): Promise<EmailPassword>;
  listExports(limit: number): Promise<ExportRecord[]>;
};

export const findKindles = async () => (await backend()).findKindles();
export const sendUsb = async (file: string, title: string, root?: string) =>
  (await backend()).sendUsb(file, title, root);
export const onCopyProgress = async (cb: (p: CopyProgress) => void) =>
  (await backend()).onCopyProgress(cb);
export const sendEmail = async (file: string, title: string) =>
  (await backend()).sendEmail(file, title);
export const sendTestEmail = async (s: EmailSettings, typed: boolean) =>
  (await backend()).sendTestEmail(s, typed);
export const emailPassword = async (retry: boolean) => (await backend()).emailPassword(retry);
export const listExports = async (limit: number) => (await backend()).listExports(limit);
