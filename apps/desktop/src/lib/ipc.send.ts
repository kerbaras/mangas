/** Send to Kindle part of the IPC contract (`mangas_core::send`, `library::export_history`). */
import type { Unlisten } from "./ipc";
import { backend } from "./ipc";

/** Mirrors `mangas_core::send::Security`. */
export type Security = "starttls" | "tls" | "none";

/**
 * Mirrors `mangas_core::send::EmailSettings` as `get_settings` sends it: empty strings mean
 * "not set", and the password never leaves the backend, only whether one is saved.
 */
export type EmailSettings = {
  to: string;
  from: string;
  host: string;
  /** null: the usual port for `security`. */
  port: number | null;
  security: Security;
  username: string;
  has_password: boolean;
};

/** The form as edited, for a test email: `password` is what was typed (empty: the saved one). */
export type EmailDraft = Omit<EmailSettings, "has_password"> & { password: string };

export const EMPTY_EMAIL: EmailSettings = {
  to: "",
  from: "",
  host: "",
  port: null,
  security: "starttls",
  username: "",
  has_password: false,
};

/** Mirrors `mangas_core::send::password::Storage`: where the SMTP password is kept. */
/** `locked`: a keychain that is locked or won't be read, so the password may be in it. */
export type PasswordStorage = "keychain" | "file" | "locked";
/** Whether a password is saved and where; `fallback`: why a new one went to the settings file. */
export type PasswordStatus = {
  has_password: boolean;
  storage: PasswordStorage;
  fallback: string | null;
};

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
  sendTestEmail(settings: EmailDraft, typed: boolean): Promise<string>;
  /**
   * Whether a password is saved and where. May bring up the keychain's unlock prompt, so only
   * for something the user did; `retry` prompts again after a dismissed one.
   */
  smtpPasswordStatus(retry: boolean): Promise<PasswordStatus>;
  /** Saves the SMTP password (empty: clears it); it is never sent back. */
  setSmtpPassword(password: string): Promise<PasswordStatus>;
  listExports(limit: number): Promise<ExportRecord[]>;
};

export const findKindles = async () => (await backend()).findKindles();
export const sendUsb = async (file: string, title: string, root?: string) =>
  (await backend()).sendUsb(file, title, root);
export const onCopyProgress = async (cb: (p: CopyProgress) => void) =>
  (await backend()).onCopyProgress(cb);
export const sendEmail = async (file: string, title: string) =>
  (await backend()).sendEmail(file, title);
export const sendTestEmail = async (s: EmailDraft, typed: boolean) =>
  (await backend()).sendTestEmail(s, typed);
export const smtpPasswordStatus = async (retry: boolean) =>
  (await backend()).smtpPasswordStatus(retry);
export const setSmtpPassword = async (password: string) =>
  (await backend()).setSmtpPassword(password);
export const listExports = async (limit: number) => (await backend()).listExports(limit);
