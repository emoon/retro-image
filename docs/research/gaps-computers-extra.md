# Format gaps: home computers, microcomputers, early PC and workstation platforms

Clean-room research notes on formats the registry does not decode yet, for platforms outside the
Amiga, console, C64, Atari and ZX/CPC groups: early IBM PC and DOS programs, Japanese and
European home computers, Apple and Mac extras, Unix workstations and terminals, handhelds and
printers. RECOIL is the baseline, not the ceiling, so this also lists formats RECOIL lacks.
No RECOIL code and no GPL/LGPL decoder code was read. Surveyed 2026-10-05, after reading
`formats.md`, `coverage.md`, `CLEANROOM.md`, `adding-a-format.md`, `gaps-ranking.md` and every
`gaps-*.md` and `next-*.md` file.

## How to read this

Claims carry a tag:

- **[fetched]**: the page or file was read in this session (prose spec, or MIT/Apache code).
- **[checked]**: I also downloaded sample files and parsed or rendered them in a scratch script
  (outside the repo) to see that the layout holds. Rendering was by eye.
- **[snippet]**: only a catalogue entry or a short description was seen. Layout is unconfirmed.
- **[memory]**: from general knowledge. The URL, if any, is a pointer and was not verified.

Difficulty: S = a day or less with a usable spec, M = a few days or a risky detail, L = a
reverse-engineering job. "Sig" says whether content detection can be strict enough for
`.signature()`.

## Environment limits

- The shared WebSearch budget (200 calls) was already spent when this survey started, so none of
  my search attempts ran. Discovery came from catalogues: the Just Solve wiki (CC0, fetched through
  `curl`, category crawl plus its own site search), Deark's `formats.txt` and module sources
  (MIT), dexvert's published sample tree at `sembiance.com/fileFormatSamples/` (names and
  files only; dexvert's code has no stated licence and was not read), PictureFan's format
  list, the Encyclopedia of Graphics File Formats pages on fileformat.info, and
  CiderPress II's file converter list. That favours formats someone has already catalogued
  and under-reports anything only discussed on forums.
- The spec pages I fetched were reachable on 2026-10-05 and also have a Wayback copy
  (`https://web.archive.org/web/2023/<url>` returned 200 for the KiSS, MicroDesign, VT340,
  Nouspikel TMS9918A, Ninerpedia and textfiles URLs I tested). The Inset PIX spec link on Just Solve
  returns 404 (see C11).
