# Gap survey: Apple, QL, Thomson, CoCo, TI-99, consoles, calculators, workstation and cross-platform formats

Clean-room research notes on formats the registry does not decode yet, for the "others" family:
Apple II/IIGS/Mac, Sinclair QL, Thomson, SAM Coupe, Oric, Enterprise, Dragon/CoCo/Tandy, Acorn,
Sharp, TI-99/4A, console tile dumps, calculators, Unix/workstation rasters, 80s-90s PC and
cross-platform formats, BBS formats and scene tools. RECOIL is the baseline, not the ceiling, so
this includes formats RECOIL lacks. No RECOIL code and no GPL/LGPL decoder code was read. Surveyed
2026-10-03, after reading `formats.md`, `coverage.md`, `sources.md`, `CLEANROOM.md` and the
research notes for amiga-apple-misc, riscos-ql, thomson, sinclair-cpc-bbc-misc and textmode.

## How to read this

Claims are tagged by how they were checked:

- **[fetched]**: page read in this session.
- **[search]**: only a web-search summary was seen, so layout details are unconfirmed.
- **[memory]**: from general knowledge; the URL is a pointer, not verified. Treat as uncertain.

Difficulty: S = small (a day or less, spec is enough), M = a few days or a risky detail, L = a
renderer or a reverse-engineering job. "Sig" = whether content detection is possible.

## Environment limits

- `fileformats.archiveteam.org` / `justsolve.archiveteam.org` refused connections from the research
  host, and `web.archive.org` is blocked for the fetch tool. Just Solve pages are cited only where a
  search result showed them; their contents were not read.
- One fetch of `pulkomandy.tk` (GrafX2 file-format wiki) was attempted before remembering that
  `thomson.md` records its robots.txt as disallowing AI agents. It returned HTTP 429, so nothing
  was read. Do not use that site.
- `ninerpedia.org` text is CC BY-NC-SA: take facts, don't copy text into the repo.

## 0. Scope question for the maintainer

`formats.md` has no generic raster formats (PCX, Targa, GIF, BMP, PBM, Sun raster, SGI, XBM).
The PC platform holds only BIOS logos, MSP and text-mode art. Sections 6 and 7 list them anyway
because scene archives (scene.org, Demozoo, Simtel) are full of them and a thumbnailer
like flea will meet them. If they are deliberately left to other crates, skip those sections and
keep the retro-specific candidates (Animator PIC/CEL, Dr. Halo, PC Paint, ColoRIX, which general
libraries rarely handle).

## 1. Summary of candidates

| # | Format | Platform | Docs | Common | Diff | Samples | Sig |
|---|---|---|---|---|---|---|---|
| 1 | Teletext pages: TTI, EP1, raw 40x25 `.bin`, Mode 7 | BBC / teletext | Spec [fetched] | Active scene | S-M | Easy | EP1 yes, TTI yes, raw no |
| 2 | PCX / PCC | DOS | Spec [memory] | Very high | S | Easy | yes |
| 3 | Autodesk Animator PIC / CEL (+ FLI/FLC frame 1) | DOS | Spec [search] | High in DOS scene | S | Easy | yes |
| 4 | CoCo 3 CM3, MGE, HRS, RAT, VEF | TRS-80 CoCo 3 | Spec (MIT docs) [search] | Medium; RECOIL lacks them | M | Medium | partly |
| 5 | Targa TGA | DOS/cross | Spec [memory] | Very high | S | Easy | weak (footer in v2) |
| 6 | Dr. Halo CUT+PAL, PC Paint PIC, ColoRIX RIX/SC? | DOS | Spec [search] | Medium | S each | Medium | PIC, RIX yes; CUT weak |
| 7 | DreamGrafix ($C0/8005, $C1/8003), Apple II packed Hi-Res/DHR (FOT) | Apple IIGS / II | Partial-Spec [fetched] | Medium | M | Medium | no |
| 8 | GB Camera SAV, NES CHR/NAM, GBTD GBR / GBMB GBM | Game Boy / NES | Spec [search] | High in homebrew | S-M | Easy | GBR/GBM yes |
| 9 | TI-Artist `_P`/`_C` (TIAP/TIAC) | TI-99/4A | Partial [search] | Medium-low | S | Medium | no |
| 10 | PBM family, Sun raster, SGI RGB, XBM, XPM, Utah RLE | Unix | Spec [search/memory] | Medium | S each | Easy | yes |
| 11 | PICT (bitmap opcodes only) | Mac | Spec (Apple docs) [fetched] | Medium | M | Easy | weak |
| 12 | GrafX2 PKM | scene tool | Partial [search] | Low-medium | S | Medium | yes |
| 13 | Psion Series 5 MBM | EPOC | Partial [search] | Low | S-M | Medium | yes |
| 14 | Calculators: TI-83+ `.8xi`, Casio G1P / G3P | calc | Spec [search] | Low-medium | S | Easy | yes |
| 15 | Aseprite `.ase` | modern pixel art | Spec (MIT-hosted doc) [search] | Medium (modern) | M | Easy | yes |
| 16 | QL: IP2C/IP3C, PD, Eye-Q, Painter | QL | Spec / None | Low | S-M | Medium | no |
| 17 | RIPscrip, NAPLPS, Minitel VDT | BBS / videotex | Spec [search] | Low-medium | L | Medium | RIP ok |
| 18 | SNES / MD / PCE / Neo Geo raw tiles | consoles | Spec [search] | Low (ROM rips) | S + options | Easy | none |
| 19 | Dragon, Enterprise, Sharp MZ/X1, Oric, SAM extras | misc | None / raw only | Very low | - | Hard | none |

