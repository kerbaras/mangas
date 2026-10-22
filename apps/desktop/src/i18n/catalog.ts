import { common } from "./en/common";
import { library } from "./en/library";
import { mangaDetails } from "./en/manga-details";
import { search } from "./en/search";

/** Every English message, keyed `<screen>.<name>`; plural forms add `_one`. */
const en = { ...common, ...library, ...mangaDetails, ...search } as const;

export type MessageKey = Exclude<keyof typeof en, `${string}_one`>;
export type Locale = "en";

/** English is the only locale for now; a picker would replace this constant. */
export const LOCALE: Locale = "en";

const catalogs: Record<Locale, Readonly<Record<string, string>>> = { en };

export const messagesFor = (locale: Locale) => catalogs[locale];
