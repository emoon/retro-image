# Gap survey: PC DOS/VGA/demoscene and Japanese 8/16-bit PCs

Clean-room research notes on formats retro-image does not decode yet, for two families: (a) IBM PC
DOS/VGA/demoscene pictures and (b) Japanese computers (PC-98, PC-88, X68000, FM Towns, MSX, Sharp
X1/MZ). RECOIL is the baseline, not the ceiling, so formats RECOIL lacks are in scope. No RECOIL
code and no GPL/LGPL decoder code was read. Surveyed 2026-10-03 with web search and page fetches
only.

Reliability: many sites were unreachable from the research sandbox (the Just Solve wiki refused
connections, `retropc.net` failed TLS, `web.archive.org` is blocked, `haniwa.technology/tech/`
index returned 403). Claims taken only from a search-result snippet are marked **(snippet)**.
Claims from memory, with no source seen this session, are marked **(unverified)**. Sample
counts come from directory listings at `sembiance.com/fileFormatSamples/image/<id>/` (dexvert's
sample set), fetched this session unless marked "dir exists".

## 1. What the repo has now

- PC platform: only RECOIL's rows (Award EPA, HS2, MSP v1/v2, FLF, Image 72 font) plus the
  text-mode family (ANSI, BIN, XBin, ADF, IDF, TND, PCB, AVT; see [textmode.md](textmode.md)).
- IFF ILBM, PBM and ACBM are decoded in `platform/amiga/ilbm.rs`, so Deluxe Paint LBM/PBM are
  covered. Not checked: the `BBM ` form type used by the PC Deluxe Paint (see 2.12).
- **No generic PC raster format exists in the registry**: no PCX, GIF, BMP, TGA, ICO. RECOIL has
  none of them either, so the oracle test can't check them (like text-mode, they need a divergence
  file with Deark or visual evidence). Scope decision needed from the maintainer (section 2.0).
- Japanese: MAG/MKI/Pi/PIC (all machines), ZIM, EBD, ArtMaster88, ARV, DaVinci, KT4, MSX BSAVE
  screens, Graph Saurus, BASIC COPY, Dynamic Publisher, G9B, MIF/MIG, Dot Designer's Club. Skipped
  in earlier waves: Q4/XLD4, ML1/MX1/NL3 (no layout), KTY (no sample), SRI (no sample). See
  [msx-japanese.md](msx-japanese.md).
- Already done, so not gaps: GL5-GL8, SH5-SH8 (BASIC COPY), DRG (AtariCAD, Atari 8-bit).

## 2. PC DOS / VGA / demoscene candidates

### 2.0 Scope question

PCX, GIF, BMP and TGA are the best-documented formats in this survey and the most common in the
wild, but they are not "retro" in RECOIL's sense and ordinary tools (`image` crate, ImageMagick)
read them. Decoding them only pays off if retro-image should open a whole DOS-era disk without
another tool. Suggested rule: add them only when a retro-specific mode needs them (EGA/CGA planar
PCX, PCX with VGA palette tricks, GIF with Fractint/GRASP trailers, BMP RLE4/RLE8, OS/2 BMP).
Everything below is rated assuming the maintainer says yes.

### 2.1 PCX (all variants), PCC, DCX

