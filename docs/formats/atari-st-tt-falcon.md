# Atari ST/STE, TT and Falcon image formats: documentation survey (clean-room)

Scope: the Atari ST/STE (95), Atari TT (3) and Atari Falcon (28) formats on RECOIL's format list. Every source below is a prose spec, a manual, a wiki page or a hardware document. **No GPL/LGPL decoder source was opened.** The only RECOIL page read was the plain list at [recoil.sourceforge.net/formats.html][recoil-list], which was used for resolution and colour-count hints. When a permissively licensed implementation exists, its license is stated.

Docs-quality levels:
- **Spec**: a public description of the file layout exists that is enough to write a decoder.
- **Partial**: some layout information (size, header or mode) is known, but there are gaps.
- **Hardware-only**: no file document was found, but the file is evidently a raw dump of a known video mode.
- **None**: nothing useful was found.

## 1. Summary

Counts are per table row. Rows follow RECOIL's list entries, so `PI1/PI2/PI3` is counted as several entries where RECOIL lists them separately (see the tables).

| Platform | Entries | Spec | Partial | Hardware-only | None |
|---|---|---|---|---|---|
| Atari ST/STE | 95 | 75 | 13 | 0 | 7 |
| Atari TT | 3 | 2 | 0 | 1 | 0 |
| Atari Falcon | 28 | 22 | 2 | 2 | 2 |
| **Total** | **126** | **99** | **15** | **3** | **9** |

Most formats are simple: a resolution word, a 16-word palette and 32000 bytes of word-interleaved bitplanes, optionally with a small RLE. The "None" entries are obscure programs: ColorSTar/MonoSTar objects, Atari Image Manager, Grafix, D-GRAPH, TRSE FLF, Music Compile RAGC and TIMG.

### Best umbrella sources (read these first)

1. **"ST Picture Formats"**, edited by David M. Baggett (1988-1991), with additions by Hans Wessels (2006-2008) and Lonny Pursell (2015-2017). The additions are explicitly **public domain**. The original text allows non-profit redistribution; reading it for facts is fine. The modern copy lives on the AtariForumWiki as an index page plus one sub-page per format. It covers about 80 of the formats below, often with small public-domain C decode snippets by Pursell or Wessels and palette-index functions by Belczyk or Wessels for Spectrum, PhotoChrome, QuantumPaint and HighresMedium.
   - Index (mirror that works): [temlib.org/AtariForumWiki ST_Picture_Formats][afw-index]. `atari-wiki.com` refused connections during this survey.
   - Raw wikitext of any sub-page: `https://temlib.org/AtariForumWiki/index.php?title=<Page>&action=raw`
   - Older plain-text versions: [textfiles.com picture.st (1992)][picture-st], [fileformat.info copy (1994)][ff-info-st], [umich picfmts.doc (1991)][umich-picfmts], [crawlycrypt picfmts.doc (1990)][crawly-picfmts]
2. **Just Solve the File Format Problem**: [Atari graphics formats][js-atari]. The per-format pages give file sizes, magic bytes, specification links and sample-file links. Some pages copy the AFW text, and a few add facts not found elsewhere: Palette Master, Extended DEGAS (PI4-PI9), Falcon True Color/XGA, MegaPaint, EZ-Art height and DeskPic's trailing palette.
3. **The Atari Compendium** (Scott Sanders), [Appendix C: Native File Formats][compendium-c]: GEM `.IMG`/XIMG, GDOS `.FNT` and GEM metafile. [Chapter 5: Hardware][compendium-5] covers video modes, word-interleaved bitplanes and Falcon RGB565.
4. **"Understanding color IMGs"** by Dr. Bob (1992), on the AtariForumWiki: [IMG_file][afw-colorimg]. It gives exact header layouts for four colour GEM IMG dialects: NOSIG, HyperPaint, XIMG and STTT.
5. **Encyclopedia of Graphics File Formats, Atari ST summary**: [fileformat.info/format/atari/egff.htm][egff-atari]

## 2. Atari ST/STE

### Hardware references

- [Atari ST/STe/MSTe/TT/F030 Hardware Register Listing][hwreg] (AtariForumWiki mirror). Shifter resolution `$FF8260` (0 = 320x200x4 planes, 1 = 640x200x2, 2 = 640x400x1). Palette `$FF8240-$FF825E`, with the bit layout shown for both ST and STe.
- [Atari Compendium ch. 5 "Video Hardware"][compendium-5]: resolution table per machine. Screen memory is **word-interleaved** bitplanes: for each 16-pixel group, one word per plane, plane 0 first, MSB = leftmost pixel.
- [Introduction to the STE][afw-ste] (AtariForumWiki): STE palette extension, hardware scrolling, and so on.
- [Atari-Forum: Shifter documentation thread][af-shifter]: Shifter internals. Useful background for overscan formats (DUO, KID, PCI, MPP) and Spectrum-style per-scanline palettes.
- Mid-line palette-change timing needed by multi-palette formats is captured as formulas in the format docs themselves:
  - Spectrum 512: `FindIndex()` by Steve Belczyk, public domain, on the [Spectrum 512 page][afw-spu].
  - PhotoChrome, QuantumPaint and HighresMedium: index functions by Hans Wessels, public domain, on their pages.

### Palette

- **ST (9-bit, 512 colours)**: word `.....RRR .GGG.BBB`. Each channel is 0-7.
- **STE (12-bit, 4096 colours)**: word `....rRRR gGGGbBBB`. The new LSB (lowercase) sits *above* the old 3 MSBs for compatibility. To get a 4-bit value: `((v & 7) << 1) | ((v >> 3) & 1)`.
- Sources: [hardware register listing][hwreg]; [Just Solve: Atari ST color palette][js-palette]. The Just Solve page notes that some non-STE files carry garbage in the unused bits, so STE autodetection is heuristic. It also documents the 15-bit "Spectrum 512 Enhanced" layout: `R0 G0 B0 x R1 R4 R3 R2 | G1 G4 G3 G2 B1 B4 B3 B2`.
- Monochrome (high res): DEGAS states that bit 0 of palette word 0 selects black-on-white (set) or white-on-black.
- Some programs store VDI-style palettes instead: 3 words per entry in the range 0-1000, in VDI pen order rather than hardware index order. Examples are Funny Paint, Prism Paint and XIMG. Canvas `.CNV` uses 3 bytes per entry (0-7).

### Common compression and packers

- **PackBits** (DEGAS Elite PC?, IFF, EZA, Prism Paint, QuantumPaint): [AFW PackBits][afw-packbits].
- **CrackArt RLE**, which DelmPaint reuses: on the [CrackArt page][afw-ca].
- **Tiny-style vertical column order**, which QuantumPaint reuses: on the [Tiny page][afw-tny].
- **Whole-file packers.** Many files on disk are wrapped by a transparent packer, with magic `ICE!`/`Ice!` (Pack-Ice), Atomik, and others. See the Just Solve list under "Atari ST transparent file compression" on [Atari graphics formats][js-atari], and [Just Solve: Pack-Ice][js-packice]. Formats that are *always* Ice-packed: HRM, PCI and SPX v1 (Ice 2.10). SPX v2 can also be Ice 2.10 or Atomik 3.5 packed. Pack-Ice references:
  - [Ancient][ancient] (BSD-2-Clause) includes a Pack-Ice decompressor.
  - Original Pack-Ice 68k assembly source: [dhs.nu][dhs-ice]. License unstated.
  - Prose: [Packing Algorithms, by Axe of Superior][axe-packing].
  - `unice68` (part of sc68) is GPL; **do not read** it.
