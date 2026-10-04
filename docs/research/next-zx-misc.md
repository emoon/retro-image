# Next-wave survey: ZX/CPC snapshots and tapes, Game Boy, NES, teletext, CoCo 3

Researched 2026-10-04. No RECOIL code and no GPL decoder code was read. Fuse, ZEsarUX, JSpeccy,
libspectrum and zxtune were not opened (zxtune's `CrazyLove.szx` was copied as a data file only).
Layouts below come from prose specs, MIT/CC0 documentation and checks against real files
(scratch scripts in the session scratchpad, not in the repo).

## Findings that change the plan

1. **None of these formats is in RECOIL.** `recoil2png` answers "file decoding error" for every
   new sample (SNA, Z80, SZX, TAP, TZX, CPC SNA, SAV, GBR, CHR, TTI, HRS). There is no oracle
   run for this wave. Correctness rests on checking real files against the spec and an emulator
   render. `Format::platform` has no RECOIL name to follow; use the existing names ("ZX Spectrum",
   "Amstrad CPC", "Game Boy", "NES", "BBC Micro", "TRS-80 Color Computer").
2. **Already registered (nothing to do):** ZX Next NXI, SL2, SLR, SHC, SHR and the +3DOS header
   (`zx_spectrum/next.rs`, `strip_plus3dos`, applied to `.scr` too). SCR, ULAplus SCR and the
   Timex hi-colour/hi-res screens exist (`timex.rs`), so snapshots can reuse them.
3. **Not registered:** SNA/Z80/SZX, TAP/TZX, SpecSCII ZXS, CPC SNA, Game Boy and NES art,
   teletext, CoCo 3.
4. Samples for everything except SpecSCII ZXS, EP1 and a real Mode 7 dump are now in
   `corpus/extra/` (details at the end).

Every decoder here can reuse existing code. Spectrum snapshots and tapes only extract a
6912-byte screen and call `standard::decode_scr` (or the Timex/ULAplus decoders). CPC SNA feeds
`amstrad_cpc/hardware.rs` (`hardware_color`, mode packing, `render`).

## Summary table

| Format | Spec quality | Licence of the docs | Difficulty | Priority |
|---|---|---|---|---|
| Z80 snapshot | Spec | WoS FAQ, no licence stated (facts only) | Easy | High |
| SNA (48K/128K) | Spec | same | Easy | High |
| SZX | Spec | Spectaculator docs, (c) Jonathan Needle, facts only | Easy-medium (zlib) | Medium |
| TAP | Spec | WoS FAQ | Easy | High |
| TZX | Spec | WoS TZX page, no licence stated | Medium (block skipper) | High |
| SpecSCII ZXS | Partial | SpectraLab guide (MIT) | Easy once samples exist | Low |
| CPC SNA | Spec | cpctech docs | Easy-medium | High |
| GB Camera SAV | Spec | Pan Docs CC0, GB Camera notes | Easy | Medium |
| GBR / GBM (GBTD/GBMB) | Spec (author's Word docs) | (c) 1999 H. Mulder, no licence | Medium | Medium |
| NES CHR / NAM / NSS | Spec | nesdev wiki (no licence shown), NESST source is public domain | Easy-medium | Low-medium |
| Teletext TTI / EP1 / raw Mode 7 | Spec | Teletext Wiki CC BY-SA 4.0 (facts only) | Medium (state machine + font) | High |
| CoCo 3 CM3, HRS, MGE, RAT, VEF | Spec (MIT docs) | KAOS Toolkit, MIT | Easy-medium | High |

## 1. ZX Spectrum snapshots

### Z80

Spec: <https://worldofspectrum.org/faq/reference/z80format.htm> (fetched; the WoS copyright notice
at <https://worldofspectrum.org/distributions.htm> covers software, not the format description).

- V1: 30-byte header, then 48K RAM from 0x4000. Header bytes 6-7 (PC) nonzero means v1. Byte 12
  bit 5 = compressed. Compression is `ED ED count value` for runs of 5 or more (and any run of
  `ED`), ending with `00 ED ED 00` in v1 only. The screen is the first 6912 bytes of RAM.
- V2/V3: PC = 0, extension length at 30 (23 = v2, 54/55 = v3), hardware byte at 34, last `OUT` to
  0x7FFD at 35. Then blocks of `u16 length (0xFFFF = 16K raw), u8 page, data`.
- Hardware byte differs between versions. Seen in the corpus: v2 hw 3 = 128K; v3 hw 0/1 = 48K,
  hw 4 = 128K, hw 5 = 128K+IF1 (the page lists the full table; map the others from it).
- Pages: 48K uses 4 (0x8000), 5 (0xC000), 8 (0x4000). 128K uses page = bank + 3, so bank 5 (the
  normal screen) is page 8 and bank 7 (shadow screen) is page 10.
- **Screen select (128K):** bit 3 of the 0x7FFD byte picks bank 7 over bank 5. Pentagon and
  Scorpion use the same bit.
- Timex TC2048/TC2068/TS2068 are hw 14/15/128. The page did not say where the port 0xFF value
  lives, so check this against a real hw-14 file before promising Timex modes in Z80.
- No ULAplus data in Z80.

Verified with a scratch renderer on a v1 compressed file (`Color Demo (1995)(Vaxalon).z80`) and a
v3 128K file (`Phantasy Demo`, 7FFD = 0x17): both rendered correctly.

### SNA

Spec: <https://worldofspectrum.org/faq/reference/formats.htm> (fetched). 27-byte header, then:
48K = 49152 bytes RAM (49179 total); 128K = banks 5, 2, the currently paged bank, then PC (u16),
0x7FFD, TR-DOS flag, then the remaining banks in ascending order (131103 or 147487 bytes). Screen
bank is 5 or 7 by 0x7FFD bit 3; bank 7 may be the "current" bank or one of the trailing ones.
Header byte 26 is the border colour. The format has no Timex or ULAplus data. Samples: 2 48K files
(maziac, Spectron tests); no 128K sample yet.

### SZX (zx-state)

Spec: <https://www.spectaculator.com/docs/zx-state/intro.shtml> (header, block, SPCR, RAMP, SCLD
pages fetched). Docs are (c) Jonathan Needle, "open specification for other emulator authors";
use for facts only.

- Header: `ZXST`, major, minor (1.4 and 1.5 seen), machine id (0 = 16K, 1 = 48K, 2 = 128K,
  8 = 2048 in samples; table on the header page), flags.
- Blocks: `u32 id, u32 size` then data, so unknown blocks skip.
- `SPCR`: border, `ch7ffd`, `ch1ffd`/`chEff7`, `chFe`, 4 reserved. `RAMP`: `u16 flags` (bit 0 =
  zlib compressed), `u8 page`, data. 48K saves pages 5, 2, 0 (page numbers are banks here, not Z80
  pages). 128K saves 0-7, in any order.
- `SCLD` (Timex): `u8 chF4`, `u8 chFf`. Port 0xFF bit 0 = screen at 0x6000, bit 1 = hi-colour,
  bit 2 = hi-res; check the Timex reference the repo already cites.
- `PLTT` (ULAplus, v1.4): `u8 flags` (enabled), `u8 current register`, `u8 regs[64]`. This block is
  absent from the block-index page; found via the Sinclair Wiki
  <https://sinclair.wiki.zxnet.co.uk/wiki/ZX-State_format>. Confirm layout against `Spectron`'s
  `2048.szx` and a ULAplus sample before relying on it.
- Needs zlib inflate. The core crate is `no_std`, so use a permissive pure-Rust inflater
  (`miniz_oxide`, MIT/Apache) or write a small one; check what `Cargo.toml` already allows.
- Samples: 6 files (Spectron MIT test files include 16K/48K/128K/2048; `CrazyLove.szx` is 128K v1.1).
  None carries ULAplus data.

### TAP and TZX loading screens

- TAP: sequences of `u16 length, flag, payload, checksum`. Spec in the same WoS FAQ.
- TZX: <https://worldofspectrum.org/TZXformat.html> (v1.20; the fetched page gave the block
  table but not every length formula; mine are in the scratch parser and parse all 25 sample
  files). Header `ZXTape!` 0x1A major minor, then blocks `id + body`:
  0x10 = `u16 pause, u16 len, data`; 0x11 = 18-byte header with `u24 len` at offset 15;
  0x12 = 4; 0x13 = `1 + 2n`; 0x14 = 10-byte header with `u24 len` at 7; 0x15 = 8-byte header with
  `u24 len` at 5; 0x18/0x19/0x32/0x35 = `u32` length first (0x32 `u16`, 0x35 is 10 + `u32`);
  0x20 = 2; 0x21 = `1 + n`; 0x22/0x25/0x27 = 0; 0x23/0x24 = 2; 0x26 = `2 + 2n`; 0x28 = `2 + u16`;
  0x2A = 4; 0x2B = 5; 0x30 = `1 + n`; 0x31 = `2 + n`; 0x33 = `1 + 3n`; 0x34 = 8; 0x5A = 9.
