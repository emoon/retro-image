# Acorn Archimedes / RISC OS and Sinclair QL formats: documentation survey

Clean-room research notes for RISC OS (Acorn Archimedes, RiscPC and later) and the
Sinclair QL. RECOIL supports neither platform, so there is no RECOIL format list to
follow: the formats below were chosen from the platforms' own documentation. No RECOIL
code and no GPL decoder source was read while collecting these notes.

Researched 2026-10-02.

- `dilwyn.theqlforum.com` (Dilwyn Jones' QL pages) sits behind a Cloudflare challenge,
  and `dilwyn.me.uk` is down and excluded from the Wayback Machine. Dilwyn Jones'
  documents were read from the copies on the `sinclairql.net` mirror
  (`https://www.sinclairql.net/djw/...`) and inside the archives distributed there.
- `riscosopen.org`'s robots.txt excludes Anthropic's crawlers. Four of its wiki pages
  ("Sprite Mode Word", "Format Of Sprite", "Screen Modes", "File formats") were fetched
  before this was noticed; nothing in the decoder relies on them any more (RISC OS 5 mode
  words, modes 47-53 and the ROOL palette-length rule were removed). The PRM on
  riscos.com is the source.
- GitHub (robots.txt) and `arcsite.de` (no response) were not used.

## 1. Summary

| Platform | Format | Docs quality | Status |
|---|---|---|---|
| RISC OS | Sprite file (`&FF9`) | **Spec** | Implemented (`platform/risc_os/sprite.rs`) |
| RISC OS | Clear (`&690`) | **None** found (no permissive spec) | Not implemented |
| RISC OS | Draw (`&AFF`), ArtWorks | Spec (vector) | Out of scope: vector formats |
| QL | Screen dump, 32 KB, mode 4 / mode 8 | **Spec** (hardware) | Implemented as `.QS4` / `.QS8` |
| QL | PIC / PSA area save (`$4AFC`) | **Spec** | Implemented, recognised by content |
| QL | Image Processor IP2C/IP3C, Page Designer PD2/PD3, Professional Publisher | **Spec** (Dilwyn Jones) | Not implemented (leftover) |
| QL | Eye-Q compressed screens, The Painter compressed pictures | **None** | Not implemented |

## 2. RISC OS

### Sources

- RISC OS PRM (Programmer's Reference Manuals, riscos.com edition):
  - "Sprites", sprite area and sprite header layout, word-aligned rows, least significant
    pixel leftmost, palette entries, masks:
    <http://www.riscos.com/support/developers/prm/sprites.html>
  - "Appendix E: File formats": a sprite file is a sprite area without its first word:
    <http://www.riscos.com/support/developers/prm/fileformats.html>
  - "Table B: Modes", mode numbers 0-46 with pixel and OS-unit resolution and colours
    (the eigen factors, i.e. the pixel shape, follow from OS units / pixels):
    <http://www.riscos.com/support/developers/prm/modes.html>
  - "VDU drivers", the default 256-colour palette bit layout (tint bits) and the VIDC1
    palette-index override: <http://www.riscos.com/support/developers/prm/vdu.html>
  - "The Window Manager", Wimp colours and their mapping in 2- and 4-colour modes; the
    `*Desktop_SetPalette` example lists the RISC OS 3 Wimp palette:
    <http://www.riscos.com/support/developers/prm/wimp.html>
  - Volume 5a, "Video": new format sprites (RISC OS 3.5 mode word with the sprite type in
    bits 27-31 and DPI fields, no left-hand wastage, types 1-8, 16 and 32 bpp pixel
    layouts, 1 bpp masks, DPI to eigen factors, palettes for up to 8 bpp from RISC OS 3.6):
    <http://www.riscos.com/support/developers/prm/video.html>
- Deark, `modules/rosprite.c` (MIT): black-box reference (`deark -m rosprite`) and the
  source of the RISC OS 3.5 Wimp palette values and flash-colour averaging. Its notice is
  reproduced in `sprite.rs`.

### Sprite files (`&FF9`)

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| `,ff9` (RISC OS-style name), `.ff9`, `.spr` | RISC OS sprite file | **Spec** | PRM "Sprites", "Video" (vol. 5a), "Appendix E" | Header: sprite count, offset to first sprite, offset to free word (area offsets, 4 more than file offsets). Each sprite: 44-byte header (next offset, 12-byte name, width in words - 1, height - 1, first/last used bit, image and mask offsets, mode word), optional palette (pairs of `&BBGGRR00` words), image, mask. Mode word: a mode number (< 256) or a RISC OS 3.5 word (type in bits 27-31, DPI fields, bit 0 set). RISC OS 5 added another mode-word layout and more sprite types; not supported (documented only on riscosopen.org, see above). |

Decisions:

- **Which sprite:** the first one in the file.
- **Detection:** RISC OS-style names (`Sprites,ff9`) have no extension, so the header is
  validated strictly and the format is marked `.signature()`: count 1-10000, offsets
  word-aligned and inside the file, a printable NUL-padded name, a known mode, used bits
  on pixel boundaries, image data inside the file.
- **Sizes:** some files had a palette added without the sprite size and the area's free
  offset being updated (sembiance `*.bin,FF9`), so the image only has to fit in the file.
- **Palettes:** a sprite palette with an entry for every colour is used; a 16-entry
  palette in a 256-colour sprite is a VIDC1 palette (low nibble picks the entry, high
  nibble overrides red bit 3, green bits 2-3, blue bit 3, guns are 4 bits; found in
  mode 28 samples); other lengths are ignored. The palette runs from the header to the
  image. Without a palette, 1/2/4 bpp sprites use the Wimp
  colours (white/black; white, light grey, dark grey, black; the 16 Wimp colours), as
  Paint shows them, and 8 bpp sprites the default 256-colour palette.
- **Modes:** 0-46 except the text modes 3, 6 and 7 (and 32, undefined in the PRM).
- **Pixel shape:** pixels are repeated to make them square, from the mode's eigen
  factors (old modes) or the DPI fields (new format). E.g. modes 0, 8, 12, 15 (640x256)
  get doubled rows, modes 2, 5, 10 (160x256) doubled columns, modes 18-21 and 25-31
  nothing.
- **Deep colour:** types 5 (16 bpp) and 6 (32 bpp). Types 7 (CMYK) and 8 (24 bpp) are
  "not supported within RISC OS" and rejected, as are reserved types.
- **Masks** are ignored (no alpha channel in `Image`).

### Clear (`&690`)

No permissively licensed or prose specification was found (the PRM does not cover it,
Deark has no module). Left out.

## 3. Sinclair QL

### Sources

- T. Tebby and D. Karlin, "Sinclair QL Software Developer's Guide" (1984), section 10.2
  "Display Control": screen memory from `$20000`, 128 bytes per line, the word formats of
  512-pixel mode (G7-G0, R7-R0) and 256-pixel mode (G F G F ... / R B R B ...), flash
  toggle semantics:
  <https://www.sinclairql.net/downloads/1984-00_Sinclair_QL_Software_Developers_Guide_by_Tony_Tebby_and_David_Karlin-OCRed-SQPP.pdf>
- Dilwyn Jones, QL documentation pages (<https://dilwyn.theqlforum.com/>, Cloudflare
  challenge; read from the mirror index
  <https://www.sinclairql.net/djw/docs/formats/index.html>), file formats section:
  - "QL Graphics File Formats" (`graphics.zip`): mode 4/8 screen word layout and colours, 32 768-byte
    screens, PIC header, IP2C/IP3C, Page Designer PD2/PD3, Eye-Q, Professional Publisher.
  - "Partial Screen Area Saves, or PIC/PSA Files" (`pics.zip`): PIC (10-byte header) and PSA (the same
    after 4 undefined bytes), width in 512-pixel coordinates, line increment, mode byte.
  - "File headers" (`filehedr.txt`): the 64-byte QDOS directory header (not part of the file data).
  - "Filename extensions" (`exts.txt`): `_scr`, `_scn`, `_pic`, `_psa` conventions.
- C. Delhez, ShowQS (1992) documentation `SHOWQS.TXT`, in
  <https://www.sinclairql.net/djw/graphics/showqs.zip>: `.QS4` / `.QS8` extensions for 32 KB mode 4 / mode 8 screens on MS-DOS.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| `QS4`, `QS8` (QL: `_scr`, `_scn`) | 32 KB screen dump | **Spec** | Developer's Guide 10.2, Dilwyn Jones, ShowQS | 256 lines of 128 bytes. The mode is not stored. Mode 4: 512x256, black/red/green/white. Mode 8: 256x256, 8 colours, flash bit per pixel pair. |
| `PIC` (QL: `_pic`) | Pointer environment area save | **Spec** | Dilwyn Jones | Big-endian header: `$4AFC`, width (512-pixel coordinates), height, line increment, mode (0/4 = mode 4, 8 = mode 8, GD2 modes 16/32/33 have other pixel formats), spare 0; then increment x height bytes. |
| `PSA` (QL: `_psa`) | George Gwilt area save | **Spec** | Dilwyn Jones | 4 undefined bytes, then a PIC file. |
| `_ip` IP2C / IP3C | Image Processor compressed screen | **Spec** | Dilwyn Jones | Tagged RLE with a per-file flag word; IP3C has a 12-byte header. |
| PD2P/PD2C, PD3P/PD3C | Page Designer pages | **Spec** | Dilwyn Jones | Mode 4 layout pages, optionally tagged-RLE compressed. |
| `DTP3PAGE` | Professional Publisher page | **Spec** | Dilwyn Jones | 1 bpp, 8-byte magic, width, height. |

Decisions:

- **Raw screens and the mode rule:** the mode can't be read from the file, and `.scr`
  is shared with ZX Spectrum, Amstrad CPC (including 32 KB overscan screens that RECOIL
  decodes), Atari 8-bit and Electronika formats, while `.pic` 32 KB files are Apple IIGS
  screens. Content can't tell a QL screen from a CPC overscan one, so raw screens are
  registered only under ShowQS's `.QS4` / `.QS8`, where the extension gives the mode.
  For files named `_scr` on the QL, a mode heuristic works well on every sample examined
  (about 90 screens): mode 8 screens have (almost) no flash bits, i.e. set bits at
  positions 6, 4, 2, 0 of the even bytes are under 5% of those at 7, 5, 3, 1, while mode 4
  screens have nearly equal counts (ratio 0.8-1.2). It is not implemented because such
  names have no extension for the registry to match (see the shared-change proposal in
  the report).
- **Pixel shape:** rows are doubled (512x256 screens show as 512x512, the 4:3 display
  shape); mode 8 pixels are two mode 4 pixels wide.
- **Flash:** mode 8 flash bits are ignored: pixels are shown in their own colour, the
  steady phase.
- **Mode 4 colours:** black, red, green, white (the Developer's Guide's colour list;
  green and blue being "tied" in mode 4 only matters for white).

## 4. To avoid (GPL code)

Do not read the source of any of these (manuals and prose documentation are fine):

- **RECOIL** (all languages and ports). It doesn't support these platforms anyway.
- **Arculator**, **RPCEmu** and **ArcEm** (Archimedes / RiscPC emulators, GPL).
- **uQLx** and the emulators derived from it, e.g. **sQLux** (QL emulators, GPL), and
  **Minerva** (QL ROM replacement, GPL).
- **netpbm** parts under GPL and **GIMP** plug-ins (GPL).
- Any sprite or QL screen converter without a stated licence: prose only.

Licences not checked in this session (check before reading code): ChangeFSI and Paint
in the RISC OS Open sources, and Q-emuLator, QPC2 and SMSQ/E.