- **LZ4** (PL4): use the public LZ4 block/frame specification. The page does not say which framing is used, so verify against samples.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| ART | Art Director | **Spec** | [AFW][afw-artdir], [JS][js-artdir] | 32512 B fixed. 32000 B low-res screen, then 8 palettes × 16 words, 8 display-time bytes and 248 unknown bytes. Use the first palette with a non-zero time. |
| ART | GFA Artist | **Partial** | [AFW][afw-gfaart], [JS][js-gfaart] | "1000 colours OFF": 32032 B (palette + screen). "1000 colours ON": 34360 B (4 planes word + screen + 70 palettes + cycling tables). How the 69 raster palettes map to pixels is **unknown**. |
| ART | Palette Master | **Spec** | [JS][js-palmaster] | 36864 B. 32768 B low-res picture area (last 768 bytes unused), then 4096 B of palettes: the first is 16 colours; later ones are `[line, colours 1..15]` (colour 0 is shared), terminated by line `-1`. |
| BIL | ColorSTar | **Partial** | [bitplane/datatypes PR #46][bp-pr46] | Said to be either the GFA Artist layout or a low-res DEGAS PI1. The only source is an MIT project validated against RECOIL *output*; see section 6. 320x200x16. |
| BL1/BL2/BL3 | DEGAS Elite block | **Spec** | [AFW][afw-bl], [AFW IFF][afw-iff] | Plain IFF ILBM (BMHD/CMAP/BODY) with a DEGAS extension. Samples in DEGELITE/BLOCKS. |
| BLD | MegaPaint | **Spec** | [AFW][afw-bld], [JS][js-bld] | 4-byte header: width-1 (negative = compressed) and height-1. 1 bpp, white = 0. RLE: bytes 0x00 and 0xFF are followed by a count. JS gives the "+1" semantics correctly; AFW is ambiguous. |
| BP1/BP2/BP4, C01/C02/C04 | UIMG | **Spec** | [uconvert README][uconvert-readme] | The `UIMG` header and palette variants are documented in the README of mikrosk/uconvert (**GPL-3.0 repo: read the README only, not the code**). `BPn` = n bitplanes, `C0n` = chunky. Falcon variants are in section 4. |
| BRU | DEGAS Elite brush | **Partial** | [JS][js-bru] | Always 64 B; RECOIL lists it as 8x8 mono. Byte layout not documented. Samples: [DEGELITE/BRUSHES][degelite-bru]. |
| CA1/CA2/CA3 | CrackArt | **Spec** | [AFW][afw-ca], [JS][js-ca] | `CA` + compression byte + resolution byte + palette (16/4/0 words) + 32000 B or CrackArt RLE (escape/delta/offset scheme). |
| CE1/CE2/CE3 | ComputerEyes | **Spec** | [AFW][afw-ce], [JS][js-ce] | Magic `EYES`, 22 B header. CE1: 3 × 64000 B R/G/B planes, 6-bit, column-major. CE2: 640x200 words 0RRRRRGGGGGBBBBB, column-major. CE3: 640x400 bytes, sum R+G+B (0-191), column-major and interlaced. |
| CEL | Cyber Paint Cell | **Spec** | [AFW][afw-cel], [JS][js-cel] | 128 B header (`$FFFF`, palette, x/y/w/h). Uncompressed low-res planes up to 320x200. |
| CMP | Public Painter | **Spec** | [AFW][afw-cmp], [JS][js-cmp] | 2 B header (escape flag, size 0 = 640x400 / 200 = 640x800). Mono RLE with public-domain C decoder on the page. |
| COL | Atari Image Manager | **None** | [Atarimania][atarimania-aim] | RECOIL list: up to 256x256, 24-bit. No layout found. Program from TU Delft, 1988. |
| CP3 | Picworks | **Spec** | [AFW][afw-cp3] | Always 640x400 mono, compressed. Public-domain C decoder (Pursell, ported from Picworks' GFA source). |
| CPT | Canvas | **Spec** | [AFW][afw-cpt], [Canvas 1.17 manual][canvas-manual], [JS][js-canvas] | 16-word palette, resolution word, then run-length lists (count, offset, 4/2/1 words) and gap-fill data → 32000 B. |
| CPT+HBL | Canvas | **Partial** | [Canvas 1.17 manual][canvas-manual], [JS][js-canvas] | Manual: HBL = 100 words, one per 4th scanline, each a "#palette" number or -1 = no change. Where those palettes come from is not specified. Samples in canvas17 `_HBLS`. |
| CRG | Calamus Raster Graphic | **Spec** | [AFW][afw-crg], [JS][js-crg] | `CALAMUSCRG`, 42 B header (width/height longs at 20/24), byte RLE, 1 bpp, white = 0. Some header fields unknown but not needed. |
| DA4 | PaintShop | **Spec** | [AFW][afw-psc] | 64000 B raw 640x800 mono. |
| DOO, ART | Doodle | **Spec** | [AFW][afw-doo], [JS][js-doo] | 32000 B raw hi-res screen. JS notes that some `.DOO` files are actually low or medium res. The `.ART` 32000-byte variant matches MonoSTar per [PR #46][bp-pr46]. |
| DU1/DUO | DUO (Anders Eriksson) | **Spec** | [AFW][afw-duo], [JS][js-duo] | 113600 B: 16-word palette + 2 × 56784 B overscan screens (416x273), flicker-blended. |
| DU2 | DUO | **Spec** | [AFW][afw-duo] | Medium-res 832x273: 4-word palette + 2 screens (+24 B optional pad). |
| EZA | EZ-Art Professional | **Spec** | [AFW][afw-eza], [JS][js-eza] | `EZ` + 44 B header + PackBits low-res. JS says the word at offset 2 is the **height** (200-640) and calls the AFW version possibly misleading. Verify with samples. |
| FLF | Turbo Rascal Syntax Error | **None** | [JS][js-trse] | The format is defined only by the TRSE source (**GPL-3.0, do not read**). Would need reverse engineering of sample files. |
| FNT | 8x16 font | **Spec** | [AFW DEGAS Elite Font][afw-degfnt] | 128 chars × 16 bytes = 2048 B, plus an optional 1-word half-height flag (2050 B). |
| FNT | GDOS font | **Spec** | [AFW][afw-gdosfnt], [Compendium App. C][compendium-c] | 88 B header (little-endian unless flag bit 2 is set), offset table, single raster form. |
| FUL | Canvas | **Partial** | [Canvas 1.17 manual][canvas-manual] | "Compact Picture + HBL + Animate (SEQ) data in one file". The concatenation order is not stated; CPT, HBL and SEQ are individually described in the manual. |
| GFB | DeskPic | **Spec** | [AFW][afw-gfb], [JS][js-gfb] | `GF25`, colours, width, height, data size (longs), raw planar data, then a 768 B colour table (per JS). |
| GRX | Grafix | **None** | [Atarimania: Grafix Art][atarimania-grafix] | STOS-based paint program. No layout found. |
| HPK/LPK/MPK | Dali (compressed) | **Spec** | [AFW][afw-dalipk], [JS][js-dali] | Palette, ASCII sizes, byte-count table + long table, column-wise expansion. Public-domain C on the page. Resolution comes from the extension. |
| HRM | HighresMedium | **Spec** | [AFW][afw-hrm] | Ice-packed. Unpacked: 64000 B (640x400 interlaced medium) + 28000 B palettes (70 B per line). Public-domain `find_hrm_index()`. RECOIL lists 640x200 × 2 frames. |
| IC1/IC2/IC3 | Imagic | **Spec** | [AFW][afw-imagic], [JS][js-imagic] | `IMDC` 65/68 B header, escape RLE. Delta frames reference a base picture by name. |
| ICN | DEGAS Elite icon | **Spec** | [AFW][afw-icn], [JS][js-icn] | ASCII C source (`#define ICON_W/H`, hex word array), like XBM. |
| IFF | Interchange File Format | **Spec** | [AFW][afw-iff], [JS ILBM][js-ilbm] | ILBM. The AFW page also documents ST DPaint's `VDAT` vertical-RLE BODY. Use the EA IFF/ILBM spec for the rest. |
| IM | Atari Image Manager | **None** | [Atarimania][atarimania-aim] | RECOIL list: up to 256x256, 256 grey levels. No layout found. |
| IMG | GEM Bit Image | **Spec** | [AFW][afw-img], [Compendium App. C][compendium-c], [seasip][seasip-img], [colour IMG doc][afw-colorimg], [JS][js-gem] | Header word count at word 1. Pattern/solid/literal runs and scanline-repeat records. Colour dialects (NOSIG, HyperPaint, STTT) are in the colour IMG doc. |
| KID | Fullscreen Construction Kit | **Partial** | [JS][js-kid], [AtariCrypt][ataricrypt-kid], [PR #46][bp-pr46] | 63054 B, magic `KD`, 448x272/274 overscan × 16 colours, said to be built from four DEGAS pieces. Internal layout not documented. |
| MPP | Multi Palette Picture | **Spec** | [zerkman blog][zerkman-mpp], [github zerkman/mpp][zerkman-gh], [JS][js-mpp] | Reference encoder/decoder by Zerkman is **WTFPL** (permissive) and can be read freely. 4 modes (3 ST, 1 STE), up to 416x273, 1 or 2 frames. The repository moved to codeberg.org/zerkman/mpp. |
| MUR+PAL | C.O.L.R. Object Editor | **Partial** | [AFW][afw-mur], [JS][js-colr] | MUR = 32000 B raw low-res. PAL = 96 B, encoding not documented (likely 16 × 3 words or 48 words; verify with the single known sample). |
| NEO | NEOchrome | **Spec** | [AFW][afw-neo], [JS][js-neo] | 128 B header + 32000 B = 32128. AFW also documents the 640x400 "virtual canvas" variant (flag `0xBABE`, 128128 B). |
| NEO | NEOchrome Master | **Spec** | [neochrom.txt on AFW][afw-neomaster], [JS][js-neomaster] | NEOchrome Master saves rasters only in IFF form: ILBM + a `RAST` chunk whose content equals an `.RST` file restricted to the used rasters. |
| NEO+RST | NEOchrome Master | **Spec** | [neochrom.txt on AFW][afw-neomaster] | RST = up to 200 × {word y_position, 16-word palette}. The first entry is the VBL palette; a later y = 0 means inactive; entries may be unsorted; `$FFFF` or EOF ends the list. |
| OBJ | ColorSTar object | **None** | [PR #46][bp-pr46] (lists it as unsupported) | RECOIL list: up to 320x200x16. No layout found. |
| OBJ | MonoSTar object | **None** | [Happy Computer 12/1986 review][hc-monostar] | RECOIL list: up to 640x400 mono. The magazine review describes the feature only, not the layout. |
| P3C | D-GRAPH | **None** | [JS][js-dgraph] | RECOIL list: compressed, 320x200, 136 colours, 2 frames (interlaced). Nothing else known. |
| PA3 | Pablo Paint 2.5 | **Partial** | [AFW][afw-pablo], [JS][js-pablo] | Header documented (43 B id, ASCII size, resolution, compression type). The **compression (type 29) is undocumented**; AFW points to 68k decoder code at gfa.atari-users.net (license unknown). Hi-res. |
| PAC | STAD | **Spec** | [AFW][afw-stad], [JS][js-stad] | `pM85` (horizontal) / `pM86` (vertical) + id/pack/special bytes, byte RLE, 640x400 mono. [ataripac2pbm][ataripac2pbm] exists (license not checked). |
| PBX | QuantumPaint | **Spec** | [AFW][afw-pbx] | 128 B header with a mode byte (128-colour low, 32-colour medium, 512, 4096). 8 × 48 B palette records or per-line palettes. PackBits + Tiny column order. Public-domain `find_pbx_index()`. Some header bytes unknown. |
| PC1/PC2/PC3 | DEGAS Elite compressed | **Spec** | [AFW][afw-pc], [JS][js-degas] | Resolution word \| 0x8000, palette, PackBits per scanline per plane, 32 B animation tables at the end. |
| PCI | Tobias Richter Fullscreen Slideshow | **Spec** | [AFW "Overscan Interlaced"][afw-pci], [JS][js-pci] | Ice-packed. Unpacked: 2 × 48928 B screens (352x278, 4 **separate** plane blocks) + 2 × 8896 B per-line palettes. |
| PCS | PhotoChrome | **Spec** | [AFW][afw-pcs], [JS][js-pcs] | 6 B header (320, 200, mode flags). RLE screen(s) + word-RLE palettes (9616 entries), with optional XOR second screen. Public-domain `find_pcs_index()`. |
| PG0 | Paintworks | **Spec** | [AFW][afw-pw], [JS][js-pw] | 128 B header with `ANvisionA` at 0x36 and a flags byte. Double-height page, 320x400. Byte RLE (public-domain C) then plane de-interleave. |
| PG1/PG2/PG3 | Graphics Processor | **Partial** | [JS][js-gp], [PR #46][bp-pr46] | Samples are 32331 B. PR #46 says raw (modes 0-2) or RLE (modes 10-12); the header is not documented. 320x200x16 / 640x200x4 / 640x400 mono. Extensions **collide** with Paintworks PG1/PG2. |
| PG1/PG2 | Paintworks | **Spec** | [AFW][afw-pw] | Same container as PG0: 640x400x4 / 640x800 mono double-height pages. |
| PI1/PI2/PI3 | DEGAS | **Spec** | [AFW DEGAS][afw-pi], [AFW DEGAS Elite][afw-pie], [JS][js-degas], [Compendium/Canvas manual][canvas-manual] | 32034 B (DEGAS) or 32066 B (Elite, with animation tables). RECOIL also accepts larger PI1 variants (up to 416x560). |
| SUH | DEGAS (hi-res) | (part of the PI3 entry) | [JS DEGAS image][js-degas] | RECOIL lists it together with PI3. JS lists `.SUH` as an alternate DEGAS Elite hi-res extension (32066 B). Treat it as PI3. Not counted separately. |
| PIC | PaintPro ST / PlusPaint ST | **Spec** | [AFW][afw-pic], [JS][js-paintpro] | 32034 B = DEGAS. 64034 B = DEGAS with a double-height bitmap. |
| PL4 | PL4 (Sascha Springer) | **Spec** | [AFW][afw-pl4], [JS][js-pl4] | LZ4-compressed. Unpacked: 2 DEGAS-like low-res screens, each with a 32-word palette, 64070 B. Two-frame blend. |
| PPP | Pablo Paint 1.1 | **Partial** | [AFW][afw-pablo] | Same container as PA3 but low-res. Type 29 compression is undocumented. |
| PSC | PaintShop | **Spec** | [AFW][afw-psc], [JS][js-psc] | `tm89PS`, 14 B header, scanline opcodes (0/10/12/99/100/102/110/200/255). 640x400 mono. |
| RGB | RGB Intermediate | **Spec** | [AFW][afw-rgb], [JS][js-rgbint] | 3 × PI1 (R, G, B gun values 0-15 per pixel), 96102 B. |
| RGH | ZZ_ROUGH | **Spec** | [AFW][afw-rgh], [JS][js-rgh] | `(c)F.MARCHAL`, ASCII size, palette, then the Dali-compressed scheme. |
| CL0/SC0, CL1/SC1, CL2/SC2 | Paintworks | **Spec** | [AFW][afw-pw], [JS][js-pw] | 128 B header, raw or RLE + plane reorder. Low, medium and high res (3 table entries). |
| SD0/SD1/SD2 | Dali | **Spec** | [AFW][afw-dali], [JS][js-dali] | 32128 B: long 0 + palette + 92 B + 32000 B. Resolution from the extension. |
| SPC | Spectrum 512 (compressed) | **Spec** | [AFW][afw-spc], [JS][js-spectrum] | `SP`, data RLE (31840 B, separate planes), 597 bit-vector-compressed palettes. Uses Belczyk `FindIndex`. |
| SPS | Spectrum 512 (smooshed) | **Spec** | [AFW][afw-sps], [JS][js-spectrum] | `SP`. Two data orders, selected by the LSB of the last colour-map byte. 14-bit header + 9-bit RGB bitstream palettes. The Anispec `SS` variant has an unknown animation tail. |
| SPU | Spectrum 512 | **Spec** | [AFW][afw-spu], [JS][js-spectrum] | 51104 B: 160 B blank line + 31840 B + 597 × 16-word palettes. Public-domain `FindIndex()`. |
| SPU | Spectrum 512 (enhanced) | **Spec** | [AFW][afw-spue], [JS palette][js-palette] | `5BIT` in the first long. The palette's top 3 bits add a 5th bit per channel (15-bit colour). |
| SPX | Spectrum 512 (extended) | **Spec** | [AFW][afw-spx], [JS][js-spx] | `SPX` header + author/description strings. More than 200 lines. Ice 2.10 (v1) or a custom backward LZ packer that is fully described (v2). |
| SRT | Synthetic Arts | **Spec** | [AFW][afw-srt], [JS][js-srt] | 32038 B: 32000 B medium-res screen + 3 words + palette at the end. PR #46 reports a `JHSy` signature. |
| SSB | Sinbad Slideshow | **Partial** | [JS][js-ssb], [PR #46][bp-pr46] | Always 32768 B, 320x200x16. Probably 32000 B screen + palette + padding, but the layout is unconfirmed. |
| TN1/TN4, TN2/TN5, TN3/TN6, TNY | Tiny Stuff | **Spec** | [AFW][afw-tny], [JS][js-tiny], [TINYST.DOC][tinyst-doc] | Resolution byte (+3 = colour animation follows), palette, control/data counts, word RLE, 4-set vertical column order. TN4-6 are the animated variants. 4 table entries. |
| XIMG | Extended GEM Bit Image | **Spec** | [colour IMG doc][afw-colorimg], [Compendium App. C][compendium-c], [JS][js-gem] | Version 2, `XIMG` at offset 16, a zero word, VDI 0-1000 RGB triplets (3 words per colour), then separate planes with standard IMG compression. |

Notes on counting: these rows expand to RECOIL's 95 ST/STE entries. Split entries count separately:
- BL1-3, CA1-3, CE1-3, IC1-3, PC1-3 and SD0-2: 3 each
- HPK, LPK and MPK: 3
- PI1, PI2 and PI3/SUH: 3
- Graphics Processor PG1-3: 3, all Partial
- Paintworks SC/CL pairs: 3
- Tiny groups: 4

UIMG BP1-C04 is a single entry.

## 3. Atari TT

### Hardware references

- [Hardware Register Listing][hwreg]: TT shifter mode `$FF8262` bits 10-8 select the mode:
  - `000` 320x200x4
  - `001` 640x200x2
  - `010` 640x400x1
  - `100` 640x480x4
  - `110` 1280x960x1
  - `111` 320x480x8

  The TT palette is 256 words at `$FF8400-$FF85FE`.
- [Atari Compendium ch. 5][compendium-5]: TT030 mode table. The data is still word-interleaved bitplanes; 8 planes for TT-low.
- [TT030 specifications][tt-spec]

### Palette

- **TT palette word**: `....RRRr GGGgBBBb`, a straight 4-bit-per-channel layout (12-bit). Unlike STE, the bits are not scrambled; in the listing, lowercase marks the LSB in normal position. Source: [hwreg].
- In ST-compatible modes, the TT palette bank bits (`$FF8262` bits 3-0) select a 16-colour bank.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PI4 | DEGAS-style TT low (View ST/TT) | **Spec** | [viewttst.txt][viewttst], [AFW DEGAS][afw-pi], [JS Extended DEGAS][js-extdegas] | Resolution word `$0007`, 256-word TT palette, 153600 B (320x480x8 planes) = 154114 B. The extension **collides** with Fuckpaint PI4 on the Falcon (77824 B); tell them apart by size. |
| PI5 | DEGAS-style TT medium (View ST/TT) | **Spec** | [viewttst.txt][viewttst], [AFW DEGAS][afw-pi] | Resolution word `$0004`, 16-word palette, 153600 B (640x480x4) = 153634 B. |
| PI6 | DEGAS-style TT high | **Hardware-only** | [JS Extended DEGAS][js-extdegas], [hwreg] | No document found. Presumably the DEGAS-style header (resolution word `$0006`?) + palette + 153600 B (1280x960x1). **Same 153600 B bitmap size as PI5**, so check the resolution word. The [A-to-Z list][atoz] confirms the extension family exists. |

## 4. Atari Falcon

### Hardware references

- [Hardware Register Listing][hwreg]: VIDEL palette at `$FF9800-$FF98FC`, 256 longs.
- [mikro.naprvyraz.sk: VIDEL registers][videl]: the best description of the VIDEL registers.
- [Atari Compendium ch. 5][compendium-5]:
  - Modes are 1/2/4/8 bpp word-interleaved bitplanes, or 16 bpp packed true colour.
  - True-colour word: `RRRRRGGGGGGBBBBB` (RGB565).
  - In overlay mode, green bit 5 is the genlock flag.
  - Widths of 320 or 640 pixels, overscan supported.

### Palette

- **VIDEL palette entry (long)**: `RRRRRRxx GGGGGGxx 00000000 BBBBBBxx`. 6 bits per channel (262144 colours); the low 2 bits of each byte are ignored. Files that copy the registers store "R, G, 0, B" bytes (DelmPaint, DuneGraph, Fuckpaint, RAG-D with 1024 B palettes). Sources: [hwreg], [uconvert README][uconvert-readme].
- In ST-compatible 4-bpp modes, the Falcon still honours the ST/STE word palette at `$FF8240`.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| B&W, B_W | ImageLab | **Spec** | [AFW][afw-imagelab] | `B&W256` + width + height (10 B). 8-bit grey, 0 = black. |
| BP6/BP8, C06/C08/C16/C24/C32 | UIMG | **Spec** | [uconvert README][uconvert-readme] | Header: `UIMG`, version, flags (palette type: none/ST/TT/Falcon/VDI), bpp, bytesPerChunk (-1 = bitplanes), width, height. Palette of 1<<bpp entries; 16-bit data is RGB565. **README only; the repo is GPL-3.0.** |
| DC1 | DuneGraph (compressed) | **Spec** | [AFW][afw-dune], [JS][js-dune] | `DGC` + method (0-3) + x + y + word + 256 Falcon longs. Byte/word/long RLE on separated planes. |
| DEL | DelmPaint | **Spec** | [AFW][afw-delm], [JS][js-delm] | 3 CrackArt-packed blocks, each unpacking to 32000 B. Then 1024 B palette + 320x240x8 planes. |
| DG1 | DuneGraph | **Spec** | [AFW][afw-dune] | `DGU` + version + x + y + 1024 B palette + 64000 B (320x200x8 planes). |
| DPH | DelmPaint | **Spec** | [AFW][afw-delm] | 11 CrackArt-packed blocks. Unpacked: palette + 4 × 320x240 quadrants interleaved into 640x480. |
| ESM | TmS Cranach Paint/Studio | **Spec** | [AFW][afw-esm], [JS][js-esm] | `TMS\0`, 812 B header with separate R/G/B 256-byte palette tables. 1/8/24 planes, chunky. |
| FTC | Falcon True Color | **Hardware-only** | [JS][js-ftc] | Raw 384x240 RGB565, exactly 184320 B, no header. |
| FUN | Funny Paint | **Spec** | [AFW][afw-fun], [JS][js-fun] | Magic 0x000ACFE2, 13 B header, 1-16 planes, multiple frames. Trailing VDI palette. |
| GOD | GodPaint | **Spec** | [AFW][afw-god], [JS][js-god] | 6 B header (unreliable id, x, y), then RGB565. Usually 153606 B. |
| HIR | Print-Technik | **Spec** | [AFW][afw-hir], [JS][js-printtechnik] | `0x0F0F 0x0001` + w + h + word. 1 byte per pixel, 0-127 grey. |
| IB3 | ICDRAW icon (group) | **Partial** | [JS][js-icdraw], [ICDRAW 1.4 distribution and docs][icdraw-dir] | Magic `ICB3`, 3 icons, 32x32x16 colours. The byte layout is not in the bundled docs. Samples in `icdraw14/icons`. |
| IBI | ICDRAW icon (single) | **Partial** | [JS][js-icdraw], [ICDRAW docs][icdraw-dir] | Magic `ICBI`, 32x32x16 colours. Same gap as IB3. |
| IIM | InShape | **Spec** | [AFW][afw-iim], [JS][js-iim] | `IS_IMAGE`, 16 B header. Types 1-bit / 8-bit grey / 24-bit RGB / 32-bit ARGB. |
| PI4, PI7, PI9 | Fuckpaint | **Spec** (PI7 only Partial) | [AFW][afw-fp], [JS Extended DEGAS][js-extdegas] | PI4/PI9: 1024 B Falcon palette + 76800 B 8-plane = 77824 B (PI4 = 320x240; PI9 = 320x200, only the first 200 rows used, sometimes 65024 B). PI7: JS reports 640x480x8 planes, 308224 B (= 1024 + 307200), without a written spec. Counted as one Spec entry. |
| PIX | PixArt | **Spec** | [AFW][afw-pix], [JS][js-pixart] | `PIXT`, version, type, planes, x, y. 1-32 bpp; planar or chunky by type. RGB palette for 2/4/8 bpp. |
| PNT, TPI | Prism Paint / TruePaint | **Spec** | [AFW][afw-pnt], [JS][js-pnt] | `PNT\0`, 128 B header, VDI 0-1000 palette. Planes interleaved for depths below 16 bpp. PackBits per plane per line (16/24 bpp are compressed "as planes"). |
| RAG | RAG-D | **Spec** | [AFW][afw-rag], [original FORMAT.TXT][rag-format], [JS][js-ragd] | `RAG-D!`, pack algorithm (0 only), length quirk (Appendix A), cols/rows/planes, palette 32 B (ST) or 1024 B (Falcon). Optional 370 B text and date tail. |
| RAGC | Music Compile 2 | **None** | — | RECOIL list: arbitrary size, 256 colours. Nothing found. The RAG-D container is a plausible relative (unverified). |
| RAW, RWH, RWL | IMG Scan | **Spec** | [AFW][afw-imgscan], [JS][js-imgscan] | Headerless 8-bit grey, 0 = **white**. RWL 320x200 (64000 B), RWH 640x400 (256000 B), RAW 640x200 (128000 B). |
| TCP | Rembrandt | **Spec** | [AFW][afw-tcp], [JS][js-rembrandt] | `TRUECOLR` 18 B header, then per image `PICT` 198 B header + RGB565. |
| TG1 | COKE | **Spec** | [AFW][afw-coke], [JS][js-coke] | `COKE format.` + w + h + data offset (18 B), then RGB565. |
| TIMG | TrueColor IMG (32768 colours) | **None** | [JS GEM Raster][js-gem] | Only named as a GEM IMG dialect ("TrueColor IMG"). The header layout was not found in any clean source. |
| TRE | Spooky Sprites (RLE) | **Spec** | [AFW][afw-spooky], [JS][js-spooky], [SPOOKY.TXT][spooky-txt] | `tre1` + w + h + chunk count. Alternating raw/RLE chunks of RGB565 words; byte counts escape to a word when the byte is 255. |
| TRP | EggPaint | **Spec** | [AFW][afw-egg], [JS][js-egg] | `TRUP` + w + h, then RGB565. Often Pack-Ice wrapped. |
| TRP | Spooky Sprites | **Spec** | [AFW][afw-spooky], [JS][js-spooky] | `tru?` + w + h, then RGB565 (`.TRU` also seen). |
| TRU | IndyPaint | **Spec** | [AFW][afw-indy], [JS][js-indy] | `Indy` + x + y + 248 zero bytes, then RGB565. |
| XGA | XGA (raw Falcon hi-colour) | **Hardware-only** | [JS][js-xga], [Falcon Image Utilities readme][xga-readme] | Raw RGB565, no header. 153600 B = 320x240, 368640 B = 384x480. Sometimes Pack-Ice wrapped. |

## 5. Sample files

- [Sembiance/Dexvert file-format samples][dexvert-samples]: almost every Just Solve page links a per-format folder, for example `image/neochrome/`, `image/spectrum512U/` and `image/xga/`. Also known as the "telparia" archive.
- [cd.textfiles.com][textfiles], the main source of real-world files. Folders per format are linked from each JS page, for example:
  - [suzybatari2/degas][tf-degas]
  - [geminiatari/FILES/GRAPHICS/SPECPICS][tf-spec] (SPC/SPS/SPX)
  - [suzybatari2/pcs][tf-pcs]
  - [806atari DEGELITE][degelite-bru] (BRU/BL?/ICN)
  - [canvas17 _picture][tf-canvas]
  - [Palette Master][tf-palmaster]
  - [Fuckpaint][tf-fpaint]
  - [ICDRAW icons][icdraw-dir]
- [samples.libav.org/image-samples/atarist/][libav-samples] (degas, neochrome, spectrum512, tinystuff, crackart, doodle, photochrome). Did not respond during this survey; try archive.org.
- [Atari Graphics Archive (Lonny Pursell)][gfa-archive]. Did not respond during this survey.
- [MPP gallery][zerkman-gallery]
- [no-fragments.atari.org archive][nofrag-egg]: Falcon EggPaint, PI9 and Spooky samples (zip files linked from JS pages).
- [RECOIL examples.zip][recoil-examples] contains *sample images only* (PI4, PI9, PAC, PCS, NEOchrome Master FISH.neo and others). Downloading it does not expose code, but check the archive contents before use.

## 6. To avoid (GPL code, or code derived from RECOIL)

| Project | License | Why it is listed |
|---|---|---|
| RECOIL (all languages, ports and forks) | GPL | The project being reimplemented. Only the [format list page][recoil-list] was read. |
| GrafX2 (its "Atari ST picture formats" module) | GPL-2.0 | Has ST format loaders. **Do not read.** |
| mikrosk/uconvert (`ushow`, converter code) | GPL-3.0 | The UIMG spec is in its README, which was the only part read. **Do not read the code.** |
| TRSE (Turbo Rascal Syntax Error) | GPL-3.0 | FLF is defined only by its source. **Do not read.** |
| unice68 / sc68 | GPL | Pack-Ice depacker. Use Ancient (BSD-2) or the original Pack-Ice docs instead. |
| roytam1/stb_gemras | Public domain / MIT | Its README says the extended GEM IMG coverage came from "RECOIL's `RECOIL_DecodeStImg`". **Treat as tainted. Do not read.** |
| bitplane/datatypes PRs [#44][bp-pr44] and [#46][bp-pr46] | MIT | Atari ST/TT/Falcon loaders cross-checked against RECOIL **output only** (the PRs state that RECOIL source was not read). They are cited above only for small facts (ColorSTar BIL, Graphics Processor RLE modes, `JHSy`). The lead should decide whether that provenance is acceptable before anyone reads the code. |

Permissive or non-GPL references that may be consulted:
- [Deark][deark] (MIT-style; supports DEGAS, NEO, Spectrum, PNT, GEM IMG, CRG, EZA, BLD and more).
- [Ancient][ancient] (BSD-2-Clause; Pack-Ice and other packers).
- Zerkman's MPP reference software (WTFPL).
- Netpbm converters `pi1toppm`, `pi3topbm`, `pc1toppm`, `sputoppm`, `spctoppm` (per-file licenses; check each).
- Licenses **not verified**: abydos (snisurset.net), wuimg (codeberg kaleido/wuimg) and ataripac2pbm. Check them before reading.

## 7. Implementation notes (retro-image)

Facts found while matching `recoil2png` output (black box) or reading sample files. The
decoders live in `crates/retro-image/src/platform/atari_st/`.

- Output conventions: 3-bit components are bit-replicated, a palette using any STE bit is
  read as STE, medium resolution doubles lines, monochrome set bits are black. VDI levels
  scale as `v * 255 / 1000` (truncated); VDI palettes are in pen order (register 15 shows
  pen 255 in 256-colour mode). Two-screen formats (DUO, PCI, PL4, HRM, PCS, MPP, PBX 4096)
  average the two screens per component.
- GEM IMG: STTT files store whole planes one after another; others interleave plane rows
  per line, and runs may cross line ends. Images without a palette use the default VDI
  colours. 16/24/32-"plane" XIMG files are chunky; TIMG files are real bitplanes holding
  R, G, B fields least significant bit first. FSNAP-style files contain `0, 0, n` records
  whose meaning is still unknown.
- Canvas CPT run offsets count 16-pixel units, not bytes. DuneGraph DC1 may stop after
  the used planes. DelmPaint DPH has 10 blocks, all with lengths. MPP files hold 199 (273)
  lines. Spectrum 512 Extended v2 match offsets are relative to the output position, and
  the line count is what the unpacked data holds (at most 199 per screen).
- Art Director: byte 32287 selects the palette shown. Palette Master: line palettes,
  always 9-bit. GFA Artist 1000 colours: line `y` uses stored palette `2 + ceil(y / 3)`.
  Spectrum 512 Enhanced (`5BIT`): the extra LSBs are bits 14-12, not 15-13.
- Formats the survey rated None or Partial, decoded from sample files: KID (274 lines of
  230 bytes, 224 used), Graphics Processor RLE (count byte, bit 7 = literal, units of one
  byte per plane), D-GRAPH P3C (two CrackArt-packed screens sharing a palette, mixed),
  ICDRAW IB3/IBI (64-byte header, 32x32 interleaved planes, default VDI colours),
  ColorSTar mono OBJ (width-1, height-1, planes, word-aligned rows), Grafix GRX
  uncompressed (256 VDI triplets at 36, data at 1586), Atari Image Manager IM/COL (square
  byte planes; COL = I, R, G, B), PI5 320x240 and PI6 1280x960, Pablo Paint uncompressed.
  NEOchrome Master writes its `RAST` chunk after the FORM, without a pad byte.
- Content detection (`.signature()`) is on for formats with real magic bytes: CRG, GFB,
  PSC, CrackArt, SPC/SPS (`SP` plus the reserved zero word), SPX, GEM IMG (strict header),
  MPP, ComputerEyes, STAD, PhotoChrome, PL4 (LZ4 frame and exact unpacked size),
  Paintworks, UIMG, Pablo Paint, KID, DEGAS Elite icon, Grafix, Imagic and the Falcon formats with
  ID strings (ImageLab, DuneGraph, Print-Technik, InShape, Rembrandt, COKE, EggPaint and
  Spooky TRP/TRE, IndyPaint, TmS Cranach, Funny Paint, PixArt, Prism Paint, RAG-D,
  ICDRAW), and NEOchrome Master, but only for an ILBM FORM directly followed by a `RAST`
  chunk (the Amiga IFF decoder declines those). Without `RAST`, `recoil2png` renders a
  NEOchrome Master FORM as a plain Amiga ILBM even as `.neo`, and so do we (checked
  black-box with FISH.neo renamed and with its `RAST` chunk cut off). Left off on purpose:
  DEGAS Elite blocks (renamed ILBMs already go to the Amiga IFF decoder, which comes first;
  ours would claim Amiga DCTV and HAM-E ILBMs that it rejects), EZ-Art (only a 2-byte `EZ`), Music Compile (RAG-D claims
  the same container first), HRM and PCI (their unpacked form has no header) and every
  headerless or size-only format.
- Companion files (black-box tests with `recoil2png`):
  - Canvas `HBL`: 200 words, one per four lines (`$FFFF` = no change), 400 unused
    bytes, then (used entries + 1) records of 16 VDI pens × 3 bytes (R, G, B; only the
    low three bits count) from offset 800. Records are stored in reverse: the first used
    entry selects the last record; the palette numbers in the table are ignored. The
    record set from line 0 maps pens like 16 colours even in medium resolution (colour 3
    = pen 6). High resolution ignores the HBL. `FUL` = HBL data + 608 bytes of animation
    data + CPT data; RECOIL ignores a sibling `.HBL` for it.
  - NEOchrome `RST` uses the same rules as the `RAST` chunk, ends at a `$FFFF` line, and
    is always shown with 3-bit ST colours (unlike `RAST`, where STE bits switch to STE).
  - `MUR` + `PAL`: the PAL holds 16 VDI triplets (0-1000, clamped) in pen order. RECOIL
    rejects a `MUR` without its PAL, and so do we.
- Imagic, from samples and black-box tests (the AFW page differs): bytes 64-65 are
  `$C8 $02` (RECOIL rejects anything else), the escape byte is at 66 and data starts at
  67. After the escape: the escape itself = literal; `0, n, v` = `n + 1` × `v`;
  `1`×o, any byte, `n`, `v` = `256·o + n + 1` × `v`; `2, 0` = end; `2, 1…` or `2, n≥3`
  = that many bytes from the base picture (zero when absent); `2, 2` = skip through the
  next zero byte; `n≥3, v` = `n + 1` × `v`. The unpacked bytes fill 160 columns of 200
  bytes, top to bottom, in every resolution; a full screen needs no end marker, and
  trailing bytes and the length word are ignored.
- Still unsupported:
  - Grafix compressed (word 28 = 1): the 14 bytes before the data hold the unpacked size
    and the lengths of two streams of high-entropy, LZW-like bit-packed data. No
    documentation was found; it needs a full reverse-engineering effort.
  - Pablo Paint compressed (type 29): no Atari sample exists. `proudnbeauty.ppp` and
    `glance .ppp` (from sembiance's pabloPaint set, moved to `extra/commodore/`) are
    Commodore 64 pictures (RECOIL renders them 296x200 with 120 colours; `.PPP` is also a
    C64 extension).
  - FSNAP-style IMG files and 8-plane IMG without palette.

<!-- link definitions -->
[recoil-list]: https://recoil.sourceforge.net/formats.html
[recoil-examples]: http://recoil.sourceforge.net/examples.zip
[afw-index]: https://temlib.org/AtariForumWiki/index.php/ST_Picture_Formats
[picture-st]: http://www.textfiles.com/programming/FORMATS/picture.st
[ff-info-st]: http://www.fileformat.info/format/atari/spec/6ecf9f6eb5be494284a47feb8a214687/view.htm
[umich-picfmts]: http://www.umich.edu/~archive/atari/Graphics/picfmts.doc
[crawly-picfmts]: http://cd.textfiles.com/crawlycrypt1/program/grfx_snd/picfrmt/picfmts.doc
[egff-atari]: https://www.fileformat.info/format/atari/egff.htm
[js-atari]: http://fileformats.archiveteam.org/wiki/Atari_graphics_formats
[compendium-c]: http://cd.textfiles.com/ataricompendium/BOOK/HTML/APPENDC.HTM
[compendium-5]: http://cd.textfiles.com/ataricompendium/BOOK/HTML/CHAP5.HTM
[afw-colorimg]: https://temlib.org/AtariForumWiki/index.php/IMG_file
[hwreg]: https://temlib.org/AtariForumWiki/index.php/Atari_ST/STe/MSTe/TT/F030_Hardware_Register_Listing
[afw-ste]: https://temlib.org/AtariForumWiki/index.php/Introduction_to_the_STE
[af-shifter]: https://www.atari-forum.com/viewtopic.php?t=21791
[js-palette]: http://fileformats.archiveteam.org/wiki/Atari_ST_color_palette
[afw-packbits]: https://temlib.org/AtariForumWiki/index.php/PackBits_Compression_Algorithm
[js-packice]: http://fileformats.archiveteam.org/wiki/Pack-Ice
[ancient]: https://github.com/temisu/ancient
[dhs-ice]: http://dhs.nu/news.php?t=single&ID=790
[axe-packing]: http://mikro.naprvyraz.sk/docs/Coding/1/PACKING.TXT
[afw-artdir]: https://temlib.org/AtariForumWiki/index.php/Art_Director_file_format
[js-artdir]: http://fileformats.archiveteam.org/wiki/Art_Director
[afw-gfaart]: https://temlib.org/AtariForumWiki/index.php/GFA_Artist_file_format
[js-gfaart]: http://fileformats.archiveteam.org/wiki/GFA_Artist
[js-palmaster]: http://fileformats.archiveteam.org/wiki/Palette_Master
[bp-pr44]: https://github.com/bitplane/datatypes/pull/44
[bp-pr46]: https://github.com/bitplane/datatypes/pull/46
[afw-bl]: https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Block_file_format
[afw-iff]: https://temlib.org/AtariForumWiki/index.php/IFF_file_format
[js-ilbm]: http://fileformats.archiveteam.org/wiki/ILBM
[afw-bld]: https://temlib.org/AtariForumWiki/index.php/MegaPaint_file_format
[js-bld]: http://fileformats.archiveteam.org/wiki/MegaPaint_BLD
[uconvert-readme]: https://github.com/mikrosk/uconvert/blob/master/README.md
[js-bru]: http://fileformats.archiveteam.org/wiki/DEGAS_Elite_brush
[degelite-bru]: http://cd.textfiles.com/806atari/401-500/438/DEGELITE/BRUSHES/
[afw-ca]: https://temlib.org/AtariForumWiki/index.php/CrackArt_file_format
[js-ca]: http://fileformats.archiveteam.org/wiki/Crack_Art
[afw-ce]: https://temlib.org/AtariForumWiki/index.php/ComputerEyes_Raw_Data_file_format
[js-ce]: http://fileformats.archiveteam.org/wiki/ComputerEyes
[afw-cel]: https://temlib.org/AtariForumWiki/index.php/Cyber_Paint_Cell_file_format
[js-cel]: http://fileformats.archiveteam.org/wiki/Cyber_Paint_Cell
[afw-cmp]: https://temlib.org/AtariForumWiki/index.php/Public_Painter_file_format
[js-cmp]: http://fileformats.archiveteam.org/wiki/Public_Painter
[atarimania-aim]: https://www.atarimania.com/utility-atari-st-atari-image-manager_38874.html
[afw-cp3]: https://temlib.org/AtariForumWiki/index.php/Picworks_file_format
[afw-cpt]: https://temlib.org/AtariForumWiki/index.php/Canvas_file_format
[canvas-manual]: http://cd.textfiles.com/crawlycrypt1/graphics/canvas17/manual.txt
[js-canvas]: http://fileformats.archiveteam.org/wiki/Canvas_(Atari)
[afw-crg]: https://temlib.org/AtariForumWiki/index.php/Calamus_Raster_Graphic_file_format
[js-crg]: http://fileformats.archiveteam.org/wiki/Calamus_Raster_Graphic
[afw-psc]: https://temlib.org/AtariForumWiki/index.php/PaintShop_file_format
[js-psc]: http://fileformats.archiveteam.org/wiki/PaintShop_(Atari_ST)
[afw-doo]: https://temlib.org/AtariForumWiki/index.php/Doodle_file_format
[js-doo]: http://fileformats.archiveteam.org/wiki/Doodle_(Atari)
[afw-duo]: https://temlib.org/AtariForumWiki/index.php/DUO_file_format
[js-duo]: http://fileformats.archiveteam.org/wiki/DUO
[afw-eza]: https://temlib.org/AtariForumWiki/index.php/EZ-Art_Professional_file_format
[js-eza]: http://fileformats.archiveteam.org/wiki/EZ-Art_Professional
[js-trse]: http://fileformats.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error
[afw-degfnt]: https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Font_file_format
[afw-gdosfnt]: https://temlib.org/AtariForumWiki/index.php/GDOS_Font_file_format
[afw-gfb]: https://temlib.org/AtariForumWiki/index.php/DeskPic_file_format
[js-gfb]: http://fileformats.archiveteam.org/wiki/DeskPic
[atarimania-grafix]: https://www.atarimania.com/utility-atari-st-grafix-art_28377.html
[afw-dalipk]: https://temlib.org/AtariForumWiki/index.php/Dali_Compressed_file_format
[afw-dali]: https://temlib.org/AtariForumWiki/index.php/Dali_file_format
[js-dali]: http://fileformats.archiveteam.org/wiki/Dali
[afw-hrm]: https://temlib.org/AtariForumWiki/index.php/HighresMedium_file_format
[afw-imagic]: https://temlib.org/AtariForumWiki/index.php?title=Imagic_Film/Picture_file_format
[js-imagic]: http://fileformats.archiveteam.org/wiki/Imagic_Film/Picture
[afw-icn]: https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Icon_file_format
[js-icn]: http://fileformats.archiveteam.org/wiki/DEGAS_Elite_icon
[afw-img]: https://temlib.org/AtariForumWiki/index.php/GEM_Bit_Image_file_format
[seasip-img]: https://www.seasip.info/Gem/ff_img.html
[js-gem]: http://fileformats.archiveteam.org/wiki/GEM_Raster
[js-kid]: http://fileformats.archiveteam.org/wiki/Fullscreen_Construction_Kit
[ataricrypt-kid]: https://ataricrypt.blogspot.com/2018/03/fullscreen-construction-kit.html
[zerkman-mpp]: http://zerkman.sector1.fr/index.php?post/2012/10/08/Atari-ST-Multipalette-Picture-file-format
[zerkman-gh]: https://github.com/zerkman/mpp
[zerkman-gallery]: http://zerkman.sector1.fr/public/mpp/gallery.zip
[js-mpp]: http://fileformats.archiveteam.org/wiki/Multi_Palette_Picture
[afw-mur]: https://temlib.org/AtariForumWiki/index.php/C.O.L.R._Object_Editor_file_format
[js-colr]: http://fileformats.archiveteam.org/wiki/C.O.L.R._Object_Editor
[afw-neo]: https://temlib.org/AtariForumWiki/index.php/NEOchrome_file_format
[js-neo]: http://fileformats.archiveteam.org/wiki/NEOchrome
[afw-neomaster]: https://temlib.org/AtariForumWiki/index.php/Neochrome_Master
[js-neomaster]: http://fileformats.archiveteam.org/wiki/NEOchrome_Master
[hc-monostar]: https://www.stcarchiv.de/hc1986/12/monostar
[js-dgraph]: http://fileformats.archiveteam.org/wiki/D-GRAPH
[afw-pablo]: https://temlib.org/AtariForumWiki/index.php/Pablo_Paint_file_format
[js-pablo]: http://fileformats.archiveteam.org/wiki/PabloPaint
[afw-stad]: https://temlib.org/AtariForumWiki/index.php/STAD_file_format
[js-stad]: http://fileformats.archiveteam.org/wiki/STAD_PAC
[ataripac2pbm]: http://scara.com/~schirmer/o/ataripac2pbm/
[afw-pbx]: https://temlib.org/AtariForumWiki/index.php/QuantumPaint_file_format
[afw-pc]: https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Compressed_file_format
[js-degas]: http://fileformats.archiveteam.org/wiki/DEGAS_image
[afw-pci]: https://temlib.org/AtariForumWiki/index.php/Overscan_Interlaced_file_format
[js-pci]: http://fileformats.archiveteam.org/wiki/Tobias_Richter_Fullscreen_Slideshow
[afw-pcs]: https://temlib.org/AtariForumWiki/index.php/PhotoChrome_file_format
[js-pcs]: http://fileformats.archiveteam.org/wiki/PhotoChrome
[afw-pw]: https://temlib.org/AtariForumWiki/index.php/Paintworks_file_format
[js-pw]: http://fileformats.archiveteam.org/wiki/Paintworks
[js-gp]: http://fileformats.archiveteam.org/wiki/Graphics_Processor
[afw-pi]: https://temlib.org/AtariForumWiki/index.php/DEGAS_file_format
[afw-pie]: https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_file_format
[afw-pic]: https://temlib.org/AtariForumWiki/index.php?title=PaintPro_ST/PlusPaint_ST
[js-paintpro]: http://fileformats.archiveteam.org/wiki/PaintPro
[afw-pl4]: https://temlib.org/AtariForumWiki/index.php/PL4_file_format
[js-pl4]: http://fileformats.archiveteam.org/wiki/PL4
[afw-rgb]: https://temlib.org/AtariForumWiki/index.php/RGB_Intermediate_file_format
[js-rgbint]: http://fileformats.archiveteam.org/wiki/RGB_Intermediate_Format
[afw-rgh]: https://temlib.org/AtariForumWiki/index.php/ZZ_ROUGH_file_format
[js-rgh]: http://fileformats.archiveteam.org/wiki/ZZ_ROUGH
[afw-spc]: https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Compressed_file_format
[afw-sps]: https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Smooshed_file_format
[afw-spu]: https://temlib.org/AtariForumWiki/index.php/Spectrum_512_file_format
[afw-spue]: https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Enhanced_file_format
[afw-spx]: https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Extended_file_format
[js-spectrum]: http://fileformats.archiveteam.org/wiki/Spectrum_512_formats
[js-spx]: http://fileformats.archiveteam.org/wiki/Spectrum_512_Extended
[afw-srt]: https://temlib.org/AtariForumWiki/index.php/Synthetic_Arts_file_format
[js-srt]: http://fileformats.archiveteam.org/wiki/Synthetic_Arts
[js-ssb]: http://fileformats.archiveteam.org/wiki/Sinbad_Slideshow
[afw-tny]: https://temlib.org/AtariForumWiki/index.php/Tiny_file_format
[js-tiny]: http://fileformats.archiveteam.org/wiki/Tiny_Stuff
[tinyst-doc]: http://cd.textfiles.com/atarilibrary/atari_cd01/PICS/PICS.40/TINYST.DOC
[tt-spec]: http://atari-explorer.ctrl-alt-rees.com/specs/Spec-TT.htm
[viewttst]: http://cd.textfiles.com/crawlycrypt1/graphics/view132/viewttst.txt
[js-extdegas]: http://fileformats.archiveteam.org/wiki/Extended_DEGAS_image
[atoz]: http://cd.textfiles.com/atarilibrary/atari_cd10/DISKS/AC10DISK/ATOZBOOK/APPEND_D.TXT
[videl]: https://mikro.naprvyraz.sk/docs/mikro/videl.html
[afw-imagelab]: https://temlib.org/AtariForumWiki/index.php/ImageLab_file_format
[afw-dune]: https://temlib.org/AtariForumWiki/index.php/DuneGraph_file_format
[js-dune]: http://fileformats.archiveteam.org/wiki/DuneGraph
[afw-delm]: https://temlib.org/AtariForumWiki/index.php/DelmPaint_file_format
[js-delm]: http://fileformats.archiveteam.org/wiki/DelmPaint
[afw-esm]: https://temlib.org/AtariForumWiki/index.php/TmS_Cranach_file_format
[js-esm]: http://fileformats.archiveteam.org/wiki/Enhanced_Simplex
[js-ftc]: http://fileformats.archiveteam.org/wiki/Falcon_True_Color
[afw-fun]: https://temlib.org/AtariForumWiki/index.php/Funny_Paint_file_format
[js-fun]: http://fileformats.archiveteam.org/wiki/Funny_Paint
[afw-god]: https://temlib.org/AtariForumWiki/index.php/GodPaint_file_format
[js-god]: http://fileformats.archiveteam.org/wiki/GodPaint
[afw-hir]: https://temlib.org/AtariForumWiki/index.php/Print-Technik_Raw_Data_file_format
[js-printtechnik]: http://fileformats.archiveteam.org/wiki/Print-Technik
[js-icdraw]: http://fileformats.archiveteam.org/wiki/ICDRAW_icon
[icdraw-dir]: http://cd.textfiles.com/suzybatari1/falcon/icdraw14/
[afw-iim]: https://temlib.org/AtariForumWiki/index.php/InShape_file_format
[js-iim]: http://fileformats.archiveteam.org/wiki/InShape_IIM
[afw-fp]: https://temlib.org/AtariForumWiki/index.php/FuckPaint_file_format
[afw-pix]: https://temlib.org/AtariForumWiki/index.php/PixArt_file_format
[js-pixart]: http://fileformats.archiveteam.org/wiki/PixArt
[afw-pnt]: https://temlib.org/AtariForumWiki/index.php?title=TruePaint/Prism_Paint_file_format
[js-pnt]: http://fileformats.archiveteam.org/wiki/Prism_Paint
[afw-rag]: https://temlib.org/AtariForumWiki/index.php/Rag-D_file_format
[rag-format]: http://cd.textfiles.com/atarilibrary/atari_cd07/GRAPHICS/PAINT/RAGDEE/ENGLISH/FORMAT.TXT
[js-ragd]: http://fileformats.archiveteam.org/wiki/RAG-D
[afw-imgscan]: https://temlib.org/AtariForumWiki/index.php/IMG_Scan_file_format
[js-imgscan]: http://fileformats.archiveteam.org/wiki/IMG_Scan
[afw-tcp]: https://temlib.org/AtariForumWiki/index.php/Rembrandt_file_format
[js-rembrandt]: http://fileformats.archiveteam.org/wiki/Rembrandt
[afw-coke]: https://temlib.org/AtariForumWiki/index.php/COKE_file_format
[js-coke]: http://fileformats.archiveteam.org/wiki/COKE_(Atari_Falcon)
[afw-spooky]: https://temlib.org/AtariForumWiki/index.php/Spooky_Sprites_file_format
[js-spooky]: http://fileformats.archiveteam.org/wiki/Spooky_Sprites
[spooky-txt]: http://cd.textfiles.com/atarilibrary/atari_cd07/GRAPHICS/PAINT/SPOOKY4/SPOOKY.TXT
[afw-egg]: https://temlib.org/AtariForumWiki/index.php/EggPaint_file_format
[js-egg]: http://fileformats.archiveteam.org/wiki/EggPaint
[afw-indy]: https://temlib.org/AtariForumWiki/index.php/IndyPaint_file_format
[js-indy]: http://fileformats.archiveteam.org/wiki/IndyPaint
[js-xga]: http://fileformats.archiveteam.org/wiki/XGA_(Falcon)
[xga-readme]: http://cd.textfiles.com/crawlycrypt1/falcon/graphics/xga__tga/read_me.1st
[dexvert-samples]: https://sembiance.com/fileFormatSamples/
[textfiles]: http://cd.textfiles.com/
[tf-degas]: http://cd.textfiles.com/suzybatari2/degas/
[tf-spec]: http://cd.textfiles.com/geminiatari/FILES/GRAPHICS/SPECPICS/
[tf-pcs]: http://cd.textfiles.com/suzybatari2/pcs/
[tf-canvas]: http://cd.textfiles.com/crawlycrypt1/graphics/canvas17/_picture/
[tf-palmaster]: http://cd.textfiles.com/atarilibrary/atari_cd12/CLASSICW/ST/GRAPHICS/PALETMST/
[tf-fpaint]: http://cd.textfiles.com/suzybatari1/falcon/fpaint/
[libav-samples]: http://samples.libav.org/image-samples/atarist/
[gfa-archive]: http://gfa.atari-users.net/gfx/
[nofrag-egg]: http://no-fragments.atari.org/no_fragments_04/archive/work/gfx/
[deark]: https://github.com/jsummers/deark
