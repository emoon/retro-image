# Format documentation survey: Amiga, Apple, PC and miscellaneous platforms

Clean-room research notes. This file lists only public format descriptions, hardware manuals and
permissively licensed references. No RECOIL code and no other GPL/LGPL decoder code was read.
The format list comes from the plain fact list at <https://recoil.sourceforge.net/formats.html>.

## Summary

40 format rows across 14 platform groups. FLF is counted twice, once under Amiga and once under PC.

| Docs quality  | Count | Formats |
|---------------|------:|---------|
| Spec          | 23 | ABK, ACBM, DEEP, HAM/HAM6, HAM8, IFF/256, INFO, LBM/ILBM, RGB8, RGBN, HGR, DHGR, 3201, APF (GS/IIGS/PNT/SHR), Paintworks PNT, SH3/3200, SHR (32K dump), MacPaint, MSP, TIM, Psion PIC/ICN, GROB, CompuServe RLE |
| Partial       | 5  | SHAM, HAM-E, EPA, HS2, DeskMate PNT |
| Hardware-only | 4  | TRS-80 HR, CoCo CLP, CoCo GRF/MAX/P41/PIX, CoCo P11 |
| None          | 8  | FLF (Amiga), DHR/DR/MP/BEAM, DCTV, Apple II SPR, IIGS 32K, FLF (PC), Image 72 FNT, MagicDraw SHR |

The best umbrella sources:

