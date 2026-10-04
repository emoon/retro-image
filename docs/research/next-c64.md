# Next C64 formats: research notes

Surveyed 2026-10-04. Scope: the twelve C64 editor formats still missing (BDP, ESH, FLM, FP, HLE, ILE,
RPH, SH1, SH2, SHS, UIF, ZOM), the "check" variants in [coverage.md](../coverage.md) (SHE, ISH, P4I),
and the non-RECOIL formats from [gaps-commodore.md](gaps-commodore.md) (Petmate, CharPad, SpritePad,
GEOS, Print Shop). **No RECOIL or GPL decoder source was read.** Sources are prose documentation,
MIT-licensed code, my own disassembly of editor binaries, and black-box runs of `recoil2png`.

## Summary

| Format | Verdict | Spec confidence | Samples | Difficulty |
|---|---|---|---|---|
| RPH | **Same as Doodle** (plain and `$FE` packed) | high (pixel-identical to RECOIL on 3 files) | Doodle/JJ files stand in | trivial |
| ZOM | Koala layout, packed with the Flimatic-style RLE (escape byte stored last) | high (real Koala data round-trips) | none | S |
| FLM | `$3C00` image, 17280 bytes, same RLE as ZOM, FLI family | medium-high | none | S-M |
| ILE | 4098 bytes, two hires bitmaps, 320x48 | medium (geometry known, blend rule not) | none | S |
| HLE | 32770 bytes, Hires-Lace map confirmed with one oddity | medium-high | none | S |
| SH1 / SH2 | same RLE as ZOM, 96x168 / 192x168, sprite-layer pictures | low-medium | none | M |
| SHS | exactly 14338 bytes, 320x200, layout unknown | low | none | M |
| FP | exactly 19266 bytes, three 1024-byte blocks first, layout unknown | low | none | M |
| ESH | PackBits-like RLE, 192x200, about 20.4 KB unpacked | low-medium | none | M |
| UIF | packed (esc-last style), 288x200, two-frame memory map known | low-medium | none | M-L |
| BDP | RECOIL's check is not understood; editor has no docs | very low | none | L |
| SHE 8642 / ISH 30738 / Botticelli hires | geometry from RECOIL size sweeps only | low | none | S each |
| Petmate `.petmate` | JSON workspace, fully documented by MIT source | high | 14 files in corpus | S (needs JSON reader) |
| CharPad CTM v4, v6, v7, v8, v8.2, v9 | spec for 4/6/7 from prose, 8/9 from MIT source | high (4-8), medium (9) | 50 files in corpus | S-M |
| SpritePad SPD v4/v5 (SpritePad 2.x) | header from MIT source | medium | none | S |
| GEOS geoPaint, Photo Album, Photo Scrap (CVT) | fully derived and verified by rendering | high | 18 files in corpus | M |
| Print Shop A/B `.gra` | header-less variants known from MIT loader | medium | none | S |
| Self-displaying PETSCII PRG, Koala viewer PRGs | **already registered** | n/a | n/a | done |

RECOIL rejects every CVT, `.petmate`, and CTM version other than 5, so for those formats
retro-image will be the only decoder, so there is nothing to compare against. The registered
PrintMaster decoder only handles the 631-byte PrintMaster pictures.

## Recommended order

1. RPH (alias), ZOM, FLM: one shared unpacker, oracle available, an hour each.
2. Petmate, CharPad v4/6/7/8/9, GEOS: sources and samples are in hand; RECOIL has no opinion.
3. ILE, HLE: small, the oracle exists, a few probing runs finish them.
4. SH1, SH2, ESH, UIF, SHS, FP: finish with the probe tools below. Without real samples, only
   black-box work against RECOIL can validate them.
5. BDP last, and only if someone makes a real BDP file with the editor.

## Licences and sources used

