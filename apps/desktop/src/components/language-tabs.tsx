import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { languageLabel } from "@/lib/format";

type Props = {
  languages: string[];
  value: string | null;
  onChange: (language: string | null) => void;
};

/** "All" plus one tab per chapter language on the details toolbar. */
export function LanguageTabs({ languages, value, onChange }: Props) {
  return (
    <Tabs value={value ?? "all"} onValueChange={(v) => onChange(v === "all" ? null : v)}>
      <TabsList>
        <TabsTrigger value="all">All</TabsTrigger>
        {languages.map((l) => (
          <TabsTrigger key={l} value={l}>
            {languageLabel(l)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
