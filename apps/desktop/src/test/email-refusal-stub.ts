/** Stands in for core's `email::email_refusal` in test backends: EPUB and PDF go by email. */
export async function emailRefusal(file: string): Promise<string | null> {
  if (/\.(epub|pdf)$/i.test(file)) return null;
  const ext = file.match(/\.([^./\\]+)$/)?.[1]?.toUpperCase();
  const what = ext ? `a ${ext}` : "this file";
  return `Send to Kindle by email takes EPUB or PDF, not ${what}. Copy it over USB, or export it as EPUB.`;
}
