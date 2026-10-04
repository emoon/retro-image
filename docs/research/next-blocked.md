# Blocked RECOIL formats: second look

Research only (2026-10-04): no decoder code, nothing committed. These are the formats still
marked "not covered" in `docs/coverage.md` that earlier waves left blocked. Method: prose docs,
a few MIT-declared prose specs, and black-box probing of `recoil2png` with mutated and
synthesised files (scratch files kept outside the repo). No RECOIL source and no GPL/LGPL
decoder was read. Difficulty: S (an hour or two), M (a day), L (open-ended).

## Summary

| Format | Verdict | Diff. | API change | Samples |
|---|---|---|---|---|
| FM Towns Pi | model `TOWN` needs its own palette precision and no height doubling, then register | S | no | none (patch a PC-98 Pi header to test) |
| Atari ST IFF | registry entry for the existing ILBM/VDAT decoder | S | no | corpus IFFs all decode already |
| KTY (PC-88) | full prose spec found; reuse the KT4 code with a 200-line mode | S | no | none; synthesise from the spec, oracle accepts |
| Rambrandt RM1, RM3 | same containers as RM2/RM4, GTIA 9/11 pixel decode | S | no | none; rename TITLE.RM2 / ADVANCED.RM4 |
| Rambrandt RM0 | same container, 96-line mode; table check differs | S-M | no | none; synthesise |
| SRI + PL7 (MSX2) | raw 108544 bytes, plain 512x424 raster, palette from PL7 | S | no (existing `pl7` companion) | none; synthesise from SR7 |
| VSC + G2F (Atari) | decoder is a loop over `decode_g2f`; needs a name-based companion lookup | S-M | **yes** (`Companions::get_named`) | 2 sets local |
| HPM (Atari) | stream solved; palette rule not; 15 samples found on the program disk | M | no | 1 local, 14 new |
| SPC (Atari Picture Painter) | command set matches the Apple II one; shapes and fill need probing | L | no | 4 local, 3 more on the ATR |
| ML1 / MX1 (PC-98) | header, palette and MX1 transport solved; pixel coder open | L (MX1 is S after ML1) | no | 2 local, 4 new |
| Q4 / XLD4 (PC-98) | container solved, codec unknown; only a licence-blocked source exists | L, blocked | no | 9 local, 12 new |
| PIX (Atari 8-bit) | no spec, no sample, no name for the program | blocked | no | none |

Suggested order: FM Towns Pi, Atari ST IFF, KTY, RM1/RM3, SRI, RM0 (each is nearly registry
work), then VSC, HPM, SPC, then ML1/MX1, with Q4 and PIX last.

## Findings that change earlier notes

- **I read Rambrandt's documentation disk** (`Rambrandt_docs.ATR`). Earlier notes list it as
  "not read". It has the picture-file layout (below), so RM0/RM1/RM3 are no longer "no layout".
- **The Grass' Slideshow program disk holds 15 `.hpm` pictures**, not one. Every one decodes in
  `recoil2png`. Earlier notes had only `JORDAN.HPM` plus the unrelated 19203-byte `DRAGON.HPM`.
- **KTY has a written spec** in an MIT-declared repo (see KTY). Earlier notes said "no byte
  layout found".
- **sembiance.com** (dexvert samples) has 12 more Q4 files and 4 more ML1 files than the corpus.

## FM Towns Pi

