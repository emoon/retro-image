# Commodore 8-bit gap survey: formats we do not decode yet

Scope: C64, C128, VIC-20, Plus/4 and PET image formats that are not in our registry
(see [formats.md](../formats.md), [coverage.md](../coverage.md) and
[commodore.md](commodore.md)). RECOIL is the baseline here, not the ceiling, so the list
includes formats RECOIL lacks. Surveyed 2026-10-03. **No GPL decoder source was read.** Every
layout claim below comes from prose docs, permissively licensed sources, or measurements of
the local CSDb dump. Claims I could not confirm are marked **(uncertain)**.

## 1. Method and limits

- Read the existing docs first. The twelve C64 formats we already know we lack (BDP, ESH, FLM,
  FP, HLE, ILE, RPH, SH1, SH2, SHS, UIF, ZOM) and the 8642-byte SHE variant stay as in
  [commodore.md](commodore.md). They have no samples and are not repeated here.
- Frequency was measured on `~/Downloads/assembly64/csdb/dump` (extracted CSDb 5.0.7 zips):
  `graphics/index.tsv` has 93,379 files (name, size, load address), `tools` 106,474, `misc`
  24,921, `demos` 156,740. Counts below are file counts in that dump unless stated. They
  over-count disks that appear in several releases, so read them as "common" versus "rare", not
  as exact numbers.
- Web access was partial. `fileformats.archiveteam.org` refused connections and
  `web.archive.org` is blocked for the fetch tool, so I could not re-read the Codebase64 grafix
  list or the Just Solve pages in this pass. GoDot's pages (MIT project), the C64 OS article,
  the CSDb forum, the cc65 wiki and the old VCR blog were readable. GoDot loader pages are
  German prose and give layouts, but often no byte-level detail.
- Sample availability: the CSDb dump, the Assembly64 zips at
  <https://hackerswithstyle.se/artifacts/>, zimmers.net `/pub/cbm/`, and format-specific archives
  noted per entry.

### What the dump says about unknown clusters

The most common (size, load address) pairs in `graphics/` that are not obviously ours or
trivially wrapped:

| Size / load | Count | What it is |
|---|---:|---|
| 10003 / `$6000` | 3910 | Koala (covered) |
| 2098 / `$0801` | 1877 | Self-displaying PETSCII PRG, see G1 |
| 10051 / `$5800`, 10241, 10050 | 358+84+42 on `.B(99)` names | Drazpaint-family multicolour (covered) |
| 16146 / `$0801` | 544 | Self-running pictures, several stubs (uncertain, see G2) |
| 10500 / `$0801` | 417 | Viewer + multicolour bitmap, see G2 |
| 10608 / `$0801` | 379 | Same idea, other viewer, see G2 |
| 8194 / `$2000` | 400 | Hires bitmap (covered) |
| 70 / `$0f0c` etc. | ~700 | PETSCII BOT (covered) |
| 631 / `$6800` (`.GRA`) | 58 | PrintMaster clip art, see G4 |
| 1778, 2032, 3556, 2286, 762 / `$0041` | ~1500 | Artefact: the first two bytes are not a load address. The files start `41 00 00 ...` and look like REL or non-PRG records misread by the extractor. Not a format. **(uncertain)** |
| 20030 (`.SCP`), 7225 (`.TUM`), 34446 (`.PPE`) / `$0801` | 53, 52, 47 | Packer-wrapped executables, one tool per extension, see G9 |

## 2. Candidates

Difficulty: S = a day or less with samples, M = several days, L = a project.
"Signature" says whether a content check can claim the file without relying on the extension.

### G1. Self-displaying PETSCII PRG (2098 bytes)

- Extensions: `.prg` (no tool tag). Platform: C64.
- Evidence: 1851 files in the dump have the identical 14-byte prefix
  `01 08 0b 08 ef 00 9e 32 30 36 31 00 00 00` (BASIC `SYS 2061`) and size 2098. One file is
  named `petmate`, so the exporter is probably Petmate's PRG export **(uncertain, the stub was
  not matched against Petmate's documentation)**.
