import { tr } from "./tr";

/** `t("library.title")` → the text for the current locale; components call this, helpers `tr`. */
export function useT() {
  return tr;
}