- State: `fm_towns.rs` registers PIC, ICN and HEL only. Pi has a decoder
  (`nec_pc/pi.rs`, source: Kirinn Bunnylin, <https://mooncore.eu/bunny/txt/pi-pic.htm>, prose).
  The spec only says the saver model "can be ignored, except to take into account how many colors
  the source system was capable of displaying".
- Probe: patching the 4-byte model field of `41_47.PI` to `TOWN` changes `recoil2png`'s output;
  `TWNS`, `FMT `, `FMTO`, `XXXX`, `PC88` and `PC98` give the PC-98 result. With `TOWN`:
  - the height is not doubled at 2:1 aspect (338x135 against 338x270), like `X68K`;
  - palette components are 5 bits: `0xEE` shows as 239, `0xCC` as 206 (`v >> 3`, scaled by
    255/31 and rounded), unlike `X68K`'s own colour word.
  The same change on `LINEARIT.PI` and `BALLCATH.PI` also moves the output, so it is not specific
  to one file.
- Plan: add `b"TOWN"` arms to `machine()` (`Machine::FmTowns` is already in the enum),
  `precision()` (a 5-bit variant) and `doubles_height()` in `nec_pc/pi.rs`, then register
  `Format::new("FM Towns", "Pi", &["pi"], ...)`.signature() in `fm_towns.rs`, next to the PIC entry.
- Samples: none from a real Towns. Test by patching the header of the PC-98 Pi files and
  comparing with the oracle. Real files would also check the aspect bytes.
- Sources: mooncore (prose, licence unstated, facts only); the rest is observed from `recoil2png`.

## Atari ST IFF

- RECOIL's row "Atari ST/STE | IFF | Interchange File Format" is uncovered only because our
  registry has no IFF entry under platform "Atari ST". The decoding exists:
  `amiga/ilbm.rs` plus `amiga/vdat.rs` (DeluxePaint ST vertical RLE) and `atari_st/iff.rs`
  (BL1-BL3, NEO with `RAST`).
- Every `.iff` and `.lbm` in the corpus decodes in both RECOIL and our CLI (checked).
- Plan: one `Format::new("Atari ST", "IFF", &["iff"], ...)` delegating to the Amiga ILBM
  decoder, no `.signature()` (the Amiga entry already owns content detection), the way the
  Falcon TIMG entry reuses the GEM IMG decoder. Check the ST palette handling (ST CMAP is 3 bits per
  channel) against a genuine DeluxePaint ST file before relying on it; the colour-map rule noted
  in `atari_st/iff.rs` (high nibble times 0x11) came from BL/NEO files.
- Samples: none with `VDAT` in the corpus (the VDAT decoder says it was found from sample files; I
  did not find which). Candidates: Atari Forum Wiki DEGELITE blocks, dexvert `iffILBM`.
- Sources: <https://temlib.org/AtariForumWiki/index.php/IFF_file_format> (prose, VDAT layout).

## KTY (NEC PC-88 Kitty, 640x200)

- Spec: `spec/KTY_FORMAT.md` in <https://github.com/rururutan/ifkty>. The repo is MIT (LICENSE
  checked, 2026). Its README says the decoder is based on analysis of xgload (no stated
  licence) and other public implementations, and the spec text credits xgload, MKIP100's KTY
  expander and sample files, drafted with ChatGPT Codex. **Only the spec was read; no source
  file in that repo was opened.** Treat it as prose with unclear provenance: use it as a
  cross-check, not as the only source. Confidence: high, because it agrees with what
  `nec_pc/kt4.rs` found by probing.
- Layout (from the spec, matching KT4): no magic, size or palette. 160x100 tiles. Shared-tile
  groups: a `mode` byte (`0xFF` ends the groups), a tile (3 bytes for modes 0 and 1: blue, red,
  green, high nibble row 0, low nibble row 1, bit 3 leftmost; 6 bytes for mode 2 and up),
  rectangle entries (`hi & 0xC0`: 00 rectangle, 40 horizontal run, 80/C0 vertical run; start cell
  `((hi & 0x3F) << 8 | lo)`), `0xFF`, single cells `(x, y)`, `0xFF`. After the groups, raw tiles for
  every unassigned cell in raster order. Height is 200 if the last mode was 0, else 400. Mode 0 is
  the KTY mode (a 4x2 tile); mode 1 repeats the 2 rows to make 4x4; mode 2+ has four independent
  rows. Up to 2 surplus bytes at the end occur in some real files, and the original loaders ignore
  them.
