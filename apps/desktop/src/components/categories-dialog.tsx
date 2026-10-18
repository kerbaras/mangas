import { Plus, Trash2 } from "lucide-react";
import { type FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { Category } from "@/lib/ipc.library";
import { notifyError } from "@/lib/notify";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  categories: Category[];
  onCreate: (name: string) => Promise<unknown>;
  onRename: (id: number, name: string) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
};

/** Create, rename (on Enter or leaving the field) and delete categories. */
export function CategoriesDialog({ open, onOpenChange, categories, ...on }: Props) {
  const [draft, setDraft] = useState("");
  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (!draft.trim()) return;
    try {
      await on.onCreate(draft);
      setDraft("");
    } catch (err: unknown) {
      notifyError("Couldn't add the category", err);
    }
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>Categories</DialogTitle>
        <DialogDescription>
          Group titles your way. A title can sit in several categories; deleting a category keeps
          its titles in the library.
        </DialogDescription>
        <ul className="mt-4 flex max-h-72 flex-col gap-1.5 overflow-y-auto">
          {categories.map((c) => (
            <CategoryRow key={`${c.id}:${c.name}`} category={c} {...on} />
          ))}
        </ul>
        <form onSubmit={add} className="mt-3 flex gap-2">
          <Input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="New category"
            aria-label="New category"
            maxLength={40}
          />
          <Button type="submit" disabled={!draft.trim()}>
            <Plus /> Add
          </Button>
        </form>
        <DialogFooter>
          <Button variant="primary" onClick={() => onOpenChange(false)}>
            Done
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

type RowProps = Pick<Props, "onRename" | "onDelete"> & { category: Category };

function CategoryRow({ category, onRename, onDelete }: RowProps) {
  const [name, setName] = useState(category.name);
  const commit = () => {
    if (name.trim() === category.name || !name.trim()) return setName(category.name);
    onRename(category.id, name).catch((e: unknown) => {
      setName(category.name);
      notifyError("Couldn't rename the category", e);
    });
  };
  return (
    <li className="flex items-center gap-2">
      <Input
        value={name}
        aria-label={`Rename ${category.name}`}
        maxLength={40}
        onChange={(e) => setName(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
      />
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${category.name}`}
        onClick={() =>
          onDelete(category.id).catch((e: unknown) =>
            notifyError("Couldn't delete the category", e),
          )
        }
      >
        <Trash2 />
      </Button>
    </li>
  );
}
