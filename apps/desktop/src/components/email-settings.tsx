import { Send, TriangleAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Field } from "@/components/field";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { errorMessage } from "@/lib/format";
import {
  DEFAULT_PORT,
  type EmailSettings,
  emailPassword,
  type PasswordStorage,
  type Security,
  sendTestEmail,
} from "@/lib/ipc.send";
import { adoptKeychainPassword, settingsWritten, useSettings } from "@/lib/settings";

const SECURITY: { value: Security; label: string }[] = [
  { value: "starttls", label: "STARTTLS" },
  { value: "tls", label: "TLS" },
  { value: "none", label: "None (local relay only)" },
];

type Note = { ok: boolean; text: string } | null;

/**
 * Where the saved password is kept, and the password itself from the keychain. Reading it may
 * bring up the keychain's unlock prompt, so only the user's actions do: opening this form, a
 * click in a locked password field (`ask(true)`: prompt again) and a save (`ask(false)`).
 * Window focus never does.
 */
function usePasswordStorage() {
  const [storage, setStorage] = useState<PasswordStorage | null>(null);
  const ask = useCallback(async (retry: boolean) => {
    try {
      const found = await emailPassword(retry);
      if (found.storage === "keychain") adoptKeychainPassword(found.password);
      setStorage(found.storage);
    } catch {
      setStorage("file");
    }
  }, []);
  useEffect(() => void ask(true), [ask]);
  return [storage, ask] as const;
}

/** Where the saved password is, or that there is none. A locked keychain loads as an empty
 * password, so it gets its own wording instead of "none saved". */
function passwordHint(saved: boolean, storage: PasswordStorage | null): string | undefined {
  if (storage === "locked") return "Your system keychain is locked or can't be read.";
  if (storage === "keychain") return saved ? "Kept in your system keychain." : "No password saved.";
  if (storage === "file") {
    return saved
      ? "Kept in the app settings (no system keychain)."
      : "No password saved. Without a system keychain it will be kept in the app settings.";
  }
  return undefined;
}

/** Send to Kindle email: edited as a draft, saved with "Save", testable before saving. */
export function EmailSettingsForm() {
  const [settings, update] = useSettings();
  const [draft, setDraft] = useState<EmailSettings>(settings.kindleEmail);
  const [note, setNote] = useState<Note>(null);
  const [testing, setTesting] = useState(false);
  const saved = JSON.stringify(settings.kindleEmail);
  useEffect(() => setDraft(JSON.parse(saved) as EmailSettings), [saved]);
  const set = (patch: Partial<EmailSettings>) => setDraft((d) => ({ ...d, ...patch }));
  const dirty = JSON.stringify(draft) !== saved;
  const [storage, askStorage] = usePasswordStorage();
  const save = () => {
    update({ kindleEmail: draft });
    void settingsWritten().then(() => askStorage(false));
  };
  const hasPassword = settings.kindleEmail.password !== "";

  const test = async () => {
    setTesting(true);
    setNote(null);
    try {
      setNote({ ok: true, text: `Test email sent to ${await sendTestEmail(draft)}.` });
    } catch (e) {
      setNote({ ok: false, text: errorMessage(e) });
    } finally {
      setTesting(false);
    }
  };
  const text = (key: keyof EmailSettings, id: string, props = {}) => (
    <Input
      id={id}
      value={String(draft[key] ?? "")}
      onChange={(e) => set({ [key]: e.target.value })}
      {...props}
    />
  );

  return (
    <div className="flex flex-col gap-4">
      <Field
        label="Kindle email address"
        htmlFor="kindle-to"
        hint="Your device's Send to Kindle address, under Manage your content and devices on Amazon."
      >
        {text("to", "kindle-to", { placeholder: "name@kindle.com", type: "email" })}
      </Field>
      <Field
        label="Sender address"
        htmlFor="kindle-from"
        hint="Add it to the approved senders list in your Amazon account. Empty: the user name."
      >
        {text("from", "kindle-from", { placeholder: draft.username || "you@example.com" })}
      </Field>
      <div className="grid grid-cols-[1fr_96px_180px] gap-3">
        <Field label="SMTP server" htmlFor="smtp-host">
          {text("host", "smtp-host", { placeholder: "smtp.gmail.com" })}
        </Field>
        <Field label="Port" htmlFor="smtp-port">
          <Input
            id="smtp-port"
            inputMode="numeric"
            value={draft.port ?? ""}
            placeholder={String(DEFAULT_PORT[draft.security])}
            onChange={(e) => {
              const port = Number.parseInt(e.target.value, 10);
              set({ port: port > 0 && port < 65536 ? port : null });
            }}
          />
        </Field>
        <Field label="Security" htmlFor="smtp-security">
          <Select value={draft.security} onValueChange={(v) => set({ security: v as Security })}>
            <SelectTrigger id="smtp-security" aria-label="Security">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SECURITY.map((s) => (
                <SelectItem key={s.value} value={s.value}>
                  {s.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <Field label="User name" htmlFor="smtp-user">
          {text("username", "smtp-user", { autoComplete: "off" })}
        </Field>
        <Field label="Password" htmlFor="smtp-password" hint={passwordHint(hasPassword, storage)}>
          {text("password", "smtp-password", {
            type: "password",
            autoComplete: "off",
            onFocus: () => storage === "locked" && void askStorage(true),
          })}
        </Field>
      </div>
      {storage === "file" && hasPassword && (
        <Alert>
          <TriangleAlert />
          <AlertTitle>No system keychain: the password is stored unencrypted</AlertTitle>
          <AlertDescription>
            It sits in the app's settings on this computer, readable by anyone with access to your
            account. Use an app password from your mail provider, not your main one. Backups never
            include it.
          </AlertDescription>
        </Alert>
      )}
      <div className="flex items-center gap-2">
        <Button variant="primary" disabled={!dirty} onClick={save}>
          Save
        </Button>
        <Button disabled={testing || !draft.host.trim()} onClick={() => void test()}>
          <Send /> {testing ? "Sending…" : "Send test email"}
        </Button>
      </div>
      {note && (
        <p
          role="status"
          className={note.ok ? "text-caption text-ink-2" : "text-caption text-accent"}
        >
          {note.text}
        </p>
      )}
    </div>
  );
}
