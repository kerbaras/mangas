# USB delivery to Kindle

Research for [#321](https://github.com/kerbaras/mangas/issues/321). It covers what a complete
"Copy over USB" needs on Linux, macOS and Windows, what calibre does, and the order to ship it in.
It does not pick the book format: that is [#320](https://github.com/kerbaras/mangas/issues/320)
(`docs/research/kindle-formats.md`). This doc only lists what USB delivery needs from the format.

calibre links point at commit `dc29097d` of
[kovidgoyal/calibre](https://github.com/kovidgoyal/calibre). Every claim is tagged **[fact]**
(read in code or docs, linked), **[inference]** (my reading, not checked on hardware) or
**[unverified]** (needs a real Kindle; see the owner checklist at the end).

## TL;DR

- There are two kinds of Kindle on the cable. Older e-ink Kindles are USB mass storage (a FAT
  drive with `documents/` and `system/`). That already works on all three OSes. 2024 and newer
  Kindles speak MTP: no drive letter, no `/Volumes` entry. Today mangas reaches those only on
  Linux, through gvfs. On macOS and Windows they show "Device not found" ([#126]).
- calibre handles both. Mass storage goes through its USBMS driver plus a per-OS eject. MTP goes
  through libmtp on Linux/macOS and Windows Portable Devices (WPD) on Windows. It recognises a
  Kindle on MTP by vendor id `0x1949` alone, not by product id.
- **Bigger problem than MTP: format.** calibre never sends EPUB to a Kindle over USB. EPUB is
  missing from every Kindle format list it has, and it converts to AZW3/MOBI/KFX instead. Amazon
  documents EPUB only for Send to Kindle (email, web, app), where Amazon converts it. My
  inference is that a side-loaded `.epub` copied by today's "Copy over USB" doesn't show up in
  the Kindle library at all. This is the first thing to check on the owner's Kindle.
- Recommended order: (1) [#126]: detect an Amazon USB device with no mounted volume and say
  "use Email" (needs `nusb`, about a day). (2) Only offer USB for formats the Kindle opens. (3)
  Safe remove for mounted volumes. (4) MTP copy on macOS/Windows with the pure-Rust `mtp-rs`
  crate, behind a feature flag. (5) Cover thumbnails once #320 lands a Kindle-native format.
- Effort: about 3 to 4 weeks of solo human work for all of it. Most of the risk sits in MTP on
  macOS (`ptpcamerad` grabs the device), and only the owner can test that.

## Recommended architecture

```
detect ──> target ──> transfer ──> index/thumbnails ──> safe remove
```

**1. Detection.** `send::usb` gets one entry point that returns every Kindle it can see:

```rust
enum KindleTarget {
    Volume(KindleVolume),      // today: a folder with documents/ (mass storage, gvfs MTP)
    Mtp(MtpKindle),            // later: an MTP device mangas talks to itself
    Unreachable(UsbKindle),    // Amazon device on USB with no way in yet (#126)
}
```

- Volumes: keep `find_kindles` as is (`/Volumes`, `D:`-`Z:`, `/media`, gvfs).
- USB bus: list devices with `nusb::list_devices()` and keep vendor `0x1949`. A device that maps
  to no volume becomes `Unreachable` (until MTP lands) or `Mtp`. This is the same rule calibre
  uses: any MTP device with vid `0x1949` is a Kindle ([mtp-driver-iskindle]).
- The MTP signal is an interface named `MTP` or class 6/1/1 (Still Image). calibre reads the
  sysfs `interface` string on Linux ([sysfs]) and IOKit on macOS ([unix-isMTP]). WPD reports
  `protocol` starting with `MTP:` on Windows ([wpd-suitable]).

**2. Transfer.** One function per target, same `Copied` result and `on_progress(done, total)`:

- `Volume`: today's `copy_to_kindle`. It writes `documents/.mangas-<pid>-<n>.part`, fsyncs it,
  then renames it over the book.
- `Mtp`: upload into storage 0 under `documents/`. MTP has no "rename over" and no fsync.
  calibre's `put_file` deletes the old object and then uploads ([unix-put], [wpd-put]), so a
  failed upload loses the old book. To keep today's promise ("a failed copy keeps the book that
  was there"), upload `.mangas-….part` first, then delete the old object, then rename the new one
  (MTP `ObjectFileName`; `mtp-rs` exposes rename). On an aborted upload, delete the partial
  object (`mtp-rs` returns its handle in `UploadError::partial`).
- `Unreachable`: an error naming the device plus the hint: "Your Kindle connects over MTP,
  which mangas can't copy to on this computer yet. Use Email (up to 50 MB) or Send to Kindle on
  the web (up to 200 MB)."

**3. Indexing and thumbnails.** Copy only. The Kindle indexes `documents/` by itself after
unplug or eject [inference]. Cover thumbnails go in `system/thumbnails/` and need a MOBI/AZW3/KFX
with EXTH uuid + cdetype ([thumb-name]), so they wait for #320. Collections and APNX page numbers
stay out of scope (see findings).

**4. Safe remove.** Mounted volumes get an "Eject" button that runs the OS eject (calibre's
recipe, below). MTP has nothing to eject. Closing the session is enough, then the UI says "You can
unplug your Kindle".

## Findings (calibre)

### Device ids and generations

- Vendor id `0x1949` (Amazon) for every Kindle **[fact]** ([kindle-ids]).
- Mass-storage product ids in calibre's USBMS Kindle drivers **[fact]**:
  | Driver | Models (calibre's names) | PID | Books folder |
  |---|---|---|---|
  | `KINDLE` ([kindle-ids]) | Kindle 1 / Keyboard | `0x0001` | `documents` |
  | `KINDLE2` ([kindle2]) | Kindle 2/3/4/Touch/Paperwhite/Voyage (and later mass-storage models) | `0x0002`, `0x0004`, `0x0324` | `documents` |
  | `KINDLE_DX` ([kindle-dx]) | DX | `0x0003` | `documents` |
  | `KINDLE_FIRE` ([kindle-fire]) | Fire (Android) | `0x0006` | `Documents`, no subfolders |
- MTP-only generations. calibre's changelog says "2024 Kindles that use the MTP protocol"
  (7.25.0, 2025-02-07) and "2024 and newer MTP based Kindles" (8.7.0, 2025-07-18) **[fact]**
  ([changelog-7.25], [changelog]). It names no models and lists no product ids. The MTP driver matches on vendor
  only. libmtp's device table (vendored in calibre) lists "Kindle Scribe 32GB" `0x9981` plus Fire
  tablets **[fact]** ([libmtp-devices]). `mtp-rs` lists the Paperwhite 12th generation (2024) as
  "Full support" over MTP **[fact]** ([mtp-rs-readme]). Which other models are MTP-only (Colorsoft,
  Kindle 2024, Scribe 2024, and whether a firmware update moved any older model to MTP) is
  **[unverified]**. Detection must therefore not hard-code product ids.

### Mass storage (USBMS)

- Books go in `documents/`, and subfolders are allowed on e-ink models **[fact]** ([kindle-ids]).
  calibre once tried and dropped "no subfolders on Paperwhite" (commented out in [kindle2]).
- The macOS volume names calibre matches are `Kindle Internal Storage Media`. Windows
  `INTERNAL_STORAGE` **[fact]** ([kindle-ids]). mangas matches on `documents/` + `system/` or a
  "kindle" name instead, which is looser and fine.
- Eject **[fact]**:
  - Linux: udisks over D-Bus, unmount each partition then `Drive.Eject` ([eject-linux], [udisks]).
  - macOS: `diskutil eject <mount>` ([eject-osx], `OSX_EJECT_COMMAND` in [usbms-device]).
  - Windows: a helper `calibre-eject.exe` ([eject-windows]) does `FSCTL_LOCK_VOLUME`,
    `FSCTL_DISMOUNT_VOLUME`, `IOCTL_STORAGE_MEDIA_REMOVAL` (allow), `IOCTL_STORAGE_EJECT_MEDIA`,
    then `CM_Request_Device_EjectW` on the parent device node ([eject-c]).

### MTP

- Linux/macOS: libmtp through a C extension ([libmtp-c]). It detects with
  `LIBMTP_Detect_Raw_Devices`, opens with `LIBMTP_Open_Raw_Device_Uncached` and uploads with
  `LIBMTP_Send_File_From_Handler` **[fact]**. Before probing, calibre asks the OS whether the
  device has an MTP interface (sysfs on Linux, IOKit via its `usbobserver` on macOS) and skips
  Apple devices **[fact]** ([unix-isMTP], [unix-detect]).
- Windows: its own WPD COM extension (`ole32`, `portabledeviceguids`) **[fact]**
  ([extensions-wpd]). A device counts when its protocol is `MTP:` and it has read-write storage
  that is not DCF (cameras, iPhones) **[fact]** ([wpd-suitable]).
- Where books go: for vid `0x1949` the first existing folder of `documents`, `kindle`, `books`
  **[fact]** ([mtp-defaults], [mtp-prefix]).
- Naming: calibre's template `{title} - {authors}`, ASCII-sanitised, max path 230 characters
  ([mtp-prefs], [mtp-uploadpath]). mangas keeps its own file name. On MTP that only needs a length
  cap and a check for characters the device rejects [inference].
- Eject on MTP just drops the session. No OS call **[fact]** ([unix-eject], [wpd-eject]).
- One program at a time: "on macOS and Linux only a single program can connect to an Android
  device at a time … not opened in the OS File manager, or the Android File Transfer utility"
  **[fact]** ([faq-mtp]). On Linux, gvfs is that file manager.

### Thumbnails, sidecars, collections

- Thumbnail path: `system/thumbnails/thumbnail_<uuid>_<cdetype>_portrait.jpg`. uuid and cdetype
  come from the MOBI EXTH header or the KFX container. Other formats get none **[fact]**
  ([thumb-name], [thumb-dir]). Height 500 px **[fact]** ([thumb-height]).
- The "Amazon cover bug": the Kindle deletes side-loaded thumbnails. calibre keeps a copy in
  `amazon-cover-bug/` on the device and restores any missing or changed one on every connect,
  over both USBMS and MTP **[fact]** ([cover-bug], [mtp-cover-bug]).
- APNX page-number sidecars exist only for MOBI/AZW/AZW3 text books. They go into `<book>.sdr/`
  when the device already has `.sdr` folders **[fact]** ([apnx], [sidecar]). Fixed-layout manga
  has real pages, so APNX doesn't apply [inference].
- Collections: "collection information is not stored in file metadata" **[fact]**
  ([collections-note]). Writing them means editing the Kindle's own database. Out of scope.

## Formats and sizes over USB

- calibre's Kindle format lists: e-ink `azw, mobi, azw3, prc, azw1, tpz, azw4, kfx, pobi, pdf,
  txt` ([kindle2]), MTP Kindles `azw3, mobi, azw, azw1, azw4, kfx, pdf` ([mtp-defaults]). **No
  EPUB, no CBZ** **[fact]**.
- Amazon lists EPUB as supported by Send to Kindle, max 200 MB for the web uploader **[fact]**
  ([amazon-stk], [amazon-formats]). The 50 MB email limit is the one mangas already enforces (PR
  [#116]).
- A side-loaded `.epub` on current e-ink firmware is **[unverified]**. My inference is that it
  doesn't appear in the library. If that holds, "Copy over USB" only makes sense today for the
  **PDF** export, and for EPUB once #320 picks AZW3/KFX. The USB part of this doc doesn't depend
  on that choice except for thumbnails.
- Size: no Amazon limit for USB found. Mass-storage Kindles use FAT32 [inference], which caps one
  file at 4 GiB. MTP `ObjectInfo.size` is u32 (4 GB), and newer devices report 64-bit sizes as a
  property **[fact]** ([mtp-rs-readme]). Exports over 200 MB (the case in #321) fit both.

## Current state in this repo

- `packages/core/src/send/usb/detect.rs`: looks in `/Volumes` (macOS), `D:`-`Z:` (Windows),
  `/media`, `/run/media`, `/mnt` and `/run/user/<uid>/gvfs/mtp:host=…/<storage>` (Linux) for a
  folder with `documents/`. `copy.rs` does the `.part` + fsync + rename copy.
- Callers: desktop `commands/send.rs` (`find_kindles`, `send_usb`, `usb::choose`), CLI
  `commands/send.rs` (`send --devices`, `--via usb`) and `commands/deliver.rs` (`export --send
  usb`). The UI state "Device not found" is `send-status.tsx`.
- There is no USB bus enumeration, so a plugged-in MTP Kindle looks the same as nothing plugged
  in ([#126]). On a gvfs mount, rename over an existing object and `sync_all` go through gvfs's
  MTP backend. Neither was tested on a real device **[unverified]**.
- Since #331, macOS and Windows builds with the `mtp` feature (on by default in the desktop
  binary) copy to an MTP-only Kindle directly: `send/mtp` (an `MtpStore` trait, `mtp-rs` behind
  it, a folder-backed fake for tests). The book goes to `documents/` on the first storage as a
  hidden scratch object that is renamed only once the upload completes; a failed or cancelled
  upload is deleted and the previous book stays. Linux keeps the gvfs folder path above and
  shows "connected over MTP" (with Email instead) when gvfs hasn't mounted the Kindle ([#332]).

## Per-OS analysis

### Linux

- **Exposed:** mass storage auto-mounts under `/media/<user>/Kindle` (udisks). MTP Kindles are
  mounted by gvfs-mtp at `/run/user/<uid>/gvfs/mtp:host=<id>/Internal Storage/` when GNOME (or a
  desktop with gvfs) is running. KDE uses kio-mtp, which has **no** FUSE path [inference], so
  KDE users see "Device not found" today.
- **Detection:** sysfs (`/sys/bus/usb/devices/*/idVendor`, `…:1.0/interface` = `MTP`), the same as
  calibre ([scanner-linux], [sysfs]). `nusb::list_devices` reads the same data.
- **Permissions:** listing needs none. Opening an MTP device needs a udev rule (libmtp's
  `69-libmtp.rules` or a `uaccess` rule); `mtp-rs` documents this **[fact]** ([mtp-rs-readme]).
  An AppImage can't install udev rules. The `.deb`/`.rpm` could.
- **Conflict:** while gvfs holds the device, a direct MTP client can't claim it [inference from
  [faq-mtp]]. Keep the gvfs path first and use direct MTP only when no gvfs mount exists.
- **Signing:** none.

### macOS

- **Exposed:** mass storage under `/Volumes/Kindle`. MTP: nothing. macOS has no MTP file system
  and Finder doesn't show the device.
- **Detection:** IOKit enumeration (`nusb`) needs no entitlement outside the App Sandbox
  [inference]. mangas is not sandboxed (`Entitlements.plist` only has
  `disable-library-validation`).
- **Transfer:** `ptpcamerad` claims MTP/PTP devices on connect and blocks other apps. Android File
  Transfer does the same **[fact]** ([mtp-rs-readme]). Workarounds are killing it in a loop or
  `sudo launchctl disable system/com.apple.ptpcamerad` (this may break Photos). `mtp-rs` reports
  this case as `Error::is_exclusive_access()`, and IORegistry `UsbExclusiveOwner` names the
  holder. The app should show that name and the command. It should not kill system daemons itself.
- **Signing/notarisation:** pure Rust (`mtp-rs`/`nusb`) adds nothing to sign. Bundling libmtp
  means shipping `libmtp.dylib` + `libusb.dylib` inside the `.app`, signed with our identity and
  hardened runtime, plus an LGPL notice. Downloading them at runtime like ONNX Runtime
  (`ml::runtime`) works only because library validation is already off, and there's no official
  signed libmtp build to pin a SHA-256 to [inference].

### Windows

- **Exposed:** mass storage gets a drive letter (works today). MTP Kindles show in Explorer under
  "This PC" through WPD, with no drive letter.
- **Detection:** `nusb` enumerates through SetupAPI. `interfaces()` is filled only for composite
  devices on `usbccgp` **[fact]** ([nusb-interfaces]). Vendor id is always there, and that is
  enough.
- **Transfer:** WPD COM is the only sane path. The device is bound to the WPD driver, so raw USB
  (WinUSB/libusb) can't open it without Zadig **[fact]** ([mtp-rs-readme]). `mtp-rs` picks its
  WPD backend on Windows by itself. `winmtp` wraps WPD directly.
- **Signing:** no change. COM is in-box.

## Rust implementation options

| Option | What | Maturity | Bundling cost | Licence | OS coverage |
|---|---|---|---|---|---|
| `nusb` 0.2.8 | USB enumeration (detect only) | 1.78M downloads, active (2026-10) | pure Rust, none | MIT/Apache-2.0 | Linux, macOS, Windows |
| `rusb` 0.9.4 | libusb bindings (detect only) | 9.7M downloads, last release 2024-04 | vendors/links libusb (C) | MIT (libusb LGPL-2.1) | all three |
| sysfs / IOKit / SetupAPI by hand | detect only | n/a | none, `windows-sys` already in desktop | n/a | per-OS code ×3 |
| **`mtp-rs` 0.33.0** | pure-Rust MTP on `nusb`, WPD backend on Windows | young: ~9.5k downloads, frequent releases, lists Kindle PW 12th gen as tested | none; pins `windows =0.62.2`, the same version already in `Cargo.lock` | MIT/Apache-2.0 | all three |
| `winmtp` 0.3.2 | WPD COM wrapper | ~9.4k downloads, 2026-03 | none, but pulls `windows` 0.52 (a second copy) | MIT | Windows |
| `libmtp-rs`/`libmtp-sys` | libmtp FFI | "alpha", last release 2020 | link-time `pkg-config`; ship libmtp+libusb dylibs | MIT bindings, libmtp LGPL-2.1 | Linux, macOS |
| libmtp via `libloading` | `dlopen` libmtp like `ort load-dynamic` | libmtp is mature (20+ years) | system lib on Linux; ship or download signed dylibs on macOS; none on Windows | LGPL-2.1 (dynamic loading keeps it replaceable) | Linux, macOS |
| `windows` crate WPD by hand | own COM code like calibre's `wpd.cpp` | n/a | none | MIT/Apache-2.0 | Windows |

Recommendation: **`nusb` for detection now. `mtp-rs` for transfer later, behind a `mtp` cargo
feature** (on by default only once the owner's Kindle passes the checklist). The libmtp route
costs a C toolchain, dylib signing on macOS, LGPL notices and a different backend on Windows. It
also doesn't fix `ptpcamerad`, which blocks libmtp just the same. Fallback if `mtp-rs` stalls:
libmtp via `libloading` on Linux/macOS plus `winmtp` on Windows.

## Effort estimate (solo, human days)

| Step | Days | Notes |
|---|---|---|
| A. [#126] detect + "use Email" hint (core, CLI, desktop) | 1-2 | new dep `nusb`, unit tests on a fake device list |
| B. USB only for formats the Kindle opens | 1 | after checklist item 6. Wording follows #320 |
| C. Eject for mounted volumes (3 OSes) | 2-3 | Windows IOCTL path is the long pole. Needs Windows CI proof |
| D. MTP copy on macOS + Windows (`mtp-rs`) | 5-8 | `.part`/rename dance, exclusive-access UX, owner testing |
| E. MTP copy on Linux without gvfs (KDE) | 1-2 | reuses D, plus a udev note in packaging |
| F. Cover thumbnails + cover-bug cache | 2-3 | only after #320 ships AZW3/KFX |

Total about 12-19 days. A unblocks users now. D is the risk.

## Proposed issues (in order)

1. **[#126] Send to Kindle over USB: detect a Kindle that has no drive and suggest Email.**
   - `send::usb` lists USB devices with vendor `0x1949`. One with no mounted volume is reported
     as reachable-later, not "Device not found".
   - Desktop send panel and `mangas send --devices` / `--via usb` show: device name, "connects
     over MTP, which mangas can't copy to on this computer yet", and a "Use Email" action (CLI:
     the `--via email` hint).
   - Mass-storage and gvfs Kindles behave exactly as today (existing `send_usb.rs` tests pass).
   - Unit tests on a synthetic device list: no device, mass storage only, MTP only, both.
   - Checked on the owner's Mac with his Kindle (checklist items 1-3).
2. **Copy over USB: only offer formats the Kindle opens.**
   - The format → "opens over USB" table lives in core, one place (PDF yes, EPUB/CBZ per checklist
     item 6 and #320).
   - Desktop and CLI refuse or warn before copying an unsupported format and point to Email
     (EPUB) or a PDF export.
3. **Eject the Kindle after a USB copy (mounted volumes).**
   - Desktop "Eject" after a successful copy, plus CLI `mangas send --eject`.
   - Linux udisks (unmount + eject), macOS `diskutil eject`, Windows lock/dismount/eject +
     `CM_Request_Device_EjectW`. A busy volume gives a message naming what to close.
   - Proof on Windows/macOS scratch-branch CI with a disk image or VHD, plus the owner's Kindle.
4. **Copy to MTP Kindles on macOS and Windows (`mtp-rs`, feature `mtp`).**
   - The same `Copied`/progress contract as volumes. A failed or cancelled upload leaves no
     partial object and keeps the previous book.
   - Book lands in `documents/` on storage 0. "Already on Kindle" works (same name and size).
   - macOS: when `ptpcamerad` or another app holds the device, the error names the holder and the
     command to free it. The app doesn't kill it itself.
   - Tested on the owner's Kindle on macOS. Windows proof on a Windows machine with an MTP device.
5. **Copy to MTP Kindles on Linux without gvfs.**
   - Used only when no gvfs mount exists for the device. Docs note the udev rule.
6. **Kindle cover thumbnails for side-loaded books** (blocked by #320 shipping AZW3/KFX).
   - Writes `system/thumbnails/thumbnail_<uuid>_<cdetype>_portrait.jpg` (500 px tall) and keeps a
     copy in `amazon-cover-bug/`, restored on the next connect, as calibre does.

## What I verified on the Linux VM

- `/sys/bus/usb/devices` is empty: the VM has no USB controller. Its kernel (`6.8.0-1061-aws`) has
  no `dummy_hcd`/`libcomposite`, so a fake gadget isn't possible either. No device-level check
  was possible here.
- gvfs isn't installed and `/run/user/1000/gvfs` doesn't exist. libmtp 1.1.19 and libusb 1.0.25
  are installed. The gvfs path stays covered only by the fake-folder tests in
  `packages/core/tests/send_usb.rs`.
- Crate facts (versions, dates, downloads, licences, dependencies) come from the crates.io API on
  2026-10-20. Amazon facts come from Wayback captures of the Send to Kindle pages.

## Owner checklist (Mac + Kindle)

Plug the Kindle in, unlock it, and note its model and firmware (Settings → Device info).

1. `system_profiler SPUSBDataType | grep -i -B2 -A14 kindle`: copy Product ID, Vendor ID
   (expect `0x1949`) and Serial Number.
2. `ls /Volumes`: is there a `Kindle` volume? Yes means mass storage. No means MTP.
3. `ioreg -r -c IOUSBHostInterface -l -w0 | grep -i -A3 '"USB Interface Name"'`: is there an
   interface named `MTP`? Also `ioreg -l -w0 | grep -i UsbExclusiveOwner` (who holds it) and
   `pgrep -l ptpcamerad`.
4. `brew install vdavid/tap/mtp-rs`, then `mtp-rs devices` and `mtp-rs ls /`. Does the Kindle
   show, and does `documents` exist? If it fails with exclusive access, run
   `while true; do pkill -9 ptpcamerad; sleep 1; done` in another tab and retry. Note whether that
   was needed.
5. `mtp-rs put <some mangas export>.pdf /documents/test.pdf`, then `mtp-rs rename
   /documents/test.pdf test2.pdf`. Unplug and check the book shows in the library with a cover.
6. Same with a mangas `.epub` (`mtp-rs put x.epub /documents/x.epub`, or a Finder copy for a
   mass-storage Kindle). Does it appear in the library? This decides issue 2.
7. Paste the outputs of 1-3 and the results of 4-6 on [#126].

[#116]: https://github.com/kerbaras/mangas/pull/116
[#126]: https://github.com/kerbaras/mangas/issues/126
[kindle-ids]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L84-L112
[kindle2]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L384-L398
[kindle-dx]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L689-L696
[kindle-fire]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L705-L719
[collections-note]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L40-L45
[thumb-name]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L48-L66
[thumb-height]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L471-L475
[sidecar]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L522-L542
[thumb-dir]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L558-L568
[cover-bug]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L570-L621
[apnx]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/kindle/driver.py#L640-L665
[usbms-device]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/usbms/device.py#L104
[eject-windows]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/usbms/device.py#L885-L900
[eject-osx]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/usbms/device.py#L902-L909
[eject-linux]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/usbms/device.py#L911-L925
[udisks]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/udisks.py#L149-L215
[eject-c]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/bypy/windows/eject.c#L125-L330
[scanner-linux]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/scanner.py#L100-L203
[sysfs]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/sysfs.py#L21-L46
[unix-isMTP]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/driver.py#L58-L91
[unix-detect]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/driver.py#L99-L133
[unix-eject]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/driver.py#L182-L193
[unix-put]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/driver.py#L399-L417
[libmtp-c]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/libmtp.c#L229-L253
[libmtp-devices]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/unix/upstream/music-players.h#L2541-L2605
[wpd-suitable]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/windows/driver.py#L208-L231
[wpd-eject]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/windows/driver.py#L292-L317
[wpd-put]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/windows/driver.py#L501-L515
[extensions-wpd]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/setup/extensions.json#L228
[mtp-defaults]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/defaults.py#L21-L34
[mtp-prefs]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/driver.py#L50-L95
[mtp-driver-iskindle]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/driver.py#L100-L102
[mtp-uploadpath]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/driver.py#L497-L515
[mtp-prefix]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/driver.py#L517-L534
[mtp-cover-bug]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/src/calibre/devices/mtp/driver.py#L616-L693
[faq-mtp]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/manual/faq.rst#L271-L278
[changelog]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/Changelog.txt#L1021-L1025
[changelog-7.25]: https://github.com/kovidgoyal/calibre/blob/dc29097d6ef22d32e5af85ce0d822cc7c08a64ea/Changelog.txt#L1355-L1361
[mtp-rs-readme]: https://github.com/vdavid/mtp-rs/blob/main/crates/mtp-rs/README.md
[nusb-interfaces]: https://docs.rs/nusb/0.2.8/nusb/struct.DeviceInfo.html#method.interfaces
[amazon-stk]: https://www.amazon.com/sendtokindle
[amazon-formats]: https://www.amazon.com/gp/help/customer/display.html?nodeId=G5WYD9SAF7PGXRNA
[#332]: https://github.com/kerbaras/mangas/issues/332
