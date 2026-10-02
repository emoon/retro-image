# Format documentation survey: IBM PC text-mode art (ANSI/BBS art)

Clean-room research notes for the text-mode art formats of the PC BBS and art-group scene.
RECOIL supports none of these formats, so the oracle can't check them: every corpus file is
recorded in `crates/retro-image/tests/divergences/textmode.tsv` with a Deark render or a visual
review as evidence. No RECOIL code and no GPL/LGPL decoder code was read. Surveyed 2026-10-02.

## Summary

| Docs quality | Count | Formats |
|---|---:|---|
| Spec | 4 | SAUCE (metadata), XBin, ANSI (ECMA-48 + viewer conventions), Avatar AVT/0 |
| Partial | 4 | BIN, ADF, PCBoard, TundraDraw |
| None | 1 | iCE Draw IDF (only permissively licensed code) |

All formats are cells of a character plus colours, drawn with a bitmap font: 8 pixels wide
(9 with VGA letter spacing), 16 high by default. The decoders are in
`crates/retro-image/src/platform/textmode/`.

## Umbrella sources

- **SAUCE specification** rev. 00.5 (ACiD): <https://www.acid.org/info/sauce/sauce.htm>.
  The 128-byte record at the end of a file (after an EOF byte 1Ah and an optional "COMNT"
  block): DataType/FileType, TInfo1 = width in columns for character streams, FileType = half
  the width for BinaryText, ANSiFlags (bit 0 non-blink/iCE colour, bits 1-2 letter spacing
  01 = 8 / 10 = 9 pixels, bits 3-4 aspect ratio) and TInfoS font names ("IBM VGA" 8x16,
  "IBM VGA50"/"IBM EGA43" 8x8, "IBM EGA" 8x14, "IBM VGA25G" 8x19, code-page and Amiga variants).
  It also states that the VGA repeats the 8th glyph column only for C0h-DFh.
- **XBin specification** (ACiD), archived:
  <https://web.archive.org/web/20120204063040/http://www.acid.org/info/xbin/x_spec.htm>.
