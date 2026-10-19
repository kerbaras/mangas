import { Download, HardDrive, Info, Mail, ScanSearch, Send, Settings2, Wifi } from "lucide-react";
import { type ComponentType, type ReactNode, useState } from "react";
import { BackupPanel } from "@/components/backup-panel";
import { DetectorSupportSettings } from "@/components/detector-support-settings";
import { EmailSettingsForm } from "@/components/email-settings";
import { Field } from "@/components/field";
import { SettingsGroup } from "@/components/settings-group";
import { clearSmtpPassword } from "@/components/smtp-password-field";
import { StoragePanel } from "@/components/storage-panel";
import { AboutSettings } from "./settings-about";
import { DownloadsSettings } from "./settings-downloads";
import { ExportSettings } from "./settings-export";
import { GeneralSettings } from "./settings-general";
import { NetworkSettings } from "./settings-network";

function StorageSettings() {
  return (
    <SettingsGroup title="Storage">
      <StoragePanel />
      <Field label="Backup">
        <BackupPanel />
      </Field>
    </SettingsGroup>
  );
}

function DetectorsSettings() {
  return (
    <SettingsGroup title="Detectors">
      <DetectorSupportSettings />
    </SettingsGroup>
  );
}

function KindleSettings() {
  // A reset also drops what the form holds unsaved (a typed password included): a new form.
  const [form, setForm] = useState(0);
  const reset = () => {
    setForm((n) => n + 1);
    return clearSmtpPassword();
  };
  return (
    <SettingsGroup
      title="Kindle"
      resets={["kindleEmail"]}
      resetHint="The Send to Kindle address and the SMTP account, password included, are cleared."
      onReset={reset}
    >
      <Field
        label="Send by email"
        hint="Books go to your Kindle address as an attachment; Amazon takes up to 50 MB per email."
      >
        <EmailSettingsForm key={form} />
      </Field>
    </SettingsGroup>
  );
}

export type GroupId =
  | "general"
  | "downloads"
  | "network"
  | "export"
  | "storage"
  | "detectors"
  | "kindle"
  | "about";

export const GROUPS: { id: GroupId; label: string; icon: ReactNode; Body: ComponentType }[] = [
  { id: "general", label: "General", icon: <Settings2 />, Body: GeneralSettings },
  { id: "downloads", label: "Downloads", icon: <Download />, Body: DownloadsSettings },
  { id: "network", label: "Network", icon: <Wifi />, Body: NetworkSettings },
  { id: "export", label: "Export", icon: <Send />, Body: ExportSettings },
  { id: "storage", label: "Storage", icon: <HardDrive />, Body: StorageSettings },
  { id: "detectors", label: "Detectors", icon: <ScanSearch />, Body: DetectorsSettings },
  { id: "kindle", label: "Kindle", icon: <Mail />, Body: KindleSettings },
  { id: "about", label: "About", icon: <Info />, Body: AboutSettings },
];