| Source | Licence | Used for |
|---|---|---|
| [Codebase64 grafix list v0.03](http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03) | CC BY-NC-SA 4.0 (text) | memory maps for HLE, ZOM, RPM, UIFLI |
| [GoDot](https://github.com/godot64/GoDot) loaders and docs | MIT | Print Shop headers, GeoPaint context |
| [wbochar/petmate9](https://github.com/wbochar/petmate9) and [nurpax/petmate](https://github.com/nurpax/petmate) | MIT | `.petmate` structure, sample files |
| [c64lib/gradle-retro-assembler-plugin](https://github.com/c64lib/gradle-retro-assembler-plugin) | MIT | CTM 5-9 and SPD v4/v5 headers, 31 CTM fixtures |
| [martinpiper/C64Public](https://github.com/martinpiper/C64Public) | none stated | prose CTM v1-v7 and CharPad help pages; sample files kept local only |
| [zimmers.net GEOS archive](https://www.zimmers.net/anonftp/pub/cbm/geos/) and [geoPaint doc](http://www.zimmers.net/geos/docs/paintfile.txt), [geoPaint format doc](http://www.zimmers.net/anonftp/pub/cbm/geos/programming/documents/geoPaint%20format.txt), [GEOS VLIR doc](https://ist.uwaterloo.ca/~schepers/formats/GEOS.TXT) | prose, no licence | GEOS layouts; sample files kept local only |
| Editor binaries in the CSDb dump (Flimatic, Zoomatic) | copyrighted | read for file-format facts only (disassembly), no code reused |

The Martin Piper repository states no licence, so only its prose pages were read and its
sample files are used as local test data, never committed. The Just Solve wiki and
`web.archive.org` were unreachable.

## Detail per format

### RPH (Run Paint hires): alias of Doodle

`recoil2png` decodes `.rph` exactly like Doodle. Renaming `JJMACROSS.JJ`, `midear.dd` and the
9026-byte `DDLIL_GAL.dd` to `.rph` gives pixel-identical output to their native extensions. A
constant-fill probe shows an RLE escape of `$FE` and an unpacked size of 9026 bytes (trimmed
Doodle), matching the existing `decode_doodle` / `decode_doodle_packed` / trimmed variant.
Plan: add `rph` to those three registry entries. I could not check this against the original
Run Paint (RUN #63 listing on archive.org); that is open. Confidence in RECOIL parity: high.

### ZOM (Zoomatic): packed Koala

- Layout (verified): 2-byte load address (ignored), then a stream of literal bytes and runs, the
  **escape byte stored as the last byte of the file**. A run is `value, count, escape` with
  count 0 meaning 256. Runs of 4 or more are packed; a literal that equals the escape byte is
  written as a run of length 1. Output is exactly 10001 bytes in Koala order (bitmap 8000,
  screen 1000, colour 1000, background 1).
- Evidence: packing real Koala data (`abydos.koa`, `paralax.koala.koa`) with that scheme and
  renaming `.zom` is pixel-identical to the `.koa` decode in `recoil2png`. Anything shorter than
  10001 output bytes is rejected; longer is accepted. Codebase64's "escape comes after byte,
  length" agrees, but it does not say the escape is chosen per file and stored last.
- The scheme is read straight from the Flimatic packer (see FLM). Zoomatic 5.7 and its
  `SHOWMATIC` viewer are on CSDb (`tools/All/Z/Zoomatic/`).
- Plan: new `unpack::escape_last_rle`, then `koala` body. Difficulty S.
- Samples: none. The editors never ship pictures; Zoomatic's own files could be made with VICE.

### FLM (Flimatic v3.7): packed FLI image

- Disassembly of `FLIMATIC.D64/000_FLIMATIC_3.7_SHP.prg` (BASIC stub, unpacked code): the save
  routine packs memory `$3C00..$7F7F` (17280 bytes) and saves from `$3C00`. Pass 1 builds a
  byte histogram and picks the least frequent value as the escape. Pass 2 is the RLE above
  (run >= 4 or value == escape becomes `value count escape`, shorter runs are literal), and the
  escape byte is appended last. The file starts with the load address `$3C00`.
- Verified: a stream built that way is accepted by `recoil2png` as `.flm` (296x200).
- Memory map (from flipping bytes in the unpacked image and watching `recoil2png`): `$3C00` colour
  RAM, `$4000` eight screen RAMs, `$6000` bitmap (8000 bytes), and 64 trailing bytes where the
  last one (`$7F7F`) recolours the whole picture, probably the background. It looks like FLI
  Editor / FLI Graph with the same hidden three columns (picture 296x200). Unconfirmed: the
  ordering inside the 64 tail bytes and how screen bytes map to lines.
- Plan: shared unpacker, then reuse `fli.rs` once the tail bytes are pinned. Difficulty S-M.

### ILE (Interlaced Logo Editor)

- `recoil2png` accepts only 4098 bytes: 2-byte load address, then two frames of 2048 bytes. Each
  frame holds a 1920-byte hires bitmap (40 x 6 cells, row-major cells of 8 bytes) plus 128
  ignored bytes. Output is 320x48.
- Frame 1 is drawn one pixel to the right of frame 2 (the byte flip moves x to 1..8 versus 0..7),
  so it's a half-pixel interlace. There is no colour RAM; the blend rule (black/white to grey)
  still needs a pixel readback from `recoil2png`.
- Plan: S. Editor on disk: `tools/All/I/Interlaced Logo Editor V1.01/`, which is packed.

### HLE (Hireslace Editor)

- 32770 bytes exactly. Codebase64's Hires-Lace v1.5 map fits: bitmap 1 at file offset 2 (`$4000`),
  screen 1 at `$6000`, screen 2 at `$8000`, bitmap 2 at `$A000`, 320x200.
- Oddity: the first cell of every row of bitmap 1 (8 bytes at offset 2, then every 320 bytes) has no
  visible effect in `recoil2png`. Everything else maps cell by cell. Check before assuming the
  full 40 columns.
- Plan: S. Hireslace Editor v1.5 is at `tools/All/H/Hireslace Editor V1.5/` (packed).

### SH1, SH2 (Super-hires Editor I/II), ESH, SHS, FP, UIF, BDP

All come from packed editors; RECOIL gives the only geometry:

| Ext | RECOIL accepts | Picture | Notes |
|---|---|---|---|
| SH1 | RLE (ZOM-style) giving at least 4.5 KB, trailing data ignored | 96x168 | the same size as `.she` (96x88) times 2 vertically, so sprite layers again |
| SH2 | RLE giving about 8.5 KB | 192x168 | the 8642-byte SHE variant is the same size class |
| ESH | PackBits-like: control byte 1..127 copies that many bytes, 129..255 repeats the next byte (n-128) times; unpacked about 20.4 KB | 192x200 | not the esc-last packer |
| UIF | esc-last style packer, load address first; unpacked about 32.5 KB | 288x200 | Codebase64 gives the two-frame UIFLI map (`$4000..$7F37` and `$C000..$FF37`, 60 sprites, and so on) but no file layout |
| SHS | exactly 14338 bytes | 320x200 | `2 + 14 x 1024`, layout unknown |
| FP | exactly 19266 bytes | 320x200 | three 1024-byte blocks (screens or colour RAM) first, then data |
| BDP | RECOIL accepts a scattering of unrelated files (size 4083 to 32768) but never random data, even with an `FF FF` header | 320x200 | check is not understood; Boogie Down Paint 5.0rc2 is on CSDb, binary only |

These were measured with `recoil2png` only and the numbers may shift. Several had no tidy
signature, so detection by size or content is unlikely to be safe without real files.

Plan for each: finish with the probe tools (see Method), then decode. Do not register any of them
before a real file from the editor is in hand: RECOIL accepts nearly anything for ESH, SH1, SH2, UIF
and ZOM-style packed data, so "RECOIL agrees" proves little. SH1/SH2 should be done after the
8642-byte SHE variant since the geometry matches.

### SHE 8642, ISH 30738, Botticelli hires (the `check` rows)

- SHE: `recoil2png` accepts 3250 (96x88) and 8642 (192x168) only, as already recorded. Layout
  for 8642 is in [commodore.md](commodore.md).
- ISH: `recoil2png` accepts 9194 (Image System hires, registered) and 30738 bytes (320x200,
  "Interlace Super Hires Painter"); only 9194 is decoded. 30738 = 2 + 30736, layout unknown.
  `tools/All/I/InterlaceSuperHires Painter V1.0/` has the editor.
- P4I: RECOIL accepts 2050 (the registered 128x64 grey variant) and 10050 bytes only. The hires
  `G.` Botticelli file is therefore probably also 10050 bytes without the `MULT` tag; check our
  decoder's behaviour on such a file before calling it covered. No hires Botticelli sample found.

### Petmate `.petmate`

- A JSON workspace (not the JSON export the earlier gap note described). Verified on 14 real files
  from petmate9 across versions 3 and 4:
  `{"version":4,"screens":[0,1,..],"framebufs":[{width,height,columnMode?,backgroundColor,borderColor,borderOn?,charset,name?,framebuf:[[{"code":N,"color":N},..],..],zoom?,guideLayer?}],"customFonts":{..},"framebufUIStates":[..]}`.
  `framebuf` is rows of cells. `code` is the screen code (256 means a transparent cell on non-VDC
  charsets, 512 on VDC); VDC cells may also carry `attr` (blink, underline, reverse, alternate set),
  and `transparent` as a boolean.
- Charset ids seen in the samples: `upper`, `c16Upper`, `c128Upper`, `c128vdc`, `vic20Upper`,
  `petGfx`; the repository assets also hold lower-case fonts for C64, C16, C128, VIC-20 and PET plus
  a Commodore Business (`cbase`) font. Custom fonts are named in `customFonts`. Sizes seen:
  40x25, 80x25 (VDC and PET), 22x23 (VIC-20), plus art-sized frames (16x48, 16x34 and 24x118).
- Palettes: C64 palette variants and the VIC-20 (PAL, NTSC), PET (white, green, amber) and TED
  choices live in petmate9's `src/utils/palette.ts`. A `.petmate` with several framebufs is a set
  of screens; render the first (or the selected one).
- Plan: tiny hand-written JSON reader in the core crate (`no_std` + `alloc`) with a strict schema;
  render `upper` and `lower` with the ROM charset already in the tree, and return `Unrecognized`
  for charsets that have no font yet. **Decision for the owner:** the C16, C128, VIC-20 and PET
  charsets are Commodore ROM dumps, as the C64 one already in the tree is; petmate9 ships them
  as assets under its MIT licence, but that licence does not cover the ROM contents. Decide
  whether the repository may carry them before covering those machines.
- Signature: `"framebufs"` key near the start. Difficulty S.
- Samples (local): `corpus/extra/commodore/petmate9/` (14 files, MIT source).

### CharPad CTM v4, v6, v7, v8, v8.2, v9

The registry decodes v5 only (`CTM\x05`, 20-byte header). RECOIL does the same.

| Version | Source | Header and blocks |
|---|---|---|
| 4 | [Piper mirror, prose](https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt) | already documented in `charpad.rs` header notes |
| 6 | CharPad Free Edition 2.7.2 help "File Format - CTM (V6)" | 10-byte header: colours (4), colouring method, flags (bit0 MCMODE, bit1 TILESYS); then marker-prefixed blocks (`DA B0`..): chars, char attributes (low nibble colour, high nibble material), tiles (16-bit cells, up to 10x10), tile colours if per-tile, tags, names, map. Markers are advisory. |
| 7 | same help, V7 page | 12-byte header: `COLOURS` (5: BG1, BG2/MC1, BG3/MC2, BG4, char), colouring method (0 per-map, 1 per-tile, 2 per-char), screen mode (0 hires, 1 multicolour, 2 extended), flags (bit0 TILESYS). Blocks as v6. |
| 8 | c64lib `CTM8Processor` (MIT) | after the version byte: display mode, colouring method, flags, then 7 colour bytes (screen, MC1, MC2, BG4, char colours 0-2); a pre-release v8 has one more colour byte (told apart by whether the next byte is `DA`). Blocks: chars, **materials** (one byte per char), per-char colours when colouring method is 2 (1, 2 or 3 bytes per char depending on screen mode), then tiles, tile colours, tags, names, map. |
| 8.2 | c64lib fixtures | version byte `0x52` (82), same layout as 8 |
| 9 | c64lib `CTM9Processor` (MIT) | v8 header plus 16-bit flexigrid width and height and one ignored byte before the colours. |

Screen modes 3 and 4 (hires and multicolour **bitmap**) exist in v8/v9: char data is then bitmap
cells. Limits seen in the readers: map up to 8192x8192, tiles up to 10x10, 65536 chars and tiles.
v9 and bitmap modes are the least verified part: no v9 sample exists in anything I could reach.
Plan: share the char/tile/map renderer already in `charpad.rs`; add v4, v6 and v7 first, then v8/v8.2,
then v9 and bitmap modes. Difficulty S for v4 to v8, M for v9 and bitmaps.

Samples (local, in `corpus/extra/commodore/`):
- `charpad-c64lib/`: 31 files (v5 x7, v6 x6, v7 x6, v8 x9, v8.2 x3), MIT, text hires and multicolour
  variants for per-char, per-tile and no-tile projects.
- `charpad-piper/`: 17 files (v1 x2, v4 x3, v6 x1, v7 x11), no licence stated.

### SpritePad SPD v4/v5 (SpritePad 2.x)

- The registry reads v1 (`SPD\x01`). c64lib's MIT reader accepts versions 4 and 5: after the
  version byte come flags, a 16-bit sprite quantity, 16-bit tile quantity, sprite animation
  quantity (minus one), tile animation quantity (minus one), tile width, tile height,
  background, MC1, MC2, then two 16-bit overlay distances: a 20-byte header, then 64 bytes per
  sprite (63 bytes plus an attribute byte).
- Not settled: whether the sprite quantity is stored minus one, and the attribute bit meanings; the
  reader ignores them. The Piper `SpritePad v2b1` examples turned out to be **v1** files, so there
  is no real v2 file in hand.
- Plan: S once one SpritePad 2.x export exists. Ask for a user-supplied sample. Difficulty S.

### GEOS geoPaint, Photo Album, Photo Scrap (CVT)

Container (checked on 18 real CVT files from zimmers.net; sizes agree with the block counts):

- Block 0, bytes `0..253`: the GEOS directory entry: `+0` file type, `+3` name (16 bytes, `$A0`
  padded), `+0x15` GEOS structure (0 sequential, 1 VLIR), `+0x16` GEOS type, then at `+0x1E` the
  text `PRG formatted GEOS file V1.0` (a signature at a fixed offset).
- Block 1 (`0xFE`): the info block, with the icon, class text (`Paint Image V1.1`, `Photo album V1.0`,
  `Photo Scrap V1.1`) and the author field.
- VLIR: block 2 (`0x1FC`, 254 bytes) is the record table, 127 entries of
  `(block count, bytes used in last block)`. `00 FF` is an empty record, `00 00` is unused. The
  records follow, each padded to whole 254-byte blocks, except that the last block of the
  file is trimmed. Sequential files put the data straight after block 1 at `0x1FC`.

geoPaint (VLIR, 45 records, each two card rows of a 640x720 page):

- Each record decompresses to 1448 bytes: 640 bitmap bytes for card row 0, 640 for row 1, 8 zero
  bytes, 80 colour bytes for row 0, 80 for row 1 (foreground in the high nibble, background in the
  low nibble). Bitmap bytes are in card order (8 consecutive bytes make one card, bit 7 leftmost).
- Compression: `$00` end, `$01-$3F` copy that many literal bytes, `$41-$7F` repeat the next 8-byte
  card (n - `$40`) times, `$81-$FF` repeat the next byte (n - `$80`) times.
- Empty records render as blank cards (white); what colour GEOS shows for them is open (GoDot says
  dark grey on light grey when no colour data exists).
- Verified by rendering `AIRCRAFT-1.cvt` and `amiga.lady.cvt`: both gave clean pictures.
  Photos use fewer than 80 cards per row, so the edge of a picture needs care.

Photo Scrap and Photo Album (monochrome clip art):

- A scrap is a 3-byte header (width in cards, height as 16-bit little endian), then
  GEOS bitmap compression, row by row (1 bit per pixel, 1 = black, rows are not card order).
- Compression: `0-127` repeat the next byte n times, `128-219` copy (n - 128) literal bytes,
  `220-255` repeat a pattern of the following length byte (n - 220) times. The last form is from
  the GEOS programmer's reference as I remember it and did not occur in my samples, so treat it as
  unconfirmed.
- A photo album is a VLIR file where each record is one scrap. Verified on `alb.geopaint.cvt`,
  `cartoons.cvt` and `borders.cvt`; a standalone `Photo Scrap.cvt` (80 cards by 144 lines) has
  the same data after block 1.
- Raw VLIR files on a `.d64` need extraction first, which is out of scope: take CVT only.

Plan: one `geos.rs` with the container reader and three entries (`geoPaint`, `Photo Album`,
`Photo Scrap`), signature = the `PRG formatted GEOS file` string at `+0x1E`, all `.cvt`. A
multi-record album renders as the first scrap, or as a contact sheet. Difficulty M.
Samples: `corpus/extra/commodore/zimmers-geos/` (18 files).

### Print Shop `.gra`

GoDot's loader (MIT) shows three header-less variants besides the registered PrintMaster
file: Print Shop A (88x52, 572 bytes after `00 58`, bits inverted), Print Shop B (48x45, 270
bytes) and PrintMaster `PG` (rows prefixed with `$8B`). Sample counts: none beyond the 58 existing
Sexcartoons PrintMaster files. Layout read from assembly, not tested on real files. Plan: S,
after a Print Shop disk is found (not on CSDb).

## Already registered

- Self-displaying PETSCII PRG, Koala viewer (10500 bytes) and Koala viewer (10608 bytes) are in
  the `commodore/mod.rs` registry. The remaining viewer-stub groups in
  [gaps-commodore.md](gaps-commodore.md) (2499 bytes: the sample I opened is a music player, not a
  picture; 16146 bytes: many stubs) are still unresolved and I would not pursue them.

## Method and tools

All throwaway scripts lived in the session scratchpad and are not committed:

- **Size and fill sweeps.** Random data of every length 1..40000 per extension shows which sizes
  `recoil2png` accepts; constant-fill files of byte value b, where the smallest accepted length
  changes with b, reveal an RLE (`N` proportional to 1/b means a count byte, `N` of `b+1` over `b`
  means literal runs).
- **Byte flips.** Flip one byte at a time in a valid file and diff the PNG (bounding box, pixel count)
  to map file offsets to picture cells. It runs about 30 files per second per core; HLE (32770
  offsets) takes a couple of minutes.
- **Packer discovery.** Reading the Flimatic packer in the editor binary gave the "escape last"
  RLE, which I then verified against real Koala data through ZOM. SH1, SH2 and FLM follow the same
  scheme; ESH does not.
- **Disassembly.** A 60-line 6502 disassembler was enough for the Flimatic routine. Most other
  editors are crunched, so their save code isn't readable without unpacking first. VICE (`x64sc`) is
  installed but has no ROM images, so editors cannot be run to produce samples; supplying the C64
  ROMs would unlock that route.

## Samples added (never committed)

All under `corpus/extra/commodore/`, with entries in `MANIFEST.tsv` (the RECOIL column says
`rejected` where `recoil2png` does not read the file):

| Group | Files | Source | RECOIL |
|---|---|---|---|
| `zimmers-geos/` | 18 (11 geoPaint, 6 albums, 1 scrap) | zimmers.net `/pub/cbm/geos/graphics/` | all rejected |
| `petmate9/` | 14 | wbochar/petmate9 `_defaults/` and `_tests/` (MIT) | all rejected |
| `charpad-c64lib/` | 31 | c64lib test resources (MIT) | v5 only (7), the rest rejected |
| `charpad-piper/` | 17 | martinpiper/C64Public (no licence stated) | all rejected |

**No samples exist for any of the twelve editor formats.** The editors are in the CSDb dump
(Boogie Down Paint 5.0rc2, Flimatic 3.7, FuckPaint 0.2, Hireslace Editor 1.5, Interlaced Logo
Editor 1.01, UIFLI Editor 1.0, Zoomatic 5.7 plus SHOWMATIC, several Super Hires editors) but
ship no saved picture. The previous hunts reached the same conclusion.

## Open questions

1. May ROM-derived charsets for PET, VIC-20, C16 and C128 enter the repository (Petmate)?
2. Does oracle testing need a "RECOIL has no decoder" mode, since CVT, Petmate and CTM v4/6-9 all
   fall outside RECOIL?
3. Is it acceptable to commit decoders for formats whose only samples are synthetic (ZOM, FLM)? A
   Koala image packed with the packer above is real data in a verified container, but still no
   file from the editor itself.
4. Does the owner want to supply C64 ROMs for VICE, so samples can be produced with the actual editors?
