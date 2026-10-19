import { Send, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { Field } from "@/components/field";
import { SmtpPasswordField, usePasswordStorage } from "@/components/smtp-password-field";
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
  type Security,
  sendTestEmail,
  setSmtpPassword,
} from "@/lib/ipc.send";
import { loadSettings, saveSettings, useSettings } from "@/lib/settings";

const SECURITY: { value: Security; label: string }[] = [
  { value: "starttls", label: "STARTTLS" },
  { value: "tls", label: "TLS" },
  { value: "none", label: "None (local relay only)" },
];

type Note = { ok: boolean; text: string } | null;

/** Send to Kindle email: edited as a draft, saved with "Save", testable before saving. */
export function EmailSettingsForm() {
  const [settings] = useSettings();
  const [draft, setDraft] = useState<EmailSettings>(settings.kindleEmail);
  const [note, setNote] = useState<Note>(null);
  const [testing, setTesting] = useState(false);
  const saved = JSON.stringify(settings.kindleEmail);
  useEffect(() => setDraft(JSON.parse(saved) as EmailSettings), [saved]);
  const set = (patch: Partial<EmailSettings>) => setDraft((d) => ({ ...d, ...patch }));
  const hasPassword = settings.kindleEmail.has_password;
  const [typed, setTyped] = useState<string | null>(null);
  const dirty = JSON.stringify(draft) !== saved || Boolean(typed);
  const [storage, askStorage, adoptStatus] = usePasswordStorage(hasPassword);
  /** Saves (empty: clears) the password, never through the settings; reloads `has_password`. */
  const savePassword = async (password: string) => {
    adoptStatus(await setSmtpPassword(password));
    setTyped(null);
    await loadSettings();
  };
  const save = async () => {
    setNote(null);
    try {
      if (JSON.stringify(draft) !== saved) await saveSettings({ kindleEmail: draft });
      if (typed) await savePassword(typed);
    } catch (e) {
      setNote({ ok: false, text: errorMessage(e) });
    }
    void askStorage(false);
  };
  const clear = () => savePassword("").catch((e) => setNote({ ok: false, text: errorMessage(e) }));

  const test = async () => {
    setTesting(true);
    setNote(null);
    try {
      const { has_password: _, ...fields } = draft;
      const to = await sendTestEmail({ ...fields, password: typed ?? "" }, Boolean(typed));
      setNote({ ok: true, text: `Test email sent to ${to}.` });
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
        <SmtpPasswordField
          saved={hasPassword}
          storage={storage}
          typed={typed}
          onType={setTyped}
          onClear={() => void clear()}
          onUnlock={() => void askStorage(true)}
        />
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
        <Button variant="primary" disabled={!dirty} onClick={() => void save()}>
          Save
        </Button>
        <Button disabled={testing || !draft.host.trim()} onClick={() => void test()}>
          <Send /> {testing ? "Sending…" : "Send test email"}
        </Button>
      </div>
      {note && (
        <p
          role="status"
          className={note.ok ? "text-caption text-ink-2" : "text-caption text-accent-text"}
        >
          {note.text}
        </p>
      )}
    </div>
  );
}