- Layout, derived by reading the operand bytes of the stub (no decoder code used) and checked
  on all 1851 files:
  - screen codes at file offset 98 (`$0861` in memory), 1000 bytes
  - colour RAM at offset 1098 (`$0C49`), 1000 bytes, every byte below 16 in all 1851 files
  - byte 20 is the `$D018` value: `$14` (upper case/graphics ROM set) in 1793 files, `$17`
    (lower/upper set) in 58 files
  - byte 25 is the border colour (`$D020`), byte 30 the background (`$D021`)
  - 98 + 1000 + 1000 = 2098, which fits the size.
- Primary source: the files themselves. No separate prose spec. Related prose:
  [C64 OS image formats](https://c64os.com/post/imageformats) for the screen+colour layout.
- Frequency: very high (about 2% of all graphics files in the dump; the Plain PETSCII compo
  releases, Popeyed Mystery, This..is..PETSCII).
- Difficulty: S. We already render PETSCII with the ROM charsets for `.pet` and `.pdr`.
- Samples: plenty in the dump, e.g. `graphics/All/T/This..is..PETSCII/thisispetscii.prg`.
- Signature: yes, the 14-byte prefix plus size 2098 is unambiguous. Other stub variants exist
  (2286, 2093, 4197 bytes, one file each); treat them as out of scope.

### G2. Viewer-wrapped bitmaps (BASIC stub + picture data, `$0801`)

- Extensions: `.prg`. Platform: C64.
- Evidence: 412 of 417 files of size 10500 share a 20-byte stub
  (`0b08f0029e32303631...a9148d18d0a200a9`); 371 of 379 files of size 10608 share another
  (`0b08e0029e32303631...a219b5029d6f31ca`); 259 of 260 files of size 2499 share a third.
  In the 10500 and 10608 files a 1000-byte run of values below 16 (colour RAM) sits at the
  tail (offsets 9499 and 9571), which says the picture data is stored in Koala order
  (bitmap, screen, colour) after a fixed-size viewer. For 10500: bitmap at offset 499, screen at
  8499, colour at 9499 (this fits the file size). The background byte is probably inside the
  viewer code **(uncertain, not located)**. The 2499 group's data is not identified
  **(uncertain: perhaps a character-mode multicolour picture)**.
- The 16146, 10193, 10436, 14145 and 46913 groups have many different stubs (33 to 93 prefix
  clusters each), so they are several tools, not one format.
- Source: the files. No prose spec. Treat each stub as its own sub-format.
- Frequency: high (about 1,100 files across the three stub groups above).
- Difficulty: S per stub, once the data offsets and background position are pinned by flipping
  bytes. Risk: each stub is one viewer, so coverage is per tool.
- Samples: `graphics/All/2/2020 Escape from New York Cancelled/efnyc2020.prg` (10500),
  `.../4colpocalypse/4colpoc.prg` (10608).
- Signature: yes, stub prefix plus size.

### G3. GEOS geoPaint, Photo Scrap, Photo Album (CVT container)

- Extensions: `.cvt` (GEOS Convert SEQ wrapper), also raw VLIR files on D64. Platform: C64,
  C128 (GEOS 128, 80-column variant).
- Sources: [Just Solve: GeoPaint](http://justsolve.archiveteam.org/wiki/GeoPaint) and
  [GEOS VLIR](http://fileformats.archiveteam.org/wiki/GEOS_VLIR) (both unreachable in this pass,
  known from search snippets: geoPaint is a VLIR file, so it needs a transfer format such as
  Convert or D64); [cc65 wiki, Apple GEOS Convert Format](https://github.com/cc65/wiki/wiki/Apple-GEOS-Convert-Format)
  (the Apple II variant, only gives the structure at a high level);
  [GoDot GeoPaint loader](https://www.godot64.de/german/l_geopaint.htm): up to 80x90 cards
  (640x720), Packbits-style RLE, default dark grey on light grey when no colour data, and "too
  complicated to present in detail" so no byte layout; [Commodore GEOS FAQ](http://www.zimmers.net/geos/GEOSFAQ.html).
  `karstenw/geosLib` converts geoPaint, Photo Album and Photo Scrap to PNG; **its licence was not
  checked**, so read only docs, not code, until it is.
- Frequency: not seen in CSDb (grep for `formatted GEOS file`, the CVT signature string, found no
  picture files; three hits were tools). Common elsewhere: GEOS archives and the Commodore
  Software / Internet Archive collections **(uncertain, not measured)**.
- Difficulty: M. The byte layout of the VLIR index and record encoding needs the Convert spec
  from filegate / Wolfgang Moser's text or sample reverse engineering. The pixel model itself
  is plain hires (bits plus one fg/bg byte per 8x8 card).
- Samples: zimmers.net GEOS FAQ links; geoPaint CVT files on Internet Archive (e.g. the
  "GEOS Convert files" collections). Not in the local dump.
- Signature: yes. A CVT starts with a directory entry and contains `PRG formatted GEOS file V1.0`
  (the string was searched, offset not confirmed here **(uncertain)**).

### G4. PrintMaster / Print Shop clip art (`.GRA`)

- Extension: `.gra`. Platform: C64.
- Source: [GoDot PrintMaster loader](https://www.godot64.de/german/l_pmaster.htm): 7-byte
  header (type word `pW`/`pG` for PrintMaster, `$00 $a0` or `$00 $58` for Print Shop variants,
  then `$58 $00`, height, width), hires bitmap, no compression, PrintMaster rows prefixed with
  `$8b`. Sizes 88x52 (PrintMaster, Print Shop A) and 48x45 (Print Shop B).
- Frequency: 58 files named `*.GRA` of size 631 at load `$6800` in the dump (the Sexcartoons
  disks). A first sample's bytes (`58 00 34 00 b4 8b`) agree with the GoDot description (0x34 =
  52 rows). The 631-byte size would be 7 + 52 x 12 = 631 **(my arithmetic, row width 11 plus
  the `$8b` prefix, uncertain)**. More Print Shop disks exist outside CSDb.
- Difficulty: S.
- Samples: `graphics/All/S/Sexcartoons/` in the dump (e.g. `THEFT.GRA`, `BEHEADER.GRA`).
- Signature: size 631 plus bytes 2-5 are weak but workable together with the extension.

### G5. Petmate `.petmate` (JSON)

- Extension: `.petmate`. Platform: C64 (also VIC-20, PET, Plus/4 charset choices).
- Source: [Petmate site](https://nurpax.github.io/petmate/) and its repository (MIT, to be
  confirmed before reading code). JSON with `version`, `framebufs[]` of `width`, `height`,
  `backgroundColor`, `borderColor`, `charset` (e.g. `upper`), `name`, `screencodes[]` and
  `colors[]` (flat `width*height` arrays). Petmate states the JSON format is meant to stay stable.
  This differs from the binary `.pet` that [commodore.md](commodore.md) mentions.
- Frequency: modern PETSCII compos. One CSDb file is named petmate; the format is usually
  shipped through the editor, not as a CSDb binary. **(uncertain, not measured)**
- Difficulty: S, but needs a JSON parser. We have no dependency policy entry for one; a tiny
  hand-written parser for this schema would do.
- Samples: Petmate's GitHub examples; PETSCII art archives such as Petscii.art **(uncertain)**.
- Signature: yes (`"framebufs"` key).

### G6. Newsroom photos and banners (`PH.`, `BN.` files)

- Platform: C64 (also Apple II, Atari).
- Sources: [Old VCR blog](https://oldvcr.blogspot.com/2023/03/printing-real-headline-news-on.html)
  (photos are 10-byte header + 1-bit rows, 29 bytes per row, MSB first, black = 0, 231x148);
  [GoDot Newsroom loader](https://www.godot64.de/german/l_newsroom.htm): header of length,
  bounds, clip count, clips x 12 + 15, `$FF` terminator, load `$A000`, banners 232x80, photos
  232x168, hires, uncompressed. **The two sources disagree on the header details**
  (10 versus 7 bytes plus a clip block); resolve with a real sample.
- Frequency: 9 matching names in the dump (`PH.`/`BN.` at `$1000`/`$A000`), small.
- Difficulty: S.
- Samples: Newsroom disks on Internet Archive; the 9 in the dump.
- Signature: weak (load address `$A000` plus size, filename prefix).

### G7. CharPad CTM v6 to v9 and SpritePad SPD v2

- Extensions: `.ctm`, `.spd`. Platform: C64.
- We decode CTM v5 and SPD v1 headers. Search results say CharPad currently writes CTM versions
  5 to 9 (the c64lib build plugin lists 5, 6, 7, 8 and 9), and SpritePad 2.0 changed the SPD
  format (new animation storage, not backward compatible).
- Sources: Martin Piper's mirror of the CharPad docs
  ([CTM V4 format](https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt),
  no licence stated); CharPad Pro / Free HTML help describes versions 1 to 7 (not obtained);
  [CSDb forum thread on the SPD format](https://csdb.dk/forums/?roomid=7&topicid=125812): magic
  `SPD`, version byte, sprite count minus 1, animation count minus 1, three colour bytes, then
  63 sprite bytes plus one flag byte per sprite, then four animation arrays. I did not check
  which SPD version our decoder implements, so some of this may already be handled.
  **(uncertain)**
- Frequency: no CTM or SPD magic at file start in the dump (the dump holds PRG exports, not
  projects). Modern projects live on GitHub and itch.io.
- Difficulty: S (SPD v2) and M (CTM v6 to v9, spec for v8/v9 not found).
- Signature: yes (magic plus version).

### G8. XRay64 IFLI

- Extension: none known. Platform: C64.
- Source: [GoDot IFLI loader](https://www.godot64.de/german/l_ifli.htm): XRay64 starts at
  `$0801` with a 466-byte header containing the display routine, then the usual IFLI frames
  (8 video RAMs of 1024 bytes and a bitmap of 8192 per image). It lists Funpaint II, Gunpaint
  and Pixel Perfect too, which we already decode.
- Frequency: 39 names containing "xray" in the dump, of unknown type **(uncertain)**.
- Difficulty: M. Frame order, shifts, colour RAM and background are not in the prose.
- Samples: not located. The dump hits are mostly other things.
- Signature: stub prefix at `$0801`, once seen.

### G9. Self-extracting crunched pictures

- Extensions: `.scp` (20030 B, 53 files), `.tum` (7225 B, 52), `.ppe` (34446 B, 47), `.seb` (8884 B,
  18), `.jam` (6488 B, 14), `.wec` (32093 B, 12), `.brc`, `.cnt`; plus Exomizer/PUCrunch/ByteBoozer
  stubs. All at `$0801`.
- Approach: run the stub in a 6502 emulator until it jumps, then decode the memory image with the
  existing C64 decoders. This is generic and would cover unknown tools at once.
- Sources: [Exomizer](https://bitbucket.org/magli143/exomizer/wiki/Home) (zlib-style licence),
  [Pasi Ojala's pucrunch](https://github.com/mist64/pucrunch) docs. The extension names above were
  taken from file names only; I did not identify the tools. **(uncertain)**
- Difficulty: L. A 6502 core is needed, and the stubs poke VIC and CIA registers.
- Samples: plenty in the dump.
- Signature: BASIC stub only. Whether a file is a picture is known only after running it.

### G10. Diashow Maker (cartridge slide format)

- Platform: C64. Extension: name prefix `B` (PETSCII 191) + `nr` **(uncertain meaning)**.
- Source: [GoDot Diashow loader](https://www.godot64.de/german/l_diash.htm): graphics start
  `$4000`, optional 275-byte autostart header; after unpacking: sprites (512 B) at `$0A00`, video
  RAM `$0C00`, colour RAM `$1000`, VIC registers (3072 B) at `$1400`, bitmap or charset at
  `$2000`. RLE with `$BF` and `$CF` control codes, count one higher than written, active from
  four identical bytes.
- Frequency: not measured; the 85 dump names containing "diashow" are slide shows, not necessarily
  this format.
- Difficulty: M. Mixed text and graphics modes; the data is a memory dump rather than a picture.
- Samples: not located.

## 3. Other formats checked

| Format | Status |
|---|---|
| TSB `.tim` (Simons' BASIC extension) | [GoDot TSB page](https://www.godot64.de/german/l_tsb.htm): load `$E000`, 41 blocks, bitmap 8192 B + 1024 B interleaved colour, background in the last byte with bit 7 as multicolour flag. 4 name hits in the dump. Difficulty S, low frequency. |
| VICE snapshot `.vsf` | Holds VIC-II state plus 64 KB RAM, so one can render the current screen. Format is described in the VICE manual (prose), the code is GPL. Frequency in the dump zero. Value is for scene screenshots; unmeasured. |
| Graphics Magician pictures | Command-list pictures (vector-like language), not bitmaps. Manual: [archive.org](https://archive.org/details/TheGraphicsMagicianPicturePainterManual); an Apple II disassembly exists at [6502disassembly.com](https://6502disassembly.com/a2-graphics-magician/). Difficulty L; the C64 data layout is not covered by what I read. |
| Plus/4 Plus4MC121, Plus4MC16, PCXprep4Pl4 | Named in GoDot's loader list. No byte-level doc found. Plus/4 World may have more. **(uncertain)** |
| M.C.S. (Mandelbrot Construction Set) | [GoDot page](https://www.godot64.de/german/l_mcs.htm): load `$4000`, pack-flag byte, 8 x 1024 video RAM, 8192 bitmap, 1024 colour RAM, background at colour byte 1001, RLE. A FLI container. 6 files with `.MCS` names are 14162 B at `$47F0`, which does not match that layout. Low frequency. |
| Raw sprite sheets (`.SPR` 194/1538/2050/3074 B at `$6400`) | 539 files, 2 + 64n bytes each, all on 'ENCYC ART' disks. Plain sprite memory dumps. Value is low (not pictures), and ambiguous with SpritePad headerless. |
| FGM Clipart, Pagesetter, HyperLink III, VideoFox, Backdrop | In GoDot's loader list; I did not obtain layouts. |
| `.0041` files (about 1500) | Extraction artefact, see section 1. |

## 4. Top 10 for this family

1. **G1 PETSCII self-displaying PRG** (S): 1851 verified files with one identical stub; the layout is fully derived and every colour byte checked; an hour of work.
2. **G2 Viewer-wrapped Koala-order PRGs** (S per stub): about 1,100 files in three stubs, the data offsets are already visible; needs the background byte located.
3. **G4 PrintMaster/Print Shop `.GRA`** (S): documented header, 58 real samples sitting in the dump.
4. **G3 GEOS geoPaint/Photo Scrap/Album** (M): a major real-world C64/C128 format RECOIL lacks; samples are easy outside CSDb, but the byte spec needs finding.
5. **G5 Petmate `.petmate`** (S): the current standard for new PETSCII art, documented and stable, needs a small JSON reader.
6. **G6 Newsroom PH./BN.** (S): two documented layouts (they disagree, so a sample settles it), uncompressed 1-bit.
7. **G7 CharPad v6 to v9 / SpritePad v2** (S to M): format drift in the tools people use today; check what our v5/v1 decoders already accept first.
8. **G8 XRay64 IFLI** (M): documented only in outline, but it extends our IFLI family with an existing loader.
9. **G9 Self-extracting crunched pictures** (L): unlocks every crunched scene picture at once through 6502 emulation, at the highest cost.
10. **G10 Diashow Maker** (M): documented layout and RLE, but samples are not yet found and the benefit is small.

## 5. Sources

- GoDot loader pages (MIT project, German prose): [format table](https://www.godot64.de/german/formats.htm),
  [all loaders](https://www.godot64.de/german/lstab.htm), pages cited above.
- [C64 OS, Image File Formats](https://c64os.com/post/imageformats).
- [Old VCR, Newsroom](https://oldvcr.blogspot.com/2023/03/printing-real-headline-news-on.html).
- [CSDb forum, SPD format](https://csdb.dk/forums/?roomid=7&topicid=125812).
- [cc65 wiki, GEOS Convert](https://github.com/cc65/wiki/wiki/Apple-GEOS-Convert-Format).
- [Petmate](https://nurpax.github.io/petmate/).
- [Kroc, proposed 8-bit Commodore image format](https://gist.github.com/Kroc/32fff4fdc1f4e90fdf5df36480128aa3):
  says Codebase64's list has about 41 bitmap formats and none for PETSCII.
- The local CSDb dump at `~/Downloads/assembly64/csdb/dump` (measurements above).