- **ECMA-48** (<https://ecma-international.org/publications-and-standards/standards/ecma-48/>):
  control sequence syntax, CUU/CUD/CUF/CUB/CUP/ED/EL/SGR.
- **FSC-0025** (<http://ftsc.org/docs/fsc-0025.001>) and **FSC-0037**
  (<http://ftsc.org/docs/fsc-0037.001>): Avatar AVT/0 and AVT/0+.
- **PabloDraw 24-bit ANSI**: <http://picoe.ca/2014/03/07/24-bit-ansi/> (`ESC[0|1;r;g;bt`).
- **Deark** (<https://github.com/jsummers/deark>, MIT; its `foreign/` files have their own
  licences and were not read): `modules/ansiart.c`, `modules/bintext.c` (BIN, XBin, ADF; IDF is a
  stub), `modules/sauce.c`. Deark 1.7.3 was also run as a black-box reference renderer
  (`-opt char:output=image`).
- **libansilove** (<https://github.com/ansilove/libansilove>, BSD-2-Clause):
  `src/loaders/{ansi,icedraw,tundra,pcboard}.c`, `src/drawchar.c`, fonts.
- **Moebius** (<https://github.com/blocktronics/moebius>, Apache-2.0): `app/libtextmode/`
  (ANSI, BIN, XBin; no IDF/TND). Skimmed for XBin only; it rejects 512-character XBin too.
- **Just Solve**: <http://fileformats.archiveteam.org/wiki/ICEDraw> (identification only).

## Fonts

| Font | Source | Licence |
|---|---|---|
| VGA 8x16, CP437 | Deark `src/deark-data.c` `vga_cp437_font_data`; byte-identical to libansilove `font_pc_80x25.h` | MIT; Deark's readme: "VGA and CGA bitmapped fonts ... have no known copyright claims". IBM VGA ROM dump. |
| VGA 8x8, CP437 | libansilove `src/fonts/font_pc_80x50.h` | BSD-2-Clause. IBM VGA ROM dump. |

Bitmap font data isn't protected by copyright in the US (37 CFR 202.1(e)); the notices are kept
in `textmode/font.rs` anyway. Not built in: 8x14 (IBM EGA), 8x19, non-437 code pages and the
Amiga fonts. They fall back to the 8x16 CP437 font.

## Formats

| Ext | Format | Docs | Sources | Notes |
|---|---|---|---|---|
| ANS | ANSI art | **Spec** | ECMA-48, SAUCE, Deark `ansiart.c`, libansilove `ansi.c`, PabloDraw page | Viewer conventions (from Deark): LF also returns to column 0; bold and iCE blink brighten at write time, then reverse swaps; erased cells are grey-on-black spaces; `ESC[?33h` sets iCE; SGR 38/48;2, 90-97/100-107 and PabloDraw `t`; SAUCE width used for 40-2048. Height = lowest written row. Ours (ANSI.SYS-like, differs from Deark only in rare cases): cursor moves clamp to the screen, row/column 0 means 1, BEL is silent, BS moves left, TAB goes to the next multiple of 8, 1Ah ends the text, a lone ESC is dropped. Underline (SGR 4) isn't drawn: VGA colour text has none. |
| BIN | Binary Text | **Partial** | SAUCE (DataType 5, width = 2 x FileType), Deark `bintext.c` | No header. Without SAUCE Deark assumes 160 columns. We accept only exact sizes: a SAUCE BinaryText record, 4000 bytes (one 80x25 screen; Deark gives 12.5 rows of 160) or a multiple of 320 bytes (160 columns). Powers of two (ROM dumps) never divide by 320. ACiDDraw quirk: FileType 1 with TInfo1 = half width. |
| XB | XBin | **Spec** | XBin spec, Deark `bintext.c` | Magic `XBIN` 1Ah. Optional 6-bit palette, font (1-32 rows), row-wise RLE, non-blink flag. 512-character mode is rejected (the spec doesn't say how the attribute selects the second set; Deark and Moebius reject it too). |
| ADF | ArtWorx Data Format | **Partial** | Deark `bintext.c` | Version byte 1, 64-entry 6-bit EGA palette (text colours are entries 0-5, 20, 7, 56-63), 8x16 font, then 80-column iCE BIN data. No magic, and `.adf` is also the Amiga disk image extension, so we require version 1, all palette bytes 0-63 (not all zero), and whole 160-byte rows. |
| IDF | iCE Draw | **None** | libansilove `icedraw.c`, samples | Header `04 "1.4"`, x1, y1, x2, y2 (LE words, width = x2 + 1); pairs where char 1 starts a run (`01 xx count-word char attr`); 4096-byte font and 48-byte palette at the end; always iCE. libansilove ignores SAUCE (and so reads the record as palette) and hard-codes 80 columns for the height. |
| TND | TundraDraw | **Partial** | libansilove `tundra.c`, SAUCE (1/8) | Header `18h "TUNDRA24"`; commands 1 (row, column BE32), 2/4/6 (char + 24-bit fg/bg/both as BE32 00RRGGBB). libansilove draws a SAUCE record as text. One sample labels itself FileType 7 but needs the TInfo1 width, so any Character record's width is used. |
| PCB | PCBoard @-codes | **Partial** | libansilove `pcboard.c`, SAUCE (1/4) | `@X` + two hex digits (bg, fg), `@CLS@`. Other `@MACRO@`s are live BBS values; we show them as text. `.pcb` is also a circuit-board extension, so at least one `@X` code is required. |
| AVT | Avatar | **Spec** | FSC-0025, FSC-0037, SAUCE (1/5) | AVT/0 commands (^L, ^Y, ^V^A-^V^H) are interpreted. AVT/0+ ones are skipped by their argument length. Unknown ^V commands (AVT/1, undocumented; e.g. dexvert `DEMO1.AVT`) make the file rejected. At least one Avatar code is required. |

## Not honoured

- **Aspect ratio** (SAUCE AR bits; legacy displays stretch 1.2x at 640x400, 1.35x at 720x400): not an
  integer factor, so pictures keep square pixels, as in Deark (which only writes PNG density) and
  ansilove (which needs an option to stretch).
- Blinking: a still picture shows blinking characters in their visible phase.
- Palette scaling: 6-bit DAC values become `v*4 + v/16`, like the other VGA palettes in this
  project (observed from `recoil2png`). Deark rounds `v*255/63`, which differs by 1 for some values.

## Deark disagreements

- 9-pixel letter spacing: Deark repeats the 8th column for B0h-DFh. The SAUCE spec and the VGA
  (Attribute Controller Line Graphics Enable) do it only for C0h-DFh, and the spec says the
  shade blocks B0h-B2h show a visible break. We follow the spec (`fil-412.bin`).
- 4000-byte BIN without SAUCE: see BIN above.

## Samples

`corpus/extra/textmode/sembiance/` (dexvert sample set, <https://sembiance.com/fileFormatSamples/image/>):
32 ANS, 14 BIN, 10 XB, 12 ADF, 7 IDF, 9 TND, 11 PCB, 20 AVT. The 16colo.rs site disallows
crawling raw `.ANS`/`.ICE` files in its robots.txt, so nothing was fetched from it; its GitHub
mirror (<https://github.com/sixteencolors/sixteencolors-archive>, packs by year) is a source
for later work.

## To avoid (do not read the code)

| Project | Licence | Relevance |
|---|---|---|
| RECOIL | GPL | Doesn't support these formats anyway. |
| FFmpeg `libavcodec/ansi.c`, `bintext.c` (BIN, XBin, IDF), `tmv.c`, `libavformat/bintext.c` | LGPL-2.1+ (from general knowledge, not checked this session) | ANSI, BIN, XBin, IDF decoding and SAUCE probing. Just Solve cites it for IDF identification. |
| SyncTERM / cterm (Synchronet) | GPL (from general knowledge, not checked this session) | ANSI terminal emulation, fonts. |
| libsauce and other SAUCE libraries | not checked | The spec is enough. |

Usable (permissive, read): Deark (MIT), libansilove/ansilove (BSD-2-Clause), Moebius
(Apache-2.0). PabloDraw is MIT but wasn't needed.