- Screen rule: the first data block (0x10, 0x11, 0x14, or a TAP block) whose payload is flag
  0xFF plus 6912 bytes plus checksum (6914 bytes) is the screen. Also accept a CODE header
  (type 3) with length 6912 and start 16384 followed by its data block. Optionally accept 6144-byte
  bitmap-only blocks.
- Hit rate in the 50 demo tapes sampled: 7 of 50 have a 6914-byte block. Expect more in games.
  Loaders that pack the screen (ZX7, Hrust) or use pulse-level data (0x15, 0x19, CSW) give nothing;
  say so as "no loading screen found" (`Unrecognized`).
- `.tap`/`.tzx` carry no signature that is safe for content detection except `ZXTape!`
  (`.signature()` applies to TZX only).

### SpecSCII ZXS

Status unchanged from `sinclair-cpc-bbc-misc.md`: RECOIL rejects SpectraLab's `.specscii` and its
`.zxs` layout is unknown. The SpectraLab guide (MIT,
<https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>, section SPECSCII)
defines the text stream: `0x0D` newline, `0x10-0x15` + 1 byte (INK, PAPER, FLASH, BRIGHT, INVERSE,
OVER), `0x16 row col` (AT), `0x17 col` (TAB), `0x20-0x7F` ROM font, `0x80-0xFF` 2x2 block graphics
(bit 0 top-right, 1 top-left, 2 bottom-right, 3 bottom-left). Rendering needs the 96-glyph Spectrum
ROM font. Precedent: `zx81.rs` takes its glyphs from an archive.org ROM dump on the strength of
Amstrad's redistribution permission; do the same for the Spectrum ROM at 0x3D00 and record the
dump's md5. No samples beyond the two SpectraLab exports; zxart.ee's SpecSCII editor
(<https://zxart.ee/specscii/>) can make more. Low priority.

## 2. Amstrad CPC SNA

Spec: <https://cpctech.cpcwiki.de/docs/snapshot.html> (fetched; cpcwiki.eu returns 403). Header
256 bytes `MV - SNA`, version at 0x10 (1, 2 or 3).

| Offset | Content |
|---|---|
| 0x2E | Gate Array selected pen |
| 0x2F-0x3F | 17 pen colours (16 pens + border), low 5 bits = hardware colour number |
| 0x40 | Gate Array multi-config (bits 0-1 = screen mode) |
| 0x41 | RAM configuration (port 0x7Fxx) |
| 0x42 / 0x43-0x54 | CRTC selected register / registers 0-17 |
| 0x55 | ROM selection |
| 0x6B-0x6C | memory dump size in KB (64 or 128; **0 in v3 files that use chunks**) |
| 0x100 | memory dump |

V3 appends 8-byte-header chunks (`id`, `u32 len`), `MEM0` to `MEM8` for RAM and `CPC+` for ASIC state
(skip unknown chunks). A `MEMx` chunk of exactly 65536 bytes is raw; otherwise it is RLE with
control byte 0xE5: `E5 n v` repeats `v` n times, `E5 00` is a literal 0xE5
(<https://www.cpcwiki.eu/index.php/Snapshot>, via search result).

Render: screen mode from 0x40, pens through `hardware_color`. CRTC R1 (chars per line), R6 (rows),
R9 (lines per char) give the visible size; R12/R13 give the start: bits 4-5 of R12 pick the 16K
page, bits 0-9 of R12/R13 are the offset in 2-byte units, and the 0x7FF wrap applies. Rows are at
`+0x800` per scan line. This is an overscan-aware cousin of `overscan.rs`. RAM config bit patterns
other than 0 only matter if the screen page was banked out (rare). Limits: a snapshot is a single
instant, so mid-frame palette/mode changes are lost, and CPC+ ASIC palette, sprites and split
screens (the `CPC+` chunk) are out of scope at first.

Verified with a scratch renderer on `aba.sna` (v3, mode 1, raw 128K, R1 = 32) and the chunked v3
`3D Monster Chase.sna` (mode 0, R1 = 40, RLE chunk): both look right. The scratch renderer did not
double mode 0 pixels as `hardware.rs` does.

Sample mix in `corpus/extra/cpc-snapshots` (45 files drawn from 738 `.sna` entries; the other 137
`.sna` entries in that set are zip files): v1 (2), v2 (82 of 64K, 5 of 128K), v3 raw 64K or 128K
(188), v3 chunked with size 0 (234). The chunked kind is the most common, so RLE support is not
optional.

## 3. Game Boy and NES

### Game Boy Camera SAV

Sources: Pan Docs (<https://gbdev.io/pandocs/Gameboy_Camera.html>, repo LICENSE is CC0) covers the
hardware and 128x112 output, not the save layout. Layout is from Raphael Boichot's notes
(<https://github.com/Raphael-Boichot/Inject-pictures-in-your-Game-Boy-Camera-saves>, no licence
stated, so facts only): 131072-byte SRAM; slot n (1-30) at `0x2000 + (n-1) * 0x1000`; image tiles
`+0x000..0xDEF` (224 tiles, 16 wide x 14 high, standard 2bpp Game Boy tile encoding: row = two
bytes, low plane first); thumbnail `+0xDF0..0xEFF` (32x32); metadata `+0xF00`. State vector at
`0x11B2` (30 bytes): value `0x00-0x1D` = photo number minus one occupying that slot, `0xFF` =
deleted. "Game Face" at `0x11FC`.

Verified: slot 1 of `gb-photo_photo.sav` decodes to a clean test image ("1" on a gradient). An
unused real save (`POCKETCAMERA.sav`, all state bytes 0xFF) holds SRAM noise in the slots, so a
decoder must use the state vector and return `Unrecognized` when no photo is active.

Plan: size 131072 exactly plus the state vector; first active slot (an `Image` is one picture, so
no gallery). Palette: 4 greys (0xFF/0xAA/0x55/0x00) or the DMG green; pick one and note it, as the
Atari/C64 palettes were handled. Easy. `.sav` is also used for any other Game Boy save, so do not
mark it as a content signature.

### GBTD GBR and GBMB GBM

Spec: Harry Mulder's Word documents `GBRS9906.ZIP` and `GBMS9910.ZIP`
(<http://www.devrs.com/gb/hmgd/supp.html>, downloaded and read via `strings`). Copyright 1999
H. Mulder, no licence; they are the author's published format docs, so use as prose spec.

- Header: `GBO` + version digit. GBR uses `GBO0`; GBM 1.0+ uses `GBO1`.
- Objects: `u16 type, u16 id, u32 length` then body (GBM adds master id and CRC fields). The docs
  say "hi-endian" but the files are **little-endian**.
- GBR TileData (type 2): name (30 bytes), `u16 width`, `u16 height` (in pixels), `u16 count`,
  4-byte colour set, then one byte per pixel (values 0-3, not packed) tile after tile. Palette
  object types seen: 0x0D and 0x0E (sizes 264, 1040 in `bar_c.gbr`; not in the part of the doc
  I read).
- GBM: Map object (width, height, tile data of 16-bit records), property objects. Tiles come from
  a GBR, so GBM needs the companion (`Format::with_companions`) or renders as an index map.
- Pre-1.0 GBM files (`GBO0`, 12054 bytes, four of them in the GBDK samples) use an older,
  undocumented layout; skip them.

Samples: 16 files from the GBDK-2020 examples (14 GBR/GBM from several tools, 2 GBO1 GBMs).
Difficulty: medium for GBM, easy for GBR as a tile sheet. Moderate value (very common in GBDK/ZGB
homebrew).

### NES CHR, NAM, NSS

- CHR: 16 bytes per tile, two 8-byte bitplanes, 4 KiB (one pattern table) or 8 KiB;
  <https://www.nesdev.org/wiki/PPU_pattern_tables> (no licence shown on the page).
- NAM: 960-byte nametable, optionally +64 attribute bytes (1024), as NES Screen Tool saves it.
  Needs a CHR and a palette: companions, or default greys. The 256-byte `.nam` files in the
  hxlnt sample are not full nametables, so size alone is a weak signature. Not decoded standalone.
- NSS: NES Screen Tool session. The famidash samples start `NSTssTXT\r\n` and then hold
  `key=value` lines (a text variant); it holds CHR, palettes and nametables in one file, which
  makes it the only self-contained NES format here. NESST's source is reported to be public domain (Shiru; a search result said so, licence file
  not checked), so verify before reading it for the layout; not done yet.
- Palette: several competing 64-entry master palettes; pick one and record it.
- Value is low: the pictures are mostly tilesets without a canonical layout. Do NSS first if any.

## 4. Teletext (TTI, EP1, raw Mode 7)

Specs (Teletext Wiki, CC BY-SA 4.0, so facts only):

- EP1 (<https://teletext.wiki.zxnet.co.uk/wiki/EP1_format>, fetched): `FE 01`, language byte, flag
  byte (0xCA when enhancement data follows, else 0), `u16` offset of the page data. Optional block
  `C2 00`, `u16 length`, up to 16 x 40-byte packets. Page data at the offset: 960 bytes (24 rows
  x 40), then a 40-byte editing buffer, then `00 00`. `JWC` + several EP1 = EPX.
- TTI (<https://teletext.wiki.zxnet.co.uk/wiki/MRG_TTI_format>; field list in the original PDF
  <https://zxnet.co.uk/teletext/documents/ttiformat.pdf> and VBIT2's page-file wiki, not read):
  text lines `DE,` (description), `PN,` (page number), `SC,` (subcode), `PS,` (status), `CT,`
  (cycle time), `FL,` (fastext links), `RE,`/`SP,`/`MS,`, and `OL,row,text`. Observed in the
  samples: rows 1-24 (row 0 is the header), rows 26-28 hold enhancement packets (level 1.5 and
  up), control codes inside text appear as `ESC` + (code + 0x40). Several sub-pages per file
  start at each `PN,` / `SC,`. Each file must pick one sub-page (the first).
- Raw: 1000 bytes (25x40) or 960 bytes, also BBC Mode 7 memory (`&7C00`, 1000 bytes). Extension
  only, no signature.
- Control-code semantics (mdfs.net, <http://mdfs.net/Info/Comp/Teletext/Controls>, fetched; no
  licence stated): colour and mosaic colour codes are "set after", hold graphics repeats the last
  mosaic, double height needs the same code on the two rows, defaults at line start are white,
  steady, single height, black background, contiguous, release graphics. The Teletext Wiki's
  `Teletext_specifications` page and ETSI EN 300 706 are the formal reference.

**Font provenance.** MAME's `saa5050.cpp` is BSD-3, but the glyph ROM is an external `ROM_LOAD`
image, so there is nothing clean to read there. `rawles/edit.tf` is GPL-3 and `ali1234/vhs-teletext`
is GPL-3: do not read them. The usable source is **Bedstead** by Ben Harris
(<https://bjh21.me.uk/bedstead/>, mirror <https://github.com/glxxyz/bedstead>, CC0-1.0): the page
says the generator "and all of the newly-designed glyphs have been released into the public
domain", the repo carries CC0-1.0, and the generator, `bedstead.c`, defines the 5x9 glyphs and the
smoothing rule. The caveat is the README's note that the original SAA5050 character designs are
"effectively out of copyright due to age": that is its claim, not a verified fact. Ask the
maintainer before reading `bedstead.c`; otherwise draw our own 5x9 set from the character tables
in ETSI EN 300 706, and compute the mosaics (they are plain 2x3 blocks, no font needed). The smoothing
(diagonal half-pixels, 5x9 to 10x18 in a 12x20 cell) is described in the SAA5050 datasheet
(<http://www.elektronikjk.com/elementy_czynne/IC/SAA5050.pdf>, [search], not read).

Samples: 48 TTI files in `corpus/extra/teletext` (Teefax mirror, nemetext, wxted, QTeletextMaker
examples incl. level 2.5/3.5 pages with DRCS; the QTeletextMaker examples sit in a GPL-3 repo, so
they are test data only). **No EP1 and no raw Mode 7 sample yet.** EP1: export from edit.tf
(<https://edit.tf>) or QTeletextMaker. Mode 7: BBC disc images in the Acorn TOSEC set already
used for BBG (`acorn-bbc-micro-complete-set-tosec` on archive.org): look in the SSD catalogue for
files with load address `&7C00` and length `&3E8`. Not done.

Plan: TTI and raw first (level 1 only, first sub-page); EP1 is a few lines more.
Output 40x25 cells at 12x20 (480x500) or 6x10 (240x250). Flash and conceal render in their steady,
revealed state. Level 1.5+ enhancements (row 26-28 packets, DRCS) are a later extension.
Medium difficulty; this is the one clear gap in `gaps-others.md` section 2.1.

## 5. CoCo 3: CM3, HRS, MGE, RAT, VEF

Spec: **KAOS Toolkit** (<https://github.com/ChetSimpson/KAOSToolkit>, MIT, (c) 2023 HyperTech
Gaming and Chet Simpson). The docs live in `AssetFoo/docs/images/{cm3,hrs,mge,rat,vef}.dox` and are
readable; code is MIT too (keep the notice if anything is derived). `jamieleecho/coco-tools` stays
GPL-2: do not read. Colour format docs there are TODO stubs.

| Format | Layout |
|---|---|
| HRS | 16-byte colour map + 30720 bytes of 4bpp packed pixels = 320x192, 16 colours (30736 bytes; the doc says "1 byte colormap", a typo) |
| MGE | 51-byte header: type (0 = 320x200 16 colours), 16-byte colour map, colour space (0 RGB222, 1 composite C4I2), compression flag (**0 = compressed**), 30-byte title, cycle delay, cycle range. Data 4bpp, 32000 bytes raw, else `count, byte` pairs ending at count 0 |
| RAT | 19-byte header: escape code, compression flag (0 = raw), 16-byte colour map, background colour. 320x199, 31840 bytes raw, else escape `E r v` runs |
| VEF | 18-byte header: flags (high bit = compressed), type, 16-byte colour map. Types 0-4: 320x200x16, 640x200x4, 160x200x16, 320x200x4, 640x200x2. Compressed: 400 half-row blocks of `len, packets` (n < 128 literal run, else repeat byte n-128 times); beware packet overruns |
| CM3 | 29-byte header (flags, 16-byte colour map, animation and cycle rates and lists), optional 243-byte pattern table, then one or two 320x192 pages. **Compression is not documented** ("\todo"); the MIT reader code is the source |

Colours: the GIME palette byte is 6 bits `R1 G1 B1 R0 G0 B0` (RGB222) or composite (2-bit
intensity plus 4-bit phase); the colour-space choice is not stored in HRS, RAT or CM3, so pick RGB
and record it. Lomont's CoCo hardware PDF
(<https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware.pdf>) is the spec for the bits; check
that against the KAOS code. CM3 page 2 gives a 320x384 image. Colour cycling and animation are
ignored (stills).

Samples: 41 files in `corpus/extra/coco3` (all the KAOS test data: 19 CM3, 1 HRS, 3 MGE, 2 RAT,
10 VEF). The artwork is copyrighted by its artists;
corpus only. These are the only decoders in this wave that have an author-written reference
reader to compare with. Good value, easy-medium: do HRS, MGE, RAT, VEF first, CM3 last.

## Samples added (not committed; all under git-ignored `corpus/`)

| Group | Files | Source |
|---|---|---|
| `corpus/extra/zx-snapshots` | 28 Z80, 25 TZX, 34 TAP, 2 SNA, 6 SZX | TOSEC demo sets on archive.org (`Sinclair_ZX_Spectrum_TOSEC_2012_04_23`), Spectron and maziac (MIT), local zxtune sample |
| `corpus/extra/cpc-snapshots` | 45 SNA | archive.org `snaps_202510` and `amstrad_abadia_del_crimen_snapshots` |
| `corpus/extra/gameboy-nes` | 16 GBR/GBM, 4 SAV, 11 NES (5 CHR/NAM, 3 CHR, 3 NSS) | GBDK-2020, gb-photo (MIT), Raphael Boichot, hxlnt (MIT), famidash (MIT) |
| `corpus/extra/teletext` | 48 TTI | Teefax mirror, nemetext, wxted, QTeletextMaker, emffax (MIT) |
| `corpus/extra/coco3` | 41 CM3/HRS/MGE/RAT/VEF | KAOS Toolkit test data (MIT repo) |

Each group has a `MANIFEST.tsv` (the `recoil_size` column is `-` because RECOIL rejects all of
them). Gaps: no 128K SNA, no ULAplus or Timex snapshot with a non-default screen mode, no EP1, no raw
Mode 7 dump, no SpecSCII ZXS, no NES `.nam` that is a real 960-byte nametable, and the TOSEC
tapes were not cut down to files that really hold a screen (only 7 of 50 sampled demo tapes do).

## Suggested order

1. CoCo 3 HRS/MGE/RAT/VEF (then CM3): MIT spec, MIT samples, nothing to invent.
2. Spectrum snapshots and tapes: Z80, SNA, TAP, TZX screen extraction (one shared `Frame` path,
   no new rendering); SZX after an inflater decision.
3. CPC SNA, including RLE `MEMx` chunks.
4. Teletext TTI/raw/EP1, after the font question is settled.
5. Game Boy Camera SAV, GBR; then NSS, CHR, GBM.
6. SpecSCII ZXS only once real `.zxs` files turn up.