## 2. Retro-specific candidates (not generic rasters)

### 2.1 Teletext pages and BBC Mode 7 (TTI, EP1, raw, hashstring)

Not in the registry. `textmode.md` covers PC text art only; `BB0..BB5` skip Mode 7 and no code
mentions teletext (the only hit is a RISC OS test for mode 7). Teletext art is a living scene
(edit.tf galleries, teletext art groups); Mode 7 is the BBC Micro text mode and it is
the same 40x25 cell model (7-bit codes, control codes for colour, mosaic, double height, flash,
hold graphics).

- Formats [fetched, Teletext Wiki index at <https://teletext.wiki.zxnet.co.uk/wiki/Main_Page>]:
  - **MRG TTI** (ASCII text, escape-coded control codes, `OL,n,...` lines, several subpages per file):
    wiki <https://teletext.wiki.zxnet.co.uk/wiki/MRG_TTI_format>; original spec PDF
    <https://zxnet.co.uk/teletext/documents/ttiformat.pdf> [search; not read].
  - **EP1** (Softel/edit.tf export): 6-byte header `FE 01 <lang> <flag> <offset16>`, optional enhancement
    block, then 24 rows x 40 bytes + 40-byte editing buffer, ends `00 00`. Wiki
    <https://teletext.wiki.zxnet.co.uk/wiki/EP1_format> [fetched]. `EPX` is a `JWC` wrapper of several EP1s.
  - **edit.tf hashstring**: URL fragment, hex char-set nibble then base64 of 25x40 7-bit cells,
    colon-delimited extras. <https://github.com/rawles/edit.tf/wiki/Teletext-page-hashstring-format>
    [fetched]. Probably not a file format; skip unless people save them to files.
  - Raw `.bin`: 1000 bytes (25x40) or 960 (24x40), also BBC Mode 7 screen memory (1000 bytes,
    from `&7C00`). No signature. Also T42 packet streams (42-byte packets, a broadcast capture
    format) and FAB ETT/TTA (named on the wiki, not examined).
- Rendering: needs the SAA5050 glyph set (6x10 cells, 12x20 rendered with smoothing, plus mosaic
  glyphs which are computable). The glyph shapes are the clean-room risk: Deark/MAME carry
  a ROM-derived font. Check MAME's licence header (BSD-3 for most of `src/devices/video/saa5050.cpp`
  [memory]) before reading, as was done for the Thomson driver. Alternative: draw our own 6x10 face.
- Difficulty: S for parsing, M for the state machine (hold graphics, double height, separated
  mosaics, flash steady-state) and the font.
- Samples: edit.tf galleries and teletext-art archives (names from memory: teefax.org.uk, zxnet
  teletext archive; uncertain). Generate more with edit.tf export.
- Sig: EP1 magic `FE 01`; TTI lines begin `DE,` / `DS,` / `PN,` / `OL,` (ASCII); raw is
  extension-only.

### 2.2 Autodesk Animator PIC / CEL, FLI / FLC first frame

DOS demoscene staple (Animator was the 1990s DOS paint tool; FLC was the demo animation format).
Not in the registry.