- Probe: a synthetic file of mode 0, one shared tile, one point and 15999 raw tiles is accepted by
  `recoil2png` as `.kty` and drawn 640x400, so RECOIL doubles KTY lines. The spec says the pixel
  aspect is 1:2. Decide: output 640x400 like RECOIL, or 640x200 with aspect metadata.
- Open: our `kt4.rs` rejects a missing or extra byte at the end, as RECOIL does for KT4; the spec
  says KTY may have 2 surplus bytes. Check RECOIL's rule on a padded synthetic KTY.
- Plan: generalise `kt4.rs` to follow the last mode (200 or 400 lines, raw tile size) and register
  `.kty` by extension only (no magic, as KT4).
- Samples: none found. wakachan's site (ateliermw.com) saves KTY since 2019 but publishes only PNG
  conversions; sembiance has only KT4. Test with synthesised files and the oracle.
- Other sources: <https://note.com/ftz/n/n84d9dd98c1e2> (prose, already cited by `kt4.rs`),
  <https://sourceforge.net/p/recoil/bugs/86/> (ticket text, no layout).

## Rambrandt RM0, RM1, RM3

- Source: `Rambrandt_docs.ATR` from <http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/RambRant/>,
  files `DOC.003` (Appendix I, save modules) and `DOC.004` (Appendix II, picture organisation;
  Appendix III, the BASIC loader). Copyright 1985 Antic Publishing and Bard Ermentrout, so read as
  prose and cited for facts only. Confidence: high for the layout, medium for the exact table rules.
- Modes (manual): Rambrandt mode 0 = ANTIC 7+16, 160x96, 4 colours; 1 = GR 9, 80x192; 2 = GR 10;
  3 = GR 11; 4 = GR 15+16, 160x192, 4 colours. The extension digit is the mode number. Display-list
  interrupts change colours on chosen lines, which is why RECOIL lists 99, 256, 104 or 128 colours.
- Picture organisation (DOC.004): a picture is 64 sectors of 128 bytes. The first 60 sectors are
  the screen, 40 bytes per scanline, 7680 bytes for the 192-line modes. In the 96-line mode only the
  first 30 sectors (3840 bytes) are screen. Sector 61 holds 9 bytes for registers 704-712. Sectors
  62-64 are three 128-byte tables: the scanlines with an interrupt (0 = none), the register to
  change, and the colour. `RAMDOS` files (`.RM0` to `.RM4`) are the Koala-compacted form plus four
  extra sectors for animation, colours and the interrupt tables (DOC.003).
- This matches `atari8/rambrandt.rs` (RM2 is the raw 8192-byte form; RM4 is Koala plus the tail).
- Probes with `recoil2png` (all black box):
  - **The extension picks the pixel mode; the container is detected from the content.** `TITLE.RM2`
    (raw 8192) and `ADVANCED.RM4` (Koala form) are both accepted renamed to `.rm1`, `.rm2`, `.rm3`
    and `.rm4`, each giving a different 320x192 picture. Renamed `.rm0`, both are rejected.
  - RM1 and RM3 therefore need only the RM2/RM4 containers plus the GTIA 9 (16 luminances, one hue)
    and GTIA 11 (16 hues, one luminance) renderers, 80x192 drawn 4 wide. The existing code has GTIA
    mode logic (`atari8/gtia.rs`, `screen.rs`). RECOIL is the oracle: the two renamed samples give
    four checkable pictures per mode without any new sample.
  - RM0: a random screen followed by zeros, exactly 8192 bytes, is accepted and drawn 320x192
    (160x96, each pixel 2x2). Screen bytes 0-3839 only; registers sit at 7680 like RM2, with
    COLPF0-2 (708-710) as pixel values 1-3 and COLBK as 0. Any other size is rejected. A file
    with `TITLE.RM2`'s tables is rejected and zero tables are accepted, so RM0 probably validates
    the line table against 96 lines. The exact rule needs a few dozen more probes. RM0's behaviour
    with the Koala container is not yet known (`ADVANCED.RM4` renamed is rejected).