- RECOIL's list (<https://recoil.sourceforge.net/formats.html>, facts only) has none of the
  candidates here. Several only share extensions with unrelated RECOIL rows (see "Extension
  collisions"). So none of them has an oracle, and section 7 says how to check each one.

## 1. What is already covered or ranked, so not repeated

Registered in `formats.md` today: PCX, GIF/FRA, BMP/DIB, ICO/CUR, Targa, MSP, PCPaint PIC/CLP,
Dr. Halo CUT/PIC, ColoRIX, Animator PIC/CEL/FLI/FLC, ANSI/BIN/XBin/ADF/IDF/TND/PCB/AVT,
EPA, HS2, CoCo 3 CM3/HRS/MGE/RAT/VEF, CompuServe RLE, DeskMate PNT, FM Towns ICN/HEL, DreamGrafix
and APF, MacPaint, Thomson, Teletext, Game Boy and NES art, Pi/MAG/MKI everywhere, KT4, ZIM,
EBD, ARV, NL3, ArtMaster88, DaVinci. IFF form `BBM ` is accepted. Atari ST is very
complete (see the registry list).

Already ranked or described in other notes, so only cross-referenced:
PICT and the IIgs QuickDraw II types (`gaps-others.md` 2.4 and 7), RIPscrip and NAPLPS (6),
TI-83/Casio calculator pictures (2.10), Aseprite (5.3), GrafX2 PKM (GPL-only spec),
Pro Motion, Image Alchemy, Yanagisawa PIC2, FM Towns TK4/Towns Paint II/ECC, Q4/ML1/MX1
(`next-blocked.md`), Alice Soft VSP (`gaps-pc-japan.md` 3.4), QL leftovers (`gaps-others.md` 2.8),
Enterprise, Sharp MZ/X1 (called "None" there; section 6 here adds the evidence).

## 2. Summary of candidates

"New platform" means no registry entry exists for the platform name, so a label has to be
chosen (RECOIL has no name for it).

| # | Candidate | Platform | Docs | Samples | Diff | Sig |
|---|---|---|---|---|---|---|
| C1 | KiSS CEL + KCF | KiSS (PC-98, Amiga, ...) new | Public-domain spec [fetched, checked] | 22 downloaded, 10 free doll sets linked | S | yes (`KiSS`) except old headerless CEL |
| C2 | Amstrad PCW: MDA, MDP, CUT, GRF, SPC | Amstrad PCW new | Author spec [fetched], MD2 [checked] | 5 downloaded | S | MDA/MDP yes, rest by size |
| C3 | Sixel | DEC VT340 new | DEC manual [fetched] | 22 listed | S | yes (`ESC P ... q`) |
| C4 | Sun raster, SGI, XWD, XBM, XPM, PNM family, farbfeld, Utah RLE | Unix, new | EGFF and Deark [fetched] | 10-21 per format listed | S each | yes |
| C5 | GRASP GL container (first picture) | PC | Deark code [fetched, checked] | 47 listed, 1 parsed | S | structure only |
| C6 | PFS First Publisher ART | PC | Deark code [fetched, checked 17/17] | 17 downloaded | S | exact size |
| C7 | Print Shop DAT, New Print Shop POG, PrintMaster SHP/SDR, PrintPartner GPH, Print Shop GS | PC, Apple IIGS | Deark, CiderPress II [fetched, SHP and POG checked] | 40+ listed | S | SHP, GPH yes; POG by size |
| C8 | Self-displaying DOS pictures: PIXIT, GIFEXE, PCX2EXE, OPTIKS, GWS EXEPIC | PC | Deark [fetched], PIXIT and GIFEXE [checked] | 60+ listed | S-M | stub fingerprints |
| C9 | GW-BASIC BSAVE dumps and raw CGA/EGA/MCGA screens | PC | Deark [fetched], hardware pages | 23 listed | M | `FD` + length |
| C10 | TI-Artist `_P` + `_C` | TI-99/4A new | Ninerpedia, TMS9918A manual [fetched] | none found | S | none |
| C11 | Inset PIX | PC | EGFF summary [fetched], Deark module (not read) | 34 + 11 listed | M | version byte, unchecked |
| C12 | Small PC batch: DCX, OS/2 icon/pointer/array, Win 1.x ICO/CUR, HP LX ICN, DGI, IBM KIPS, Lumena CEL | PC | Deark [fetched], KIPS and ICN [checked] | 5-10 each | S each | most yes |
| C13 | IBM Storyboard PIC/CAP | PC | Deark code (partial) [fetched] | 28 listed | M | partial |
| C14 | Apple II set: Print Shop clip art, IIgs Finder icons, LZ4FH hi-res, Paintworks animation, lo-res/double lo-res | Apple II, IIGS | CiderPress II [fetched], Just Solve | 11-13 listed | S each | needs ProDOS type, mostly |
| C15 | EPOC MBM, Sketch, AIF | Psion Series 5 | Deark code [fetched, header only] | 10 listed | M | UID words |
| C16 | PC-98 plane dumps B1/R1/G1/E1, BLK, FRM | NEC PC-98 | names only [snippet] | none found | S | none |
| C17 | WordPerfect Graphics WPG | PC | EGFF, Deark [snippet] | 19 listed | M | yes |
| C18 | Palm bitmap, Palm ImageViewer, TealPaint | Palm OS | Deark [snippet] | 10 listed | M | partial |
| C19 | Epson ESC/P bit-image streams | printers | Epson manual [snippet] | none found | M | none |
| C20 | Slow-scan TV `.HRZ` | amateur radio | Just Solve [fetched] | 1 listed | XS | size only |

Rows C17 to C20 are lower value and are not ranked; C16 is ranked 15th but is blocked on samples.
All are described in section 4.

## 3. Ranked top 15 for this area

Ranked by retro relevance, how often the file turns up in archives, a spec that does not depend
on a licence decision, samples in hand, and cost.

| Rank | Candidate | Why |
|---|---|---|
| 1 | C1 KiSS CEL + KCF | Complete public-domain spec, headers matched on all 22 downloaded samples (3 rendered), an active community, a strong signature, S. No other tool in the registry covers it. |
| 2 | C4 Unix rasters (Sun, SGI, XWD, XBM/XPM, PNM, farbfeld, Utah RLE) | Specs and samples are easy to find, and every one is S. They turn up in `graphics/` folders of FTP and CD mirrors. Pillow can act as an oracle for most. Needs the generic-raster scope decision first. |
| 3 | C2 Amstrad PCW (MDA, MDP, CUT, GRF, SPC) | A whole platform, with a primary spec from the program's own publisher. MD2 matched 5 of 5 samples byte for byte. All 1-bit, so the decoders are tiny. |
| 4 | C3 Sixel | Primary DEC spec, MIT reference code and 22 samples in hand. Strong signature. Terminal and BBS art. |
| 5 | C5 GRASP GL | A common DOS BBS animation format. Container holds PCPaint PIC/CLP files, which we already decode, so the first picture costs one small parser. |
| 6 | C6 + C7 DOS clip-art libraries (FP ART, Print Shop, PrintMaster, PrintPartner) | Hundreds of shareware CD archives. Layouts checked on samples. 1-bit, so tiny. The multi-image question needs a decision. |
| 7 | C8 Self-displaying DOS pictures | The DOS twin of the C64 self-displaying PRG item (`gaps-ranking.md` #1): fixed stub, known payload offset. Disk magazines and BBS packs. |
| 8 | C9 BSAVE and raw video dumps | Common in old DOS trees. The `FD` header is a usable check. Raw dumps with no header stay extension-gated. |
| 9 | C10 TI-Artist | A new home-computer platform, reuses the MSX Screen 2 code. Held back by missing samples. |
| 10 | C11 Inset PIX | WordStar, MultiMate and PC-Write art, 45 samples, but the spec page is gone and there are two incompatible versions. |
| 11 | C12 small PC batch | Each is S, none matters much alone. DCX is XS. |
| 12 | C13 IBM Storyboard | 28 samples but only partial knowledge (Deark itself calls most of it experimental). |
| 13 | C14 Apple II set | Easy decodes, but identification needs the ProDOS file type that our byte-only API never sees. |
| 14 | C15 EPOC MBM | Documented compression types and a strong signature, but niche and large. |
| 15 | C16 PC-98 plane dumps | Trivial once seen, but no sample was found and the palette sidecar formats are unverified. |

Cheap first batch: C1, C2, C3 and the simplest half of C4 (Sun, SGI, XBM, PNM).

## 4. Candidate notes

### C1. KiSS CEL and KCF (Kisekae Set System)

- Platform: started on NEC PC-9801 in March 1991, then spread to Amiga, Atari, Mac, Windows and
  Linux. Suggested label "KiSS". Extensions `.cel`, `.kcf`; the scene is described by a text
  `.cnf` and the whole doll usually ships as LZH or ZIP.
- Pixels: a CEL is a transparent sprite (index 0 = transparent) with a placement offset. A KCF is
  a separate palette file. Both start with a 32-byte header [fetched, checked].
- CEL header: `"KiSS"`, byte 4 = `0x20`, byte 5 = bits per pixel (4 or 8), word 6 reserved,
  then little-endian width, height, x offset and y offset at 8, 10, 12 and 14, 16 bytes
  reserved. Pixels follow at 32: 4 bpp packs two pixels per byte, first pixel in the high
  nibble, odd widths padded with one colour-0 pixel; 8 bpp is one byte per pixel.
- Old format: no `KiSS` signature. Little-endian `u16` width, `u16` height, 4 bpp rows from
  offset 4. Checked: `1peace.cel` is 139x252, and `4 + 70*252 = 17644` bytes, its exact size.
- KCF header: `"KiSS"`, byte 4 = `0x10`, byte 5 = bits per colour (12 or 24), colours per
  palette group at 8, group count at 10 (up to 10). 12-bit entries are two bytes `rrrr bbbb`,
  `0000 gggg`; 24-bit entries are R, G, B. Old KCF has no header: 10 groups of 16 12-bit
  colours (320 bytes). Palette group 0 is the default; the doll's `$` lines pick groups.
- CKiSS (Cherry KiSS): 32-bit RGBA with header bytes `20 20` or `21 20`, no KCF. Checked: two
  samples are 800x600 and `32 + 800*600*4 = 1920032` bytes. The component order is not in the
  English spec translation. Leave it out or confirm the order from a render.
- Primary source: "KISS/GS" by ITO Takayuki (translation of the Japanese `kissgs.doc`, declared
  public domain), <http://otakuworld.com/kiss/download/kissfrmt.txt> [fetched]; Wayback
  `https://web.archive.org/web/2023/http://otakuworld.com/kiss/download/kissfrmt.txt`.
  Cross-check: Just Solve "KiSS CEL" [fetched], <http://fileformats.archiveteam.org/wiki/KiSS_CEL>.
- Checked: 11 dexvert CELs and 11 KCFs parse against the header layout above. Three CELs
  rendered with a KCF look right (doll, a shirt with lettering, a bitmap). The 4-bit nibble order
  and the `rrrr bbbb` / `0000 gggg` colour order are therefore confirmed.
- Reference code: none known to be permissive. `abydos` (<http://snisurset.net/code/abydos/>)
  and `wuimg` (<https://codeberg.org/kaleido/wuimg>) have unchecked licences, so prose only.
- Samples: dexvert `image/kissCel` (11, with `abydos.kiss.cel` and `test.cel` being the 32-bit
  kind) and `image/kissCELColorPalette` (11); free dolls linked from
  <http://otakuworld.com/kiss/free.htm> (10 LZH sets), plus the "Big KiSS Page" doll index at
  <http://otakuworld.com/kiss/>. Verified-download: dexvert files yes; otakuworld LZH not
  fetched.
- API fit: one CEL decodes to an `Image` with transparency; the palette comes from a `.kcf` with
  the same stem (`Companions::get`), grey when absent (as XnView does). A CNF names palettes
  whose stems differ, which is what `Companions::get_named` is for. Composing a whole doll from
  the CNF is optional and M.
- Difficulty: S for CEL+KCF, M for CNF composition. Relevance: medium; a large 1990s archive
  and an active hobby. RECOIL: no. Validate by eye against XnView (KCF next to CEL) or a KiSS
  viewer from the otakuworld list.

### C2. Amstrad PCW graphics (MicroDesign, Stop Press, The Desktop Publisher)

The Amstrad PCW (Joyce) word-processor line ran CP/M Plus with bitmap pictures from desktop
publishing programs. All of these are 1-bit [fetched].

| Ext | Program | Layout |
|---|---|---|
| `.MDA` / `.MDP` | MicroDesign 2/3 area and page | 128-byte stamp, then `u16` height (multiple of 4) at 128, `u16` width in bytes at 130, data from 132 |
| `.CUT` | Stop Press, MicroDesign | `u16 h1`, `u16 w1`, rows; height = (h1+3)/2, width = w1+2, bytes per row = floor((w+8)/8) |
| `.GRF` | The Desktop Publisher | `u16` width, `u16` height in pixels, rows of ceil(w/8) bytes |
| `.SPC` | Stop Press Canvas | no header, 720x256, 32 blocks of 720 bytes; in each block eight lines are interleaved: line `k` of the block is the bytes at `k, k+8, k+16, ...` |

- MDA stamp: `".MDA"` (4 bytes), `"MicroDesignPCW"` (14), version `"v1.00"` (MicroDesign 2) or
  `"v1.30"` (MicroDesign 3) at 18, CR LF, 7-digit serial, CR LF, zero fill. `.MDP` is the same
  with `".MDP"` and extra page fields at 34 to 36 (dpi, paper format, RAM blocks).
  Bit 1 = white. MD2 RLE: a `00` or `FF` byte is followed by a repeat count (0 means 256),
  and a count may run past the end of the line. MD3 RLE: each line starts with a type byte:
  0 = one fill byte for the whole line, 1 = PackBits-style blocks (negative control byte =
  repeat the next byte `-n+1` times, positive = `n+1` literals), 2 = the same blocks but XORed
  with the previous line.
- Primary source: John Elliott's page, quoting Creative Technology's 1992 MicroDesign 3 format
  sheet, with his own experiments for CUT and GRF:
  <http://www.chiark.greenend.org.uk/~jacobn/cpm/mdaspec.html> [fetched]. SPC from Just Solve
  "Stop Press Canvas", quoting John Elliott's SPC2BMP source (1996)
  <http://fileformats.archiveteam.org/wiki/Stop_Press_Canvas> [fetched]. The CUT and GRF text
  says "not official" and "from experiment"; treat CUT/GRF as snippet-grade.
- Checked: all 5 dexvert `microDesign` files are MD2 (`v1.00`). Decoding them gives exactly
  `height * width_in_bytes` bytes each, and the images render cleanly (castle, hat, lettering).
  MD3, MDP, CUT, GRF and SPC are not sample-checked.
- Extension collisions: `.CUT` is also Dr. Halo CUT and Atari 8-bit Cut Creator, `.GRF` is
  CoCo and ZX Profi, `.SPC` is Atari ST Spectrum 512 and Atari 8-bit Graphics Magician. The
  SPC size (23040 bytes exactly) and the MDA/MDP stamp keep these apart. CUT needs a size
  equation: `4 + h * wb` bytes.
- Pixel shape: PCW pixels are about twice as tall as wide (the SPC note says it is similar to
  CGA 640x200); MicroDesign calls the PCW pixel a "half-pixel". Produce aspect metadata or
  double the width.
- Samples: dexvert `image/microDesign` (5, downloaded); `diagram.cut` inside `mdaspec.com` (a
  PMA archive on the classiccmp mirror,
  <http://www.classiccmp.org/cpmarchives/cpm/mirrors/ftp.demon.com/pub/cpm/mdaspec.com>);
  `joyce.spc` in `joyce-z80-2.1.10.tar.gz`, the Z80 utilities for the Joyce emulator, per Just
  Solve (the Joyce page <https://www.seasip.info/Unix/Joyce/> is reachable; I did not fetch the
  tarball).
  John Elliott's catalogues of the PCW public domain libraries
  (<https://www.seasip.info/AmstradXL/> lists them) point to disk images, which would need a PCW
  disk reader.
- Difficulty: S. Relevance: low to medium; a niche, but it is a platform nobody else decodes.
  RECOIL: no. Validate with John Elliott's `SPC2BMP` (DOS/CP/M, source in `sp2bmsea.com`) or by
  eye; polarity of SPC is unconfirmed.

### C3. Sixel (DEC VT330/VT340 and printers)

- Extensions `.six`, `.sixel`, also raw terminal captures. Not a "home computer" format, but a
  workstation and BBS-art one, and it has a strict spec.
- Layout [fetched]: a DEC control string, `ESC P p1 ; p2 ; p3 q <data> ESC \` (or C1 `0x90`
  ... `0x9C`). Data characters `?` (0x3F) to `~` (0x7E) are six vertical pixels, value = code minus
  0x3F, least significant bit on top. Controls: `!n c` repeats a character, `"Pan;Pad;Ph;Pv`
  sets pixel aspect and nominal size, `#Pc` selects a colour, `#Pc;Pu;Px;Py;Pz` defines one
  (`Pu` 1 = HLS with hue 0-360, lightness and saturation 0-100; 2 = RGB with 0-100 each),
  `$` returns to the left edge of the same band, `-` moves down one band (six rows). `p1` is a
  legacy aspect code (default 2:1), `p2` 1 means zero bits leave the old colour (transparent).
  The VT340 has 16 colour registers, the VT330 four; the default register colours are in a
  different chapter that I did not read.
- Primary source: VT330/VT340 Programmer Reference Manual, chapter 14,
  <https://vt100.net/docs/vt3xx-gp/chapter14.html> [fetched]; Wayback
  `https://web.archive.org/web/2023/http://vt100.net/docs/vt3xx-gp/chapter14.html`.
- Reference code: libsixel, MIT (`LICENSE` read, <https://github.com/saitoha/libsixel>), with an
  `images/` folder of test pictures.
- Samples: dexvert `image/sixel` (22 files, e.g. `CHESS.SIX`, `Shuttle.SIX`, `leonardo.six`);
  libsixel's `images/`. Listing seen, files not downloaded.
- Difficulty: S (an image is a state machine; a 6-row band loop). Sig: strong on the DCS
  prefix and `q`. Relevance: medium-low for a retro archive, higher in terminal and BBS art.
  RECOIL: no. Validate with `img2sixel`/`sixel2png` from libsixel or ImageMagick (both outside the
  tree, as black boxes).
- Related, not proposed: DEC ReGIS (vector command stream, 1 dexvert sample, L) and NCSA Telnet
  ICR (Mac, a command stream, spec in the NCSA Telnet manual, L).

### C4. Unix and workstation rasters

A single batch. The specs are old, public and settled; the decision is scope, as with PCX and
Targa. All have strong signatures except XBM, XPM and PNM text forms, which still start with
fixed text.

| Format | Ext | Signature | Layout facts |
|---|---|---|---|
| Sun raster | `.ras .sun .im1 .im8 .im24` | `59 A6 6A 95` | 32-byte big-endian header: magic, width, height, depth (1, 8, 24, 32), length, type (0 old, 1 standard, 2 RLE, 3 RGB order, 4 TIFF, 5 IFF), colormap type and length; scanlines padded to 16 bits; 24-bit is BGR unless type 3 [fetched, EGFF <https://www.fileformat.info/format/sunraster/egff.htm>] |
| SGI image | `.sgi .rgb .rgba .bw` | `01 DA` | 512-byte header (storage 0 verbatim or 1 RLE, bytes per channel, dimensions, size x/y/z); rows stored bottom-up, one plane per channel; RLE keeps a start/length table per row [fetched code facts in Deark `sgiimage.c`, MIT; spec by Paul Haeberli, mirrored at <http://paulbourke.net/dataformats/sgirgb/>, which was reachable] |
| X Window dump | `.xwd` | file version word 7 in the header [memory] | `xwd` screen capture: fixed header, colormap, pixels; Deark `xwd.c` (MIT) [fetched, not read in depth] |
| XBM / XPM | `.xbm .xpm` | `#define`, `/* XPM */` | text C source; XBM is 1 bit LSB first, XPM needs the X11 colour-name table [memory] |
| PNM family | `.pbm .pgm .ppm .pam` | `P1`-`P7` | netpbm man pages, <https://netpbm.sourceforge.net/doc/pbm.html> [reachable]; farbfeld `farbfeld` + 16-bit RGBA |
| Utah RLE | `.rle` | `52 CC` | Spencer Thomas's RLE design paper, <https://sarnold.github.io/urt/docs/rle.pdf> [memory]; also <https://paulbourke.net/dataformats/urt/> |
| MGR bitmap, Sun icon, CMU Andrew raster, Lisp Machine bitmap, FaceSaver, Xerox Doodle brush, Starbase | various | partial | only named by Just Solve and the netpbm converter pages; no layout page was read [snippet] |

- Samples: dexvert `image/sunRaster` (12), `sgi` (16), `xwd` (11), `xbm` (21), `xpm` (12), `pbm` (13),
  `pgm` (13), `ppm` (14), `pam` (5), `farbfeld` (2), `utahRLE` (10), `mgr` (17), `sunIcon` (19),
  `lispMachineBitmap` (1). Listings seen; not downloaded. The netpbm and libtiff test sets add more.
- Reference code: Deark `sunras.c`, `sgiimage.c`, `xwd.c`, `pnm.c` (MIT, fetched). Pillow
  (HPND) also has Sun, SGI, XBM, XPM and PNM readers and is a quick oracle; it is the only
  candidate oracle installed on this machine (Pillow 9.4).
- Difficulty: S each (Sun 1/8/24/32 plus RLE; SGI RLE; XWD is the only awkward one because the
  header has a colormap-size trap). Relevance: medium (common but not retro). RECOIL: no.
- Skip the obscure rows (MGR, Lisp Machine, Doodle brush): one or two samples each, no spec read.

### C5. GRASP GL animation container

- Extension `.gl`. GRASP (Bridges / Paul Mace, DOS, 1986) is a presentation and animation player;
  a GL file bundles its pictures, animation frames and a script.
- Layout [fetched, Deark `modules/grasp.c`, MIT; checked on a sample]: `u16` little-endian index
  size at 0; entries of 17 bytes from offset 2, each a `u32` file offset and a 13-byte
  NUL-padded name; the end entry has offset 0. At each offset is a `u32` length and then
  the file. The Amiga GL variant starts with `41 47 01 00` and is big-endian.
- Checked: `ACLOCK.GL` (786246 bytes) has an index of 2652 bytes and its first entry is
  `b1big_1.clp` at 2654, which starts `34 12`, the PCPaint CLP signature. The next entries are
  named `1.dff`, `2.dff`, ... (animation frames, probably delta frames; not decoded).
- Value: a still-image decoder can return the first PIC or CLP and hand it to
  `pcpaint::decode_pic/decode_clp`. Frames in `.dff` (delta frames), GRASP fonts and GIF
  members are out of scope. Just Solve says members can be PCPaint PIC, CLP, GRASP font and
  GIF [fetched, <http://fileformats.archiveteam.org/wiki/GRASP_GL>].
- Samples: dexvert `video/grasp`, 47 files, <https://sembiance.com/fileFormatSamples/video/grasp/>
  (one downloaded). Many `.GL` files in the textfiles CDs linked from the Just Solve page.
  The `next-amiga-pc.md` sample group (`corpus/extra/next-amiga-pc/`) already holds some
  GRASP-derived CLP files, so check it before downloading more.
- Difficulty: S for "first picture". Sig: weak (an index sanity check). Relevance: medium
  in the DOS BBS scene. RECOIL: no. Validate by comparing the extracted CLP against our own
  PCPaint decoder, and by eye.

### C6. PFS: First Publisher ART

- Extension `.art` (collides with several Atari ST and Atari 8-bit `.ART` formats). Black and
  white clip art from PFS: First Publisher on DOS.
- Layout [fetched, Deark `modules/misc2.c`, MIT; checked 17 of 17 samples]:
  - Standard resolution: `u16` left, `u16` right, `u16` top, `u16` bottom (the first word is the
    left edge), then 1-bit rows of `ceil(w/16)*2` bytes starting at offset 8, where
    `w = right - left`, `h = bottom - top`. A set bit is white.
  - High resolution: first word `FFFF`, then `u16` x dpi, y dpi, width, height, a word that must
    be 1, then PackBits-compressed rows. Bytes per row are inferred from the decompressed
    length (between `ceil(w/8)` and that plus 3).
- Checked: all 17 files in dexvert `image/pfsFirstPublisher` are standard resolution and each
  file size equals `8 + rowspan * h` exactly, so size equality is a strong validator even though
  there is no magic. Renders show a man with a phone, an apple, a tall ship, a clock.
- Samples: dexvert (17, downloaded); the textfiles CD `swinnund/disk3/CLIPART/` has `FPART1.EXE`,
  `ART_FPUB.EXE`, `1STPUB3.EXE`, `ARTMART*.EXE` self-extracting archives
  (<http://cd.textfiles.com/swinnund/disk3/CLIPART/>, 129 entries).
- Difficulty: S, plus the existing `codec::packbits` for high resolution. Relevance: medium.
  RECOIL: no. Validate with Deark's PNG output.

### C7. DOS and Apple clip-art libraries: Print Shop, New Print Shop, PrintMaster, PrintPartner

All four store many small 1-bit pictures per file, with no real signature except where noted.
The decision is how to present a library as one `Image` (first picture, or a contact sheet),
the same question as for C5 and C1's CNF.

| Format | Files | Layout |
|---|---|---|
| The Print Shop (DOS) | `GR*.DAT` + `.NAM` | no header; 88x52 pictures, 11 bytes per row, 572 bytes each; `.NAM` holds 16-byte names |
| The New Print Shop | `.POG` + `.PNM` | 10-byte header with the picture count as a `u16` at 8; then 572-byte pictures; `.PNM` has 16-byte names |
| PrintMaster | `.SHP` + `.SDR` | repeated `0B h w ?` header (4 bytes), `ceil(w/8)*h` bytes, 1 trailing byte; first image starts `0B 34 58` |
| PrintPartner | `.GPH` | text `PrintPartner ...` up to a `1A`, then pictures: name length, 20-byte name, compression 1/2/3, height, width in bytes, data |
| Print Shop GS | ProDOS `$F8` | 88x52, three bit planes of 572 bytes, 1716 bytes, 8 colours |

- Sources: Deark `modules/printshop.c` and `printptnr.c` (MIT) [fetched]; Just Solve "The Print
  Shop" and "PrintMaster" <http://fileformats.archiveteam.org/wiki/The_Print_Shop>
  [fetched]; CiderPress II `FileConv/Gfx/PrintShopClip.cs` (Apache-2.0) for the Apple II and
  GS variants [fetched]. PrintPartner's own RLE notes: "reverse engineering and guesswork" on
  Just Solve.
- GPH compression: type 1 raw; type 2 is a byte RLE (bit 7 set = repeat the next byte,
  count in the low 7 bits; clear = that many literal bytes) with a `u16` compressed length;
  type 3 is a nibble RLE (high bit = black run, low 3 bits = length 1 to 7) with a nibble count.
  Pixel aspect is 2:1. Deark says what a run length of 0 does is unknown.
- Print Shop GS colours (CiderPress II, from an emulator screen grab): white, blue, red,
  purple (`CC00CC`), yellow, green, orange (`FF6600`), black for plane values 0 to 7.
- Checked: dexvert `image/printMasterShape/STANDARD.SHP` is 70394 bytes and parses into exactly
  122 pictures of 88x52, consuming the whole file; they render as a clip-art sheet.
  `CAT1.POG` is `10 + 200 * 572 = 114410` bytes with count 200 at offset 8, and `SPACE.POG` has 36;
  `CAT1.PNM` is `200 * 16 = 3200` bytes. `archive/printShopDAT/grcps.dat` is 26368 bytes, which is
  not a multiple of 572 (46 pictures plus 56 bytes), so Deark's "size is a multiple of 572"
  identification does not cover every DAT file.
- Samples: dexvert `image/printMasterShape` (12), `image/printShopGSGraphic` (11),
  `archive/pog` (10 files), `archive/printShopDAT` (1), `image/printMagicGraphic` (9);
  many more in the textfiles CD folders `swinnund/disk3/CLIPART/` (e.g. `NPS1.EXE` to `NPS5.EXE`,
  `BOBPMGR2.EXE`...) and `powerpakgold/GRAPHV_E/PM_PS_BS.ZIP`. One SHP and four POG/PNM/DAT files
  were downloaded; the rest are listings.
- C64 (`commodore/printshop.rs`, `printmaster.rs`) and Atari 8-bit (PSF) versions already exist,
  so the 88x52 bitmap helpers are there.
- Difficulty: S each. Sig: SHP (`0B 34 58`) and GPH (`PrintPartner`) yes; POG by `10 + n*572`
  with `n` at 8; DAT by `size % 572` plus a `GR*.DAT` name, which is weak. Relevance: medium
  (a big DOS shareware pile). RECOIL: no for these (it has Atari 8-bit Print Shop only).
  Validate with Deark.

### C8. Self-displaying DOS picture executables

DOS `.COM` and `.EXE` programs made by picture-to-executable tools. The picture is a payload
behind a fixed stub, so a fingerprint plus a payload offset is enough. Deark documents several
[fetched, MIT].

| Tool | Stub | Payload |
|---|---|---|
| PIXIT / pix320 | COM starting `BC 00 01 B8 13 00 CD 10` (set mode 13h) | `PX` header at `u16 at byte 18` minus 272: `"PX"`, `u16` width, `u16` height, 16-byte header, 768-byte 6-bit VGA palette, then 8-bit pixels. A bare `.PIX` file is `16+768+64000` bytes and starts `50 58 40 01 C8 00`. |
| GIFEXE | MZ stub starting `4D 5A EE 00 21 00` | a plain GIF87a at 16622 |
| PCX2EXE variants | different MZ stubs | an embedded PCX at a stub-dependent offset (7218 and 43190 in the two samples) |
| OPTIKS COM | starts `E9 39 01 0D 0A "OPT"` | PackBits mono rows after the stub (offset 5680 or 5744 by version) |
| Graphic Workshop EXEPIC | MZ | a GIF-like payload; layout not read |

- Checked: `pixit/GENESIS.COM` is 64856 bytes, the `u16` at byte 18 gives 344, the `PX` header is
  at 72 and reads 320x200. I downloaded four `gifexe` samples: all four have the stub bytes
  `4D 5A EE 00 21 00` and `GIF87a` at 16622 (the other six were not opened). Each of the two
  PCX2EXE samples has a byte pattern like a PCX header (`0A`, version, `01`, bits per pixel) at
  the offsets above; I did not decode them.
- Sources: Deark `modules/misc2.c` (PIXIT, OPTIKS) and `gws.c` (Graphic Workshop) [fetched].
- Samples: dexvert `image/pixit` (12), `gifexe` (10), `pcx2com` (2), `pcx2exeArminioGrgic`
  (1), `pcx2exeFDelPozo` (1), `graphicWorkshopSelfDisplayingImage` (14), `grabber` (19).
  Text-mode COM pictures (TheDraw, ACiDDraw, Laughing Dog) belong to the text-mode notes.
- Difficulty: S per tool, M together. Sig: stub fingerprints (strong). Do not scan for any `GIF8` or
  PCX header anywhere in an EXE; false hits are likely. Relevance: medium-low (disk magazines,
  BBS packs). RECOIL: no. Validate against the standalone picture the tool started from, or Deark.

### C9. BSAVE dumps and raw video-mode screens

- GW-BASIC/BASICA `BSAVE` writes a 7-byte header: `FD`, `u16` load segment at 1, `u16` offset at 3,
  `u16` data length at 5 (a few files have a bad length). Optional `1A` at the end. The load
  segment `B800` marks CGA memory. [fetched, Deark `modules/bsave.c`, MIT].
  (MSX BSAVE uses `FE`, already handled.)
- Layouts named in Deark's code, each with the CD that supplied the examples in its comments:
  CGA 320x200 4-colour and 640x200 2-colour (even scan lines in the first 8000 bytes, odd lines
  in the second 8000, 80 bytes per line); the CGA 160x100 16-colour text-mode trick; MCGA 13h
  320x200x256 (64000 bytes); a 16-colour 4-plane format with an 11-byte header holding width
  and height; and a few program-specific variants. The CGA even/odd interleave is also in the
  CGA article <https://en.wikipedia.org/wiki/Color_Graphics_Adapter> [fetched].
- Related headerless dumps: dexvert's `image/vzi` folder holds 11 files that are all exactly
  16000 bytes (`loadscrn.cga`, `cgamlogo.scn`, `menu.dat`, ...), which looks like 320x200 at 2 bits
  per pixel without the CGA interleave. I did not check the layout [snippet].
- Deark itself treats this as heuristic ("TODO: better autodetection"), and PCPaint writes a
  compressed BSAVE variant. A decoder would register by extension (`.scr .pic .bsv .raw`) and use
  the `FD` header plus length as the only check; headerless raw dumps (16384, 32000, 64000
  bytes) have no safe signature, as in `adding-a-format.md`.
- Samples: dexvert `image/bsave` (20) and `image/bsaveCompressed` (3); Deark's comments name
  textfiles CD folders (`bthevhell/100/21`, `bthevhell/200/111`, `advheaven2/PUZZLES/DRCODE12`).
- Palettes are the problem: CGA palette 1 high intensity and palette 0 are guesses, and the
  Tandy/PCjr and EGA palettes are fixed. Record the choice, as for Atari and C64.
- Difficulty: M. Relevance: medium. RECOIL: no. Validate with Deark's `bsave` module.

### C10. TI-99/4A TI-Artist

- TI-Artist saves a picture as two files named `NAME_P` (pattern table) and `NAME_C` (colour
  table), each 6144 bytes: a dump of the TMS9918A Graphics II (bitmap mode) VRAM tables
  [fetched, Ninerpedia, CC BY-NC-SA, facts only,
  <https://www.ninerpedia.org/wiki/Graphic_file_formats>]. Same pixel model as MSX Screen 2
  (6144 + 6144 + the 768-byte name table, which is a fixed 0..255 x 3 in bitmap mode). The
  TMS9918A manual is the hardware source:
  <https://map.grauw.nl/resources/video/texasinstruments_tms9918.pdf> [reachable] and Thierry
  Nouspikel's page <http://www.unige.ch/medecine/nouspikel/ti99/tms9918a.htm> [fetched].
- Disk files may carry a 128-byte TIFILES header (`07 'TIFILES'`) [memory], which would make the
  size 6272 bytes.
- Companions: `_P` and `_C` do not share an extension, so the existing `Companions::get(ext)` does
  not fit; the stem plus a suffix is needed, or `get_named`.
- Other TI formats on the same page: MyArt and YAPP for the Geneve (V9938 modes with RLE) and
  FRACTALS!. Too niche; see the rejections.
- Samples: no TI-Artist file was found. The TOSEC TI-99/4A collection on archive.org
  (<https://archive.org/details/Texas_Instruments_TI-99_4a_TOSEC_2012_04_23>, 37,556 downloads) is
  a set of disk images, which need a TI disk reader; I did not open it. dexvert has no TI
  directory at all. Samples are therefore the open item.
- Difficulty: S. Relevance: low to medium. RECOIL: no. The same decode path is already tested via
  MSX Screen 2. Validate against a V9T9 or Classic99 screenshot of the same picture.

### C11. Inset PIX

- INSET (American Programmers Guild), later HiJaak, and the WordStar, MultiMate and PC-Write
  word processors used `.PIX` for raster or character graphics. At least two incompatible
  versions exist; version 3 is the common one [fetched, Just Solve "Inset PIX",
  <http://fileformats.archiveteam.org/wiki/Inset_PIX>; EGFF summary
  <https://www.fileformat.info/format/inset/egff.htm>: up to 16 colours, little-endian,
  "proprietary, documented"].
- The EGFF spec link Just Solve gives returns 404, so the layout was not read. Deark has an
  `insetpix` module (marked experimental in `formats.txt`) that I did not open.
- Samples: textfiles `psl/psl9309/DOS/PCWRITE/PIX/` (34 entries, about 29 `.PIX`,
  <http://cd.textfiles.com/psl/psl9309/DOS/PCWRITE/PIX/>); dexvert `image/insetPix` (11).
- Difficulty: M with an unread spec. Relevance: low to medium. RECOIL: no. Validate with Deark.

### C12. Small PC batch

| Format | Layout | Source | Samples |
|---|---|---|---|
| DCX | `B1 68 DE 3A`, a list of `u32` offsets, then PCX files; return the first page [memory] | Deark lists a `dcx` module [snippet]; the PCX decoder exists | dexvert `dcx` 10 |
| HP 100LX/200LX `.ICN` | `01 00 01 00`, `u16` width at 4, `u16` height at 6, 1-bit rows from 8 | Deark `hpicn` [fetched, checked] | dexvert `hpPalmtopIcon` 10; the 3 downloaded are 44x32 and 200 bytes = `8 + 6*32` |
| IBM KIPS | `"DFIMAG00"`, `u16` height, `u16` width, 32-byte header, 8-bit pixels; palette in `.PAL` (no signature) or `.KPL` (signature) | Just Solve "IBM KIPS bitmap" [fetched]; sizes [checked] | dexvert `ibmKIPS` 10; 64032 = `32 + 320*200`, 307232 = `32 + 640*480` |
| DGI (Digi-Pic) | exactly 64008 bytes, 640x400 4-colour CGA as four interlaced 320x200 quadrants; magic `01 04 00 00 00 00 00 00` at 32000 | Deark `dgi` [fetched] | dexvert `dgi` 6 |
| Lumena CEL | `u16` width, `u16` height, then 16-bit RGB555 or 32-bit RGBA rows, bottom-up; bits per pixel from the file size | Deark `lumena_cel` [fetched] | listing only |
| OS/2 bitmap array, icon, pointer | `BA`, `CI`, `CP`, `IC`, `PT` record markers around BITMAPCOREHEADER-style bitmaps [memory] | Deark lists `os2bmp` [snippet]; our `bmp.rs` has no `BA`/`CI`/`CP` handling | dexvert `icoOS2`, `os2Pointer` |
| Windows 1.x ICO/CUR | word `0001`/`0101`/`0201`, then a 12-byte header; monochrome | Just Solve "Windows 1.0 Icon" [fetched] | Deark `win1ico` |

IBM KIPS is the one I would think twice about: it is obscure. The rest are small.

### C13. IBM Storyboard PIC and CAP

- PC Storyboard (IBM, 1984) pictures: CGA, EGA and VGA with two compression schemes, one of which
  has "long" codes. Deark's `modules/storyboard.c` (MIT) says the old `EP_CAP` format is
  supported and some newer formats "partially"; it notes it cannot work out the image bit to
  palette index mapping, how dimensions are stored in captured files, or whether a file can hold
  several images [fetched].
- Samples: dexvert `image/ibmStoryboardPic` (28, plus 23 `.TXM` text files); listing only.
- Difficulty: M and partly unknown. Relevance: low. RECOIL: no. Validate with Deark.

### C14. Apple II set

CiderPress II (Apache-2.0 code, CC BY-SA docs) converts these [fetched,
<https://ciderpress2.com/features.html>]; the converters live in `FileConv/Gfx/` of
<https://github.com/fadden/CiderPress2>.

| Item | Identification | Notes |
|---|---|---|
| Print Shop clip art, Apple II | ProDOS `B`, aux `$4800`, `$5800`, `$6800` or `$7800`, 572 or 576 bytes | 88x52 mono, same as DOS Print Shop |
| Print Shop GS | ProDOS `$F8`, aux `$C313` (mono) or `$C323` (colour), 572 or 1716 bytes | three planes, 8 colours |
| IIgs Finder icon file | ProDOS `ICN` (`$CA`) | each entry has large and small images with masks |
| LZ4FH hi-res | ProDOS `FOT` `$8066`; first byte `0x66` (`f`) | an LZ4 variant; the only item with a magic byte |
| Paintworks animation | ProDOS `ANI` `$0000` | first frame as a still |
| Apple II lo-res / double lo-res | no file type defined; raw 1024-byte text-page dump (40x48) or two pages | memory layout from Just Solve <http://fileformats.archiveteam.org/wiki/Apple_II_graphics_formats> [fetched]; CiderPress II has no lo-res converter |
| Apple II icon archive (`.ICONS`) | dexvert `apple2Icons` | Deark module is experimental |

Our API has no ProDOS file type, so most of these would need an extension convention
(`.shr` and `.hgr` already work that way) or the type placed in the AppleSingle/2IMG-style
wrapper that we do not parse. Hence the low rank. Samples: dexvert `image/printShopGSGraphic` (11),
`image/apple2Icons` (12); lo-res has no sample set. RECOIL: no. Validate with CiderPress II's
PNG output.

### C15. Psion Series 5 (EPOC) MBM, Sketch, AIF

- Series 3 `PIC`/`ICN` is already registered. The Series 5 / netBook / Symbian 6 era uses a
  different multi-bitmap file. Signature: UID words `37 00 00 10`, then `42 00 00 10`
  (MBM) or `8A 00 00 10` (exported MBM) [snippet, from `gaps-others.md` 2.9]. Sketch files start
  `37 00 00 10 6D 00 00 10 7D 00 00 10 9C F9 08 55` [checked: the 3 dexvert `epocSketch` files I
  downloaded have identical headers].
- Deark's `modules/epocimage.c` (MIT) handles MBM, Sketch and AIF, most compression types, a
  256-colour palette it describes as "web safe" plus 40 extra greys and RGB shades, and masks
  [fetched, header only].
- Samples: dexvert `image/epocSketch` (10); listing only.
- Difficulty: M (several bit depths with RLE variants, a palette to confirm). Relevance: low.
  RECOIL: no. Validate with Deark.

### C16. NEC PC-98 raw plane dumps and VRAM-style formats (blocked)

PictureFan lists several Japanese formats with no layout beyond a one-line description
[snippet, <https://iooiau.net/picturefan/help/formats.html>]:

| Format | What it is | Known |
|---|---|---|
| `B1`, `R1`, `G1`, `E1` | one 1-bit plane (blue, red, green, intensity) of a 640x400 PC-98 screen, written by Z's STAFF Kid98 and others; N88-BASIC and Quick-BASIC header variants; optional `.ALG` or `.RGB` palette next to it | `.RGB` is 16 colours x 3 bytes, low nibble used (48 bytes) per Data Crystal, via WebFetch summary [snippet]; the `.ALG` layout was not found |
| `BLK` | 4-bit 640x400 VRAM dump from Alice Soft games | name and size only |
| `FRM` | 4-bit 640x400 dump, with a `.PAL` file next to it | name only |
| `GRP`, `I_G`, `IUX` | 1 or 3 bit pictures from Japanese word processors (Kanri Kogaku, Assist, Panasonic U1PRO) | name only |
| `Q0` / `QLD` | 24-bit and 8-colour formats from the NIFTY-Serve and PC-VAN scene; `Q0` has `.FAL`/`.IPR` info files | name only |
| `MISA` | animation format | name only |
| TownsPAINT `P16`/`P25`/`P32` | FM Towns paint program, 4, 8 and 16 bit | name only |
| FM Towns `KYG` | header `KYGformat ver`; a C parser exists in a gist (<https://gist.github.com/Sembiance/2ee395b8d3ebf5e6bfce12918c6aceb4>) with no stated licence, so it is on the "to avoid" list | 25 dexvert samples |

`EBD` and `pc88_planes.rs` already work on the plane model, so B1/R1/G1/E1 would be small if a
sample set showed up. No sample set was found for any of these except KYG. Treat all of this as
blocked on spec or samples.

### C17. WordPerfect Graphics (WPG)

DOS clip-art libraries (WordPerfect, Corel). A record stream with vector shapes and bitmap
records; a bitmap-only decode would cover photo-like clip art but not line art. EGFF has a
summary <https://www.fileformat.info/format/wpg/egff.htm> [snippet]; Deark's `wpg.c` extracts the
bitmaps (MIT, not read). Samples: dexvert `image/wpg` (19). M. Not ranked: mostly vector.

### C18. Palm OS bitmaps and TealPaint

Palm BitmapType (1/2/4/8/16-bit, with scanline, RLE and PackBits compression), Palm Database
ImageViewer and TealPaint `.pdb`. Deark has `palmbitmap.c` and `palmpdb.c` (MIT, not read). The
Just Solve "Palm" category lists TealPaint, GrayPaint, Diddle and DiddleBug sketches
[fetched]. Samples: dexvert `image/palmBitmap` (10) and `palmDatabase`. A PDA, not a home
computer; M. Not ranked.

### C19. Printer streams: Epson ESC/P bit images

ESC/P bit-image commands (`ESC K/L/Y/Z`, `ESC *`, ESC/P2 raster) can be decoded
deterministically. Epson's ESC/P reference manual is the primary spec
(<https://files.support.epson.com/pdf/general/escp2ref.pdf> [snippet, link taken from Just Solve
"Epson Printer Bitmaps"]). A decoder would have to pick a head model, density and page width
because these were screen-dump captures from C64, Atari, Apple and PC. No capture file was
found, so there is nothing to test against. Low; do not start without a sample.

### C20. Slow-scan TV `.HRZ`

A raw RGB bitmap, 256x240, 184320 bytes, no header [fetched, Just Solve "Slow-scan television"].
XS if wanted, but origin and use are unclear and it has no retro-computer connection.

## 5. Candidates rejected, and why

| Candidate | Reason |
|---|---|
| Geneve MyArt, YAPP, FRACTALS! (TI Geneve 9640) | Ninerpedia describes them (V9938 modes with RLE), but the machine is a rarity and no samples were found. |
| ThunderScan, NeXT 2-bit RLE | TIFF compression codes, not standalone files. |
| RISC OS Clear (`&690`, Translator) | Spec lives in the Translator program's docs (`translator_830.bin`, an unopened RISC OS archive) and in SPRtools `clear.h` (licence unchecked, <https://armclub.org.uk/free/>); the Acorn scene uses Sprites. |
| Lotus PIC, AutoCAD SLD/SLB, GEM Draw, Harvard Graphics | Vector or metafile; not raster pictures. |
| Image Alchemy, KONTRON, Zeiss BIVAS, Intergraph, CALS, Interleaf | Scientific, engineering or document-system formats. |
| GEM IMG from PC Ventura/GEM Paint | Same file as the Atari ST `GEM Bit Image` we already decode; only the platform label differs. |
| Sierra AGI/SCI pictures, game resource formats | A vector command stream inside game archives: a game-extractor job. |
| Nokia logos (NOL, NGG, NSL, NLM, NPM) | Phones, not computers. Deark handles them. Out of this area. |
| Windows/OS/2 "boot logo" and thumbnail caches | OS artefacts, not retro pictures. |
| Nitrogen Fingers Paint, MegaZeux MZM | Not tied to retro hardware. |
| Commodore PET, Kaypro, Osborne | Text-mode machines; see section 6. |
| KYG, PIC2, Q0/QLD, TK4, Towns Paint II | Specs missing, licence-blocked or arithmetic-coded (PIC2 uses arithmetic coding per the XV Japanese extension page, <http://www.ikemo.to/xv-jp-extension/e/pic2.html> [fetched]). |
| MSX Screen 1 and TMS9918 dumps for Sord M5, Memotech MTX, Coleco ADAM, Spectravideo | They use the same Graphics I/II layouts that MSX SC2/SC3 already decode, and no platform-specific file convention was found. |
| CoCo/Dragon PMODE 0/2/3 screens, DragonDOS `55 02` header | A variant or header gap, not a format. `trs80.rs` reads RS-DOS binaries (5-byte preamble) only. I did not look for samples. |
| Geneve and other tape/disk wrappers (TI DSK, X1/PC-98 D88, Apple DSK/WOZ) | A container layer, not an image format. Real pictures live inside; reading them needs a file system reader per platform. |

## 6. Systems with no image formats worth supporting

Evidence is negative: a search of the Just Solve wiki (`index.php?search=` on 2026-10-05) and the
Deark, dexvert and PictureFan catalogues turned up no picture-format page or entry. Where a
system's pictures circulate as MAG, Pi or MAKI, those are decoded already.

| System | Finding |
|---|---|
| Sharp X1 / X1turbo, MZ-700/800/1500/2500 | Just Solve has no page. Pictures circulate as MAG and Pi (machine code in the header). Tape/disk containers only. |
| Fujitsu FM-7/77/77AV, NEC PC-6001/6601, PC-8001 | No page or catalogue entry; MAG/Pi cover the scene. |
| X68000, FM Towns beyond the listed ones | Covered by PIC, MAG, Pi, ICN, HEL; the rest are in C16. |
| Commodore PET/CBM | PETSCII screens only (C64 group's work); Just Solve lists the character set and disks. |
| Dragon 32/64 | PMODE screens saved as binaries (the CoCo decoders cover RS-DOS ones); no Dragon paint format found. |
| Memotech MTX, Sord M5, Spectravideo SVI, Coleco ADAM, Tatung Einstein | No picture formats; TMS9918 layouts shared with MSX. |
| Jupiter Ace, Camputers Lynx, Mattel Aquarius, Exidy Sorcerer, Ohio Scientific, Acorn Atom, Microbee, Compucolor II | Just Solve has BASIC and character set pages only (Compucolor, Aquarius, Exidy, OSI) or none. |
| Hector, Exelvision, Philips P2000/VG5000, Thomson beyond what exists | Hector: no Just Solve result. Philips: only unrelated results. Exelvision, P2000 and VG5000 were not searched on their own. Thomson is covered (`thomson.md`). |
| Enterprise 64/128 | Programmable display list; no file format (see `gaps-others.md` 2.7). |
| Kaypro, Osborne, Victor 9000, Sirius, Apricot | CP/M or MS-DOS text machines; Just Solve has disk formats only. The Epson QX-10 was not searched on its own. |
| Apple Lisa, Xerox Star, DEC Rainbow | Lisa: only MacDraw and disks; Xerox: only the Doodle brush. DEC Rainbow was not searched; terminals in general are C3. |
| Russian/Soviet: Specialist, Orion-128, Korvet, Radio-86RK, Agat | No catalogue entry (Just Solve search found nothing for these five; Pravetz and DVK/UKNC were not searched). BK, MC 0515 and Vector-06C are registered. Raw VRAM dumps have no known file convention. [snippet] |
| Czech, Polish, Hungarian, East German: PMD 85, Primo, KC 85, Robotron | No Just Solve result for these four (Poly-880 was not searched). [snippet] |
| Korean and Chinese clones | Most were MSX, Apple II or PC clones [memory]; nothing separate found in the catalogues. |
| Acorn Electron, BBC Master | Same modes as the BBC; BB0-BB5 and Mode 7 are registered. Mode 3 and 6 are text. |
| Amstrad CPC, PCW, CPC+ | CPC covered; PCW is C2. |

The Eastern European rows have the weakest evidence. They rest on absence from four catalogues
and on my not finding anything; a search in Russian-language retro forums could change that.

## 7. How to validate formats RECOIL does not have

| Format | Independent check |
|---|---|
| KiSS | XnView or Konvertor with the KCF beside the CEL (by eye). |
| PCW | `SPC2BMP` (John Elliott); by eye against screenshots in MicroDesign manuals. |
| Sixel | `img2sixel`/`sixel2png` (libsixel, MIT) or ImageMagick, black box. |
| Sun, SGI, XBM, XPM, PNM | Pillow, ImageMagick or netpbm, black box. |
| Print Shop, PrintMaster, PrintPartner, ART, Storyboard, PIXIT, GL, BSAVE, HP ICN, EPOC, DGI, Lumena | Deark's PNG output (MIT). It is not installed here, but it builds from source with a C compiler and was already used as an oracle for BMP and PCX (see the headers of `pc/bmp.rs` and `pc/pcx.rs`). |
| TI-Artist | An emulator screenshot (Classic99 or V9T9) of the same picture. |

Per `adding-a-format.md`, record each reviewed sample in `crates/retro-image/tests/divergences/`
with the evidence, as for the text-mode formats.

## 8. Extension collisions to plan for

| Extension | Existing claim | New claim |
|---|---|---|
| `.CEL` | Cyber Paint Cell (Atari ST), Animator CEL (PC) | KiSS CEL (signature `KiSS` for the main kind; old headerless kind collides) |
| `.CUT` | Dr. Halo CUT | Amstrad PCW CUT (size equation) |
| `.GRF` | CoCo `GRF`, ZX Profi `GRF` | Amstrad PCW GRF |
| `.SPC` | Atari ST Spectrum 512 compressed | Amstrad PCW Stop Press (23040 bytes) |
| `.ART` | several Atari ST/8-bit ART formats | PFS First Publisher (exact-size check) |
| `.PIX` | CoCo `PIX`, Atari Falcon PIX | Inset PIX, PIXIT raw `.PIX` |
| `.SHP`, `.DAT`, `.POG` | Loadstar SHP, Atari raw DAT | PrintMaster, Print Shop |
| `.PIC` | PCPaint, Animator, Dr. Halo, QL, FM Towns | IBM Storyboard PIC |
| `.ICN` | DEGAS Elite icon, FM Towns, Psion | HP LX ICN, Print Shop icons |

Content detection (`.signature()`) is available for KiSS main kinds, MDA/MDP, Sixel, SGI, Sun,
PrintMaster, PrintPartner, GL (index check), EPOC and BSAVE-with-segment-and-length. Do not mark
size-only or headerless formats.

## 9. Samples and provenance

- dexvert's tree <https://sembiance.com/fileFormatSamples/> is the broadest index
  (857 image format directories). It holds other people's files with unclear permissions, so
  the usual caveat applies: samples are copyrighted by their artists and stay in the
  git-ignored `corpus/`, with a `MANIFEST.tsv` per group.
- Downloaded and parsed for this survey (scratch folder, not the repo): 22 KiSS files, 5 MicroDesign
  files, 1 PrintMaster SHP, 4 POG/PNM/DAT files, 17 FP ART files, 1 GIFEXE, 1 PIXIT, 1 GL file,
  headers of 3 HP ICN, 3 KIPS, 3 EPOC Sketch and 3 Palm bitmap files.
- Self-extracting clip-art archives on textfiles CDs (`cd.textfiles.com/swinnund/disk3/CLIPART/`,
  `powerpakgold/GRAPHV_E/`, `psl/psl9309/DOS/PCWRITE/PIX/`) are live today; Wayback has a copy of
  the first one.
- A sample pass for C10 and C16 was not possible without search. If one is wanted: TOSEC
  TI-99/4A disk images need a TI disk reader, and PC-98 D88/HDI disk images need a FAT reader.

## 10. Decisions needed

1. Scope of generic Unix rasters (C4). The registry already has GIF, BMP, ICO and Targa, so I
   assumed yes.
2. How to present multi-picture libraries (Print Shop, PrintMaster, GL, a KiSS set): first
   picture, or a contact sheet with a fixed grid. One policy for all of them.
3. Platform labels outside RECOIL's list: "KiSS", "Amstrad PCW", "TI-99/4A", "DEC VT340",
   "Unix".
4. Whether Deark may be built and used as a black-box oracle on this machine (it is MIT).
   Several candidates have no other oracle.
5. KYG: the only spec is C code in a gist with no stated licence. Treat as not readable unless
   the maintainer decides otherwise.

## 11. Permissive references and what to avoid

Safe to read, with licences:

| Source | Licence | Used for |
|---|---|---|
| Deark `modules/*.c` (<https://github.com/jsummers/deark>) | MIT (`COPYING` read; the `foreign/` folder has other licences and was not touched) | Print Shop, PrintPartner, PFS ART, PIXIT, GL, BSAVE, Storyboard, HP ICN, DGI, Lumena, Sun/SGI/XWD, EPOC |
| libsixel (<https://github.com/saitoha/libsixel>) | MIT (`LICENSE` read) | Sixel samples and a tool oracle |
| CiderPress II (<https://github.com/fadden/CiderPress2>) | Apache-2.0 code, CC BY-SA 4.0 docs | Apple II converters |
| Just Solve the File Format Problem | CC0 | catalogue and prose |
| KISS/GS spec translation | declared public domain | KiSS |
| John Elliott's MicroDesign page | Creative Technology's 1992 sheet, no licence stated | PCW facts only |
| DEC VT330/VT340 Programmer Reference | DEC, no licence | Sixel facts only |
| Ninerpedia | CC BY-NC-SA | TI facts only; no text copied |
| Pillow, ImageMagick | HPND, Apache-style | oracle |

To avoid (do not read the code):

| Project | Reason |
|---|---|
| dexvert (<https://github.com/Sembiance/dexvert>) | GitHub shows no licence (NOASSERTION). Only its published sample folders were used. |
| The KYG parser gist by Sembiance | no licence stated |
| abydos (snisurset.net), wuimg (codeberg kaleido), XV and its Japanese extension (<https://github.com/jasper-software/xv>), GBM | licences not checked; XV's own licence is not OSI |
| netpbm source | mixed GPL and permissive parts; man pages only |
| SPRtools `clear.h` | licence not checked |
| GrafX2, TRSE and the others listed in `CLEANROOM.md` | unchanged |
| MAME, Fuse and other emulators | licences vary per file; not opened |

Scratch work is in `/tmp/claude-1000/.../scratchpad/` only; nothing was written to `corpus/` or
the repo apart from this file.