- PIC (320x200, 8-bit, palette + one compressed frame) begins `19 91`; CEL is the same with a
  header with x/y offset and size. FLI/FLC are the animation containers (`11 AF` / `12 AF` at
  offset 4). Documentation: <https://www.compuphase.com/flic.htm>,
  Dr. Dobb's "The FLIC File Format" (Mar 1993) <https://jacobfilipp.com/DrDobbs/articles/DDJ/1993/9303/9303a/9303a.htm>,
  <https://www.fileformat.info/format/cel/corion.htm>, Steve Hollasch's copy
  <https://steve.hollasch.net/cgindex/formats/fli.html> [all via search; not read]. Just Solve page
  "Animator PIC/CEL" exists [search].
- The chunk compressions (BRUN, LC / DELTA, COLOR_64 / COLOR_256, BYTE_RUN) are all small.
  A decoder for "first frame of a FLIC" is S; full animation is out of scope for a still-image API.
- Samples: scene.org / Simtel / ftp.cdrom.com mirrors have thousands of `.FLI/.FLC`; `.PIC/.CEL`
  fewer. Pro Motion NG stores layers as FLC too (section 5.2).
- Sig: yes, strong.

### 2.3 Tandy Color Computer 3: CM3, MGE, HRS, RAT, VEF

RECOIL lists CoCo 1/2 raw formats only (`formats.md`: CLP, GRF/MAX/P41/PIX, P11). The CoCo 3 has
its own graphic programs and file formats:

- The **KAOS Toolkit** (<https://github.com/ChetSimpson/KAOSToolkit>, **MIT** [fetched]) documents
  and reads: **CM3** (CoCo Max III, with colour cycling and animation), **HRS** (Hi-Res images,
  used by DaVinci 3), **MGE** (ColorMax Deluxe), **RAT** (The Rat Graphics Package) and
  **VEF** (OS-9 Level II, raw and compressed, 2/4/16 colours). The fetch did not show file paths
  inside the repo; the docs live in Doxygen comments for the "AssetFoo" readers. Code is MIT, so
  it is readable under the project's rules (keep the notice if anything is derived).
- "CoCo Graphics File Formats" by Reinaldo Torres is cited as a prose reference [search]; the URL
  was not seen. `coco-tools` (jamieleecho) is GPL-2: **do not read** (already in
  `amiga-apple-misc.md`).
- Hardware docs: Chris Lomont's CoCo hardware PDF <https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware.pdf>
  and Robert Gault's "Coco 3 Graphics" <https://aaronwolfe.com/robert.gault/Coco/Unpublished/CC3PART1.html>
  [search].
- Difficulty: M (palette model of the GIME: 64-colour RGB222 table in 16 slots; several
  resolutions; per-format RLE).
- Samples: CoCo archives (colorcomputerarchive.com, "Color Computer Archive" repo; names from the
  Dragon search; not browsed). Sig: partly (CM3 and MGE have sizes/headers; verify).

### 2.4 Apple IIGS and Apple II: what the registry lacks

The registry has APF, Paintworks, 3201, 3200, packed SHR (`$C0/0001`), HGR, DHGR, MacPaint.
`$C0`/`$C1` types per CiderPress II's Super Hi-Res page [fetched,
<https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>]:

| File type | Format | Status |
|---|---|---|
| `$C0/0000` | Paintworks packed | done |
| `$C0/0001` | PackBytes SHR | done |
| `$C0/0002` | Apple Preferred | done |
| `$C0/0003` | Packed QuickDraw II Picture File | **missing** (PICT-like opcode stream inside) |
| `$C0/8005` | DreamGrafix packed (LZW variant) | **missing** |
| `$C1/0000` | Uncompressed SHR | done (`.shr`) |
| `$C1/0001` | QuickDraw PICT | **missing** |
| `$C1/0002` | 3200-colour "Brooks" | done |
| `$C1/8003` | DreamGrafix unpacked | **missing** (256-colour / 3200 variants) |

DreamGrafix and a Hi-Res/Double Hi-Res packed variant (ProDOS `$08` FOT with aux `$4000`/`$4001`,
PackBytes; <https://prodos8.com/docs/technote/ftn/08/>, already cited in
`amiga-apple-misc.md`) are the best-documented gaps. The DreamGrafix trailer ("DreamWorld"
signature and mode word) is described in CiderPress II's doc [search, not read in detail].
Source licence: docs CC BY-SA 4.0, code Apache-2.0.

Not worth chasing: Print Shop / Print Shop GS clip art, Dazzle Draw, Beagle Graphics, Fantavision
(CiderPress converts Print Shop to BMP [search]; no public layout page was found). Mark as
**None** until a spec turns up.