- **AmigaOS Documentation Wiki**, which holds the IFF specs: [EA IFF 85](https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files), [ILBM](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap) (BMHD/CMAP/CAMG/BODY, ByteRun1, HAM/EHB, PCHG, CTBL), [ACBM](https://wiki.amigaos.net/wiki/ACBM_IFF_Amiga_Continuous_Bitmap), [DEEP](https://wiki.amigaos.net/wiki/DEEP_IFF_Chunky_Pixel_Image), [RGBN/RGB8](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data) and the [FORM/Chunk Registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry).
- **CiderPress II format docs** (docs are CC BY-SA 4.0; the CiderPress II code is Apache-2.0). They cover [Hi-Res](https://ciderpress2.com/formatdoc/HiRes-notes.html), [Double Hi-Res](https://ciderpress2.com/formatdoc/DoubleHiRes-notes.html), [Super Hi-Res](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html) ($C1/0000, $C0/0000 Paintworks, $C0/0001, $C0/0002 APF, $C1/0002 Brooks, .3201, DreamGrafix) and [MacPaint](https://ciderpress2.com/formatdoc/MacPaint-notes.html). The full list is in the [doc index](https://ciderpress2.com/doc-index.html).
- **Apple II File Type Notes** (gno.org mirror), e.g. [ftn.c0.0000](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0000), [ftn.c0.0002](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0002), [ftn.c1.0000](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0000) and [ftn.c1.0002](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0002). The same directory also has c0.0001, c0.0003, c1.0001, 08.0000, 08.4000 and 08.4001.
- **Deark** by Jason Summers ([repo](https://github.com/jsummers/deark), MIT-style license from 1.4.x onward). This is a permissive reference implementation for many formats in this file: abk, amigaicon, ilbm (ILBM/ACBM/RGBN/RGB8/HAM6/HAM8), deep, awbm (EPA), cserve_rle, deskmate_pnt, grob, hr, hs2, macpaint, msp, psionpic and tim (experimental). The module list is in `formats.txt` in the repo. You may read this code, but note its license in anything derived from it.
- **Just Solve the File Format Problem** (fileformats.archiveteam.org / justsolve.archiveteam.org) has pages for nearly every obscure format here. The site refused HTTPS connections during this research, so those pages are cited from search results and were not fetched.

**Open gap: the Apple IIGS PackBytes encoding.** APF, Paintworks, $C0/0001, .3201 and $08/4000-4001 all depend on PackBytes. [IIgs TN #94](https://apple2.gs/technotes/tn/iigs/TN.IIGS.094.txt) defers the byte-level format to *Apple IIGS Toolbox Reference* vol. 1, p. 14-39, and that page was not found online during this research. TN #94 confirms two details: flag `$3F` means "64 literal bytes follow", and flag `$FF` means "64 repeats of the next 4 bytes". The next step is to locate a scan of the Toolbox Reference on archive.org.

---

## Amiga

### Hardware references
- Amiga Hardware Reference Manual (1985), [PDF on archive.org](https://ia801902.us.archive.org/4/items/Amiga_Hardware_Reference_Manual_1985_Commodore/Amiga_Hardware_Reference_Manual_1985_Commodore.pdf). Covers bitplanes, HAM6, EHB and the copper.
- HAM: [Glossary: hold-and-modify (ADCD)](http://amigadev.elowar.com/read/ADCD_2.1/Hardware_Manual_guide/node01F6.html) and [How Amiga HAM mode works (ADCD)](https://amigadev.elowar.com/read/ADCD_2.1/Devices_Manual_guide/node02DC.html). The elowar host had an expired TLS certificate during this research.
- HAM8/AGA: [AGA chipset notes (howtocode)](https://jvaltane.kapsi.fi/amiga/howtocode/aga.html) and [Wikipedia: Hold-And-Modify](https://en.wikipedia.org/wiki/Hold-And-Modify). In HAM8 the low 2 bits are control (00 = palette index 0..63, 01 = blue, 10 = red, 11 = green) and the upper 6 bits are data that replace the top 6 bits of the component.
- The ILBM spec gives the HAM detection rule: CAMG bit 0x800, or 6 planes with no CAMG chunk.
- RKM Libraries ch. 14, Workbench and Icon Library: [theflatnet mirror](http://www.theflatnet.de/pub/cbm/amiga/AmigaDevDocs/lib_14.html). Also the [Icon Library wiki page](https://wiki.amigaos.net/wiki/Icon_Library), which states the classic icon rules: depth 2, PlanePick 3.

### Palette
- ILBM: from the CMAP chunk, 8 bits per component. OCS files often hold 4-bit values in the high nibble. The ILBM spec covers scaling, and Deark has an option to correct such palettes.
- HAM6: 12-bit (4096) colours. HAM8: 18 bits effective, out of a 24-bit space.
- RGBN is 12-bit direct colour and RGB8 is 24-bit direct colour.
- Classic icons have no palette in the file. They use the Workbench pens: 4 colours that differ between OS 1.x and OS 2.x+. Rendering is ambiguous, as Deark notes ("not just one correct way").

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| ABK | AMOS banks: AmSp sprites, AmIc icons, AmBk "Pac.Pic." pictures | **Spec** | [alvyn: AMOS file formats](http://alvyn.sourceforge.net/amos_file_formats.html) (sprite/icon), [ExoticA: AMOS Pac.Pic. format](https://www.exotica.org.uk/wiki/AMOS_Pac.Pic._format), [JS: AMOS Picture Bank](http://fileformats.archiveteam.org/wiki/AMOS_Picture_Bank), [JS: AMOS Sprite Bank](http://fileformats.archiveteam.org/wiki/AMOS_Sprite_Bank), [amigacoding: sprite/icon banks](https://www.amigacoding.com/index.php/AMOS:Sprite_and_Icon_bank_formats) | The alvyn page is complete for AmSp/AmIc (planar data, w\*h\*depth\*2 bytes, palette) and defers Pac.Pic to the ExoticA page. The ExoticA page was behind a bot check and not fetched, but search snippets show it is a full spec (big-endian, 3-stream RLE). Permissive references: Deark (MIT) and [amos-abk on PyPI](https://pypi.org/project/amos-abk/0.2.0/) (WTFPL). [dschwen/amosbank](https://github.com/dschwen/amosbank) is CC-BY-SA 3.0 (share-alike), so do not port it. |
| ACBM | Amiga Contiguous Bitmap (IFF) | **Spec** | [AmigaOS wiki ACBM](https://wiki.amigaos.net/wiki/ACBM_IFF_Amiga_Continuous_Bitmap), [ADCD ACBM.doc](https://amigadev.elowar.com/read/ADCD_2.1/Devices_Manual_guide/node0210.html) | Same as ILBM, except BODY is replaced by ABIT, which stores whole planes in sequence (plane 0..n, each h rows). |
| DEEP | TVPaint deep (chunky) IFF | **Spec** | [AmigaOS wiki DEEP](https://wiki.amigaos.net/wiki/DEEP_IFF_Chunky_Pixel_Image) | Chunks DGBL, DPEL, DLOC, DBOD and DCHG. Element types RGB/RGBA/YCM and so on; several compression types are defined. Deark `deep` (MIT) is a reference. |
| FLF | Turbo Rascal Syntax Error "Fluff" image | **None** | [JS: Turbo Rascal Syntax Error](http://justsolve.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error), [TRSE downloads/tutorials (lemonspawn)](https://lemonspawn.com/turbo-rascal-syntax-error-expected-but-begin/downloads/) | One container format used across platforms (Amiga up to 320x256, PC 320x200, also C64 and VIC-20). The only authoritative description is the TRSE source, which is GPL-3, so do not read it. A search snippet mentions a "13+32 byte header". You would need to reverse-engineer files produced by the TRSE IDE (the TRSE binary itself can be used to make samples). |
| HAM, HAM6 | Hold-And-Modify 6 (ILBM + CAMG) | **Spec** | [ILBM spec](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap), HRM (above) | 6 planes. The top 2 bits are control: 00 = palette index, 01 = blue, 10 = red, 11 = green. Each line starts from palette colour 0 (border colour). |
| HAM8 | Hold-And-Modify 8 (AGA ILBM) | **Spec** | ILBM spec, [AGA notes](https://jvaltane.kapsi.fi/amiga/howtocode/aga.html), [Wikipedia HAM](https://en.wikipedia.org/wiki/Hold-And-Modify) | 8 planes with HAM in CAMG. Control is in the low 2 bit-planes (planes 0-1) and data in planes 2-7. 64-entry base palette. |
| IFF, 256 | IFF ILBM (generic, incl. 8-plane/256-colour, EHB) | **Spec** | [EA IFF 85](https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files), [ILBM](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap), [fileformat.info IFF](https://www.fileformat.info/format/iff/egff.htm) | "256" is presumably an 8-plane ILBM. EHB (CAMG 0x80, 6 planes) gives colours 32-63 as half-brightness copies of 0-31. Also handle the PBM (chunky) FORM. |
| INFO | Amiga Workbench icon | **Spec** | RKM ch. 14 ([mirror](http://www.theflatnet.de/pub/cbm/amiga/AmigaDevDocs/lib_14.html)), [krashan: Amiga icon file format](http://krashan.ppa.pl/articles/amigaicons/), [PureBasic forum .info description](https://www.purebasic.fr/english/viewtopic.php?t=33749) | Layout: DiskObject (78 bytes, magic 0xE310), optional DrawerData (56 bytes), then Image header (20 bytes) + planar data, and an optional second image. Rows are (w+15)/16\*2 bytes. The krashan and PureBasic pages were found by search but not fetched (DNS failure and HTTP 403). NewIcons and GlowIcons (FORM ICON) are later extensions, see below. Deark `amigaicon` (MIT) and [bitplane/datatypes](https://github.com/bitplane/datatypes) (MIT) are references. |
| LBM, ILBM | Interleaved Bitmap | **Spec** | [ILBM](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap), [Wikipedia ILBM](https://en.wikipedia.org/wiki/ILBM), [LightWave SDK copy of ILBM spec](http://www.etwright.org/lwsdk/docs/filefmts/ilbm.html) | BMHD/CMAP/CAMG/BODY; ByteRun1 is fully specified in Appendix D; masking modes. Best-case format. |
| DHR, DR, MP, BEAM | Multi-palette ILBM variants (per-line palettes, 4096 colours) | **None** | [ILBM spec, PCHG section](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap), [IFF registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry), [AmigaLove: Dynamic Hi-Res thread](https://www.amigalove.com/viewtopic.php?t=620) | No public spec was found for the DHR/DR/MP/BEAM chunk or file layouts. NewTek Dynamic HiRes uses a 16-colour palette per line via the copper. PCHG (fully specified: header, line mask, Huffman-compressed option) is the documented way to store per-line palettes, but these files apparently predate or bypass it. The registry lists CTBL ("array of words, one per colour, 0rgb") and DYCP ("3 longwords") with no further detail. Reverse engineering needed. |
| RGB8 | Impulse Turbo Silver / Imagine 24-bit | **Spec** | [AmigaOS wiki RGBN/RGB8](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data) | BMHD nPlanes = 25 and compression = 4. BODY is 32-bit longwords: 24-bit RGB (MSB), a genlock bit, then a 7-bit repeat count. CAMG chunk required. |
| RGBN | Impulse Turbo Silver 12-bit | **Spec** | same as RGB8 | nPlanes = 13. BODY is 16-bit words: 12-bit RGB, a genlock bit, then a 3-bit count. Count 0 means a following byte count, and a byte count of 0 means a following word count. |
| SHAM | Sliced HAM (ILBM + SHAM chunk) | **Partial** | [ILBM spec, PCHG section (critique of SHAM/CTBL)](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap), [IFF registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry), [HAM Eager write-up](https://dump.platon42.de/hameager/) | The registry entry has no linked spec. The PCHG text states SHAM is "hardwired to 200 lines", i.e. a per-line 16-colour palette. Byte layout (version word, then per-line 16 x 0RGB words, and interlace handling) is not in any document found. Verify against sample files. |

### NewIcons and GlowIcons (INFO extensions)

Two later extensions store a better picture in the same `.info` file. RECOIL shows only the classic planar image, and it rejects every sample that has a GlowIcon appended.

- **Spec:** Dirk Stöcker, "Amiga Icon Format" (17 Feb 2002), [evillabs.net copy](http://www.evillabs.net/index.php/Amiga_Icon_Formats). It covers the whole DiskObject (including the DefaultTool/ToolTypes text encoding, ToolWindow and DrawerData2), the NewIcons tool type encoding and the OS 3.5 `FORM ICON` chunks.
- **NewIcons:** stored in tool types after a `*** DON'T EDIT THE FOLLOWING LINES!! ***` line. `IM1=` lines hold the normal image and `IM2=` lines the selected one. The first line starts with `B` (transparent) or `C` (opaque), then width + `$21`, height + `$21` and a two-character colour count (`(c0-$21)<<6 + c1-$21`). After that come 8-bit RGB palette bytes and chunky pixels with ceil(log2(colours)) bits each. Bits are packed 7 per character: `$20-$6F` stands for 0-`$4F`, `$A1-$D0` for `$50-$7F`, and `$D1-$FF` for 1-47 times 7 zero bits. Each line is flushed and padded. The spec doesn't say where the palette ends or which colour is transparent. Deark's `amigaicon.c` (MIT) starts the pixels at the first line end after the whole palette and treats colour 0 as transparent, and our output matches Deark on all 78 sample NewIcons.
- **GlowIcons (OS 3.5):** an IFF `FORM ICON` after the DiskObject's parts. `FACE` holds width-1, height-1, flags (bit 0 frameless), aspect (x and y nibbles) and max palette bytes-1. One or two `IMAG` chunks hold the normal and selected images: transparent colour, colours-1, flags (bit 0 transparency, bit 1 palette attached; the second image may reuse the first palette), image and palette storage (0 = one byte per entry, 1 = ByteRun1 over a bit stream of `depth`-bit pixels or 8-bit palette bytes), depth, image bytes-1, palette bytes-1, the image data, then the palette.
- **OS4 `ARGB` chunk** (32-bit zlib-compressed images): not described by Stöcker. Not decoded yet, since the shared inflate code lives in the Atari 8-bit module.
- **Decoder choice** (`platform/amiga/icon.rs`): the normal image of the GlowIcon, else of the NewIcon, else the classic one. If a better image fails to decode, the next one is used. NewIcons and GlowIcons are shown with square pixels, and their transparent colour becomes the Workbench 2.x background grey (`$959595`), the same colour the classic image's colour 0 shows as.
- **Samples:** Aminet `pix/icon` packs: AtariGlowIcons, HunoGlowsAnima, GoVD_GI and Exoticons35 for GlowIcons (3,149 icons compared with Deark 1.7.3, all pixel-identical); HomersNI and FM_NewiconDock for NewIcons. The NewIcons.lha and newicons2.lha packs hold only classic images.

## Amiga DCTV

**Hardware references:** [Amiga Hardware Database: DCTV](http://amiga.resource.cx/exp/dctv); [archive.org: DCTV + RGB Converter](https://archive.org/details/dctv-rgb-amiga), which has the user guides as PDF, the software as ADF and demo videos; [Big Book of Amiga Hardware: DCTV](https://bigbookofamigahardware.com/bboah/product.aspx?id=282). DCTV encodes the composite waveform into an ordinary Amiga planar screen and marks it with a signature in the upper-left corner.

**Palette:** Effectively YUV/composite, about 6 million colours. No public decode description was found.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| DCT, DCTV | Digital Creations DCTV encoded ILBM | **None** | [amiga.resource.cx DCTV](http://amiga.resource.cx/exp/dctv), [archive.org DCTV](https://archive.org/details/dctv-rgb-amiga), [EAB thread: DCTV user's guide](https://eab.abime.net/showthread.php?p=1510982) | The container is ILBM (or IFF "DCTV" group ID). Detect it by the signature near the start of the pixel data. The encoding algorithm is undocumented publicly. The user guide PDFs on archive.org may help, but expect reverse engineering. |

## Amiga HAM-E

**Hardware references:** [Amiga Hardware Database: HAM-E](http://amiga.resource.cx/exp/hame) (384x480/560 modes; Register mode with 256 colours from 24-bit; Extended HAM mode); the [comp.sys.amiga "Black Belt Video" post](https://groups.google.com/g/comp.sys.amiga/c/y0hUJpHRESo/m/XRUxLp2maTkJ), which was rate-limited and not fetched (content below is from search snippets).

**Palette:** A 16-pixel "magic cookie" at the start of the first line. The rest of that line holds 384 pixels of palette data: 64 RGB triplets, 8 bits per component. Up to 4 trigger lines load 4 x 64 = 256 registers. The image itself is a 640-wide, 4-plane hi-res ILBM, and the hardware pairs adjacent pixels into 8-bit pixels at half the width.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| IFF | HAM-E (Black Belt Systems) | **Partial** | links above | The outline is known: cookie line, palette lines, then 4-plane hi-res with pixel pairing. Still missing: the exact cookie values, how register vs. HAM mode is selected, the HAM-E-mode modify encoding and the bit order of the pairing. |

---

## Apple II / IIe

### Hardware references
- [CiderPress II: Hi-Res notes](https://ciderpress2.com/formatdoc/HiRes-notes.html). Covers the 8K screen at $2000/$4000 and 7 pixels per byte, with the high bit as a palette shift. Lines are interleaved in groups of 3 inside 128-byte blocks, leaving screen holes. Files are $1FF8, $1FFC or $2000 bytes. Types are FOT $08 and BIN $06, and PackBytes and LZ4FH variants exist.
- [CiderPress II: Double Hi-Res notes](https://ciderpress2.com/formatdoc/DoubleHiRes-notes.html). Files are 16K: aux 8K first, then main 8K. The display is 560x192 mono or 140x192 in 16 colours; the page recommends a sliding 4-bit window for colour.
- [Apple II File Type Note $08 (prodos8.com)](https://prodos8.com/docs/technote/ftn/08/). Covers aux types $0000-$3FFF and the mode byte at +120 (modes 0-7), plus $4000/$4001 PackBytes-compressed variants.
- Apple IIe Tech Note #3, Double High-Resolution Graphics: [Higher Intellect copy](https://wiki.preterhuman.net/Apple_II_Technical_Notes:_Apple_IIe_Double_High-Resolution_Graphics) (HTTP 403 to the fetcher). [Apple II Technical Notes index](https://downloads.reactivemicro.com/Apple%20II%20Items/Documentation/Manuals/Apple%20II%20Technical%20Notes/Apple%20II%20Technical%20Notes%20Index.htm).
- [Wikipedia: Apple II graphics](https://en.wikipedia.org/wiki/Apple_II_graphics).

### Palette
- HGR: 6 colours (black, white, green, purple, orange, blue) from NTSC artifacts. These are not stored in the file. Sources: [Meresh: The Apple II palette](https://www.meresh.com/a2colors.html), [appleoldies: DHGR colour considerations](http://www.appleoldies.ca/a2b/DHGRColors2017.htm), [Paleotronic article](https://paleotronic.com/2018/10/03/apple-ii-colour-computer-graphics/), [Wikimedia: Apple II High Res palette](https://commons.wikimedia.org/wiki/File:Apple_II_High_Res_Palette.png). IIgs TN #63 "Master Color Values" is cited by CiderPress II as the canonical RGB list.
- DHGR: the 16 lo-res colours, from the same sources.
- AppleWin's NTSC palette (Sheldon Simms) is often quoted. AppleWin is GPL (from general knowledge, not checked in this session), so take values only from the non-code pages above.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| HGR | Apple II High Resolution screen dump (280x192) | **Spec** | [CiderPress II Hi-Res](https://ciderpress2.com/formatdoc/HiRes-notes.html), [FTN $08](https://prodos8.com/docs/technote/ftn/08/) | A raw 8K memory dump. RECOIL classes it as mono. Colour rendering via NTSC artifacts is optional. |
| SPR | Apple II "Sprites" (mono) | **None** | — | No program or format was identified. Some candidate modern tools (not the format): [SpriteGen](https://github.com/blondie7575/SpriteGen), [appleoldies DHGR sprites](http://www.appleoldies.ca/bmp2dhr/sprites/). Reverse engineering needed. |
| DHGR, DHR | Apple IIe Double Hi-Res (560x192) | **Spec** | [CiderPress II DHR](https://ciderpress2.com/formatdoc/DoubleHiRes-notes.html), [FTN $08](https://prodos8.com/docs/technote/ftn/08/), IIe TN #3 | 16384 bytes, aux half first. Each byte holds 7 pixels with the high bit unused. |

## Apple IIGS

### Hardware references
- Apple IIGS Hardware Reference: [archive.org (1st ed)](https://archive.org/details/Apple_IIgs_Hardware_Reference), [archive.org (2nd ed, 1988)](https://archive.org/details/Apple_IIGS_Hardware_Reference_1988_Adn-Wesley_Publishing_second_edition).
- [Apple IIGS Graphics Modes (Tech Info Library text)](https://archive.org/stream/Apple_IIGS_Graphics_Modes/Apple_IIGS_Graphics_Modes_djvu.txt).
- [CiderPress II Super Hi-Res notes](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html). Memory map in bank $E1: pixels $2000-$9CFF (32000 bytes, 160 bytes per line), SCBs $9D00-$9DC7, reserved, then 16 palettes x 16 colours at $9E00-$9FFF. 320 mode is 2 px/byte and 640 mode is 4 px/byte. A colour word is low byte G<<4|B and high byte R.
- [Bumbershoot: Apple IIgs Super High-Res mode](https://bumbershootsoft.wordpress.com/2023/05/27/apple-iigs-super-high-res-mode/).
- PackBytes: [IIgs TN #94 "Packing It In (and Out)"](https://apple2.gs/technotes/tn/iigs/TN.IIGS.094.txt). Partial only; see the open gap in the summary.

### Palette
4096-colour RGB444. The palettes are stored in the file, so no external palette is needed. In 640 mode the 4 sub-palettes depend on pixel column; the Hardware Reference covers this.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| 3201 | Compressed 3200-colour picture | **Spec** | [CiderPress II SHR](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html) | +0: "APP" in high ASCII followed by $00. +4: 6400 bytes holding 200 x 16-colour tables. +$1904: PackBytes data that unpacks to 32000 bytes. Check whether the tables are reversed like Brooks; CiderPress does not say. |
| 32K | Multi-palette 3200-colour | **None** | — | Name suggests a 32K-sized multi-palette file, but no description was found. It may be a raw $C1/0002 variant; verify with samples. |
| GS, IIGS, PNT, SHR | Apple Preferred Format, $C0/$0002 | **Spec** | [FTN $C0/$0002](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0002), [CiderPress II SHR](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html) | Blocks of [len:u32, Pascal-string kind, data]. MAIN holds MasterMode, PixelsPerScanLine, colour tables, NumScanLines, a scan-line directory and per-line PackBytes data. Other blocks: MULTIPAL (3200 colours), MASK, PATS, SCIB, PALETTES. Depends on PackBytes. |
| PNT | Paintworks, $C0/$0000 | **Spec** | [FTN $C0/$0000](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0000), [CiderPress II SHR](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html) | +0: 32-byte palette. +$20: background word. +$22: 16 patterns x 32 bytes. +$222: PackBytes data in 320 mode. The height is not stored: unpack until the data ends, giving 396 lines (63360 bytes) or 200 lines. |
| SH3, 3200 | Brooks 3200-colour, $C1/$0002 | **Spec** | [FTN $C1/$0002](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0002), [appleoldies PDF of same](https://appleoldies.ca/graphics/shr/ft_c1.0002_3200_color.pdf) | 32000 pixel bytes, then 200 x 32-byte palettes. Each palette is stored reversed: colour 15 first. Uncompressed, 38400 bytes. |
| SHR | Super Hi-Res screen dump, $C1/$0000 (320x200, 16 palettes, so 256 colours) | **Spec** | [FTN $C1/$0000](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0000), [CiderPress II SHR](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html), Hardware Reference | 32768 bytes holding $E12000-$E19FFF: pixels, SCBs, reserved bytes, palettes. The FTN only says it is a RAM dump; the layout comes from the HW Ref and CiderPress. |

## Apple Macintosh

**Hardware/palette:** 1-bit, 1 = black, MSB is the leftmost pixel.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| MAC, PNT, PNTG | MacPaint | **Spec** | [Apple Tech Note PT24 / #86 (archived)](https://leopard-adc.pepas.com/technotes/pt/pt_24.html), [CiderPress II MacPaint](https://ciderpress2.com/formatdoc/MacPaint-notes.html), [MacTech 1985 article](http://preserve.mactech.com/articles/mactech/Vol.01/01.07/MacPaintfiles/index.html), [fileformat.info EGFF](https://www.fileformat.info/format/macpaint/egff.htm), [JS: MacPaint](http://justsolve.archiveteam.org/wiki/MacPaint) | 512-byte header (version u32, 38x8 bytes of patterns, 204 reserved), then 720 rows of 72 bytes, each row PackBits-compressed. The file may start with a 128-byte MacBinary header; detect it heuristically. PackBits TN1023 is [archived](http://web.archive.org/web/20080705155158/http://developer.apple.com/technotes/tn/tn1023.html). Deark and bitplane/datatypes (both MIT) are references. |

---

## PC

**Palette notes:** MSP and HS2 are 1-bit. EPA v1 is 1-bit or uses text attributes, and v2 is 4- or 8-bit. Deark notes an RGB/BGR ambiguity for v2.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| EPA | Award BIOS logo (EPA v1 / AWBM v2) | **Partial** | [JS: Award BIOS logo](http://fileformats.archiveteam.org/wiki/Award_BIOS_logo), [Changing the Award BIOS logo](https://unstable.nl/andreas/bios.html), [VirtualPlastic BIOS branding](https://www.virtualplastic.net/html/misc_bios.html), [Pinczakko: Award BIOS structure](https://sites.google.com/site/pinczakko/award-bios-file-structure) | Known facts: v1 is 136x84 or 136x126, 1bpp, for BIOS 4.5x; v2 (AWBM) is 16 colours, for BIOS 6.x. No byte layout was found in the pages fetched. Deark `awbm` (MIT) is the readable reference. The JS page was not fetched. |
| HS2 | Handy Scanner 2000 "POSTERING" | **Partial** | [JS: HS2 (POSTERING)](http://fileformats.archiveteam.org/wiki/HS2_(POSTERING)) | An obscure raw bi-level bitmap, dated 1991 or earlier. Width and header rules are not known from fetched docs. Deark `hs2` (MIT) is a reference. |
| FLF | Turbo Rascal Syntax Error | **None** | see Amiga FLF | Same format family. PC images are 320x200. |
| FNT | Image 72 font (mono) | **None** | — | The only trace is a "PC Image 72 font" variant mentioned in an FNT listing. Reverse engineering needed. |
| MSP | Microsoft Paint 1.x/2.x | **Spec** | [fileformat.info EGFF](https://www.fileformat.info/format/mspaint/egff.htm), [JS: MSP](http://fileformats.archiveteam.org/wiki/MSP_(Microsoft_Paint)) | 32-byte header: Key1/Key2 ("DanM" for v1, "LinS" for v2), width, height, aspect fields, XOR checksum. v1 rows are uncompressed. v2 has a u16 per-row size map, then RLE rows: `00 n v` is a fill, `n` followed by n bytes is a literal. Deark and bitplane/datatypes (both MIT) are references. |

## PlayStation

**Palette:** 16-bit framebuffer colour: 5:5:5 plus an STP bit. CLUTs are stored in the file.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| TIM | Sony TIM texture | **Spec** | [psx-spx (problemkaputt): TIM/PXL/CLT](https://problemkaputt.de/psxspx-cdrom-file-video-texture-image-tim-pxl-clt-sony.htm), [Kaitai psx_tim.ksy (CC0)](https://formats.kaitai.io/psx_tim/), [FFHacktics wiki](https://ffhacktics.com/wiki/.TIM_File_Format), [qhimm wiki](https://qhimm-modding.fandom.com/wiki/PSX/TIM_file), [psxdev forum](https://www.psxdev.net/forum/viewtopic.php?t=109), [JS: TIM](http://justsolve.archiveteam.org/wiki/TIM_(PlayStation_graphics)) | Header is 0x10, then version 0. Flags: bits 0-2 type (0 = 4bpp, 1 = 8bpp, 2 = 16bpp, 3 = 24bpp, 4 = mixed), bit 3 = CLUT present. Each block is [len u32, x, y, w, h u16, data]. Widths are in 16-bit halfwords. The Kaitai spec is CC0. |

## Psion Series 3

**Palette:** 1-bit. Pairs of bitmaps can combine into 4-level grey.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PIC, ICN | Psion Series 3 bitmap / icon | **Spec** | [Psionics Files: bitmap.fmt](https://www.davros.org/psion/psionics/bitmap.fmt) ([index](https://www.davros.org/psion/psionics/), [w3.org mirror](https://www.w3.org/Tools/pypsion/psionics/)), [PRONOM fmt/1744](https://www.nationalarchives.gov.uk/PRONOM/fmt/1744) | Header: "PIC" $DC, version $30 $30, then a u16 count. Each entry is 12 bytes (crc, w, h, size, offset relative to the end of the record). Rows are padded to even bytes, LSB is the leftmost pixel. ICN files are assumed to be PIC-format icons. Deark `psionpic` (MIT). |

## HP 48

**Palette:** 1-bit. Deark supports 2-plane greyscale GROBs as an option.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| GRB, GRO | HP 48 GROB | **Spec** | [hpcalc.org: GROB format (grobs.txt)](https://www.hpcalc.org/hp48/docs/programming/grobs.txt), [HP48 FAQ ch. 8](https://www.hpcalc.org/hp48/docs/faq/48faq-8.html), [JS: GROB](http://justsolve.archiveteam.org/wiki/GROB) | "HPHP48-x" 8-byte header, then 5-nibble fields (each stored reversed) for prolog 02B1E, length, height and width. Rows are padded to bytes. LSB of each nibble is the leftmost pixel. Text (ASCII "GROB w h hex") variants also exist. Deark `grob` (MIT). |

## Tandy 1000

**Palette:** The Tandy/PCjr 16-colour CGA-like palette.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PNT | DeskMate Paint (312x176, 16 colours) | **Partial** | [JS: DeskMate Paint](http://justsolve.archiveteam.org/wiki/DeskMate_Paint), [Tvdog's DeskMate page](http://www.oldskool.org/guides/tvdog/deskmate.html), [comp.sys.tandy: .PNT viewer/converter thread](https://groups.google.com/g/comp.sys.tandy/c/sECOUR6Igcg/m/mxBgvL_N_0kJ) | Signature is `13 50 4E 54` ("\x13PNT"). Files may be compressed or uncompressed and have at most 16 colours. The RLE details were not found in fetched docs. Deark `deskmate_pnt` (MIT) is the readable reference. |

## TRS-80 (Model III/4 hi-res)

**Hardware references:** [trs-80.org: Model 4 Grafyx Solution](http://www.trs-80.org/model-4-grafyx-solution.html), [trs-80.org: Radio Shack Model 4 Hi-Res board](http://www.trs-80.org/radio-shack-model-4-high-resolution-board/), [trs-80.org: hi-res graphics for the TRS-80](http://www.trs-80.org/high-resolution-graphics-for-the-trs-80.html), [Byte Cellar article](https://bytecellar.com/2018/10/17/enjoying-high-res-graphics-on-a-text-only-trs-80-model-4-from-1983/). Both boards display 640x240 at 1bpp from 32K of RAM.

**Palette:** Monochrome.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| HR | TRS-80 hi-res screen (640x240) | **Hardware-only** | [JS: HR (TRS-80)](https://fileformats.archiveteam.org/index.php?action=edit&amp=&title=HR_%28TRS-80%29) | Raw 1bpp, 80 bytes per row x 240 rows = 19200 bytes. Some files are slightly larger. Deark `hr` (MIT). |
| RLE | CompuServe RLE (256x192; also 128x96) | **Spec** | [Brutman: Rediscovering CompuServe RLE](http://www.brutman.com/RLE/RLE_Graphics.html), [RevCurtisP/csrle (has original RLE.TXT)](https://github.com/RevCurtisP/csrle), [VOGONS thread](https://www.vogons.org/viewtopic.php?p=965054) | Starts with `ESC G H` (256x192) or `ESC G M` (128x96). Each following character c gives a run of c-32 pixels, alternating black and white and starting with black. Ends with `ESC G N`. The csrle repo has no license, so do not copy its code. Deark `cserve_rle` (MIT). |
| SHR | MagicDraw (David Goben, 640x240 mono) | **None** | [JS: MagicDraw](http://fileformats.archiveteam.org/wiki/MagicDraw) | Probably compressed or headered, since otherwise it would just be an .HR. No layout found, and the JS page was not fetched. Reverse engineering needed. |

## TRS-80 Color Computer

**Hardware references:** Chris Lomont, *Color Computer 1/2/3 Hardware Programming*: [v0.82 PDF](https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware.pdf), [newer PDF](https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware_2.pdf). [CoCopedia: Video Display Generator](https://www.cocopedia.com/wiki/index.php/Video_Display_Generator) (MC6847). PMODE 4 is 256x192 at 1bpp, 6144 bytes. PMODE 1 is 128x96 at 2bpp, 3072 bytes. An RS-DOS binary has a 5-byte preamble (00, len u16, addr u16) and a 5-byte postamble ([Color Computer Emulator tech notes](http://cpmarchives.classiccmp.org//trs80/mirrors/www.discover-net.net/~dmkeil/coco/cocotech.htm)).

**Palette:** MC6847 colour sets. 4-colour CSS = 0 gives green/yellow/blue/red; CSS = 1 gives buff/cyan/magenta/orange. 2-colour modes are black/green or black/buff ([Sub-Etha: the 9 colours](https://subethasoftware.com/2024/02/08/the-9-colors-of-the-cocos-high-resolution-screens/)). For RGB approximations see [Lospec CoCo RGB](https://lospec.com/palette-list/tandy-color-computer-rgb) and [Wikipedia: list of 8-bit computer hardware graphics](https://en.wikipedia.org/wiki/List_of_8-bit_computer_hardware_graphics).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| CLP | Clip (40x56 mono) | **Hardware-only** | Lomont PDF | Presumably raw: 5 bytes x 56 rows = 280 bytes, possibly with a small header. This is unverified, so check against samples. |
| GRF, MAX, P41, PIX | PMODE 4 picture (256x192 mono): CoCo Max .MAX, Graphicom and others | **Hardware-only** | Lomont PDF, [CoCopedia FAQ](https://www.cocopedia.com/wiki/index.php/Color_Computer_FAQ), RS-DOS binary notes above | Core payload is 6144 bytes of PMODE 4 VRAM. .MAX is reportedly an RS-DOS binary, i.e. preamble + 6144 bytes + postamble. Other extensions may be headerless or have different headers, so check by file size. [jamieleecho/coco-tools](https://github.com/jamieleecho/coco-tools) handles MAX/ART but is GPL-2, so do not read it. |
| P11 | PMODE 1 picture (128x96, 4 colours) | **Hardware-only** | Lomont PDF | 3072 bytes of 2bpp VRAM. Palette from CSS. Check by file size. |

---

## Implementation status after wave 2

- **Content detection (`.signature()`):** IFF (incl. DEEP/TVPP), AMOS AmSp/AmIc/Pac.Pic.,
  Workbench icons (magic `$E310` + version 1), APF, `.3201`, MacPaint in MacBinary (type `PNTG`),
  MSP (`DanM`/`LinS`), Award AWBM, TIM, Psion PIC (`PIC` `$DC` `00`), GROB (`HPHP48-`, or text
  starting with `GROB` / a `%%HP:` line), DeskMate PNT, CompuServe RLE. Not marked: memory dumps
  and size-only formats (HGR, DHGR, Brooks, SHR dumps, HS2, HR, CoCo), Paintworks, bare MacPaint,
  EPA v1, packed SHR.
- **CompuServe RLE acceptance**, probed with `recoil2png` as a black box: run characters must be
  `$20-$7F` up to an `ESC` or the end, and cover the whole picture (one pixel short only when an
  `ESC` follows).
- **Packed Super Hi-Res** ($C0/0001, [FTN](https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0001)):
  the 32 KB screen through PackBytes; decoded from `.shr` when it unpacks to exactly 32 KB.
- **3-plane Workbench icons:** 2.x pens, then dark grey, light grey, beige, pink (MagicWB; order
  observed from `recoil2png`, beige from Deark).
- **Still undecoded, no usable documentation found:** HAM-E (amiga.resource.cx describes only
  the modes, not the cookie, palette lines or HAM encoding), DCTV, FLF, Apple II SPR, TRS-80
  MagicDraw SHR, Image 72 FNT. RECOIL renders all of them; they need reverse engineering.

## Sample file sources (URLs only; nothing downloaded)

- CompuServe RLE: [Brutman page](http://www.brutman.com/RLE/RLE_Graphics.html) (three zips: assorted RLEs, original 1987 BBS set, Walnut Creek CP/M set), [csrle repo](https://github.com/RevCurtisP/csrle) ("Assorted RLE Files.zip").
- HP 48 GROBs: [hpcalc.org grobs collection](https://www.hpcalc.org/hp48/graphics/grobs/).
- Apple IIGS: [Paintworks Gold disk (archive.org)](https://archive.org/details/a2gs_Paintworks_Gold_v1.0_1988_Activision), [appleoldies Apple II graphics index](https://appleoldies.ca/graphics/index.htm) (converter outputs and examples), [appleoldies SHR converter comparison](http://www.appleoldies.ca/a2b/Summer2015/SuperConvertA2BComparisonSummer2015.htm).
- Tandy DeskMate: [Tvdog's Tandy 1000 archive](http://www.oldskool.org/guides/tvdog/tandy1000.html), [DeskMate page](http://www.oldskool.org/guides/tvdog/deskmate.html), [IBM DeskMate (archive.org)](https://archive.org/details/hdemudeskmate).
- DCTV: [archive.org DCTV item](https://archive.org/details/dctv-rgb-amiga) (software ADFs that can generate samples).
- AMOS: [Java AMOS Sprite Bank Viewer files (SourceForge)](https://sourceforge.net/projects/javaamosabk/files/abk/). This is a viewer project; its license was not checked, so use it only for sample banks if any are included.
- TRSE (for generating FLF samples): [lemonspawn downloads](https://lemonspawn.com/turbo-rascal-syntax-error-expected-but-begin/downloads/).
- TRS-80 hi-res: [Ira Goldklang's TRS-80 archive](https://www.trs-80.com/main-emulators.htm).

## To avoid (GPL code: do not read)

| Project | License | Relevance |
|---|---|---|
| RECOIL (recoil.sourceforge.net, all language ports) | GPL | The reference this project replaces. Do not open its source, bug tracker attachments or forks. |
| [TRSE / Turbo Rascal Syntax Error](https://github.com/leuat/TRSE) and [Fluff64](https://github.com/leuat/Fluff64) | GPL-3 (TRSE; Fluff64 merged into TRSE, so assume the same) | The only authoritative definition of FLF. |
| [AmigaFFH (R package)](https://cran.r-project.org/web/packages/AmigaFFH/AmigaFFH.pdf) | GPL-3 | ILBM, HAM6/8 and icons. |
| [format198x / format198x-commodore-amiga-ilbm (Rust)](https://github.com/format198x/format198x) | GPL-2.0-or-later | ILBM. |
| [jamieleecho/coco-tools](https://github.com/jamieleecho/coco-tools) | GPL-2.0 | CoCo MAX/ART/MGE/PIX/CM3 converters. |
| AppleWin | GPL (from general knowledge, not checked this session) | Apple II NTSC palette and rendering. Take palette numbers only from non-code pages. |

**Use with care (not GPL, but not freely reusable as code):**

- [dschwen/amosbank](https://github.com/dschwen/amosbank) is CC-BY-SA 3.0 (share-alike).
- [RevCurtisP/csrle](https://github.com/RevCurtisP/csrle) states no license. Its documents (RLE.TXT) are fine to read as a spec.
- [svanderburg/libilbm](https://github.com/svanderburg/libilbm) and [amigazen/ifftools](https://github.com/amigazen/ifftools): licenses not checked; check before reading. libilbm's `doc/ACBM.asc` is a copy of the public ACBM spec.

**Permissive references found:** Deark (MIT-style, from 1.4.x), [bitplane/datatypes](https://github.com/bitplane/datatypes) (MIT; AROS datatypes incl. MacPaint, MSP, CompuServe RLE, Amiga icon), [amos-abk](https://pypi.org/project/amos-abk/0.2.0/) (WTFPL), [Kaitai psx_tim.ksy](https://formats.kaitai.io/psx_tim/) (CC0), [CiderPress II](https://github.com/fadden/CiderPress2) (Apache-2.0 code; CC BY-SA 4.0 docs).

## Wave 5: Amiga and misc

Everything here was reverse engineered from the corpus samples and by feeding `recoil2png`
hand-built or byte-mutated files (kept outside `corpus/`). All decoded samples match its
output pixel for pixel; no divergences were recorded. The decoders cite this section.

### Amiga DCTV (`amiga/dctv.rs`, platform "Amiga DCTV")

All 10 samples match (the 11th, `hostile/.../Crop.dctv`, is a truncated file that RECOIL
rejects and we do too).

- Container: a 3- or 4-plane hires ILBM with CAMG `0x8000`. Colours are not in the palette;
  they are encoded in the pixels. The existing ILBM reader already refused such files, and
  now `decode_form` tries DCTV, then HAM-E, when the plain ILBM decode refuses.
- Signature: the top plane of the first 256 pixels of row 0 spells a fixed 32-byte string
  (`00 49 87 28 de 11 0b ef ...`, see `dctv.rs`); other planes there are ignored, as are the
  rest of the row. Interlaced files (CAMG bit 2) repeat it on row 1 and need both. Flipping
  any signature bit makes RECOIL fall back to a plain ILBM. RECOIL also falls back for CAMG
  values like `0x9000` or `0x18000` that we still decode as DCTV; those are not in the corpus.
- Samples: two pixels `(a, b)` form one 8-bit value with interleaved bits `a3 b3 a2 b2 a1 b1
  a0 b0` (a is the more significant of each pair). 3-plane files are the 4-plane layout without
  the lowest plane, so each pixel is shifted left by one. Luma is `(((v[m] + v[m-1]) >> 1) - 64)
  * 8 / 5`, clamped to 0..255 before chroma is added.
- Chroma: `(e[m] + 2 e[m-1] + e[m-2]) / 4` (truncating) of `e[j] = (-1)^j v[j]`. Scan lines
  alternate between two phases. In phase A (even lines of a field) pixels pair as `(2j, 2j+1)`
  and the chroma is the red difference; in phase B pairs start one pixel later, the line is shifted
  one pixel right, there is a final zero pair, and the chroma is the blue difference. Each line takes the
  other component from the line before it in the same field (nothing before the first line).
  Interlaced pictures are two such fields in alternate rows; progressive ones show each line
  twice (height `(h - 1) * 2`, interlaced `h - 2`). The first four pixels of each line are
  ordinary samples (they set up the filter); column 0 of phase B lines is black.
- RGB: `R = Y - 1164 cr / 640`, `G = Y + (593 cr - 404 cb) / 640`, `B = Y + 4143 cb / 1280`
  with truncating division. Black-box fitting narrowed these to `R in [1.8182, 1.8194)`,
  `B in [3.2364, 3.2368)`, `G` coefficients `0.926562..0.926585` and `0.63125..0.631265`; the
  rationals are in every interval and match about 4 million real pixels plus random full-range
  tests. The exact constants RECOIL uses are unknown.
- Not understood: what CAMG values besides `0x8000` plus lace RECOIL requires, and what the
  program's NTSC or PAL flag (`0x19000` vs `0x29000`) changes (nothing visible in the samples).

### Amiga HAM-E (`amiga/ham_e.rs`, platform "Amiga HAM-E")

All 5 samples (`beautyfc`, `hameset1/2`, `breakfst`, `nishi`) match.

- Detection: row 0 starts with the 16-pixel cookie `a 2 f 5 8 4 d c 6 d b 0 7 f 1` and a mode
  pixel. RECOIL additionally needs the standard 16-colour CMAP (bit 7 of every channel and the
  low bit of blue's high nibble carry the pixel value back to the hardware) and only treats
  hires pictures without the HAM flag as HAM-E. We check the cookie, hires and no HAM.
- Palette lines: any row that starts with the cookie. Mode pixel 4 = register mode, 8 =
  hold-and-modify. The rest of the row is 192 bytes (one byte per pixel pair, high nibble
  first) = 64 RGB colours of 8 bits each. Palette lines fill 4 banks in turn and wrap; interlaced
  files carry every palette line twice and each field keeps its own banks. A palette line is drawn black.
- Data lines: pairs of pixels make a byte. Register mode: `bank[byte >> 6][byte & 63]`. Modify
  mode: HAM8 control in the top 2 bits (0 = bank 0 colour, 1 blue, 2 red, 3 green set to
  `data << 2`, not bit-replicated), starting from black on every line.
- Output is half the bitmap width; interlaced pictures are doubled again.

### Apple II SPR "Sprites" (`apple/sprites.rs`)

Only `test.spr` is this format: a text file of numbers (decimal or `$hex`, any whitespace)
`width height kind x y` plus `width * height` bytes column by column, ended by a record
with height 0. Sprites are ORed onto a 320x200 black canvas, bit 7 left, `x`/`y` in pixels;
`kind` is ignored; every number must be below 320 and a sprite must fit. See the module
comment for the accept/reject rules that were probed.

`running-cat.spr`, `cinema-counter.spr` and `vial.spr` are not Apple II. They start with
`Spr!` and are SprEd files (Atari 8-bit, RECOIL's "SprEd", 128 colours); see below.

### Atari 8-bit SprEd (`atari8/spred.rs`)

3 samples match. Header (all counted from the file start): `Spr!`, version (ignored), byte 9
= lines shown twice, byte 10 bit 2 = second sprite column, bit 0 = missiles, byte 14 bit 0 =
per-line colours, bytes 12/13 = width of the second column. Frames/lines come from bytes 17/18
when byte 16 is 0, else from 16/17 (the one sample like that, `vial.spr`, also has byte 18 =
extra canvas width). Data from byte 19: per-frame colour of each player, one plane per player
(each frame x line bytes), missile planes if flagged (low 2 bits), 5 per-line colour planes if
flagged (background, 4 players); extra bytes at the end are ignored. Only canvases of the
shapes seen are accepted: byte 12 = 0 and byte 13 = width / 2 for two columns, byte 18 = 0
for one. How RECOIL treats other widths (they shift the second column, clip, or fail) was
not worked out.

### PC "Image 72 font" (`pc/image72.rs`)

`00 08 h` + 256 glyphs of `h` bytes (`3 + 256 h` bytes; any `h` from 1), or a headerless
896-byte file of 8-byte glyphs of which the first 96 are shown (FATCAT.FNT is the CPC OCP
font with 128 zero bytes added; AMSDOS-headed 896-byte files stay with the CPC decoder).
Sheet of 32 glyphs per row, white on black. The name "Image 72" is RECOIL's; no program
was identified.

### TRS-80 MagicDraw SHR (`trs80/magicdraw.rs`)

Run-length coded 640x240 mono screen (shown with doubled lines): control with bit 7 set repeats
the next byte `control & 127` times, otherwise `control` literal bytes follow; zero counts are
fine; at least 19200 bytes must come out, the rest (822 bytes in the sample, more image
data) is ignored. `.SHR` is shared with Apple IIGS and C64, and RECOIL takes any stream that
reaches 19200 bytes, so we also require the operations to end exactly at the end of the
file (only the sample and one 32 KB IIGS dump pass of the 19 SHR files in the corpus; the
platform order lets the IIGS decoder win).

### Atari Falcon TIMG

Already decoded by the GEM IMG decoder (see the implementation notes in
`atari-st-tt-falcon.md`); the only gap was that RECOIL lists the extension under Atari Falcon,
so a second registry entry was added for that platform.

### Left undone

- "Atari ST/STE IFF" (uncovered in `docs/coverage.md`): none of the corpus IFFs turned out to
  be a separate Atari ST format; the five IFFs RECOIL alone decoded were HAM-E.
- DCTV: the CAMG variants above; `Crop.dctv` could be decoded partially (missing rows
  black) but RECOIL does not.
