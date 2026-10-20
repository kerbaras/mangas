# Kindle-native formats (AZW3/KF8, KFX) vs EPUB

Research for [#320](https://github.com/kerbaras/mangas/issues/320). Measured on 2026-10-20 against
mangas [`f3bb0db`](https://github.com/kerbaras/mangas/commit/f3bb0db778e96d4a8c50e2087c4d21f82d253633)
and calibre [`dc29097`](https://github.com/kovidgoyal/calibre/commit/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea)
(calibre 9.16.0 binaries). Lines marked **Inference** are my reading of the evidence, not something
a source states or I measured.

## TL;DR

- **Recommendation: partial.** Keep fixed-layout EPUB as the Send-to-Kindle format. Add a
  fixed-layout **AZW3 (KF8)** output from our own pure `export::azw3` writer (~2 weeks), and use
  it for USB delivery. `kindling-cli`, `calibre-debug --inspect-mobi` and the `mobi` crate are
  dev/CI-only oracles, never linked into the app. Skip **KFX**:
  nothing outside Amazon's Kindle Previewer writes fixed-layout KFX. Before any of that, fix two
  bugs in today's EPUB: the writing-mode value is invalid, and Kindle exports are always
  left-to-right.
- The reasons. KF8 is documented well enough to write: calibre has a writer (GPL, so we can learn
  from it but not copy it), and an MIT Rust crate (`kindling-mobi`) wrote valid fixed-layout
  EXTH from our EPUB in 8.8 s. It is not usable as our exporter, though: it dropped our title,
  author and chapter TOC and re-encoded every page much smaller
  ([below](#kindling-mobi-on-our-epub)). Amazon lists EPUB only for Send to Kindle, and calibre's Kindle USB
  driver never copies EPUB to a Kindle. Yet mangas's USB delivery copies the EPUB into
  `documents/` today. Converting with calibre is not a fix either: calibre's AZW3 drops every
  fixed-layout flag. KFX is not worth it. The only Rust KFX writer (`boko`, GPL) wrote a reflowable
  book from our input, and Amazon already turns Send-to-Kindle EPUBs into KFX on its servers.
- **Size.** A 304-page export is 212 MB. That is over Send to Kindle's 200 MB web limit and four
  times the 50 MB email limit mangas already enforces.
- **Effort:** about 3–4 weeks of human work (~3–4 Devin sessions), spread over 4 issues (below).
  The KF8 writer is the bulk of it, about 2 weeks. Each one also needs a check on a real Kindle,
  which only the owner can do.

## Findings

### calibre's KF8 writer (GPLv3)

- Every file in [`writer8/`](https://github.com/kovidgoyal/calibre/tree/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8)
  starts with `License: GPLv3 Copyright: Kovid Goyal`
  ([exth.py#L2](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/exth.py#L2)).
  mangas is MIT ([Cargo.toml#L11](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/Cargo.toml#L11)),
  so none of that code can be ported or translated. What we can reuse: the record layout, the
  EXTH ids and the order of the steps, which are facts about the format. A Rust writer has to be
  a clean implementation that is checked against the format and against independent readers.
- Pipeline in [`main.py`](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/main.py#L35),
  `KF8Writer`: clean up the markup, rewrite resource links to `kindle:embed:XXXX`, move CSS/SVG
  into flows, add `aid` attributes, cut each XHTML file into a skeleton plus fragments
  ([`chunk_it_up` L308](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/main.py#L308),
  [`Chunker` in skeleton.py L206](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/skeleton.py#L206)),
  PalmDoc-compress the text into 4096-byte records
  ([L324](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/main.py#L324),
  [utils.py#L20](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/utils.py#L20)),
  then build the SKEL, FRAG, NCX and guide INDX records
  ([L370](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/main.py#L370)).
  [`KF8Book.build_records`](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/mobi.py#L245)
  writes the records in this order: text, chunk/skel/guide/NCX indexes, resources (images), FDST,
  FLIS, FCIS, EOF.
- Output modes: `mobi_file_type` is `old` (MOBI 6), `both` (MOBI 6 + KF8) or `new` (KF8), and the
  default is `old`
  ([mobi_output.py#L117-L119](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/conversion/plugins/mobi_output.py#L117-L119)).
  AZW3 is KF8 only ([L297](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/conversion/plugins/mobi_output.py#L297)),
  and it builds its resources with `process_images=False`
  ([L358](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/conversion/plugins/mobi_output.py#L358)),
  so images are copied in unchanged. The only image fix-up is Amazon's JPEG/JFIF normalisation
  ([writer2/resources.py#L23](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer2/resources.py#L23)).
- **Fixed layout is not written.** The EXTH writer knows cover (201), thumbnail (202),
  `primary-writing-mode` (525) and `page-progression-direction` (527)
  ([exth.py#L31-L38](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/exth.py#L31-L38),
  [L212-L220](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/exth.py#L212-L220)).
  The fixed-layout ids 122 `fixed-layout`, 123 `book-type`, 124 `orientation-lock`,
  126 `original-resolution`, 127 `zero-gutter`, 128 `zero-margin` and 132 `RegionMagnification`
  appear only in the debug reader
  ([debug/headers.py#L151-L158](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/debug/headers.py#L151-L158)).
  Our measured AZW3 confirms it (see below). KCC's README says the same: "Calibre doesn't properly
  support fixed layout EPUB/MOBI" ([kcc README](https://github.com/ciromattia/kcc#faq)).
- RTL: with an `rtl` spine and no writing mode set, calibre fills in `horizontal-rl`
  ([mobi.py#L238-L243](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/writer8/mobi.py#L238-L243)).
  A `primary-writing-mode` that is already set is copied byte for byte into EXTH 525.
- Comic input (CBZ/CBR, not EPUB) splits landscape pages into two portrait pages and puts them
  in reverse order when `right2left` is set
  ([comic/input.py#L141-L143](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/comic/input.py#L141-L143),
  [comic_input.py#L53-L91](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/conversion/plugins/comic_input.py#L53-L91)).
  mangas's Kindle pipeline has no spread splitting (see the table below).

### KFX

- calibre has **no KFX writer**. Its conversion plugins have no KFX output
  ([plugins/](https://github.com/kovidgoyal/calibre/tree/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/conversion/plugins)).
  KFX shows up only in the metadata reader
  ([metadata/kfx.py](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/metadata/kfx.py))
  and the Kindle driver, which calls KFX "a virtual book format"
  ([driver.py#L117-L119](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L117-L119)).
- **Confirmed:** the third-party [KFX Output plugin](https://www.mobileread.com/forums/showthread.php?t=272407)
  (jhowell) needs Amazon's Kindle Previewer. Previewer runs on Windows and macOS only
  ([KDP](https://kdp.amazon.com/en_US/help/topic/G202131170)); on Linux it works only under Wine.
  The plugin's first post says it is "not able to convert fixed layout comics and manga from
  source formats such as CBR, CBZ, EPUB, or MOBI"; comics go through a KPF file and the Previewer
  CLI. The same post says "Amazon converts most personal documents sent using send-to-kindle to
  KFX", and that KFX comics sideloaded on firmware older than 5.16.6 open in manga mode.
- `boko` 0.5.0 says it writes KFX without Previewer. It is GPL-3.0-or-later, so we can't link it.
  Its KFX from our EPUB declares only the `reflow-style` feature, with each page as a
  `scale_fit` 1400×2100 image (raw logs). **Inference:** that is a reflowable book of images, not a
  fixed-layout comic.

### Send to Kindle and USB side-loading

- Send to Kindle accepts PDF, DOC(X), TXT, RTF, HTML, images and EPUB, up to 200 MB
  ([amazon.com/sendtokindle](https://www.amazon.com/sendtokindle),
  [help G5WYD9SAF7PGXRNA](https://www.amazon.com/gp/help/customer/display.html?nodeId=G5WYD9SAF7PGXRNA)).
  The list has no MOBI and no AZW3. mangas caps email at 50 MB
  ([message.rs#L10-L11](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/send/email/message.rs#L10-L11)).
- Server-side conversion: Amazon converts what you send (the Send to Kindle page links "Tips about
  converting personal documents to a Kindle format"). The KFX Output post above says the result is
  KFX. Amazon does not document whether a fixed-layout EPUB stays fixed-layout through that step.
  **Unverified.**
- USB: calibre's driver for every modern e-ink Kindle copies `azw, mobi, azw3, prc, azw1, tpz,
  azw4, kfx, pobi, pdf, txt` and no EPUB (`KINDLE2`, which later e-ink models inherit)
  ([driver.py#L389](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L389)).
  KCC tells users to drag a MOBI/AZW3 into `documents/` ([README FAQ](https://github.com/ciromattia/kcc#faq)).
  mangas's USB delivery copies the exported file, an EPUB for every Kindle profile, into
  `documents/`
  ([copy.rs#L21](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/send/usb/copy.rs#L21),
  [profiles.rs#L43-L47](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/profiles.rs#L43-L47)).
  **Inference:** on current firmware a side-loaded EPUB doesn't show up as a readable book, so
  USB delivery to a Kindle doesn't work as it stands. Not checked on hardware; the owner should
  confirm with one copy.
- Firmware notes from [kindling's README](https://github.com/ciscoriordan/kindling#known-kindle-firmware-issues),
  which are third-party device reports. Firmware 5.19.2 had side-loaded fixed-layout regressions,
  fixed in 5.19.3+. A side-loaded KF8-only `.azw3` shows no library cover tile (their issue #20);
  a dual MOBI7+KF8 `.mobi` does, and KF8-only bytes named `.mobi` don't open (#24). An image
  record over 128 KB "closes the reading app on the page that uses it". Records over ~100 MB give
  "Unable to Open Item". The README also says Amazon deprecated MOBI for Send to Kindle in August
  2022. KCC reports blank pages with PNG on Scribe and with any format on Colorsoft
  ([kcc FAQ](https://github.com/ciromattia/kcc#faq)).

### Rust crates (crates.io, 2026-10-20)

| Crate | Licence | Writes | Notes |
|---|---|---|---|
| [`kindling-mobi`](https://crates.io/crates/kindling-mobi) 0.45.6 | MIT | KF8 `.azw3`, MOBI7+KF8 | First release 2026-04, 55 versions, 1,379 downloads. 68k lines (dictionaries, StarDict, EPUB repair…); `kf8.rs` 3.4k, `mobi.rs` 8.5k, `comic.rs` 3.4k. 0.45.6 released 2026-09-21; one crates.io owner (`ciscoriordan`); 188 crates in its dependency tree. Library API is path in, path out with `Box<dyn Error>`. Comic mode writes the fixed-layout EXTH ids but loses our metadata ([below](#kindling-mobi-on-our-epub)). |
| [`boko`](https://crates.io/crates/boko) 0.5.0 | GPL-3.0+ | AZW3, KFX | Can't be used in an MIT app. calibre's inspector rejects its AZW3 index; its KFX is reflowable (above). |
| [`epub3-kindle`](https://crates.io/crates/epub3-kindle) 0.5.0 | MIT | AZW3/MOBI | 99 downloads. README covers text and vertical Japanese; it doesn't mention comics or fixed layout. |
| [`kindlecc`](https://crates.io/crates/kindlecc) 0.3.0 | GPL-3.0+ | via kindlegen | Shells out to Amazon's kindlegen. |
| [`mobi`](https://crates.io/crates/mobi) 0.8.0 | MIT | none (reader) | 678k downloads; good for reading our own output back in tests. |

### kindling-mobi on our EPUB

Checked after #323 merged, against the same `kindling.azw3` (commands in the raw logs).

- **Metadata lost.** `kindling-cli dump` shows `mobi.full_name = "Comic"` and
  `exth[100] = "kindling"`, not "Yotsuba v1-2" / "Azuma Kiyohiko". The comic path rebuilds its own
  OPF and NCX: `docTitle` is the literal `Comic`
  ([comic.rs:3072](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/comic.rs#3072)), the author defaults to `DEFAULT_AUTHOR = "kindling"`
  ([lib.rs:58](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/lib.rs#58)), and the TOC is one "Page N" entry per page
  ([`build_comic_ncx` comic.rs:3048](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/comic.rs#3048)). Our chapter TOC is gone.
  `ComicOptions` has `title_override`/`author_override`, but no way to pass a TOC.
- **Images re-processed.** It pulls the JPEGs back out of the EPUB and runs its own pipeline
  (crop, contrast, resize, re-encode, default `jpeg_quality: 85`,
  [comic.rs:232](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/comic.rs#232)). Any page over `LD_IMAGE_MAX_BYTES = 128 * 1024`
  ([mobi.rs:1943](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/mobi.rs#1943)) goes through `fit_ld_image`, which lowers the quality and
  then halves the dimensions until it fits ([mobi.rs:2032](https://docs.rs/crate/kindling-mobi/0.45.6/source/src/mobi.rs#2032)). Its doc comment says
  "the full-resolution original belongs in the HD container", but this output has no `CONT`/`CRES`
  records (the HD container calibre reads, [reader/mobi8.py#L432-L437](https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/ebooks/mobi/reader/mobi8.py#L432-L437)).
  So the device gets only the small copies.
- **Per-page quality, measured** (Pillow; quality estimated from the JPEG quantisation tables):

| | Our EPUB pages | kindling `-d paperwhite` | kindling `--no-optimize --no-split --crop 0` |
|---|---|---|---|
| Images | 305 | 309 (308 collected + library thumbnail) | 306 |
| Average / max size | 680 KB / 1.29 MB | 111 KB / 128 KB | 114 KB / 128 KB |
| JPEG quality | 90 on every page | 85 ×91, 75 ×75, 65 ×31, 55 ×20, 45 ×22, 35 ×30, 25 ×30 (re-encoded pages, from its log) | 65 ×129, 85 ×47, 55 ×42, 35 ×25 (top four, from the quantisation tables) |
| Page height | 133 at 1680 px, the rest 872–1200 | 239 at 700–999 px, 38 under 700, 7 at ≥1400 | 243 at 700–999, 43 under 700, none ≥1000 |
| Bits per pixel | 4.03 | 2.43 | 2.03 |

  On a 1448-px (Paperwhite 3/4) or 1680-px (Paperwhite 12) screen, most kindling pages are shown
  at about half our resolution, and 82 pages at quality 45 or lower. **Inference:** that
  will show as soft line art and visible JPEG blocking in screentones, which is worse than our
  EPUB pages, not equal at a sixth of the size. Not checked on a device.

### What a manga book needs, and where mangas stands

| Concern | KF8/AZW3 | mangas EPUB today |
|---|---|---|
| Fixed layout | EXTH 122 `true`, 123 `comic`, 126 `WxH`; one XHTML page per image with a viewport | OPF has `pre-paginated`, `fixed-layout`, `book-type comic`, `original-resolution` ([xml.rs#L100-L122](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/epub/xml.rs#L100-L122)) ✓ |
| Writing mode | EXTH 525 = `horizontal-lr` / `horizontal-rl` ([KPG 2026.3 p.56](https://kindlegen.s3.amazonaws.com/AmazonKindlePublishingGuidelines.pdf)) | emits `horizontal-ltr` / `horizontal-rtl` ([xml.rs#L97](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/epub/xml.rs#L97), [L113](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/epub/xml.rs#L113)), not a valid value; calibre copies it into EXTH 525 |
| RTL | EXTH 527 `rtl` + `horizontal-rl` | the Kindle pipeline hard-codes `right_to_left: false` ([kindle/export.rs#L69](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/kindle/export.rs#L69)), so manga reads left to right |
| Spreads | `rendition:spread none` + split pages; zero-gutter/zero-margin | no spread splitting or rotation in `export/kindle` (grep finds none), so a landscape spread is shrunk to fit the portrait page; `zero-gutter`/`zero-margin` are set |
| Cover | EXTH 201/202 + thumbnail record; side-loaded `.azw3` has no library tile (third-party report) | `cover-image` item ✓ |
| Resolution | `original-resolution` = profile size; KPG asks for 300 DPI, cover 1600×2560 | profiles carry width/height/dpi ([profiles.rs](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/profiles.rs#L43-L47)) ✓; JPEG quality 90 at ≥300 DPI ([process.rs#L34](https://github.com/kerbaras/mangas/blob/f3bb0db778e96d4a8c50e2087c4d21f82d253633/packages/core/src/export/kindle/process.rs#L34)) |
| Image size | 128 KB per record (third-party report) + an HD container for originals | 299 of 305 JPEGs are over 128 KB (avg 680 KB) |

## Measurements

Yotsuba&! (MangaDex `58be6aa6-…`), chapters 1–10, **304 pages**, profile `kindle-paperwhite12`
(1264×1680, grayscale), on this VM (x86_64). Full commands and output are in the raw logs.

| Step | Tool | Wall time | Output size | Image handling |
|---|---|---|---|---|
| Download | `mangas download --chapters 1-10` | 1 m 15.7 s | 799 MB of PNG | source |
| Export EPUB | `mangas export --format epub` | 38.0 s | **212,458,877 B** | 305 grayscale JPEGs (q90): 1181×1680 ×133, 600×872 ×99, 841×1200 ×29, … (no upscaling) |
| EPUB → AZW3 | calibre `ebook-convert` 9.16.0 | 6.6 s | 212,386,521 B | copied unchanged; **no EXTH 122/123/126**; EXTH 525 = `horizontal-ltr` |
| EPUB → AZW3 | `kindling-cli comic -d paperwhite` | 8.8 s | 35,368,406 B 299 of 305 pages re-encoded to fit 128 KB, 158 also scaled down; most pages end up 700–999 px tall (details [below](#kindling-mobi-on-our-epub)); EXTH 122 `true`, 123 `comic`, 126 `1072x1448`, 525 `horizontal-lr` |
| EPUB → AZW3 | `kindling-cli comic --no-optimize --no-split --crop 0` | 8.6 s | 36,089,137 B | the 128 KB cap still re-encodes pages; 181 scaled down |
| EPUB → AZW3 | `boko convert` | 0.3 s | 212,352,199 B | `calibre-debug --inspect-mobi` fails: "Trailing bytes after last IDXT entry" |
| EPUB → KFX | `boko convert` | 0.8 s | 212,457,835 B | `reflow-style` only, not fixed layout |

What the numbers say:

- Wrapping the container costs nothing. All three AZW3 writers finish in seconds, and calibre and
  boko come out the same size as the EPUB. **The 212 MB → 35 MB drop with kindling comes from its
  re-encoding of our pages, not from the KF8 container**: calibre and boko put the same JPEGs in a
  KF8 container and stay at 212 MB, and kindling's own image bytes total 35.1 MB of its 35.4 MB file.
- At 212 MB, the EPUB can't go through Send to Kindle at all (200 MB web, 50 MB email). One volume
  works out to about 700 KB per page.

## Options compared

| Format | Producible from Rust? | Licence of the path | Size (304 pp) | Rendering on Kindle | Send-to-Kindle / USB |
|---|---|---|---|---|---|
| Fixed-layout EPUB (today) | yes, shipped | MIT (ours) | 212 MB | **Inference:** Amazon converts it on the server (to KFX, per MobileRead); whether fixed layout survives isn't documented. Writing-mode/RTL bugs above | S2K: accepted ≤200 MB web / 50 MB email. USB: not a Kindle sideload format (calibre driver; unverified on device) |
| AZW3 (KF8) via calibre | no (Python, shell-out) | GPL tool, run as a separate program | 212 MB | **loses fixed layout** (no EXTH 122/126), so pages come out reflowed | S2K: rejected (not listed). USB: opens |
| AZW3 (KF8), own Rust writer | yes, ~2k lines (estimate) | MIT (ours) | same as the EPUB, or less with a size budget | fixed layout, RTL, Panel View off; no library cover tile when side-loaded (third-party) | S2K: rejected. USB: native format |
| AZW3 (KF8) via `kindling-mobi` | yes (crate) | MIT | 35 MB (it re-encodes) | fixed layout in EXTH, but title `Comic`, author `kindling`, a "Page N" TOC, and pages cut to q25–85 and mostly 700–999 px tall (no HD copy) | S2K: rejected. USB: native |
| MOBI7+KF8 (`.mobi`) | yes (kindling `--legacy-mobi`, or own writer + KF7 part) | MIT | about the KF8 size again | as AZW3, plus a cover tile when side-loaded (third-party) | S2K: MOBI deprecated 2022. USB: opens |
| KFX | **no**, for fixed layout | Previewer is Amazon freeware (Win/mac, Wine); boko is GPL and reflow-only | n/a | best on device (Guided View, page flip) | S2K makes it from EPUB anyway; USB: opens |

## Effort estimate

Rough sizes; a week of human work is about one Devin session.

1. EPUB metadata fixes (writing mode, RTL): ½ day.
2. Size budget for Kindle profiles: 1–2 days.
3. Fixed-layout KF8 writer in `packages/core/src/export/azw3/`: about 2 weeks. That covers PalmDB
   + MOBI/KF8 header + EXTH (fixed layout, 525/527, cover/thumbnail, title, author, language),
   one skeleton/fragment per page, the SKEL/FRAG/NCX INDX records with a chapter-level TOC, FDST,
   image + cover + thumbnail records, the HD image container (`CONT`/`CRES`, see below),
   FLIS/FCIS/EOF. Text compression can be skipped (images
   dominate). **Inference:** about 1.5–2.5k lines over several files of <200 lines. That is the
   same order as kindling's `kf8.rs` plus the parts of `mobi.rs` we'd need, without dictionaries.
4. Wire up `Format::Azw3` (CLI `--format azw3`, desktop picker, Kindle profile default for USB,
   history, file names): 3–4 days.

**Decision: write our own `export::azw3`; don't depend on `kindling-mobi`.** Depending on it would
cut item 3 to about 2–3 days, but we would still have to pass title/author through its overrides,
accept a "Page N" TOC (`build_comic_ncx` takes only a page count), run our processed pages through
a second image pipeline, and take a path-in/path-out API with `Box<dyn Error>` and 188 crates from
a six-month-old, single-owner crate. Our own writer takes our book model and returns bytes, which
fits AGENTS.md (small, pure modules). kindling's CLI, calibre's `--inspect-mobi` and the `mobi`
crate serve as test oracles. They run as separate tools in dev and CI and are never linked, so
their licences don't reach our code.

## Proposed issues

1. **fix(export): valid Kindle writing mode and RTL in fixed-layout EPUB**
   - The OPF writes `primary-writing-mode` as `horizontal-lr` or `horizontal-rl` (KPG values), and
     a test pins both.
   - A series can be marked right-to-left (provider metadata where the site has it, otherwise a
     per-series setting; neither exists today), and it then exports with
     `page-progression-direction="rtl"` and `horizontal-rl` through `mangas export` and the
     desktop export; `kindle/export.rs` no longer hard-codes `false`.
   - calibre's `ebook-convert` of the result shows EXTH 525 `horizontal-rl` and 527 `rtl`
     (recorded on the PR).
2. **feat(export): size budget so a Kindle volume fits Send to Kindle**
   - With an email delivery, the export plan shows the estimated size against 50 MB, and against
     200 MB otherwise, before writing (CLI and desktop).
   - When over budget it offers fewer chapters per book or a lower JPEG quality. Nothing is
     silently re-encoded.
   - The 304-page Yotsuba&! fixture from this report is re-measured on the PR.
3. **feat(core): fixed-layout AZW3 (KF8) writer**
   - New `export::azw3` module, pure (book in, bytes out), every file under 200 lines, no GPL code.
   - The output has EXTH 122 `true`, 123 `comic`, 126 = the profile size, 525/527 from the book,
     201/202 cover and thumbnail, and one page per image in spine order. RTL books keep the
     reversed page order.
   - Each page keeps its exported JPEG unchanged in the HD container (`CONT`/`CRES`). The
     low-definition record is ≤128 KB, and the export reports which pages needed a smaller one.
   - Title, author and a chapter-level TOC from the book model survive (checked with
     `kindling-cli dump`).
   - `calibre-debug --inspect-mobi`, `kindling-cli dump` and the `mobi` crate all read the
     304-page fixture back with 304 pages (CI or a recorded run).
   - The owner opens it on a Kindle (Paperwhite 12th gen) side-loaded over USB, LTR and RTL:
     pages are full screen, page turns go in the right direction, the cover shows in the book.
4. **feat(export): AZW3 format in CLI and desktop, default for USB delivery to a Kindle**
   - `mangas export --format azw3` and the desktop format picker; Kindle profiles keep EPUB for
     email and switch to AZW3 when the delivery is USB.
   - USB delivery of an EPUB to a Kindle is refused with a message that names AZW3, instead of
     copying a file the Kindle can't open.
   - The missing library cover tile for side-loaded `.azw3` is documented in the UI or handled
     (thumbnail in `system/thumbnails`, as `kindling thumbnail` does); the owner confirms which
     on a device.

Out of scope: KFX output. Revisit only if Amazon ships a documented fixed-layout KFX path that
doesn't need Previewer.

<details>
<summary>Raw measurement logs</summary>

```text
$ export XDG_DATA_HOME=~/k320/data
$ mangas-cli add 58be6aa6-06cb-4ca5-bd20-f1392ce451fb --lang en
$ time mangas-cli download 58be6aa6-06cb-4ca5-bd20-f1392ce451fb --chapters 1-10
  ch 1..10 pages: 51 36 28 26 29 32 20 32 24 26  (= 304)
real 1m15.670s
$ du -sh ~/k320/data/mangas/chapters/mangadex/58be6aa6-…/
799M

$ time mangas-cli export 58be6aa6-… --chapters 1..10 --title "Yotsuba v1-2" \
    --format epub --profile kindle-paperwhite12 -o ~/k320/out
writing 304 pages
/home/ubuntu/k320/out/Yotsuba v1-2.epub: 304 pages
real 0m38.016s  user 0m24.187s  sys 0m11.869s
-rw-r--r-- 212458877 Yotsuba v1-2.epub
unzip -l: 305 jpg, 306 xhtml, 1 opf, container.xml, mimetype; largest page 1,324,892 B
images (PIL): mode L; (1181,1680)x133 (600,872)x99 (841,1200)x29 (757,1100)x28 (1182,1680)x7
avg 679.7 KB, max 1293.8 KB, 299/305 > 128 KB
OPF: rendition:layout pre-paginated; orientation portrait; spread none; fixed-layout true;
     original-resolution 1264x1680; book-type comic; primary-writing-mode horizontal-ltr;
     zero-gutter true; zero-margin true; orientation-lock portrait; RegionMagnification false

$ time ebook-convert "Yotsuba v1-2.epub" calibre.azw3     # calibre 9.16.0
AZW3 output written to /home/ubuntu/k320/out/calibre.azw3
real 0m6.640s  user 0m5.753s  sys 0m1.205s
-rw-r--r-- 212386521 calibre.azw3
$ calibre-debug --inspect-mobi calibre.azw3   (EXTH, 22 records)
Creator (100): b'Azuma Kiyohiko'   KF8_Masthead/Cover_Image (129): b'kindle:embed:002T'
CoverOffset (201): 92   ThumbOffset (202): 93   Fake Cover (203): 0
primary-writing-mode (525): b'horizontal-ltr'   page-progression-direction (527): b'ltr'
Override Kindle fonts (528): b'true'
(no 122 fixed-layout, 123 book-type, 126 original-resolution)

$ cargo install --root ~/k320/tools kindling-mobi boko   # outside the repo; resolved kindling-mobi 0.45.6
$ kindling-cli --version
kindling 0.45.6
$ time kindling-cli comic "Yotsuba v1-2.epub" -o kindling.azw3 -d paperwhite
Detected book content, building KF8-only (.azw3)
Collected 308 images (35132443 bytes total)
Generated 75786 byte library thumbnail (recindex 309, EXTH 202=308)
KF8: 89 text records (362882 bytes), 2 flows
Detected fixed-layout content
KF8-only: 413 total records
Wrote kindling.azw3 (35368406 bytes)
MOBI check: 23 P0 checks passed, 2 P1 warnings (EXTH 503 missing; EXTH 307/538 missing)
real 0m8.813s  user 0m50.642s
$ calibre-debug --inspect-mobi kindling.azw3   (EXTH, 24 records)
fixed-layout (122): b'true'  book-type (123): b'comic'  orientation-lock (124): b'none'
original-resolution (126): b'1072x1448'  zero-gutter (127): b'true'  zero-margin (128): b'true'
CoverOffset (201): 0  ThumbOffset (202): 308  Language (524): b'en'
primary-writing-mode (525): b'horizontal-lr'  page-progression-direction (527): b'ltr'

$ time kindling-cli comic "Yotsuba v1-2.epub" -o kindling-noopt.azw3 -d paperwhite \
    --no-optimize --no-split --crop 0
KF8-only: 709 total records
Wrote kindling-noopt.azw3 (36089137 bytes)
real 0m8.554s

$ time boko convert "Yotsuba v1-2.epub" boko.azw3      real 0m0.285s  212352199 B
$ calibre-debug --inspect-mobi boko.azw3
ValueError: Trailing bytes after last IDXT entry: b'!\xd2'
$ time boko convert "Yotsuba v1-2.epub" boko.kfx       real 0m0.827s  212457835 B
$ boko kfx-dump boko.kfx
features: [ {namespace: "com.amazon.yjconversion", key: "reflow-style", major 6},
            {namespace: "SDK.Marker", key: "CanonicalFormat", major 1} ]
305 x { layout: scale_fit, fixed_width: 1400, fixed_height: 2100 }
writing_mode: horizontal_tb
$ kindling-cli dump kindling.azw3 | grep -E 'full_name |exth\[100\]\.value '
mobi.full_name = "Comic"
exth[100].value = "kindling"
$ grep -c "over 128 KB" kindling.log; grep -c "scaled down" kindling.log
299
158
$ grep -o "quality [0-9]*" kindling.log | sort | uniq -c | sort -rn
     91 quality 85     75 quality 75     31 quality 65     30 quality 35
     30 quality 25     22 quality 45     20 quality 55
page-00037.jpg: over 128 KB, re-encoded at quality 75 instead of 85 and scaled down to 456x724
page-00151.jpg: over 128 KB, re-encoded at quality 85 and scaled down to 322x502
$ python3 jpeg-stats.py   # PDB records starting FF D8 FF; quality from quant tables
epub           305 imgs; avg 679.7 KB max 1293.8 KB; >128KiB: 299; bpp 4.03; q90 x305
kindling       309 imgs; avg 111.3 KB max 128.0 KB; >128KiB: 0; bpp 2.43; CRES 0 CONT 0
kindling-noopt 306 imgs; avg 113.6 KB max 128.0 KB; >128KiB: 0; bpp 2.03; CRES 0 CONT 0
```

Library equivalent of the CLI call (kindling-mobi 0.45.6; not compiled or run in this spike):

```rust
let profile = kindling::comic::get_profile("kpw6").ok_or("profile")?; // 1264-wide PW 12th gen
let opts = kindling::comic::ComicOptions {
    rtl,
    title_override: Some(title),
    author_override: Some(author),
    ..Default::default()
};
kindling::comic::build_comic_with_options(&epub, &out_azw3, &profile, &opts)?;
```

</details>
