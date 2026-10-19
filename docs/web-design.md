# Landing page v2: research and direction (#198)

I studied the four references at 1440 and 390 wide, light and dark, scrolling in 200 to 700 px steps. I also probed the DOM for scroll-driven CSS, IntersectionObservers, rAF loops and reduced-motion rules.

## What the references do

**httpie.io** (the owner's favourite)
- The type carries the brand. "FLOW THROUGH APIs" is set in a condensed headline face at 109/87 px over three lines. Body copy is a light grotesk at 18/25 and section titles are 30/42 semibold. No sizes sit in between.
- The product is drawn from its own parts. You see two app windows with skeleton lines and a URL that types out `hello-world`. Green blobs are cut from the logo shape. The hero assembles in about 1.2 s. Big slabs, barely lighter than the background, run the logo's diagonal behind the sections.
- Motion comes in short entrance sequences that one IntersectionObserver fires (128 targets). Nothing scrubs with scroll. The depth comes from overlapping windows and the slabs, not from DOM parallax.
- Every block follows the same pattern: a pill badge ("Public beta"), a title, two lines of copy and a pair of pill buttons. One green does all the accent work.
- Avoid copying three things. It's dark only, since the light scheme renders the same. The colourful tweet wall and logo row break the restraint. And it never shows the real product.

**cursor.com**
- The headline is a modest 26/32. The product does the talking: the real UI, rendered in HTML inside a window placed over a painted landscape. The demos play on their own, and 20 observers start and stop them.
- The primary CTA knows your OS ("Download for Linux"), with a quiet secondary pill beside it. The pair comes back at the end.
- Light mode is a warm `#f7f7f4` and dark mode is a real dark theme. One orange is used, for links only.
- Avoid the length (about 8,200 px of logos, quotes, changelog and blog) and the cookie banner covering the first screen.

**pi.dev**
- It has the best pairing of the four: an editorial serif, italic at about 48 px for display, and a mono in letter-spaced uppercase for labels and nav. A faint graph-paper grid covers the page.
- On load, the logo builds from falling blocks and the headline un-blurs. On scroll, the logo flies into the sticky nav.
- Its scrollytelling uses a sticky stage on the left that plays real terminal recordings. Copy blocks scroll past on the right, and each one swaps the recording. It has four reduced-motion rules.
- Avoid an empty first screen while the intro plays, and long italic body copy.

**claude.com/product/claude-code**
- The hero is a split. On the left are a 64 px serif title, a 23 px sans lede, a black "Download for Linux", docs, and "Available for macOS, Linux, and Windows". On the right is a calm product UI built in HTML.
- Reveals are short fade-and-rise entrances (40 observers). The tabbed terminal demo changes content without moving the layout.
- It has the best reduced-motion handling: 26 rules, including a global `transition-duration: .01ms`, and the page-transition overlay is removed.
- Avoid cream plus serif, which is now the default AI-made look. Pricing, quotes and the FAQ also bury the product.

**What I take from them.** One display face with character, used big and rarely. The real product carries the page. A sticky stage suits a product with a real sequence (follow, download, export, read). Motion is a few short entrances and one scrubbed moment, never on body text. We have no logos, quotes, pricing or FAQ, and the page is better without them.

## Direction for Mangas

Mangas is the only one of these products that turns long webtoon strips into pages. It cuts at the gaps between panels. The page uses that cut line (a dashed red rule) as its single motif and makes it the one scroll-scrubbed moment.

**Type.** Display is Archivo, self-hosted as one variable WOFF2. It's instanced to `wdth` 62–100 and `wght` 500–800 and subset to Latin, which comes to 30 KB. It's set extra-condensed (wdth 62, wght 760, line-height .92) in sentence case. Condensed gothic is close to manga cover lettering, and closer to httpie than a serif would be. Body copy uses the app's own system stack (`--font-sans` in `tokens.css`, 0 KB). Labels and the CLI snippet use `--font-mono`. The scale is display 104/96 at 1440 (54/50 at 390), step titles 44, lede 20/28, body 16/24 and labels 12 mono uppercase.

**Palette.** All colours come from `tokens.css`. Paper is `#f7f7f5`/`#1f1f1e`, sheet `#fff`/`#292928`, ink `#1b1b1a`/`#f2f2f0`, ink-2 `#5c5c58`/`#b4b4af` and rule `#e1e1de`/`#3c3c3a`. The accent `#b3261e`/`#e0584f` appears only on the CTA (which keeps `#b3261e` in both modes for contrast) and on cut marks and step numbers. The one addition is a screentone: a dot pattern in ink at 7% (9% in dark).

**Sections.**
1. The hero has a label row ("Desktop app · macOS · Windows · Linux") and the headline "Manga and webtoons, sized for your Kindle." The lede comes from README facts. Then the OS-aware Download button, "All downloads", and the Library window on a screentone field.
2. The showcase is a sticky window. Four numbered steps scroll past (follow, download, export, read), and the window crossfades between Library, Details (chapter groups), the Export dialog and the EPUB on a Kindle.
3. Smart cut is a tall strip of panels we draw ourselves in SVG. It scrolls through a fixed page frame, red cut marks snap to the gutters, and the strip splits into pages. The captions cover the `balanced`, `fewest-cuts` and `no-shrink` presets and the optional detectors.
4. Four capability blocks:
   - Six sources, one search.
   - The queue the app and the CLI share.
   - EPUB/CBZ/PDF for Kindle, Kobo, reMarkable or a tablet, sent by email or USB.
   - One executable that is also the CLI, with a three-line `mangas` snippet.
5. Download is the existing per-OS block, unchanged (`lib/download.ts`, `lib/platform.ts`, the releases fallback).
6. The footer links GitHub, the changelog and the MIT licence.

**Motion.** It uses only transform and opacity, all inside `@media (prefers-reduced-motion: no-preference)`.

| element | movement | scroll range |
|---|---|---|
| hero window | rises 48 px, `rotateX(10deg) scale(.96)` to flat | `view()` from `entry 0%` to `cover 40%` |
| screentone field | moves at 0.85× (40 px over the hero) | same |
| hero copy | fades in and rises 12 px, 400 ms, 60 ms stagger | on load, once |
| showcase steps | inactive steps at 40% opacity, 240 ms crossfade between screenshots | each step's `contain 30%–70%` |
| smart-cut strip | moves −60% through the frame, cut marks scale-x 0→1, pages part by 16 px | a 250vh sticky section |
| capability blocks | fade in and rise 16 px | `entry 10%–40%`, once |

With reduced motion, everything renders in its end state. The window is flat, every step is at full opacity with its screenshot inline, and the strip shows as finished pages side by side.

**Tech.** CSS scroll-driven animations (`animation-timeline: view()`) inside `@supports`. They run off the main thread in Chromium and Safari 26. Firefox ships them behind a flag. Where they aren't supported, one IntersectionObserver of about 1 KB gz sets `data-in` on the reveals. The scrubbed pieces (hero tilt, strip) render in their end state there. The active showcase step uses the same observer in every browser, because it's state, not animation. I'm not using a library. GSAP with ScrollTrigger is about 45 KB gz. Both it and Motion (about 4 KB) run on the main thread, while CSS costs 0 KB. Budget: JS ≤ 4 KB gz including the download script, CSS ≤ 12 KB gz, 30 KB of font, Lighthouse ≥ 95 in all four categories.

## Mocks

There are two static mocks of the hero and first showcase, in `scratch/mocks/` on the scratch branch. Their screenshots use the desktop app's dev-mode UI with its sample library. The real build will use captures of the real app.

- **A, "Gutter."** A centred display headline. The Library window tilts on a screentone field with a dashed cut line through it. The showcase is a sticky Details window with four numbered steps, and step 01 is active.
- **B, "Spread."** A split hero with a three-line display and two overlapping windows (Library, Details) with the cut line across them. The showcase is a full-bleed ink band with step tabs and a large Export window that bleeds out of the band.

I recommend A. It gives the product the most room and reads calmer. B's ink band could still work as the smart-cut section, so the page has a single dark interruption.
