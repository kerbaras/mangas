import { LOCALE, type MessageKey, messagesFor } from "./catalog";
import { translate, type Vars } from "./translate";

/**
 * The text for `key` outside a component (pure helpers, module-level tables). Tables built at
 * import read the locale once; with English only that is enough, a locale picker would turn
 * them into functions.
 */
export const tr = (key: MessageKey, vars?: Vars) => translate(messagesFor(LOCALE), key, vars);