- Difficulty: M (LZW dialect). Samples: Apple IIGS archives (archive.org Paintworks Gold item,
  appleoldies SHR converter pages, both cited in `amiga-apple-misc.md`). Sig: none (use ProDOS
  type from a wrapper or extension; our decoder only sees bytes).

### 2.5 TI-99/4A: TI-Artist

- TI-Artist stores a picture as two files, `_P` (pattern table) and `_C` (colour table), each
  6144 bytes: a dump of TMS9918A Graphics II VRAM [search, Ninerpedia
  <https://www.ninerpedia.org/wiki/Graphic_file_formats>, fetched summary]. Conversion tools
  (Convert9918, TIAP/TIAC names) also emit RLE-packed variants [search]; the RLE layout was not
  seen. The Ninerpedia page also covers MyArt and YAPP (Geneve, V9938 G6/G7 modes) and FRACTALS!:
  byte-level tables with RLE [fetched summary]; text CC BY-NC-SA.
- Existing decoder reuse: MSX Screen 2 (SC2) is the same VRAM layout (6144 pattern + 6144 colour),
  and the TMS9918 palette is shared.
- Disk files may be wrapped in a TIFILES/FIAD header (128 bytes, starts `07 'TIFILES'`) [memory].
  Companion pairing (`_P` + `_C`) matches our existing "Companions" mechanism.
- Difficulty: S (+ RLE variants unknown). Samples: TI-99/4A archives (names unconfirmed); the
  harmlesslion paper <https://harmlesslion.com/text/Modern%20Graphics%20on%20the%209918.pdf> (not
  readable by the fetch tool) covers Graphics II layout.
- Sig: none.

### 2.6 Console tile and bitmap dumps

Relevant to scene and homebrew art (NES, Game Boy) more than ROM rips.