- Extensions: PCX, PCC, DCX (multi-image wrapper: magic `B1 68 DE 3A`, offset table) **(unverified)**.
- Platform: PC. Spec: ZSoft PCX spec (public), [ModdingWiki PCX Format](https://moddingwiki.shikadi.net/wiki/PCX_Format),
  [Wikipedia PCX](https://en.wikipedia.org/wiki/PCX), LEADTOOLS notes on PCX/DCX.
  Deark (MIT) lists PCX as "most of the common varieties" (formats.txt on GitHub, snippet read).
- Wild: ubiquitous. Scene.org, every DOS game, every BBS pack. dexvert has about 35 samples in
  `image/pcx` plus `dcx`.
- Difficulty: S for the RLE core (1, 2, 4, 8 bpp, 1-4 planes, 24-bit); M for the odd variants:
  EGA 16-colour header palette, CGA 4-colour palettes, 256-colour palette at EOF (`0C` marker),
  version 3 (no palette, use default EGA/VGA), version 0/2 palettes, PCX with BytesPerLine
  padding and odd widths, PCC (same layout, 256-byte tail variants) **(unverified)**.
- Signature: first byte `0A`, version 0/2/3/4/5, encoding 1. Good. False-positive rate low.

### 2.2 GIF87a / GIF89a (CompuShow GIFs, Fractint FRA, GIF-EXE)

- Platform: PC (and everything). Spec: GIF89a spec (public). LZW (Unisys patent expired; free).
- Fractint: versions 5-13 saved GIF87a with fractal parameters appended after the trailer (FRA);
  later ones use a GIF89a application extension ([Just Solve FRA (Fractint)](http://justsolve.archiveteam.org/wiki/FRA_(Fractint)),
  snippet). The picture itself is a plain GIF, so a GIF decoder covers it. Fractint's own
  parameters can be ignored.
- GIF-EXE / GifBlast / GifPress: self-displaying or packed GIFs (dexvert dirs `gifexe`, `gifblast`,
  `gifpress`, dir exists, no layouts read). Low value.
- Wild: ubiquitous. Difficulty S (LZW, interlace, transparency, palette). First frame only for
  animation. Signature `GIF87a`/`GIF89a`.

### 2.3 BMP / DIB / OS/2 BMP, RLE4, RLE8

- Spec: Microsoft BITMAPINFO docs (public). Difficulty S. Variants: 1/4/8/16/24/32 bpp, RLE4/RLE8,
  bottom-up/top-down, OS/2 BITMAPCOREHEADER (12 bytes), OS/2 2.x 64-byte header (Huffman 1D
  compression is rare). Signature `BM` (plus `BA`, `CI`, `CP` for OS/2 arrays/icons). Wild: ubiquitous.

### 2.4 TGA (Targa)

- Spec: Truevision TGA 2.0 spec; [Paul Bourke's TGA page](http://www.paulbourke.net/dataformats/tga/)
  has the older layout. Types 1/2/3/9/10/11 (colour-mapped, true colour, grey, RLE), 15/16/24/32 bpp,
  origin bits. Difficulty S. Signature only in the TGA 2.0 footer (`TRUEVISION-XFILE.`); v1 files
  need extension plus header sanity checks, so detection by content is weak. Wild: very common in
  scene texture and 3D-demo packs. dexvert also has `bloodLaceCompressedTGA` (Blood Lace, dir exists).

### 2.5 Autodesk Animator PIC/CEL and FLI/FLC (first frame or poster)

- Extensions: PIC, CEL (original Animator, identical layouts), CEL/FLC (Animator Pro, "identical
  to an FLC in all respects"), FLI, FLC, COL (palette).
- Specs: [CompuPhase FLIC page](https://www.compuphase.com/flic.htm),
  [Jim Kent's Dr. Dobb's article, 1993](https://jacobfilipp.com/DrDobbs/articles/DDJ/1993/9303/9303a/9303a.htm),
  [ModdingWiki FLIC Format](https://moddingwiki.shikadi.net/wiki/FLIC_Format),
  [fileformat.info CEL](https://www.fileformat.info/format/cel/corion.htm),
  [Just Solve Animator PIC/CEL](http://justsolve.archiveteam.org/wiki/Animator_PIC/CEL).
- Magic: FLI `AF11`, FLC `AF12` at offset 4 (confirmed by search results). Original PIC/CEL magic
  `19 91` **(unverified)**. Frame chunks: COLOR_64, COLOR_256, DELTA_FLI, DELTA_FLC, BYTE_RUN, LITERAL,
  BLACK. First frame of an FLI is a full-screen image (BYTE_RUN) or a delta against black.
- Samples: dexvert `animatorPICCEL` has 21 files (`.CEL`, `.RAW`, `.PIC`, sizes 840 B to 481 KB;
  many 64800 bytes = 320x200 + 768-byte palette + header).
- Wild: very common in 1990s DOS games (Autodesk Animator was the standard 320x200 tool). Difficulty S.
  Decide whether an animation yields frame 0 only; the FLI delta codecs are small.

### 2.6 Dr. Halo CUT/PAL and Dr. Halo PIC

- Specs: [fileformat.info Dr. Halo](https://www.fileformat.info/format/drhalo/egff.htm),
  [Just Solve Dr. Halo CUT](http://fileformats.archiveteam.org/wiki/Dr._Halo_CUT),
  [Just Solve Dr. Halo PIC](http://justsolve.archiveteam.org/wiki/Dr._Halo_PIC),
  [LEADTOOLS CUT](https://www.leadtools.com/help/sdk/v20/main/api/dr-halo-cut.html),
  [Graphics Academy](https://www.graphicsacademy.com/format_drhalo.php). Deark has a module
  (MIT, snippet).
- CUT: width/height LE16, a zero word, then RLE rows each preceded by a length word. 256 levels;
  without a `.PAL` the picture is greyscale. Companion PAL is a separate file (fits the repo's
  "Companions" feature). No magic in CUT, so extension-only. PAL header detail **(unverified)**.
- Wild: moderate (Dr. Halo III, Hijaak, early scanner output; used in Fallout-era tools too).
  Samples: dexvert `drHalo` 22 files (CUT up to 977 KB, 2 PAL, 4 PIC).
- Difficulty S (CUT), S-M (PIC, a different block layout). Rank: worth doing.

### 2.7 PCPaint / PICtor PIC and CLP

- Specs: [Wikipedia PICtor](https://en.wikipedia.org/wiki/PICtor_PIC_image_format),
  [Just Solve PCPaint PIC](http://fileformats.archiveteam.org/wiki/PCPaint_PIC),
  [fileformat.info Pictor](https://www.fileformat.info/format/pictor/egff.htm),
  [PMView notes](http://www.pmview.com/help/pmview/pc_paint_pictor_format_pic_.html). Deark: yes (MIT, snippet).
- Magic `34 12` (LE `0x1234`) at offset 0; 17-byte fixed header, then variable extra data (usually
  palette), RLE or raw by the last header word; 2/4/16/256 colours and later 24-bit. CLP is the
  clip variant (same codec).
- Wild: moderate. Early DOS standard, used by GRASP and Pictor Paint, later GLPaint.
  Samples: dexvert `pcPaint` 23 files (PIC and CLP up to 936 KB). Difficulty S-M (mode variants:
  CGA/EGA plane layouts).
- Note: dexvert's `glPaintPIC` (14 `.PIC`, GLPaint 7 era, adult art) is the later extended PICtor.

### 2.8 ColoRIX (RIX, SCI, SCx, VMG, "RIX3")

- Specs: [Just Solve ColoRIX](http://fileformats.archiveteam.org/wiki/ColoRIX),
  [fileformat.info RIX](https://www.fileformat.info/format/rix/egff.htm),
  [Fallout Mods RIX](https://falloutmods.fandom.com/wiki/RIX_File_Format),
  [eloj/rix-magic](https://github.com/eloj/rix-magic) (libmagic definition, not a decoder; licence unchecked).
- New format: `RIX3` magic, 10-byte header, optional extension block, palette, pixels. Old format:
  no header. Extensions starting `SC` are full-screen, the third letter being the mode.
  Compression (RLE + Huffman, with XOR filter for 256-colour, split into segments) is "fairly
  complex" per Just Solve; uncompressed files are trivial. Planar modes selected by the storage byte
  low nibble (snippet).
- Wild: scene-adjacent and game use (Fallout title art, many 256-colour BBS/demo pictures).
  Samples: dexvert `rix` 13 files (SCI, SCX, DAT, DLZ, `abydos.rix`).
- Difficulty S raw, M-L compressed. The spec text is sufficient but needs sample verification.

### 2.9 GRASP GL

- Animation container by Bridges/Pictor lineage ([Just Solve GRASP GL](http://justsolve.archiveteam.org/wiki/GRASP_GL),
  page exists, content not read). Frames use PICtor-style codecs. Difficulty M, wild: moderate
  (1980s-90s DOS demo and BBS animations). Rank low until PCPaint PIC works.

### 2.10 Scene text-mode executables (TheDraw COM/EXE, ACiDDraw COM, Optiks, Fontastic, Laughing Dog)

- Self-displaying `.COM` pieces: an 8086 stub plus screen cells. dexvert has dirs `theDrawCOM`
  (12 files, 765 B to 4176 B, one `.EXE`), `aciddrawCOM`, `optiksCOM`, `fontasticCOM`,
  `laughingDogCOM`, `pcx2com`, `pcx2exeFDelPozo`, `pcx2exeArminioGrgic` (dirs exist). Layout facts
  not found; Deark's `ansiart.c` may know them (MIT, not read). Hands over to the text-mode agent.
  Also TheDraw `.TDF` fonts (not a picture format, but needed to render ANSI logos). Difficulty M,
  detection by stub fingerprint.

### 2.11 BSAVE dumps and raw video-mode dumps (CGA/EGA/VGA/Mode 13h/Mode-X)

- GW-BASIC/BASICA `BSAVE` header: `FD`, segment, offset, length **(unverified; MSX uses `FE`)**.
  Deark has a `bsave` module (CGA, MCGA modes, snippet read). Hardware docs: CGA interleaved
  (even/odd 8 KB banks), EGA 4 planes, VGA 13h linear 64000 bytes with a 768-byte 6-bit palette.
- No signature for raw 64000-byte VGA dumps or 16384-byte CGA dumps; accept by extension and
  size, with a sibling palette (VGA `.PAL` 768 bytes, 0-63 values, or Animator `.COL`) as a
  Companion. Palette conventions vary (6-bit vs 8-bit), so decide per file by max value.
- Wild: common in demo and tool output; hard to identify. Difficulty S. Divergence evidence only
  through Deark's `bsave`.

### 2.12 Deluxe Paint PC (`BBM `), Deluxe Paint ANIM

- PC Deluxe Paint II Enhanced saved IFF with form type `ILBM` (`.LBM`), and EA's `BBM ` form
  type for brush files **(unverified)**. If `ilbm.rs` accepts only `ILBM`/`PBM `/`ACBM`, add `BBM `.
  Check with a corpus search; probably a one-line fix.

### 2.13 Windows icons/cursors (ICO, CUR), Windows 3 `.RLE`, MacPaint-in-PC

- ICO/CUR: documented by Microsoft; dexvert dirs `ico`, `cur`, `icoOS2`. Difficulty S. Not DOS but
  period-adjacent; low priority.

### 2.14 GrafX2 PKM, Pro Motion, Image Alchemy

- **PKM**: GrafX2's native format ([Just Solve PKM](http://justsolve.archiveteam.org/wiki/PKM)):
  `PKM` magic, version byte, 780-byte header, comment records, simple RLE. The authoritative text
  is `TECH_ENG.TXT` shipped with the DOS GrafX2, and GrafX2 is GPL. **Do not read GrafX2 source or
  its docs**; only third-party prose such as the XnView forum thread. Wild: low (late-90s scene
  paint tool). Difficulty S. Hold until a non-GPL layout source is found.
- **Pro Motion**: Cosmigo documents its own formats at
  [cosmigo.github.io/pmotion-assets/PMNG_File_Formats.html](https://cosmigo.github.io/pmotion-assets/PMNG_File_Formats.html)
  (licence of that repo not checked). Pro Motion is mostly Windows; the formats are sprite/stencil
  files, not an old scene staple. Low priority.
- **Image Alchemy** (Handmade Software): converter with its own formats (`.HSI`, `.BIF`, `.HST`,
  `.RAW`, `.PAL`, snippet). Manuals are on [tex.imm.uran.ru/alchemy.pdf](http://tex.imm.uran.ru/alchemy.pdf)
  and [mpoli.fi addendum.pdf](https://files.mpoli.fi/unpacked/software/dos/graphics/pcdemo.zip/addendum.pdf)
  (neither read). Layout detail not found. Low priority (L, rare).

### 2.15 Not found / not distinctive

- **PictureMaker** (Cubicomp): files are `.A8` (24-bit animation) and `.CUBI` bitmaps (snippet);
  no layout doc found. Skip.
- **256-byte/4k/64k intro textures**: procedural, no file formats. Nothing to decode.
- **Tracker-era scene art**: in practice PCX, GIF, LBM, ANSI/XBin, Animator CEL and RIX (above).
- Other DOS/game formats with permissive docs that could ride along cheaply: Doom/Quake lump
  pictures (dexvert dirs `doomPicture`, `quakeGFXLMP`), IBM Storyboard PIC (`ibmStoryboardPic`),
  PFS First Publisher, WPG, Utah RLE (`utahRLE`). Not scene formats; not researched.

## 3. Japanese computer candidates

### 3.1 Yanagisawa PIC2 (`.P2`, magic `P2DT`)

- Platform: PC-98 / X68000 (**snippet**: Just Solve PIC2, "files begin with ASCII `P2DT`").
  Sibling of Pi and PIC by the same author.
- Spec: not found. [Just Solve PIC2](http://justsolve.archiveteam.org/wiki/PIC2) exists (unread, site
  down). Check the Vector archive for a `PIC2` spec text next to PITECH and PIC_FMT (**unverified**).
- Samples: dexvert `yanagisawaPIC2` 12 files (30 KB to 850 KB, `.P2`). Wild: moderate (BBS CG).
- Difficulty M if a spec exists, L otherwise. Signature distinguishable (4-byte magic).

### 3.2 FM Towns formats missing from the registry

- **Pi on FM Towns**: the decoder exists; coverage.md lists "FM Towns PI" as missing only because the
  platform is not registered. Cost: S (registry only). No FM Towns sample (known, wave 6).
- **ICN** (FM Towns icons): three layouts with signatures `CRI-FUJI`, `CRI-FJ2 ` and `ICNFILE` per
  [Just Solve ICN (FM Towns)](http://justsolve.archiveteam.org/wiki/ICN_(FM_Towns)) (snippet).
  Deark has an `fmtowns_icn` module (MIT, readable). Samples: dexvert `fmTownsIcons` 12 files
  (140 B to 121 KB). Difficulty S-M. Distinguishable by magic.
- **HEL** (FM Towns animation): Deark `fmtowns_hel` ("a simple animation format", MIT, readable).
  dexvert `fmTownsHEL` 10 files; every size ends in `...412` or `...812` (all are 12 mod 1000),
  which hints at fixed-size frames plus a small header **(inference from file sizes)**. Difficulty S-M with Deark.
- **TK4** (`.tk4`): FM Towns image, name only; dexvert `fmTownsTK4` 7 files (4.8 KB to 64 KB).
  No spec found. L (reverse engineer), small files suggest 16-colour sprites.
- **Towns Paint II** (`.pii`): dexvert `townsPaintII` 4 files (54 KB to 412 KB). No spec found. L.
- **Towns TIFF** (32768-colour, 16-bit LE words, TIFF header declaring greyscale): from the
  [TIFF-Town readme](https://github.com/DerekPascarella/TIFF-Town) (snippet; licence not checked). It
  is a TIFF container, so handle only if general TIFF is ever added.

### 3.3 "ECC" (Ecchi) and the Susie-era CG formats

- `.ECC`: "Ecchi picture format", Japanese anime/manga CG (snippet, filext and reaConverter). dexvert
  `ecchi` has 8 samples (13 KB to 256 KB). Platform unknown, probably PC-98 or Windows BBS CG. No spec
  found. L.
- Other format names turned up in converter lists (Vector DOS converters, the `DLOAD` loader list on
  toshiki.la.coocan.jp, snippets only): ALG, B1/R1/G1/E1 (per-plane files?), BQ/RQ/GQ/EQ, FRM, IM4,
  KTX, MPT, PCK, SCP, SQ4, SQF, ST2/ST4/ST5, STF, WML, FMAG, GMP, PIF/II, VHP, BLK, DJP, Q0.
  None has a spec found. B1/R1/G1/E1 are probably raw 1-bpp planes (blue, red, green, intensity)
  **(inference from names; EBD and `pc88_planes.rs` already work on that model)**. Q0 is a 24-bit
  format ("Japanese Q0 24-bit images", snippet). Worth one targeted search each before any work.
- **MAG-adjacent**: the PC-98 `RGB` palette file is 16 x 12-bit entries, 48 bytes (Data Crystal,
  [File formats for PC-98](https://datacrystal.tcrf.net/wiki/File_formats_for_PC-98)). Trivial; useful
  only as a Companion for raw plane dumps.

### 3.4 Alice Soft VSP (PC-98), PMS, GP4

- VSP: 4-bit planar, 8-pixel-wide columns with 4 planes; header holds x0/y0/x1/y1 (x in 8-pixel units),
  a palette bank byte and a 48-byte palette; compression is RLE plus copy-from-previous-column and
  XOR-with-plane commands (opcodes 0x00-0x06). Source:
  [haniwa.technology/tech/vsp.html](https://haniwa.technology/tech/vsp.html) (prose read; the page's
  example code is taken from xsystem35, which is GPL, so **do not read the code**). No signature.
- Wild: moderate to low; Alice Soft PC-98 games (Rance, Toushin Toshi series, System 1/2/3
  era). Samples: extract from game archives (`.ALD` etc.); no standalone set found. dexvert has no VSP
  directory.
- PMS8/PMS16/QNT/AJP/FLAT are Windows-era System 4 formats (haniwa page, snippet). Out of retro
  scope unless the maintainer wants PC-98-to-Windows transition formats. Difficulty VSP: M.
- GP4: not documented on the haniwa page (searched, not found).

### 3.5 MicroProse PIC / PIC93 and SCP (Civilization PC-98)

- A [CivFanatics thread](https://forums.civfanatics.com/threads/civ-pc-98-graphics-formats-and-potential-windows-port.692067/)
  says CanadianAvenger documented "PIC93" and an SCP container of 32x32 sprites in a blog post
  ("PIC: The Next Generation"). The post itself was not located. Licence/accuracy unknown. L, niche.

### 3.6 Platforms with nothing found

- **Sharp X1 / MZ-2000 / MZ-2500 / MZ-700**: no native picture format documentation turned up. Their
  CG circulated as MAG and Pi (machine code in the MAG header). Check which `machine` bytes
  Mooncore's makichan.htm lists for X1/MZ (not read this pass). **(unverified)**
- **X68000** beyond PIC, MAG and Pi: native pictures are Pi/PIC/MAG; no other format spec found in this
  pass (the retropc.net graphics library list, which would have answered it, failed to load over TLS).
- **PC-88/98 BASIC "BSAVE" plane dumps and VRAM dumps**: no layout docs found beyond hardware
  manuals. The existing `pc88_planes.rs` model probably covers them.
- **MSX1 Screen 1 (Graphic 1) dumps**: spec complete in the MSX2 Technical Handbook
  [Appendix 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html) (name table
  1800h, patterns 0000h, colours 2000h, 32 bytes). Rare as pictures. S, low value.

## 4. Samples to collect (sources)

- dexvert sample tree: `https://sembiance.com/fileFormatSamples/image/<id>/` for `animatorPICCEL`,
  `drHalo`, `pcPaint`, `glPaintPIC`, `rix`, `pcx`, `yanagisawaPIC2`, `ecchi`, `fmTownsIcons`,
  `fmTownsHEL`, `fmTownsTK4`, `townsPaintII`, `theDrawCOM`.
- Archive.org: PC88 image collection (1817) and "256color" PC-98 collection (already in msx-japanese.md).
- scene.org, textfiles.com and `files.mpoli.fi` (DOS software mirror) for DOS-era tools and their
  output.

## 5. Top-10 ranking for this family

Ranked by (usefulness, difficulty, spec clarity, samples), assuming generic PC rasters are accepted.

1. **PCX (all variants)**: the most-met DOS picture; spec public; EGA/CGA planar variants are
   genuinely retro; ~35 samples.
2. **Autodesk Animator PIC/CEL + FLI/FLC first frame**: DOS-game and scene staple; spec complete (CompuPhase,
   Dr. Dobb's); 21 samples; small codec set.
3. **Dr. Halo CUT/PAL/PIC**: simple RLE, clean spec, 22 samples, fits the Companions model.
4. **PCPaint/PICtor PIC and CLP**: first DOS standard, Deark documents it, 23 samples, shares code
   ideas with GRASP.
5. **ColoRIX RIX/SCx**: scene-heavy 256-colour art; raw mode trivial, compressed mode needs care; 13 samples.
6. **GIF87/89 (Fractint FRA, CompuShow)**: universal, easy; do it only if the scope decision is yes.
7. **BMP RLE4/RLE8 + OS/2**: easy; DOS-period bitmaps from Windows 3 art tools.
8. **FM Towns ICN, HEL (via Deark's MIT modules)** plus **FM Towns Pi registration**: closes the only "FM
   Towns missing" row at near-zero risk, with 12+10 samples.
9. **Yanagisawa PIC2 (`P2DT`)**: sibling of Pi/PIC, clear magic, 12 samples; blocked on finding a spec.
10. **Alice Soft VSP**: documented prose spec, PC-98 era, but no standalone samples and a niche audience.

Near misses: TGA (S, easy, weak content detection), BSAVE/raw VGA dumps with palette companions
(useful glue, no signature), TheDraw/ACiDDraw COM executables (hand to the text-mode survey), TK4/PII/ECC (samples exist,
specs do not).

## 6. Do not read

- **GrafX2** source and its bundled `TECH_ENG.TXT` (GPL).
- **xsystem35** (GPL; the haniwa VSP page's example code derives from it).
- **RECOIL**, and `q4toppm.zip` on its tracker (see msx-japanese.md section 15).
- **SuperSakura, mag2png, XGLOAD, APICG, Susie plug-ins, PicLoader2022, emk loaders, MIF/MIGVIEW** (see
  msx-japanese.md section 15 for licences).
- Code of unchecked licence: `eloj/rix-magic` (definition only; read its prose), `cosmigo/pmotion-assets`
  (check the licence first), `TIFF-Town` (check the licence first).
- **Deark** (MIT) is the permissive reference for PCX, Dr. Halo, PCPaint, Animator PIC/CEL, BSAVE, FM Towns
  HEL and ICN. Its `foreign/` directory has other licences and must not be read.