- Plan: extend `rambrandt.rs` to take the mode from the extension: three more registry entries
  sharing the container code, a 160x96 branch for RM0, GTIA 9/11 pixel mapping for RM1/RM3.
  RM1/RM3 first, RM0 after the table rule is known.
- Samples: none found (`Rambrandt.ATR` is a copy-protected program disk with no DOS directory;
  `Rambrandt_Utilities.ATR` holds fonts and `LOADPX.BAS`, the loader that Appendix III describes).
  The program itself, run in an emulator as a black box, could make samples.

## SRI + PL7 (MSX2 Graph Saurus interlaced screen 7)

- RECOIL: 512x424, 16 colours, "2 frames". `bitplane/datatypes` PR #62 (<https://github.com/bitplane/datatypes/pull/62>)
  says "exactly 108544 bytes, no header" with a `.pl7` palette beside it. Licence of that repo
  not checked; the facts below were re-derived by probing, not copied.
- Probes (synthetic files made from `ILL6-1.SR7`'s pixel data and a row-index test picture):
  a 108544-byte file named `.sri` is accepted; 108543 and 108545 are rejected; the output is a
  plain 512x424 raster, file row `r` is output row `r`, with no interleaving. The palette comes
  from the `.pl7` beside it. 108544 = 2 x 54272, the size of two Screen 7 fields.
- Open: whether real Graph Saurus SRI files store the two fields stacked (as RECOIL draws them) or
  interleaved. No genuine SRI file turned up in any archive searched, and the SR0 + SR1 pair
  (MSX-FAN) is a separate case already recorded as a divergence.
- Plan: new decoder in `msx/` reusing the Screen 7 bitmap path and the `pl7` companion lookup used
  by SR7 (bank 0), width 512, height 424, exact length check, extension `sri` only.
- Samples: none. Synthesise from two SR7 files and check against the oracle.

## VSC + G2F (Atari Graph2Font vertical scroll)

- Already analysed in `atari-8bit.md` section 9.5 (a list of names, stacked 336x240 renders, no
  limit on count, case-sensitive names, a last name without CR LF dropped).
- Samples: `katon.vsc` (+ `katon_0.g2f`, `katon_1.g2f`) in the corpus root and
  `extra/atari8/g2f-full/vscroll/girl/` (3 G2F files). Both sets have their companions.
- API change: `Companions::get(extension)` only returns files that share the main file's stem. It
  needs `fn get_named(&self, file_name: &str) -> Option<Vec<u8>>` (look up the name in the main
  file's directory, ignoring directory parts of the name), a `SiblingFiles` implementation in
  `tests/common` that sees every file in the directory, and the oracle test copying each named file
  next to the `.vsc` before running `recoil2png`. Every `Companions` implementor changes; a caller
  with no sibling access (a sandboxed thumbnailer) gets `None` and the decoder returns
  `Unrecognized`.
- Plan: decode each listed G2F with `graph2font::decode_g2f` and stack the images. S once the trait
  method exists.

## HPM (Atari 8-bit, Grass' Slideshow)

- Not documented anywhere. The program is a 1996 MEC slideshow (credits Grass, Lewis, Bober,
  Flash) at <https://atari.fox-1.nl/atari-400-800-xl-xe/400-800-xl-xe-demos/grass-slideshow/>.
  The archive `Grass-Slideshow.7z` (<http://atari.fox-1.nl/wp-content/uploads/Grass-Slideshow.7z>)
  holds `Grass Slideshow.atr`, whose directory has `GRASS`, `SLIDESHOW` and 15 `.HPM` files (KISS,
  ZWIEWKA, KOPALNY, SALEM, CZASZKA, STAR, VIVALDI, GIRL, FATHER, DN, ALIEN, RAPER, HPZ, JORDAN,
  FSILY). The directory's sector counts are 0, so follow the sector links to read them. Files run
  2.8 to 8.9 KB; `JORDAN` has the same size as the corpus file. All 15 decode in `recoil2png`.
- Solved so far (from samples): the file is one run-length stream, `00 v n` = n copies of v
  (n = 0 means 256) and `n` literal bytes otherwise. It unpacks to exactly 7680 bytes of 40-byte
  lines (a 160x192 2-bit screen) plus **one trailing byte**; all 15 samples unpack to exactly 7681
  bytes. Trailer values seen: `34` (7 files), `74` (2), `05`, `04`, `35`, `30`, `51`, `e4`.
- Probe: replacing the trailer of `JORDAN.HPM` with every value 0-255: 249 values give the same four
  greys (0, 68, 136, 204); `04` gives other greys (0, 68, 102, 170); only `30`, `34`, `35`, `51`,
  `74` and `e4` give colours. So the trailer picks among a few palettes. The palette does not depend
  on the trailer alone: ALIEN and JORDAN both end in `34` yet show different colours (ALIEN has
  green `(104,179,0)`, JORDAN pink shades). The rest must come from the pixel data or from
  something else in the stream.
- Feasibility: M. 15 samples with oracle output and a cheap probe loop make a lookup of the
  trailer palettes easy; the rest probably needs one or two targeted probes (flip pixel bytes in
  one file and watch which colours appear). Reading the slideshow's code would answer it directly,
  but that means disassembling a program with no licence; `CLEANROOM.md` only names sample files, so the
  maintainer should decide.
- Plan: new decoder `atari8/hpm.rs` (unpack, 7680-byte check, trailer palette), extension `hpm`
  only. The 19203-byte HiRes Player Missile `.hpm` (`DRAGON.HPM`) is a different format that
  RECOIL rejects; it differs by size.

## SPC (Atari Graphics Magician Picture Painter)

- Spec for the Apple II version: <https://6502disassembly.com/a2-graphics-magician/> (page by
  Andy McFadden, copyright 2025, describing a Penguin Software program; read as prose, code
  licence not checked). The high nibble selects the command and the low nibble is part of the
  argument. `0x` end, `1x` text cursor, `2x` line colour (0-7), `3x` reverse text, `4x` brush
  (0-7), `5x` normal text, `6x` fill pattern (0-107), `8x` set line start, `Ax` line to, `Cx` brush
  at, `Ex` fill at. Brushes are 14x16 bitmaps. The fill scans up to a border and fills lines
  downward from the midpoint of the left and right borders.
- Atari files: `LE16 length` (file size - 3), the stream, a closing `00`. `COIN1.SPC` is
  `80 4b 3c a0 46 3d a0 43 40 ...`: set line start (x 75, y 60), line to (70, 61), and so on, with x
  in 0-159 and y in 0-191. `ROCKETOR.SPC` has `60 00 20 a0 45 2f 60 03 23 a0 3f 30`: `6x` takes one
  following byte and `2x` sets the line colour. So the command set looks like the Apple II one
  with a one-byte x. `TEST.SPC` is built from 7-byte records starting `70`, a command that is not in
  the Apple II list.
- Not documented for the Atari: the pattern table (RECOIL shows 128 colours), the brush shapes, the
  line drawing rule, the fill rule and the `7x` command.
- Samples: 4 local (`COIN1`, `COIN2`, `TEST`, `ROCKETOR`), 2 more on `GRMAGIC.ATR` (`COIN3.SPC` and
  `SEQPIC.SPC`; the four `R10.SPC` entries are empty), plus the `.PIC` and `.PTX` files.
- Feasibility: L but bounded. Everything RECOIL draws can be read back from the oracle with
  synthetic streams: `Cx` on a blank canvas for each brush, `6x nn` plus `Ex` for each pattern,
  single lines for the line rule, and a few closed shapes for the fill. The risk is the fill:
  differences show up only on irregular shapes, and the real program's behaviour is only knowable
  from the program itself (on the ATR). Decide whether to match RECOIL exactly or the original.
- Plan: `atari8/graphics_magician.rs`, a stream interpreter over a 160x192 index buffer with the
  probed tables. Work out `TEST.SPC`'s `7x` command first.

## ML1 / MX1 (NEC PC-98 Mapletown Network)

- Sources: only signatures and tool names are public. Just Solve
  (<http://fileformats.archiveteam.org/wiki/Mapletown_Network>, prose). A Susie plug-in
  `ifml1304.lzh` (v0.04, 1996) with source is listed on wakachan's pages (search snippet only; the
  page text above was not readable), licence unknown, so any source stays unread
  (<http://www.ateliermw.com/cglib/software.html>). The format was devised by NOZOMU for the
  Mapletown-Network BBS; the editors are `nedi3`/`MEDI-98`/`ml1.x` (binaries). PictureFan lists ML1
  as a 7-bit index-colour format, 160x100 (<https://iooiau.net/picturefan/help/formats.html>).
- Already solved (`msx-japanese.md`, `gaps-corpus-other.md` section 8): the ML1 header (`"100" 1A`,
  date and time, four BE16 for the rectangle, text to 0x60), an MSB-first bit stream from there, the
  two palette forms and base-9 colours, the MX1 text wrapper (7 bits per character) and its tile
  grids, and NL3, a text sibling with a simple column-wise run coder (decoded fully).
- The pixel coder is open. Known: gamma-like tokens (`n` ones, a zero, `n` bits, value
  `2^n + bits`) in pixel pairs; a flat tile is `N/2, colour + 2, 1, N/2 + 1`; the stream looks
  region-oriented. One new observation: this token code is the same one `nec_pc/pi.rs` uses for
  repeat lengths (`Bits::length`), and Pi also codes in pairs. That hints at move-to-front colours
  and repeat references as in Pi, so Pi's primitives may be reusable. There is no other evidence yet.
- New samples: `SAMPLE2.ML1` (615 bytes), `SAMPLE4.ML1` (315), `SAMPLE5.ML1` (757) and `SAMPLE6.ML1` (775),
  all 160x100 and decoded by `recoil2png`, at <https://sembiance.com/fileFormatSamples/image/mapletownNetwork/>.
  `SAMPLE4` has only about 220 bytes of pixel stream, the best candidate for hand analysis. The
  earlier plan (enumerate the valid streams for 2x1 and 2x2 pictures through the oracle) also
  still holds.
- Feasibility: M-L. There is plenty of oracle access and there are small samples, but the coder
  uses context from previous lines that single-bit probing exposes only partly. MX1 then costs about
  a day (transport and tile grid are known).
- Plan: `nec_pc/mapletown.rs` for ML1 and MX1, no API change (an MX1 tile grid is one `Image`).

## Q4 / XLD4 (NEC PC-98)

- Sources: the public docs are prose only (<https://ja.wikipedia.org/wiki/Q4>,
  <http://fileformats.archiveteam.org/wiki/XLD4>): 16 colours of 4096, 640x400, `.q4` plus a
  `.q4d` text, developed by MAJYO, algorithm and loader kept by QLD and never published.
- The only readable implementation is `rururutan/ifxld4` (<https://github.com/rururutan/ifxld4>,
  MIT per its LICENSE). Its README says it is "based on analysis of xgload and q4toppm".
  `xgload` has no stated licence and `q4toppm` is attached to RECOIL's bug tracker, which
  `CLEANROOM.md` forbids. The repo has no spec document, only C source (`src/xld4decode.c`).
  **Not read.** The maintainer decides whether an MIT repo derived from an unlicensed tool and a
  RECOIL-tracker attachment counts as permissive. My recommendation: no.
- Known from earlier notes: the container is solved on all 9 local samples (size at 8, `MAJYO` at
  11, six independent bands of 75, 75, 75, 75, 75 and 25 rows, each `LE16 L1`, `LE16 L2`, `L1 + 2`
  bytes); the palette part is entropy-coded; a flipped bit changes pixels for only 40-75 lines
  after it. That last point argues for self-synchronising variable-length codes that reference
  earlier lines (Huffman or LZ with a window of a line or two) over an adaptive arithmetic coder,
  which would damage the rest of the band. The earlier note leaned the other way; neither is proven.
- New samples (12), all decoded by `recoil2png`: `ABOUT_M`, `ASAT161X`, `ASAT163X`, `ASAT165X`,
  `ASAT169X`, `ASAT16CX`, `BIRDGIRL`, `C_SHOWR1`, `LADY_J`, `MA3`, `watari05`, `watari08`, from
  <https://sembiance.com/fileFormatSamples/image/q4/> (8 to 60 KB). The five `ASAT16xX` files may be
  related pictures (judging by name only), which would help differential analysis.
- Feasibility: L and blocked. Reverse engineering a bit-level entropy coder from output alone has
  no precedent in this repo (ZIM, KT4 and Pi had simpler structure or a public spec). Without the
  licence decision, do it last or skip.
- Reverse engineering from the samples alone (decision: do not read `ifxld4`): each band starts
  with 2 header bytes that don't affect the picture. The pixel code then reads 3 bits: 0 stops, 1
  switches to 4-bit literals, 2 to 7 pick one of six ranked colours. After the switch each pixel is
  a 4-bit value, with 2 to 15 mapping to 14 palette colours; the meaning of 0 and 1 is unknown. With
  an all-ones stream a pixel costs exactly 3 bits. A flat white band reaches 48000 pixels in 30
  bytes, so long-run tokens or a model that makes repeats nearly free exist. Truncating the data
  grows the decoded pixel count in steps of 288. Flipping bits in the first 25 bits of a band
  changes pixel 0, which hints at an arithmetic or range coder with a skewed model, contradicting
  the self-synchronising guess above. The preamble palette is coded as a shared-prefix R, G, B
  tree, not a plain 16x12-bit array. The context model and adaptation are still missing.

## PIX (Atari 8-bit, 160x192, 4 colours)

- RECOIL lists only "PIX, 160x192, 4 colours, 1 frame". No name, program or documentation turned
  up in Just Solve, gury.atari8.info, AtariAge or the RECOIL news (its only `PIX` entry is TRS-80
  CoCo). Every `.PIX` in the corpus belongs to TRS-80 or another platform.
- `recoil2png` rejects zero, random and patterned files of every size to 12000 bytes (earlier note)
  and also the HPM files renamed `.pix`. Validation by a packer would explain that, but the header
  or packer is unknown.
- Verdict: blocked until a genuine file or the program name turns up. Not worth probing further
  blind.

## Provenance and licences used

| Source | Licence | Used for |
|---|---|---|
| Rambrandt docs (`Rambrandt_docs.ATR`, pigwa archive) | copyright 1985 Antic Publishing / B. Ermentrout | file layout facts only |
| 6502disassembly.com Graphics Magician page | copyright 2025 A. McFadden; code licence not checked | Picture Painter command set |
| `rururutan/ifkty` `spec/KTY_FORMAT.md` | MIT-declared; derived from xgload (unlicensed) and MKIP100 | KTY cross-check; source files not read |
| `rururutan/ifxld4` | MIT-declared; derived from xgload and the RECOIL-tracker `q4toppm` | not read |
| mooncore.eu Pi page | prose, licence unstated | Pi (already used) |
| temlib.org Atari Forum Wiki IFF page | wiki prose | VDAT layout (already used) |
| `bitplane/datatypes` PR #62 | licence not checked | SRI size, re-derived by probing |
| dexvert samples at sembiance.com; `Grass-Slideshow.7z`; `GRMAGIC.ATR` | samples copyrighted by their authors | sample sources, never committed |

Every other fact above comes from `recoil2png` probing (mutated and synthesised files).