- **NES / NES Screen Tool (NESST, NEXXT)**: `.chr` (2bpp planar, 16 bytes per 8x8 tile, 4 KiB
  or 8 KiB), `.nam` (960 or 1024 byte nametable, optional 64-byte attributes), `.pal` (16 bytes of
  NES palette indices), `.rle` (RLE-packed nametable) [search; nesdev forum
  <https://forums.nesdev.org/viewtopic.php?t=7237>]. A `.nam` needs a `.chr` + `.pal`, so this fits
  the Companions model. Palette: 64-entry NES master palette (several competing tables; pick one
  and record the choice, as Atari/C64 were handled).
- **Game Boy**: tile format, 2 bytes per 8-pixel row [fetched, Pan Docs
  <https://gbdev.io/pandocs/Tile_Data.html>; Pan Docs licence not shown on the page, believed CC0 [memory]: check the repo LICENSE].
  - **GB Camera save RAM**: 128 KiB SRAM, photos at `0x2000 + n*0x1000`, image at `+0x000..0xDEF`
    (128x112, 2bpp, tile order), thumbnail `+0xDF0..0xEFF` [search]; Pan Docs
    <https://gbdev.io/pandocs/Gameboy_Camera.html>, raphnet gbcam2png
    <https://www.raphnet.net/programmation/gbcam2png/index_en.php> (check its licence before reading
    code). Extra: the save also holds a photo-state table. 30 photos per file; one `Image` per file means first photo
    or a photo index option.
  - **GBTD `.gbr` / GBMB `.gbm`**: Harry Mulder's Game Boy Tile Designer / Map Builder; public spec
    zips `GBRS9906.ZIP` and `GBMS9910.ZIP` on <http://www.devrs.com/gb/hmgd/supp.html> [search]. The
    tile set / map renders to a sheet. Very common in GBDK/ZGB homebrew.
- **SNES 4bpp** (planes 0/1 then 2/3 per tile), **Mega Drive** (4bpp packed nibbles), **PC Engine** (CH0-CH3
  planes in 16-bit words), **Neo Geo C-ROM** (16x16 sprite tiles, planes split across odd/even ROM
  files: needs paired files) [search]: SNESdev <https://snes.nesdev.org/wiki/Tiles>, NeoGeo Dev Wiki
  <https://wiki.neogeodev.org/index.php?title=Sprite_graphics_format>, PC Engine dev wiki
  <https://www.silentdebuggers.com/doku.php?id=grafx:start>. All are headerless. They only make
  sense as a "tile sheet" decoder with bpp / layout options and a 16-wide sheet layout, with
  no way to detect them. Low priority.
- ColecoVision/Intellivision/Vectrex: ColecoVision uses the TMS9918 (same as SC2/TI-Artist).
  Intellivision GRAM cards are 8x8 1-bit; Vectrex is vectors. Nothing image-file-like found.
- Difficulty: S each for CHR/NAM/GB Camera; M for GBR/GBM (read the zip specs). Samples: easy
  (nesdev, gbdev, ROM-hack sites). Sig: GBR/GBM have magic strings (per the spec, unverified);
  CHR/NAM/SAV none.

### 2.7 CoCo/Dragon/Enterprise/Sharp/Oric/SAM leftovers

- **Dragon 32/64**: PMODE 4 screens are SAVEd as `.BIN` / `.PIX` with the DragonDOS binary header
  (`55 02 <load> <len> <exec> AA`), 6144+1 bytes [search, <https://dragon32.info/info/binformt.html>
  and Dragon Archive pages]. The CoCo `GRF/MAX/P41/PIX` decoder probably already shows these
  after stripping the header; check that DragonDOS/CoCo DECB (`00 len load ... FF 0000 exec`)
  headers are skipped. No Dragon paint-program format found. **None** beyond that.
- **Enterprise 64/128**: no image file format found. NICK has a programmable display list,
  so there is no fixed screen layout. Docs: Nick programmers' guide
  <http://ep.homeserver.hu/Dokumentacio/Konyvek/EXOS_2.1_technikal_information/hardware/Nick.html>
  [search]. ep128emu is GPL (do not read). **None**.
- **Sharp MZ / X1**: searches found only tape-image containers (MZF, MZT, QD) and no image files.
  X1 graphics programs exist but no public layouts were found. **None**. (X68000 and MAG/Pi are
  covered under the Japanese platforms.)
- **SAM Coupe**: FLASH! and SAM Paint save SCREEN$ files (mode 4: 24576 bytes plus 16+4+16+4 palette
  bytes, so at least 24617 bytes; `.ss4` elsewhere) [search,
  <https://www.worldofsam.org/products/sampaint>, <https://www.worldofsam.org/products/screen-modes>].
  The registry's SS1-SS4 already match this. SAM Paint's own save format (beyond SCREEN$) and
  compressed variants: no layout found. **None** beyond SCREEN$.
- **Oric**: HIRES + tape header done. No other documented image format found.
- **Thomson**: raw RAMA/RAMB dumps remain hardware-only; nothing new found.
- **Acorn**: RISC OS Clear (`&690`) still has no permissive spec; Draw/ArtWorks are vector.
  **BBC Mode 7** is the real gap (section 2.1).

### 2.8 Sinclair QL (known leftovers)

IP2C / IP3C, Page Designer PD2/PD3, Professional Publisher are Spec (Dilwyn Jones) per
`riscos-ql.md` and still unimplemented. Eye-Q and The Painter compressed pictures: Dilwyn
Jones's Graphics Viewer v1.15 shows QL screens, compressed screens, Painter and Eye-Q files
[search, <https://ia801404.us.archive.org/0/items/SinclairQLHomepage/graphics/index.html>] but no
layout page was seen. The "Screen Compression package" (freeware) names several older compression
formats. Mirror <https://www.sinclairql.net/djw/> for the docs. S each for the specced ones;
samples are in the same archives. Sig: weak.

### 2.9 Psion Series 5 MBM

EPOC MBM (`37 00 00 10 42 00 00 10`; exported MBM `37 00 00 10 8A 00 00 10`) holds several bitmaps
[search, Just Solve "EPOC MBM"]. 2/4/16-greyscale and colour depth variants with RLE. No layout
page was seen. Series 3 PIC/ICN is already done. Low priority, S-M.

### 2.10 Calculators

- **TI-83+/84+**: `.8xi` picture variable, 96x64 monochrome (stored as 96x63 or 96x64) inside the
  TI link file with an 8-byte signature `**TI83F*` [search, Just Solve "TI picture file", Merthsoft
  link guide <https://merthsoft.com/linkguide/ti83+/fformat.html>]. TI-89/92 `.89i/.9xi` pictures
  also exist [memory]. `.8ci` (colour CE) [memory]. Easy once the variable table is known. Sig: yes.
- **Casio fx-9860G `.g1p`** (two 128x64 bitmaps overlaid; capture files one) and **fx-CG `.g3p`**
  (3-bit or 16-bit colour, 384x216) [search; Universal Casio Forum "About Image Files"
  <https://community.casiocalc.org/topic/4394-about-image-files/>; WikiPrizm [memory]].
- **HP 48 GROB** done; HP-49/50 GROB uses `HPHP49-` header [memory]: check the existing decoder
  accepts it.
- Difficulty: S. Common among calculator-game fans; very low in scene archives.

## 3. Cross-platform DOS-era paint formats

### 3.1 PCX / PCC

Header 128 bytes (`0A`, version, RLE flag, bpp, window, DPI, 16-colour palette, planes, bytes per
line), RLE per scanline, 256-colour palette in the last 769 bytes (`0C` + 768) [memory].
Variants: 1-bit mono, 2-bit CGA, 4-bit EGA planar, 8-bit VGA, 24-bit. ZSoft spec is public
[memory]; Just Solve and fileformat.info have summaries. Extremely common in 90s demos, intros,
BBS packs and game assets. S. Sig: first byte `0A` + version 0/2/3/4/5 + encoding 1 + sane bpp.

### 3.2 Targa TGA

Truevision TGA File Format Specification 2.0 [memory;
<https://www.dca.fee.unicamp.br/~martino/disciplinas/ea978/tgaffs.pdf> is the commonly cited copy;
not fetched]. Types 1/2/3 (+9/10/11 RLE), 8/15/16/24/32 bpp, origin bit, optional `TRUEVISION-XFILE.`
footer in v2. S. Sig: weak for v1 (validate header fields).

### 3.3 Dr. Halo `.CUT` + `.PAL`

CUT: three 16-bit header fields (width, height, reserved) then RLE'd 8-bit rows; PAL: starts `AH`,
byte at offset 6 is `0x0A`, 16-bit RGB values [search: fileformat.info
<https://www.fileformat.info/format/drhalo/egff.htm>, Accusoft/LEADTOOLS docs]. RLE: run byte,
bit 7 set = repeat next byte, else literal. Companions: `.PAL`. S. Samples: older DOS paint
archives. Sig: PAL yes, CUT no.

### 3.4 PC Paint / Pictor `.PIC` (and `.CLP`)

17-byte header starting `34 12`, optional palette in "extra data", 5-byte-headed RLE blocks, up to
64Kx64K, EGA/CGA/VGA/24-bit [search: <https://www.fileformat.info/format/pictor/egff.htm>,
Wikipedia "PICtor PIC image format"]. Conflicts in the `.PIC` extension with Mac, Atari ST, QL,
FM Towns and Softimage: signature-first dispatch needed. S.

### 3.5 ColoRIX `.RIX` / `.SCI` `.SCP` `.SCG` `.SCX`

"RIX3" + 10-byte header (width, height, palette type, storage flags) + optional extension block +
palette + data. The compressed variant uses segments, XOR filter, RLE and Huffman [search: Entropymine
<https://entropymine.wordpress.com/2023/04/12/colorix-compressed-image-format/>; Fallout modding
wiki <https://falloutmods.fandom.com/wiki/RIX_File_Format> for the simple variant; Just Solve
ColoRIX]. Uncompressed: S. The compressed scheme: M, documented by Entropymine's analysis (Deark
is MIT and handles it [memory]). Scene relevance: low-medium (1990s VGA gallery packs).

### 3.6 Others noted, not recommended yet

- **GIF / BMP / ICO**: universal; if generic rasters are in scope, defer to a vetted library.
- **Windows 1.x/2.x BMP, Win3 RLE, DIB, Windows `.PAL`**: tiny; Deark covers them.
- **Lotus 1-2-3 `.PIC`** (vector chart metafile), **AutoCAD `.SLD`**: vector, not images.
- **Pixar `.PXR`**: could not confirm the signature; mark uncertain, skip.
- **Alias/Wavefront PIX, Softimage PIC, RLA**: 3D-workstation outputs; found in raytracing/ demo
  texture dumps. Specs exist [memory]; low scene priority.
- **VGA raw 320x200 + 768-byte `.PAL`**: headerless; no signature. Possible as extension-only
  `.RAW`/`.PAL` pair (6-bit VGA DAC values). Mark "low".

## 4. Unix / workstation rasters

All of these have public specs and trivial decoders; the gaps are a decision, not research.

| Format | Sig | Spec | Notes |
|---|---|---|---|
| Sun raster `.ras` `.sun` | `59 A6 6A 95` | `rasterfile.h` 1989 [search; Multimedia Wiki <https://wiki.multimedia.cx/index.php/Sun_rasterfile>, fileformat.info] | depth 1/8/24/32; RT_STANDARD, RT_BYTE_ENCODED, RT_FORMAT_RGB/TIFF/IFF; colormaps |
| SGI RGB `.rgb .sgi .bw` | `01 DA` | Paul Haeberli's spec [memory; mirrored at paulbourke.net/dataformats] | 8/16-bit, verbatim or RLE, rows stored bottom-up in planes |
| Utah RLE `.rle` | `52 CC` | Spencer Thomas, "Design of the Utah RLE Format" <https://sarnold.github.io/urt/docs/rle.pdf>; Bourke <https://paulbourke.net/dataformats/urt/>; <https://trap.mtview.ca.us/~tom/tech/file-formats/Utah-Rasterfile.html> [search] | 15-byte header, optional colour map, op stream; M |
| PBM/PGM/PPM/PAM | `P1`-`P7` | netpbm man pages [memory] | ASCII and binary |
| XBM | text `#define ..._width` | X11 docs [memory] | 1 bpp, C source; trivially text parsed |
| XPM | text `/* XPM */` | X11 docs [memory] | C source, colour names need the X11 colour table |

Retro-art relevance is low except XBM/XPM (icon art) and PBM (the lingua franca for converters).
Sig for all of them except raw `.bin` is strong, so content detection works.

## 5. Scene tool-native formats

### 5.1 GrafX2 PKM

`PKM` + version byte, basic RLE, palette; the native format of classic GrafX2 / Sunset Design
[search, Just Solve "PKM"; the page could not be read]. The format is documented only in
GrafX2's own GPL sources and the pulkomandy wiki (robots-blocked); the rest of the field
(header size, RLE escape bytes) is unconfirmed, so **Partial/None**: reverse-engineer from
samples if PKM files are found. The project moved to GIF with extensions; modern GrafX2 saves
GIF/PNG. Common in the 90s DOS/Amiga scene, but the corpus is small. S once understood.

### 5.2 Pro Motion (Cosmigo)

Pro Motion's `.pmp` project is a **zip** with an INI control file, a brush container (`.brc`),
a tile set (BMP) and per-layer `.pmd` FLC files holding the frames [search,
<https://community.cosmigo.com/t/file-format-of-pmp-files/1451>; assets repo
<https://github.com/cosmigo/pmotion-assets>]. Decoding needs ZIP + INI + FLC (section 2.2), so it
builds on Animator FLC. Scene relevance: medium (Amiga/pixel-art scene use it), difficulty M.
The format's licence/openness is stated only by community answers, so confirm with Cosmigo.

### 5.3 Aseprite

`.ase`/`.aseprite`: header magic `0xA5E0`, frames, chunks, zlib-compressed cels; indexed, grayscale
and RGBA. Spec <https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md> [search; not
opened: check the licence of the docs file, the Aseprite code is under a custom EULA, so read
only that doc]. A permissive Rust loader exists (`aseprite-loader`, docs.rs/aseprite-loader
[search], licence not checked). Common in modern pixel art and in modern retro-style demos;
M (layer blending, tags, tilemaps). Include only if modern pixel-art is in scope.

## 6. BBS and videotex

- **RIPscrip `.RIP`** (TeleGrafix, 1993): vector command stream, BGI-based; spec ripscrip 1.54
  (`ripsc154.zip`, `ripspec.zip`) in the BBS Documentary Library
  <http://www.bbsdocumentary.com/library/PROGRAMS/GRAPHICS/RIPSCRIPT/> and the white paper there
  [search]. A renderer for lines/arcs/flood fill/BGI fonts/icons: L. Common-ish in BBS art
  archives (ACiD/iCE RIP packs). The `.ICN` icon format and `.CHR` BGI fonts are part of the job.
- **NAPLPS `.nap`**: Prodigy/Telidon vector graphics, 6-bit coordinate stream [search,
  <https://www.fileformat.info/format/naplps/egff.htm>]; L, rare.
- **Minitel VDT / VTX**: 7-bit videotex stream with semi-graphic mosaics (40x24, 8 colours) [search,
  <https://anto80.com/en-us/format-minitel-videotex>]. Shares the teletext-style mosaic model
  (section 2.1). M. Low.
- ANSI/BIN/XBin/ADF/IDF/PCB/TundraDraw/Avatar are already done.

## 7. Mac

- **PICT**: serialised QuickDraw opcodes. A bitmap subset (v1/v2 header, `BitsRect`, `PackBitsRect`,
  `DirectBitsRect`, `PackBitsRgn`, 1/2/4/8/16/32-bit pixmaps with colour tables) is a bounded task. Apple's
  "Imaging With QuickDraw" (Appendix A) <https://developer.apple.com/library/archive/documentation/mac/QuickDraw/QuickDraw-461.html>
  and the PDF <https://developer.apple.com/library/archive/documentation/mac/pdf/ImagingWithQuickDraw.pdf>
  [search]. `oxideav-pict` (MIT, written clean-room from Inside Macintosh) is a permissive
  cross-check <https://github.com/OxideAV/oxideav-pict> [fetched]. It handles BitsRect/PackBitsRect/
  DirectBitsRect plus shapes and QuickTime-embedded raw/JPEG. A PICT starts with a 512-byte header
  in files (often zeroed) then `size, frame, 11 01 / 00 11 02 FF`; Sig: moderate.
  M for bitmap-only, L with vector shapes and text. The same opcodes appear in IIGS `$C0/0003`
  and `$C1/0001`, so one decoder would serve both.
- Mac Startup screens, HyperCard bitmaps, Kid Pix, SuperPaint: no documentation found in this
  pass. Deark (MIT) is the known permissive reference for some [memory].

## 8. Samples and corpus plan

- Teletext: edit.tf exports; archives of Ceefax/Teefax art (unconfirmed names).
- DOS formats: scene.org file archive, Simtel, `ftp.cdrom.com` mirrors, Demozoo downloads
  (all general knowledge; not browsed).
- CoCo 3, TI-99, QL: community archives named above; likely small. Bring this into the existing
  `corpus/extra/<group>/MANIFEST.tsv` scheme.
- Game Boy and NES: easy to produce samples (GBTD, NESST, own GB Camera saves).
- Since RECOIL does not decode most of these, the oracle test cannot check them. Follow the
  `textmode.md` precedent: record a `divergences/<group>.tsv` entry per sample with a Deark or
  reference-viewer render as evidence.

## 9. Top 10 for this family

1. **Teletext pages (EP1, TTI, raw Mode 7)**: living scene, public specs, and fills the one clear
   BBC Micro gap; fonts need a provenance decision.
2. **PCX**: ubiquitous in DOS scene archives, S, strong signature. (If generic rasters are in scope.)
3. **Autodesk Animator PIC / CEL (and FLI/FLC frame 1)**: DOS demoscene staple, well documented, S,
   and a stepping stone to Pro Motion.
4. **CoCo 3 CM3, MGE, HRS, RAT, VEF**: RECOIL lacks them; MIT-licensed docs and readers exist, M.
5. **Targa**: second most common DOS/raytracing format, S.
6. **Dr. Halo CUT/PAL + PC Paint PIC + ColoRIX**: three small DOS paint formats generic libraries
   skip; each S, shared RLE ideas.
7. **DreamGrafix + Apple II packed Hi-Res/DHR**: the best-documented remaining Apple gaps (CiderPress II
   docs); M because of the LZW dialect.
8. **GB Camera SAV + NES CHR/NAM + GBTD GBR/GBM**: active homebrew art, cheap decoders, easy samples.
9. **TI-Artist `_P`/`_C`**: trivial reuse of the TMS9918 / SC2 code with Companions; limited samples.
10. **PBM / Sun raster / SGI / XBM**: batch of strong-signature S formats, low retro value but
    near-free if generic rasters are in scope.

Just outside the list: PICT bitmap subset (also unlocks two IIGS types), Pro Motion and GrafX2 PKM
(need reverse engineering or FLC first), TI-83+/Casio calculator images, RIPscrip (L), Aseprite
(modern).

## 10. To avoid (GPL / unverified: do not read code)

| Project | Licence | Note |
|---|---|---|
| GrafX2 (incl. PKM, Thomson, many retro loaders) | GPL-2 | prose docs only; pulkomandy.tk wiki is robots-blocked |
| ep128emu (Enterprise) | GPL | emulator |
| jamieleecho/coco-tools | GPL-2 | CoCo converters |
| netpbm GPL parts, GIMP plug-ins | GPL | use format specs instead |
| RIPterm, NAPLPS viewers of unknown licence | unknown | prose specs only |
| raphnet/gbcam2png, bashaus/gbtiles, bbbbbr/gimp-tilemap-gb, aseprite-loader | not checked | check licences before reading; prose first |
| Arculator/RPCEmu/ArcEm, uQLx/sQLux, Minerva | GPL | already listed in `riscos-ql.md` |

Safe to read: KAOS Toolkit (MIT [fetched]), oxideav-pict (MIT [fetched]), CiderPress II
(Apache-2.0, docs CC BY-SA 4.0), Deark (MIT), MAME BSD files (check each header).
