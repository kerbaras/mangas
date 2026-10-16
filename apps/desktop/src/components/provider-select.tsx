import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAsync } from "@/hooks/use-async";
import { listProviders } from "@/lib/ipc";

type Props = { value: string; onChange: (id: string) => void };

/** Picks the provider a search runs against. */
export function ProviderSelect({ value, onChange }: Props) {
  const providers = useAsync(listProviders, []);
  const items = providers.state.kind === "ready" ? providers.state.data : [];
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger aria-label="Provider" className="h-8 w-40">
        <SelectValue placeholder="Provider" />
      </SelectTrigger>
      <SelectContent>
        {items.map((p) => (
          <SelectItem key={p.id} value={p.id}>
            {p.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
