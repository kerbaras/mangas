import { useCallback, useEffect, useState } from "react";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type PasswordStatus,
  type PasswordStorage,
  setSmtpPassword,
  smtpPasswordStatus,
} from "@/lib/ipc.send";
import { notifyError } from "@/lib/notify";
import { loadSettings } from "@/lib/settings";

/**
 * Where the saved password is kept. Asking may bring up the keychain's unlock prompt, so only
 * the user's actions do: opening this form, a click in a locked password field (`ask(true)`:
 * prompt again) and a save (`ask(false)`). Window focus never does. A password the keychain
 * holds but the settings didn't know about (saved before this version) reloads them.
 */
export function usePasswordStorage(hasPassword: boolean) {
  const [storage, setStorage] = useState<PasswordStorage | null>(null);
  const adopt = useCallback(
    (found: PasswordStatus) => {
      setStorage(found.storage);
      if (found.has_password !== hasPassword) void loadSettings();
    },
    [hasPassword],
  );
  const ask = useCallback(
    async (retry: boolean) => {
      try {
        adopt(await smtpPasswordStatus(retry));
      } catch {
        setStorage("file");
      }
    },
    [adopt],
  );
  // biome-ignore lint/correctness/useExhaustiveDependencies: asks once, when the form opens.
  useEffect(() => void ask(true), []);
  return [storage, ask, adopt] as const;
}

/** Clears the saved SMTP password (Kindle settings reset); false when it couldn't (and says so). */
export async function clearSmtpPassword(): Promise<boolean> {
  try {
    await setSmtpPassword("");
    await loadSettings();
    return true;
  } catch (e: unknown) {
    // The Kindle reset's own toast id, so a later successful reset replaces this failure.
    notifyError("Couldn't clear the SMTP password", e, { id: "settings-reset-Kindle" });
    return false;
  }
}

/** Where the saved password is, or that there is none. A locked keychain may hold one, so it
 * gets its own wording instead of "none saved". */
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

type Props = {
  saved: boolean;
  storage: PasswordStorage | null;
  /** The new password being typed; null while a saved one is kept as is. */
  typed: string | null;
  onType: (typed: string | null) => void;
  onClear: () => void;
  onUnlock: () => void;
};

/** Write-only: a saved password shows as a placeholder with Change and Clear, never its text. */
export function SmtpPasswordField({ saved, storage, typed, onType, onClear, onUnlock }: Props) {
  const keeping = saved && typed === null;
  return (
    <Field label="Password" htmlFor="smtp-password" hint={passwordHint(saved, storage)}>
      <div className="flex gap-2">
        <Input
          id="smtp-password"
          type="password"
          autoComplete="off"
          disabled={keeping}
          placeholder={keeping ? "••••••••  saved" : undefined}
          value={typed ?? ""}
          onChange={(e) => onType(e.target.value)}
          onFocus={() => storage === "locked" && onUnlock()}
        />
        {keeping && <Button onClick={() => onType("")}>Change</Button>}
        {keeping && <Button onClick={onClear}>Clear</Button>}
        {saved && typed !== null && <Button onClick={() => onType(null)}>Cancel</Button>}
      </div>
    </Field>
  );
}
