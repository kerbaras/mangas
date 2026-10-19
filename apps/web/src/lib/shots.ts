// App screenshots in assets/shots, named `<shot>-<scheme>-<size>.webp`. Sizes: 1280 and 2560
// wide, plus `crop`, a readable part of the window for phones.
export type ShotName = "library" | "details" | "export" | "result";
export type Scheme = "light" | "dark";
type Size = "1280" | "2560" | "crop";

const files = import.meta.glob<{ default: ImageMetadata }>("../assets/shots/*.webp", {
  eager: true,
});

function file(name: ShotName, scheme: Scheme, size: Size): ImageMetadata {
  const hit = files[`../assets/shots/${name}-${scheme}-${size}.webp`];
  if (!hit) throw new Error(`missing screenshot ${name}-${scheme}-${size}.webp`);
  return hit.default;
}

export function shot(name: ShotName, scheme: Scheme) {
  const small = file(name, scheme, "1280");
  const large = file(name, scheme, "2560");
  return {
    small,
    crop: file(name, scheme, "crop"),
    srcset: `${small.src} 1280w, ${large.src} 2560w`,
  };
}
