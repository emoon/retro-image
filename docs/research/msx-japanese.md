# MSX and Japanese computer formats: documentation survey

Clean-room research notes. Everything listed here is documentation, datasheets, or
permissively licensed material. GPL and unknown-licence decoders are listed under
"To avoid" at the end and were not read.

## 1. Summary

56 format rows across 10 platform tables (the cross-platform MAG, PI and PIC appear in each
platform's table but are described once in section 2):

| Docs quality  | Count |
|---------------|-------|
| Spec          | 26    |
| Partial       | 21    |
| Hardware-only | 0     |
| None          | 9     |

There are no Hardware-only rows. Every MSX screen dump is a BSAVE file, and the BSAVE
header and the per-mode VRAM maps are documented. Those dumps are rated Spec. Where pages
or palettes have to be paired by naming convention, they are rated Partial.

Best umbrella sources:

- **MSX2 Technical Handbook** (English, transcribed): [index](https://konamiman.github.io/MSX2-Technical-Handbook/).
  Covers the VRAM map per screen mode ([Appendix 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html)),
  the palette register format and pixel layouts ([Ch. 4](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html)),
  and BSAVE ,S plus the COPY-to-file format with its 4-byte width/height header ([Ch. 2](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html)).
- **MSX Assembly Page resources** ([map.grauw.nl/resources](https://map.grauw.nl/resources/)): TMS9918, V9938, V9958 and V9990 data books; [YJK article](https://map.grauw.nl/articles/yjk/).
- **uniskie GSRLE readme** (Japanese): [GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE).
  Has BSAVE and Graph Saurus headers, the full Graph Saurus RLE algorithm, and the extension and interlace naming table. The repo licence permits reuse and modification without attribution.
- **Mooncore (Kirinn Bunnylin) English specs**: [Maki-chan (MKI/MAG)](https://mooncore.eu/bunny/txt/makichan.htm), [Pi/PIC](https://mooncore.eu/bunny/txt/pi-pic.htm).
  These are English rewrites of the Japanese specs. The text is documentation. The same author's SuperSakura code is GPL, so do not read it.
- **Original Japanese specs**: MAG "MAGBIBLE" ([metanest.jp](http://metanest.jp/mag/mag.xhtml), lzh download, "free distribution"), Pi [PITECH.TXT on Vector](https://www.vector.co.jp/soft/data/art/se003018.html), PIC [PIC_FMT on Vector](https://www.vector.co.jp/soft/data/art/se003198.html) and [PIC header page (retropc.net)](http://retropc.net/x68000/software/graphics/pic/picheader.htm). The last one was not fetchable (TLS error); the URL comes from ja.wikipedia.
- **MarMSX Sketch project** (Portuguese/English): [format index](https://marmsx.msxall.com/projetos/sketch/formats.php), which covers the Dynamic Publisher PCT, FNT and STP formats.
- `file(1)` magic for MSX (BSD licence): [Magdir/msx](https://web.mit.edu/freebsd/head/contrib/file/magic/Magdir/msx). Has signatures for SC/GE/SR dumps, Graph Saurus compressed files, Maki-chan, PIC and G9B.

Not reachable from this environment: fileformats.archiveteam.org / justsolve.archiveteam.org
(connection refused) and msx.org/wiki (403). The archiveteam pages are known to exist and are
worth checking manually: [MAKIchan Graphics](http://fileformats.archiveteam.org/wiki/MAKIchan_Graphics),
[Pi](http://justsolve.archiveteam.org/wiki/Pi_(image_format)), [MSX BASIC graphics](http://fileformats.archiveteam.org/wiki/MSX_BASIC_graphics),
[Graph Saurus](http://fileformats.archiveteam.org/wiki/Graph_Saurus), [Dynamic Publisher](http://fileformats.archiveteam.org/wiki/Dynamic_Publisher),
[MIF (MSX)](http://fileformats.archiveteam.org/wiki/MIF_(MSX)), [XLD4](http://fileformats.archiveteam.org/wiki/XLD4),
[Mapletown Network](http://justsolve.archiveteam.org/wiki/Mapletown_Network), [ArtMaster88](http://fileformats.archiveteam.org/wiki/ArtMaster88).
Search snippets from them are quoted in the Notes columns where they were used.

---

## 2. Cross-platform Japanese formats (MAG/MKI, PI, PIC)

### MAG (MAKI02) and MKI (MAKI01A/B): Spec

- Author: Woody RINN (まきちゃんNET / まぐろBBS). Official spec "MAGBIBLE" (Japanese) is linked from [metanest.jp/mag](http://metanest.jp/mag/mag.xhtml).
  That page also notes the spec is vague about 256-colour width alignment and 12-bit palette expansion.
  The English rewrite is at [Mooncore makichan.htm](https://mooncore.eu/bunny/txt/makichan.htm). Background: [ja.wikipedia MAGフォーマット (via Wikiwand)](https://www.wikiwand.com/ja/articles/MAG%E3%83%95%E3%82%A9%E3%83%BC%E3%83%9E%E3%83%83%E3%83%88).
- MAG layout, per Mooncore:
  - The file starts with `"MAKI02  "` (8 bytes), a 4-byte machine name, and a Shift-JIS user/comment string terminated by `0x1A`.
  - The real header starts at the next `0x00` byte. All header offsets are relative to it. It is LSB-first.
  - Header fields, in order: machine code, machine-dependent flag, screen mode, x0, y0, x1, y1, then offset and size of flag A, offset and size of flag B, offset and size of the colour (pixel) stream, then a GRB palette (16 or 256 entries, 1 byte per component).
  - Screen mode bits: bit 7 = 256 colours, bit 0 = 200-line (display double height), bit 1 = 8 colours, bit 2 = digital.
  - Decoding: flag A is a bit stream. For each set bit, XOR the next flag-B byte into a per-row action buffer. Each nibble of the action buffer is then either 0 (take the next 16-bit value from the colour stream) or 1–15 (copy a 16-bit unit from one of 15 fixed relative positions).
- Machine codes, per search snippets from the RECOIL bug tracker and the bitplane PR description (facts only):
  - `0x00`: PC-98, PC-88 or X68000. Other fields must be checked to tell them apart.
  - `0x03`: MSX2/MSX2+.
  - `0x62`: PC-98SA.
  - `0x68`: X68000.
- MSX specifics: the machine-dependent flag selects the MSX screen mode. Per Mooncore, values `0x24`, `0x34` and `0x44` mean YJK pixel data, which needs YJK→RGB conversion.
  For MSX, double height applies when machine = 3 and the machine-dependent flag = 4.
- MKI (MAKI01A/MAKI01B):
  - Fixed 640×400, 16 colours, MSB-first.
  - Header: signature, machine name, 20-byte comment, 48-byte GRB palette, then sizes of flag B and pixel data A/B, an extension flag, and dimensions.
  - Data: a 1000-byte flag A, then flag B, then pixel data A and B.
  - Final pass is a vertical XOR filter: rows 2 lines above for 01A, 4 lines above for 01B.
- MAX: the same MAKI02 format with a different extension (an MSX convention).

### PI (Yanagisawa): Spec

- Author: やなぎさわ (Yanagisawa), around 1990. It started on PC-98 (16/256 colours) and was also used on PC-88, MSX, X68000 and FM Towns.
- Original spec: [PITECH.TXT (Vector, Japanese)](https://www.vector.co.jp/soft/data/art/se003018.html). English rewrite: [Mooncore pi-pic.htm](https://mooncore.eu/bunny/txt/pi-pic.htm). Overview: [ja.wikipedia Pi (画像圧縮)](https://ja.wikipedia.org/wiki/Pi_(%E7%94%BB%E5%83%8F%E5%9C%A7%E7%B8%AE)).
- Header (MSB-first):
  - `"Pi"`. This signature is sometimes missing or zeroed.
  - A Shift-JIS comment up to `0x1A`. The real header starts at the following `0x00` byte.
  - Mode byte. Pixel aspect X and Y bytes: non-zero values give the aspect, e.g. a PC-98 200-line source.
  - Bit depth: 4 or 8, where `0xFF` means default to 4.
  - 4-byte saver/machine signature, then a 2-byte length of machine-specific data followed by that data.
  - Width and height as 16-bit values, then the palette as RGB triplets (16 or 256 entries).
- Compression:
  - Colours are delta-coded through a per-previous-colour move-to-front table.
  - Variable-length bit codes are used for the deltas.
  - Repeats use 5 position codes: 4 or 2 pixels left, 1 row up, 2 rows up, 1 row up ±1.
- Machine/aspect: there is no fixed machine code. Use the aspect bytes plus the 4-byte saver signature.

### PIC (Yanagisawa, X68000 "柳沢PIC"): Spec, not verified

- Mainly X68000 15/16-bit colour (65536). Also PC-88VA, FM Towns and Macintosh variants, and 8/16/256 colours up to full colour.
- Original spec: [PIC_FMT (Vector, Japanese)](https://www.vector.co.jp/soft/data/art/se003198.html) and the [PIC header doc at retropc.net](http://retropc.net/x68000/software/graphics/pic/picheader.htm), which was not fetchable here.
  The reference loader APICG by GORRY lives at [retropc.net apicg](http://retropc.net/x68000/software/graphics/pic/apicg/); its licence is unknown, so do not read it.
- Overview: [ja.wikipedia PIC (画像圧縮)](https://ja.wikipedia.org/wiki/PIC_(%E7%94%BB%E5%83%8F%E5%9C%A7%E7%B8%AE)). Algorithm summary: [Qiita (Japanese)](https://qiita.com/tomotaco/items/705f79ae59368417aef8).
  - Signature `"PIC"`.
  - 2D run-length coding: start points of same-colour runs are chained vertically. Colours go through an internal cache and are coded as cache indices.
  - The header carries a machine/type field and colour bits. Mooncore's PIC section is "tbd".
- Large images (LPIC) are split over several files. This is out of scope.

---

## 3. MSX (MSX1, TMS9918/9928/9929)

### Hardware references

- TMS9918 data sheet and VDP Programmer's Guide: [grauw resources](https://map.grauw.nl/resources/), also as [texasinstruments_tms9918.pdf](https://map.grauw.nl/resources/video/texasinstruments_tms9918.pdf) and [ti-vdp-programmers-guide.pdf](https://map.grauw.nl/resources/video/ti-vdp-programmers-guide.pdf). Wikipedia: [TMS9918](https://en.wikipedia.org/wiki/TMS9918).
- VRAM map for screens 1–3 (same addresses on MSX1): [Appendix 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html).
  - Screen 2: names at 1800h, patterns at 0000–17FFh, colours at 2000–37FFh, sprite attributes at 1B00h, sprite patterns at 3800h.
  - Screen 3: names at 0800h, patterns at 0000–07FFh.
- BSAVE header (7 bytes): `FE`, start (LE16), end (LE16), exec (LE16). See [MSX2 Tech Handbook Ch. 2](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html), [Wikipedia BSAVE](https://en.wikipedia.org/wiki/BSAVE) and [uniskie GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE).
  The data length is end−start+1. Screen dumps use `BSAVE ...,S` (VRAM).

### Palette

- The TMS9918 has a fixed 15 colours plus transparent. The datasheet gives Y/R-Y/B-Y values; [Wikipedia TMS9918](https://en.wikipedia.org/wiki/TMS9918) tabulates them with RGB approximations.
  There is no single canonical RGB set; see the discussion in [msx.org "Question about MSX1 palette"](https://www.msx.org/forum/semi-msx-talk/emulation/question-about-msx1-palette).
- An SC2 file loaded on MSX2 may include the V9938 palette table at 1B80h (see section 4).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SC2, GRP | Screen 2 BSAVE dump | **Spec** | [Handbook App. 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html), [TMS9918 DS](https://map.grauw.nl/resources/video/texasinstruments_tms9918.pdf), [file magic](https://web.mit.edu/freebsd/head/contrib/file/magic/Magdir/msx) | BSAVE 0000–37FFh (sometimes up to 3FFFh, which includes sprites). 256×192 Graphics II: 3 banks of 256 8×8 patterns, colour byte per pattern row (fg/bg nibble). Use the header end address to see what is present. |
| SC3 | Screen 3 BSAVE dump | **Spec** | [Handbook App. 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html), [Wikipedia TMS9918](https://en.wikipedia.org/wiki/TMS9918) | Multicolour mode: 64×48 blocks of 4×4 pixels. Pattern bytes hold 2 colour nibbles; the name table selects pattern and the row group selects the byte. |

---

## 4. MSX2 (V9938)

### Hardware references

- V9938 Technical Data Book: [yamaha_v9938.pdf](https://map.grauw.nl/resources/video/yamaha_v9938.pdf) (also on [archive.org](https://archive.org/details/bitsavers_yamahaYamanicalDataBookAug85_6932685)). Application manual (HTML): [v9938.xhtml](https://map.grauw.nl/resources/video/v9938/v9938.xhtml).
- MSX2 Technical Handbook:
  - [Ch. 4](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html) covers pixel formats:
    - G4 = Screen 5: 256×212, 4 bpp, 2 pixels per byte, high nibble first.
    - G5 = Screen 6: 512×212, 2 bpp.
    - G6 = Screen 7: 512×212, 4 bpp.
    - G7 = Screen 8: 256×212, 8 bpp as `GGGRRRBB`.
  - [Appendix 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html) is the VRAM map:
    - Screens 5/6: bitmap at 0000–69FFh (212 lines), palette table at 7680h, sprites at 7400–7FFFh.
    - Screens 7/8: bitmap at 0000–D3FFh (212 lines), palette at FA80h, sprites at F000–FA7Fh.
    - Screen 4: like Screen 2, but palette at 1B80h and sprite mode 2 (sprite colours at 1C00h, attributes at 1E00h).
  - [Ch. 2](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html) covers `COPY (x1,y1)-(x2,y2) TO "file"`: the file has a 4-byte header (width LE16, height LE16) followed by packed pixels in the native bit depth. This is the basis of GLx files.
- Interlace: R#9 bit 3 plus even/odd page alternation. An interlaced picture is two pages (two files) whose lines alternate, giving 424 lines.
  Naming conventions differ between tools. The uniskie table uses `S50/S51`, `R50/R51`, etc. The pairs `SC5+S15`, `SC7+S17`, etc. are another convention, and no written doc for it was found.

### Palette

- 16 entries × 2 bytes in VRAM and in palette files:
  - byte 0 = `0RRR0BBB`
  - byte 1 = `00000GGG`
  - Source: [Handbook Ch. 4](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html).
- 3-bit to 8-bit expansion: there is no hardware-defined mapping. [grauw YJK article](https://map.grauw.nl/articles/yjk/) maps 3-bit to 5-bit as `c<<2 | c>>1`.
- V9938 power-on palette (R,G,B 0–7). Colours 0–4 were confirmed by search snippets; the rest is commonly cited and should be verified against the V9938 data book:
  - 0 (0,0,0)
  - 1 (0,0,0)
  - 2 (1,6,1)
  - 3 (3,7,3)
  - 4 (1,1,7)
  - 5 (2,3,7)
  - 6 (5,1,1)
  - 7 (2,6,7)
  - 8 (7,1,1)
  - 9 (7,3,3)
  - 10 (6,6,1)
  - 11 (6,6,4)
  - 12 (1,4,1)
  - 13 (6,2,5)
  - 14 (5,5,5)
  - 15 (7,7,7)
- When a dump has no palette (it ends before the palette table, or it is an SRx without a PLx), use the default palette.
- Screen 8 has no palette. Map `GGGRRRBB` directly; blue is 2 bits.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| CMP + PL5 | Dot Designer's Club (T&E Soft) | **None** | [generation-msx entry](https://www.generation-msx.nl/software/tesoft/dot-designers-club/1346/), [AGE web edition](https://age.tsjakoe.com/) | Compressed Screen 5 image plus palette. No layout doc found. The AGE tool reportedly reads and writes CMP (licence unknown). |
| FNT | Dynamic Publisher font | **Spec** | [MarMSX fnt_en](https://marmsx.msxall.com/projetos/sketch/fnt_en.php), [MarMSX pct_en](https://marmsx.msxall.com/projetos/sketch/pct_en.php) | Header `"DYNAMIC PUBLISHER FONT"`. Height at 0x80, 256 char widths at 0x100, compressed data at 0x200 (PCT RLE). The image is 512×160 mono, 32×8 cells of 16×20. |
| GL5 / SH5 + PL5 | BASIC COPY file, Screen 5 | **Partial** | [Handbook Ch. 2](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html), [uniskie viewer note](https://note.com/unimsx/n/n35f1acdf7dc0?hl=en) | GLx = 4-byte w/h header plus 4 bpp packed rows. The meaning and layout of SHx is undocumented (it seems to be an alias). PL5 layout: see SR5. |
| GL6 / SH6 + PL6 | BASIC COPY file, Screen 6 | **Partial** | as above | 2 bpp. |
| GL7 / SH7 + PL7 | BASIC COPY file, Screen 7 | **Partial** | as above | 4 bpp, 512 wide. |
| GL8 / SH8 | BASIC COPY file, Screen 8 | **Partial** | as above | 8 bpp GGGRRRBB, no palette. |
| MAG, MAX | Maki-chan Graphics | **Spec** | see section 2 | Machine code 0x03. The machine-dependent flag gives the MSX screen mode (YJK variants included). |
| MIF | MSX Interchange Format (MIF package, Louthrax) | **None** | [msx.org MIF 2.2 thread](https://www.msx.org/forum/msx-talk/software/mif-package-22-released), [MSX FAQ suffixes](https://www.faq.msxnet.org/suffix.html) | GIF-like compressed format covering MSX1 to MSX2+ modes. No public layout doc. Viewer sources ship in the MIF package (licence unstated). Reverse engineered from samples, see section 18. |
| MIG | MIG (MIF package / SofaRun) | **Partial** | [msx.org MIF 2.2 thread](https://www.msx.org/forum/msx-talk/software/mif-package-22-released), [MSXHub MIGVIEW](https://msxhub.com/MIGVIEW) | BitBuster-compressed stream of VDP operations (VRAM writes, palette writes, masked register writes) after a header naming the target VDP. Opcode encoding is undocumented; reverse engineered from samples, see section 18. |
| PCT | Dynamic Publisher screen | **Spec** | [MarMSX pct_en](https://marmsx.msxall.com/projetos/sketch/pct_en.php), [msx.org thread](https://www.msx.org/forum/msx-talk/software/dynamic-publisher-file-formats-structure) | 384-byte header starting `"DYNAMIC PUBLISHER SCREEN"`, data at 0x180. 512×704 mono. RLE: counter MSB=1 means repeat the next byte (n+1) times; MSB=0 means (n+1) literal bytes. Runs break at line end. Nibbles are swapped; 1 = black. |
| PI | Pi | **Spec** | see section 2 | Same format as PC-98. |
| PIC | YPIC | **None** | — | Nothing found. The .PIC extension clashes with Screen 8 dumps; tell them apart by the BSAVE `FE` header. |
| SC4 | Screen 4 BSAVE dump | **Spec** | [Handbook App. 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html) | Screen 2 layout plus palette at 1B80h and sprite mode 2 tables. |
| SC5, GE5 | Screen 5 BSAVE dump | **Spec** | [Handbook App. 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html), [msx.org SC5 thread](https://www.msx.org/forum/msx-talk/development/msx-screen-5-format), [uniskie readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) | 256×212×4 bpp at 0000h. Palette at 7680h if the end address reaches it. Height comes from the end address (192 or 212 lines). |
| SC5 + S15 | Screen 5 interlaced | **Partial** | [Handbook Ch. 4](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html), [uniskie readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) | Two SC5 dumps (even and odd field) interleaved to 256×424. Which file is the even field is a convention; nothing written down for the S1x naming. |
| SC6 | Screen 6 BSAVE dump | **Spec** | as SC5 | 512×212×2 bpp, palette at 7680h. |
| SC6 + S16 | Screen 6 interlaced | **Partial** | as SC5+S15 | 512×424. |
| SC7, GE7 | Screen 7 BSAVE dump | **Spec** | as SC5 | 512×212×4 bpp, palette at FA80h. |
| SC7 + S17 | Screen 7 interlaced | **Partial** | as SC5+S15 | 512×424. |
| SC8, GE8, PIC | Screen 8 BSAVE dump | **Spec** | as SC5, [file magic](https://web.mit.edu/freebsd/head/contrib/file/magic/Magdir/msx) | 256×212 GGGRRRBB. Typical signature is `FE 00 00 FF D3 00 00`. |
| SC8 + S18 | Screen 8 interlaced | **Partial** | as SC5+S15 | 256×424. |
| SR5 + PL5 | Graph Saurus Screen 5 | **Partial** | [uniskie GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) | Header `FE` (raw) or `FD` (RLE), 2 unused bytes, then a **data size** (not an end address). RLE algorithm is fully described there. PL5 holds up to 8 banks of 16 colours; its byte layout is not spelled out (probably the 2-byte VRAM palette format), so verify it on samples. |
| SR6 + PL6 | Graph Saurus Screen 6 | **Partial** | as SR5 | The readme's extension table omits SR6, but `file` magic and viewers treat it the same way. |
| SR7 + PL7 | Graph Saurus Screen 7 | **Partial** | as SR5 | Uncompressed signature is `FE 00 00 00 D4 00 00`. |
| SR8 | Graph Saurus Screen 8 | **Spec** | [uniskie GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) | Raw only (per readme), no palette. |
| SRI + PL7 | Graph Saurus interlaced Screen 7 | **Partial** | [bitplane PR #62 (facts)](https://github.com/bitplane/datatypes/pull/62) | 512×424. The way the two fields are stored in one file is undocumented. |
| STP | Dynamic Publisher stamp | **Spec** | [MarMSX stp_en](https://marmsx.msxall.com/projetos/sketch/stp_en.php) | Width LE16 and height LE16, then w*h/4 bytes. Each byte is 4×1 pixels using even pixels only (Screen 6 style). Mono. |

---

## 5. MSX2+ (V9958, YJK/YAE)

### Hardware references

- V9958 Technical Data Book: [yamaha_v9958.pdf](https://map.grauw.nl/resources/video/yamaha_v9958.pdf), [OCR version](https://map.grauw.nl/resources/video/yamaha_v9958_ocr.pdf).
- [grauw "The YJK screen modes"](https://map.grauw.nl/articles/yjk/) is the key doc. Also [MarMSX "MSX 2+ Colors" PDF](https://marmsx.msxall.com/artigos/sc12en.pdf), [Steckschwein YJK post](https://www.steckschwein.de/post/2024/03/v9958_yjk_mode/) and [Wikipedia YJK](https://en.wikipedia.org/wiki/YJK).
- Layout: the VRAM layout is the same as Screen 8 (256×212, 1 byte per pixel, palette at FA80h for Screen 10). Each 4-pixel group shares J and K:
  - Byte n has Y in bits 7–3 and a part of K or J in bits 2–0: K low, K high, J low, J high for the four bytes.
  - J and K are 6-bit two's complement. Y is 5-bit.
- YAE (Screen 10/11): bits 7–4 = Y (even values only), bit 3 = A. When A=1, bits 7–4 are a palette index instead.
- Conversion (grauw):
  - `r = clamp(y+j, 0, 31)`
  - `g = clamp(y+k, 0, 31)`
  - `b = clamp(floor((5y − 2j − k + 2)/4), 0, 31)`

### Palette

Screen 12 has none. Screens 10/11 use the 16-entry V9938 palette (section 4) for A=1 pixels.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| GLA / GLB / SHA / SHB + PLA | COPY file, Screen 10/11 (YJK+YAE, 12499 YJK colours + 16 palette colours) | **Partial** | [Handbook Ch. 2](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html), [grauw YJK](https://map.grauw.nl/articles/yjk/), [uniskie viewer note](https://note.com/unimsx/n/n35f1acdf7dc0?hl=en) | COPY w/h header plus 8 bpp YAE data. SHx and PLA layouts are undocumented. The "12515 colours" in RECOIL's list vs grauw's 12499 is just a counting difference. |
| GLC / GLS / SHC | COPY file, Screen 12 (YJK, 19268 colours) | **Partial** | as above | Same as above. GLS is presumably SRS-style; unconfirmed. |
| SCA, SCB | Screen 10/11 BSAVE dump | **Spec** | [Handbook App. 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html), [grauw YJK](https://map.grauw.nl/articles/yjk/) | Screen 8 layout plus YAE decode; palette at FA80h when present. |
| SCA + S1A | Screen 10 interlaced | **Partial** | as SC5+S15 | Two fields, 256×424. uniskie uses SA0/SA1. |
| SCC, SRS, YJK | Screen 12 BSAVE dump / Graph Saurus / raw | **Spec** | [grauw YJK](https://map.grauw.nl/articles/yjk/), [uniskie GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE), [image-to-msx-converter (Unlicense)](https://codeberg.org/randysimons/image-to-msx-converter) | SRS is a raw Graph Saurus file (`FE`, size header). .YJK from image-to-msx-converter is a page file, possibly headerless; check samples. |
| SCC + S1C | Screen 12 interlaced | **Partial** | as SC5+S15 | uniskie uses SC0/SC1. |

---

## 6. MSX V9990 (GFX9000)

### Hardware references

[V9990 application manual and datasheet](https://map.grauw.nl/resources/video/yamaha_v9990.pdf), [datasheet](https://map.grauw.nl/resources/video/yamaha_v9990_datasheet.pdf), [MSX FAQ GFX9000](https://www.faq.msxnet.org/gfx9000.html).

### Palette

The V9990 palette is 64 entries of 5-bit RGB. G9B stores its own palette as 3 bytes per colour.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| G9B | GFX9k library (Team Bomba) | **Partial** | [Team Bomba G9B doc](https://www.teambomba.net/g9b.html), [Gfx9k lib](https://www.teambomba.net/gfx9klib.html), [file magic](https://web.mit.edu/freebsd/head/contrib/file/magic/Magdir/msx), [BitBuster 1.2 news](https://www.msx.org/news/software/en/bitbuster-12) | Header is fully specified: `"G9B"`, header size (11), depth (2/4/8/16), colour type (0 palette / 64 fixed 256 / 128 YJK / 192 YUV), nrColors, width, height, compression, 24-bit data size, palette nrColors*3, then data. Compression 1 = BitBuster (byte-aligned LZ77), which is documented only by its distributed source (licence "free use", terms unverified). Uncompressed files can be implemented from the doc. |

---

## 7. NEC PC-80 (PC-8001)

Hardware: 8 digital colours (no hardware references collected; low priority).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| MAG | Maki-chan Graphics | **Spec** | see section 2 | Machine code/name identifies the PC-80. Expect a digital palette and 200-line aspect. |

---

## 8. NEC PC-88 (PC-8801)

### Hardware references

Background: [PC88.gr.jp](http://www.pc88.gr.jp/vafaq/view.php/article/88va/vafaq/64). The PC-8801 has 640×200 with 8 digital colours (3 bitplanes). SR and later models have an analog palette of 512 colours (3 bits per channel).

### Palette

Digital 8 colours (one bit each for B, R, G per pixel). There is also a 640×400 monochrome mode, so double height applies for 200-line images. MAG and Pi carry their own palettes.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| IMG | ArtMaster88 (SystemSoft) | **Partial** | [archiveteam ArtMaster88 (unfetched)](http://fileformats.archiveteam.org/wiki/ArtMaster88), [戸田孝 "アートマスター画像を読む" (Japanese, TLS error)](http://nanyanen.jp/library/readart.html), [PC88 image collection (samples)](https://archive.org/details/PC88-image-collection-1817) | Signature `SS_SIF`. A Japanese page on reading ArtMaster files exists but could not be fetched; look at it manually (check its licence before reading any code). |
| IMG | DaVinci (Popcom) | **None** | [8-bits.info entry (Japanese)](https://www.8-bits.info/gamelist/PC88/info/info_n21XpU9XjMi7XL4U.php) | Nothing on the file layout. The tool also ran on X1 and FM-7 and could exchange data between machines. |
| KTY | Kitty | **Partial** | [note.com "kty画像ローダー" (Japanese)](https://note.com/ftz/n/n84d9dd98c1e2) | 640×200, 3 bpp (8 colours). Compression fills rectangles of repeated 4×2 pixel blocks. No byte layout found; xgload (licence unknown) supports it. |
| MAG | Maki-chan Graphics | **Spec** | see section 2 | 200-line flag gives double height. |
| PI | Pi | **Spec** | see section 2 | Aspect bytes mark 200 lines. |

---

## 9. NEC PC-88VA

### Hardware references

[PC-88VA hardware (pc88.gr.jp)](http://www.pc88.gr.jp/~va/va-hard.html), [PC-88VA spec](http://www.pc88.gr.jp/va/va-spec.html). Both pages were found but not fetched (TLS). The VA has 640×400 at 256 colours or 640×200 at 65536 colours.

### Palette

The 65536-colour mode is direct colour (GRB 6-5-5? unverified). The 16 and 256-colour modes are palettised.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| KT4 | Kitty | **Partial** | [note.com kty loader (Japanese)](https://note.com/ftz/n/n84d9dd98c1e2) | KTY variant with 400 lines. Same block compression; byte layout unknown. |
| MAG | Maki-chan Graphics | **Spec** | see section 2 | — |
| PIC | Yanagisawa PIC (VA variant) | **Spec** | see section 2 | The PIC header machine/type field distinguishes the VA. |

---

## 10. NEC PC-98 (PC-9801)

### Hardware references

640×400 with 16 of 4096 colours (4 bitplanes, analog palette of 4 bits per channel), or 640×200 / 8 digital colours on early models. 256-colour PEGC exists on later models.

### Palette

- Analog: 16 entries of 4-bit G, R, B (GRB order is the PC-98 convention; MAG and MKI store palettes as GRB).
- Digital 8-colour: one bit each for G, R, B (bit order unverified).
- [Data Crystal PC-98 file formats](https://datacrystal.tcrf.net/wiki/File_formats_for_PC-98) has an RGB palette file entry. Its MAG description is unreliable.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| ARV | ARTV (Art/v, SystemSoft) | **None** | [wakachan CG software list (Japanese)](http://www.ateliermw.com/cglib/software.html) | Only known to exist. |
| EBD | EBD (16 colours) | **None** | — | Nothing found. |
| MAG, MKI | Maki-chan Graphics (MAKI02 / MAKI01A/B) | **Spec** | see section 2, [wdic MAKI (Japanese)](https://www.wdic.org/w/TECH/MAKI) | MKI is fixed 640×400×16. |
| ML1, MX1, NL3 | Mapletown Network | **None** | [justsolve Mapletown Network (unfetched)](http://justsolve.archiveteam.org/wiki/Mapletown_Network) | Signatures only (search snippet): ML1 `"100" 1A`, MX1 `"@@@ "`, NL3 `20 20 78 25`. |
| PI | Pi (16/256 colours) | **Spec** | see section 2 | 8 bpp variant has 256-entry palette. |
| Q4 | XLD4 (MAJYO / QLD) | **None** | [ja.wikipedia Q4](https://ja.wikipedia.org/wiki/Q4), [RECOIL ticket #14 (description only)](https://sourceforge.net/p/recoil/bugs/14/) | 640×400×16. Paired with a .Q4D text file. ASCII `"MAJYO"` at offset 11. Wikipedia states the algorithm was never published. |
| ZIM | Z's Staff Kid98 (Zeit) | **None** | [emk HTML5 ZIM viewer page](https://emk.name/2016/01/zim.html) | The emk page calls it "completely proprietary, no public spec"; emk reverse-engineered it via XGLOAD source. |

---

## 11. Sharp X68000

### Hardware references

Background: [Wikipedia X68000](https://en.wikipedia.org/wiki/X68000). Graphics are 512×512 at 65536 colours or 768×512 at 16 colours. The 16-bit colour word is GRB 5-5-5 plus an intensity bit (bit layout unverified). Also see the [IPSJ museum page (Japanese)](https://museum.ipsj.or.jp/computer/personal/0038.html).

### Palette

The 65536-colour mode is direct colour. PIC typically encodes 15-bit colour.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| MAG | Maki-chan Graphics | **Spec** | see section 2 | Machine code 0x68 (or 0x00 with name). |
| PIC | Yanagisawa PIC (65536 colours) | **Spec** | see section 2, [PIC_FMT](https://www.vector.co.jp/soft/data/art/se003198.html) | Primary platform of the format. Spec text not yet read (Vector download). |

---

## 12. Fujitsu FM Towns

Hardware: 640×480 with 256 colours, or 320×240 at 32768 colours. No reference collected; low priority. FM Towns PIC loaders are on [Vector](https://www.vector.co.jp/download/file/towns/art/fh045764.html).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PI | Pi | **Spec** | see section 2 | — |
| PIC | Yanagisawa PIC (Towns variant) | **Spec** | see section 2 | The PIC header machine field distinguishes Towns. |

---

## 13. Sample file sources

- [Internet Archive: PC 88イメージ集 (1817 images)](https://archive.org/details/PC88-image-collection-1817)
- [Internet Archive: 256color (PC-98 era images)](https://archive.org/details/256color_202203)
- [flat2d PC-98 mega pack](https://www.flat2d.com/pc98.html)
- [msx.org "Where to find Maki-chan files"](https://www.msx.org/forum/msx-talk/general-discussion/where-find-maki-chan-files)
- [MarMSX imgs.zip (MSX screen samples)](http://marmsx.msxall.com/msxvw/oldmsxvw/imgs.zip). The URL was cited in search results and not verified.
- [msx.org graphics downloads](https://www.msx.org/downloads/utilities/graphics)
- uniskie GSRLE has a `test` folder in [MSX_MISC_TOOLS/GSRLE](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) (only image reuse is discouraged by the author).

## 14. Permissive / usable references (licence stated)

- **uniskie MSX_MISC_TOOLS (GSRLE, HTML5 MSX graphics viewer)**: custom licence, "modification and redistribution of source and programs are free, no attribution needed". Reusing the bundled images is discouraged. [repo](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE).
- **image-to-msx-converter** (Randy Simons): Unlicense / public domain, YJK/YAE/SC8 encoder. [codeberg](https://codeberg.org/randysimons/image-to-msx-converter).
- **file(1) magic** (BSD): [Magdir/msx](https://web.mit.edu/freebsd/head/contrib/file/magic/Magdir/msx).
- **bitplane/datatypes** (MIT): [PR #48 Japanese formats](https://github.com/bitplane/datatypes/pull/48), [PR #62 MSX screens](https://github.com/bitplane/datatypes/pull/62).
  - The PR text says RECOIL 6.4.5 was used "as a tool only; none of its code was copied", and RECOIL output was the test oracle. It is AI-assisted.
  - Treat it with caution for clean-room purposes. Reading its PR descriptions for facts is fine; consider not reading its code.

## 15. To avoid (GPL / unclear-licence code)

- **RECOIL** (GPL): all source and ports. Its bug tracker pages were only seen as search titles.
- **SuperSakura** (Kirinn Bunnylin, GPL-3.0) contains the MAG/MKI/Pi/PIC decoders ([github](https://github.com/bunnylin/supersakura), [gitlab](https://gitlab.com/bunnylin/supersakura)).
  Do not read its code (e.g. `decomp.pas`). The same author's Mooncore spec pages are prose documentation and fine to use.
- **mag2png**: described as "open source C utility"; licence not verified, so avoid until checked.
- **emk HTML5 MAG loader ("まぐろーだー") and ZIM viewer** ([emk.name](https://emk.name/2015/03/magjs.html)): no licence stated, so do not read the code.
- **XGLOAD** (Unix loader for KTY, ZIM and others): licence unknown, avoid.
- **APICG** (GORRY, PIC loader) and the PIC_FMT sample source: licence unknown. Read only the spec prose.
- **PicLoader2022** ([novisoftware](https://github.com/novisoftware/PicLoader2022)): licence not checked (fetch timed out). The [Qiita article](https://qiita.com/tomotaco/items/705f79ae59368417aef8) C# code and [doga-cganime PlayAnimationImporter](https://github.com/doga-cganime/PlayAnimationImporter) have no licence verified, so do not read the code.
- **MSX Viewer** (MarMSX): "open source" with no licence specified, so avoid the code. The MarMSX format prose pages are fine.
- **MIF package / MIGVIEW sources** (Louthrax): licence unstated, avoid.
- **Susie Pi plug-in** ([cetus.sakura.ne.jp](https://cetus.sakura.ne.jp/softlab/software/ifpi.html)): licence unknown, avoid.
- **BitBuster** depacker sources (Team Bomba): "free to use", exact terms unverified. Confirm before reading.

## 16. Implementation status (wave 1)

Implemented, all corpus samples matching `recoil2png` (126 files):

- MSX/MSX2/MSX2+ BSAVE dumps SC2/GRP, SC3, SC4, SC5/GE5, SC6, SC7/GE7, SC8/GE8/PIC, SCA/SCB, SCC/YJK, with sprites (`msx/screen.rs`, `msx/vdp.rs`).
- Graph Saurus SR5-SR8, SRS (raw and RLE); BASIC COPY GL5-GL8/SH5-SH8, GLA/GLB/SHA/SHB, GLC/GLS/SHC.
- Dynamic Publisher PCT, FNT, STP. G9B (V9990), including BitBuster (reverse engineered from samples).
- Maki-chan MAG/MAX/MKI and Pi (`nec_pc/maki.rs`, `nec_pc/pi.rs`), Yanagisawa PIC (`sharp_x68000/pic.rs`), registered per machine.
- PIC spec used: `pic_fmt.txt` from PIC_FMT (Vector), whose text allows free reproduction; `picl.c` in the same archive was not read.

Skipped: MIG/MIF (undocumented VDP command stream), ML1/MX1/NL3, Q4, ZIM, EBD, ARV, KTY/KT4, ArtMaster88 IMG, CMP (no layout docs; need reverse engineering), interlaced pairs and +PLx palettes (multi-file), SRI (no sample).

## 17. Implementation status (wave 2)

- Shared helpers adopted (`Image::from_indexed`, `scaled`, `set`/`get`); no output change.
- Content detection (`.signature()`): MAG, MKI, Pi, PIC (every machine entry; only the saving
  machine's entry accepts a file), G9B, Dynamic Publisher PCT and FNT. BSAVE dumps, Graph Saurus
  pages and COPY files are not marked: their headers can't tell the screen mode apart.
- Companion files, all checked against `recoil2png` given the same files plus synthesized edge
  cases (`msx/screen.rs` lists the observed rules):
  - `PL5`/`PL6`/`PL7`/`PLA` for Graph Saurus `SRx` and COPY `GLx`/`SHx` (bank 0 only; BSAVE dumps
    ignore them; RECOIL ignores `PL8`).
  - Interlaced `SCx` + `S15`/`S16`/`S17`/`S18`/`S1A`/`S1C`: even/odd lines, the even page's
    palette, no sprites.
  - `SR0` + `SR1` + `PL7` (interlaced Graph Saurus Screen 7, MSX-FAN): RECOIL rejects; divergence.
- Dot Designer's Club `CMP` (+`PL5`): reverse engineered (`msx/dot_designer.rs`): header bytes
  per line / lines / first XORed line, then per 64 bytes a group-flag byte, group masks and the
  non-zero bytes; lines are XORed with the line above. Matches RECOIL on both samples.
- MSX-FAN "ukp" packer (`msx/ukp.rs`), unwrapped before dump/Graph Saurus/COPY decoding; Graph
  Saurus Screen 5/6 pages saved from page 1 (start 0x8000); palette files saved as a BSAVE of the
  VRAM palette table. RECOIL rejects or misreads these samples; divergences recorded.

Still skipped:

- `TITLE.PIC` (MSX-FAN): ukp-packed BASIC COPY Screen 7 under a `.PIC` name; the `.PIC`
  extension gives no way to know the inner format, so it isn't guessed from one sample.
- **MIG**: done in wave 3, see section 18.
- MIF (done in wave 3), Mapletown ML1/MX1/NL3, Q4, ZIM, EBD, ARV, KTY/KT4, ArtMaster88 IMG: unchanged (no layout
  docs; RECOIL decodes the corpus samples, so they are candidates for black-box reverse
  engineering).

### Layouts observed in hostile samples

- **"ukp" packer** (MSX-FAN 03): now supported, see section 17.
- **Sunrise Picture Disk CMP** (not T&E Dot Designer's Club CMP): no header, palette or mode; RLE
  as `n v` = v repeated n times, `00 n` = n literal bytes. RECOIL rejects these. (Sunrise's
  `INTRO1.CMP` in `extra/` is a Dot Designer's Club file.)
- **Punincess Maker `.MP`**: X68000 MAG variant with no machine name, header big-endian, machine
  byte unreliable (0x00/0xFF/0x68 seen). Supported when named `.MAG`.

## 18. Implementation status (wave 3)

Both MIF package formats are implemented by black-box reverse engineering of the 14 + 14
RECOIL samples (every sample pixel-identical to `recoil2png`). The MIF package and MIGVIEW
sources were not read.

- **MIG** (`msx/mig.rs`, content-detected): `"MSXMIG"`, LE32 length of the rest of the file
  (from offset 6), a machine byte (0 MSX1, 1 MSX2, 2 MSX2+), then blocks of LE16 unpacked
  size, LE16 packed size and BitBuster data, ended by an unpacked size of 0 (the last two
  bytes). Each block unpacks on its own to exactly its size. This BitBuster differs from the
  G9B one: the offset byte (plus 4 bits when bit 7 is set) comes before the length, and the
  length is plain Elias gamma (n 1 bits, a 0, then n bits), not interleaved. The unpacked
  blocks are one command stream: `00 n` + n × (register, value, mask); `01 first n` + n
  palette entries; `02` + LE24 VRAM address + LE24 length + bytes; `FF` end. The picture is
  then rendered from the VDP state: mode from R#0/R#1 (M1-M5) and R#25 (YJK/YAE), 192/212
  lines from R#9 LN, page from R#2, and R#9 EO selects interlace (even field: page with the
  low page bit clear). Screen 3 files write only the 0x600-byte pattern table, so the BASIC
  name table is assumed.
- **MIF** (`msx/mif.rs`, content-detected): a mode byte (low nibble 0-5 = Screens 5, 6, 7,
  8, 10/11, 12; 8 = Screen 2; 9 = Screen 3; bit 4 = interlaced, Screens 5-12 only), an
  ignored byte (RECOIL output unchanged by any value), 16 palette entries (not for Screens
  8 and 12; the palette is re-sorted relative to the same picture's MIG), then an LZW
  stream, MSB-first: `1` + literal byte, or `0` + a code of bit-length(entries + 2) bits.
  Codes below the entry count are dictionary strings, the count itself is the KwKwK case,
  count + 1 restarts the dictionary (seen when it reaches 4094 entries) and count + 2 ends
  the stream. Every token after the first since a restart adds previous string + first byte
  of the current one. Unpacked sizes are exact: 212 lines per bitmap page (interlaced
  pictures hold 424 lines in display order), 0x3800 for Screen 2, 0x600 for Screen 3. RECOIL
  rejects a byte appended or removed, so the end code must fall in the last byte.
- Shared rendering moved into `screen.rs` (`render_bitmap`, `render_tiled`,
  `set_basic_multicolour_names`) so dumps, MIG and MIF draw pages the same way.

Not done in wave 3 (no layout docs; archiveteam pages unreachable from here), with what black-box
probing of `recoil2png` established so far:

- **Mapletown ML1** (`GIRL.ML1`, `Win8Draw20151202_ml1.ml1`, both 160x100): `"100" 1A`, date and
  time bytes (year - 1900, month, day, hour, minute, second), BE16 x1, y1, x2, y2 (0, 0, 159, 99),
  then text fields (name at 0x12, machine at 0x20, software at 0x30, Shift-JIS title at 0x40);
  RECOIL ignores the date and text. The bit stream starts at 0x60, MSB first. Colours are
  10-bit base-9 numbers (R × 81 + G × 9 + B, each level 0-8 shown as level × 255 / 8); values
  of 729 or more are rejected. The first bit selects the palette form: `0` then 128 colours
  (Win8Draw; one more bit before them); `1` then 8 bits of count - 1 and that many (7-bit index,
  colour) pairs (GIRL: 27 pairs, indices 0-31). The pixels that follow are not a raster:
  single bit flips recolour whole regions or move a region's edge one line at a time, which
  looks like region / edge-chain coding. Not decoded further.
- **MX1** (5 samples) is a text wrapping of ML1: `"@@@ name.ml1 by author : title (n lines) @@@"`
  CRLF, then printable-ASCII lines. **NL3** (2 samples) is a different text format. Neither was
  started, since they depend on ML1.
- **Q4 / XLD4** (TOM093.Q4 plus 8 in `extra/`): LE16 file size at 8 (RECOIL rejects a
  mismatch), `"MAJYO"` at 11, bytes 0-7 and 0x0A ignored, LE16 fields at 0x10/0x12/0x14 (only
  0x10-0x11 change the output), `44 67 41` at 0x16. From about 0x19, 4-bit colour components
  sit in a packed palette (single bit flips recolour one colour), then an entropy-coded pixel
  stream. Not decoded further.
- **ZIM** (`lockonstar.zim`, 640x400): `"FORMAT-A"`, `"PC9801"` at 0x16, the file name twice and
  `"Z'sSTAFF KID98"` at 0x6A. Not started.

### Archived wiki pages (Wayback Machine)

The archiveteam wiki refused connections during wave 3; its pages were read from the Wayback
Machine instead. They add little layout information:

- [Mapletown Network](http://web.archive.org/web/20250524112620/http://fileformats.archiveteam.org/wiki/Mapletown_Network):
  a **NEC PC-98** format (so the decoder belongs in `nec_pc`, not `msx`). Signatures only: ML1
  `"100" 1A`, MX1 `"@@@ "`, NL3 `20 20 78 25`. Samples: dexvert `image/mapletownNetwork`
  (<http://sembiance.com/fileFormatSamples/image/mapletownNetwork>).
- [XLD4](http://web.archive.org/web/20251221202725/http://fileformats.archiveteam.org/wiki/XLD4):
  `.q4` picture + `.q4d` text document (`"===============>>>>> XLD4 GRAPHIC DATA  DOCUMENT
  <<<<<==============="`, Shift-JIS). `"MAJYO"` at offset 11. Samples: `XLD4_images.zip` on
  RECOIL bug #14 and dexvert `image/q4`. **Do not read** `q4toppm.zip`, attached to the same
  RECOIL bug: code attached to RECOIL's tracker, licence unknown (CLEANROOM.md).
- [MIF (MSX)](http://web.archive.org/web/20251219160336/http://fileformats.archiveteam.org/wiki/MIF_(MSX)):
  no layout; extra samples in `mifui2.1.zip` (msx.org) and dexvert `image/msxInterchangeFormat`.
  `MIF2BMP.EXE` is an MS-DOS binary converter.
- The wiki's `ZIM` page is the unrelated openZIM wiki-dump format; there is no page for the
  Z's Staff Kid98 ZIM. emk's ZIM viewer page
  ([Wayback](http://web.archive.org/web/20170105145627/https://emk.name/2016/01/zim.html),
  prose read, script not read) says the format is proprietary with no public spec. Software that
  still reads it: Zeit's free "イメージファイルコンバータ" (Image File Converter), Paintgraphic 3,
  Photo ReColor, PictureFan and the Susie plug-in IFZIM. Sample sources it names: the
  *recollections vol.3* collection and the SMC-NETWORKS Toyohashi node CG library. The page also
  says ML1, like MAG, came from the PC-communications (BBS) world, so a Japanese ML1
  description may exist in BBS-era archives.

## Wave 5: Japanese

NEC PC-98 / PC-88 formats that had samples but no decoder. All four decoders below were found by
black-box probing of `recoil2png` (mutated and synthesised copies kept outside the corpus) and are
pixel-identical to it on every sample. No RECOIL or GPL source was read. The only prose read was
emk's ZIM page (no layout in it, its script was not read) and Toda Takashi's ArtMaster page (no
layout either, only that the palette is HSV).

- **ZIM** (`nec_pc/zim.rs`, signature `FORMAT-A`, 1 sample): 640x400, 16 colours. Palette of 4-byte
  entries (blue, red, green, unused; full 8-bit values) at 0x218, a skipped table (count at 0x258),
  then one block per line: width 640, x 0, y, size, 320, and `size - 2` bytes. A packed line is a
  3-level bitmap tree (1 byte, then up to 8 bytes, then up to 64 leaf bytes) saying which of 320 delta
  bytes are literals. Deltas are cumulatively XORed with the byte 4 before, then each byte is XORed
  with the one before it, over the whole 320-byte line; the result is the 4 bit planes (80 bytes each,
  plane 3 first). Not understood: 0x216 (zero draws a black picture in RECOIL, so we refuse it), the
  skipped table's contents, the rest of the text header, widths other than 640. RECOIL pads missing
  lines and takes the height from 0x206; we require all blocks.
- **EBD** (`nec_pc/ebd.rs`, 1 sample): 16 x (R, G, B) nibbles (x17; > 15 is rejected), then four
  whole-picture bit planes (plane 0 first, MSB first). 640 wide, height from the file size (any
  multiple of 320 bytes after the 48-byte palette). Not content-detected.
- **ArtMaster88 IMG** (`nec_pc/artmaster88.rs`, signature `SS_SIF    0.00`, 1 sample): the file is
  `_REMSM4.IMG`. `"I"` at 0x10, `"BBRG"` at 0x12 (other letters select other modes that were not
  decoded: the picture changes), 640 and 200 at 0x18/0x1A. At 0x28 two records that each begin with
  their own length; the pixel data starts after the second. Three RLE bit planes, blue, red, green
  (`XX XX n`: n copies, 0 = 256), drawn at 640x400 by doubling lines. The two records (brush/pattern
  tables and the HSV palette) don't change the picture in RECOIL. Bytes after the green plane are
  ignored.
- **DaVinci IMG** (`nec_pc/davinci.rs`, no magic, 1 sample): `REMSM3.IMG`, which RECOIL names DaVinci.
  A stream of control bytes: bit 7 set repeats the next (blue, red, green) byte triple `c & 0x7f` times,
  otherwise `c` triples follow; 80 x 200 triples, then exactly 35 trailing bytes whose content RECOIL
  ignores (any other tail length is rejected). Because it has no signature it is only tried for
  `.img` and demands the exact stream length, so GEM IMG and other `.img` files are not claimed
  (checked with the full oracle).

### Q4 / XLD4: skipped

Header notes from wave 3 still hold (`MAJYO` at 11, size at 8). New this wave:

- RECOIL refuses a truncated file unless most of the stream is present: with the size field patched,
  files cut below 54451 bytes of 58352 are rejected, and from there the picture is correct up to
  about row 374, so the pixels come in raster order (about 14.5 bytes a line).
- Replacing everything after 0x19, 0x31 or 0x40 with 0x00, 0xFF or 0x55 is rejected, so the decoder
  validates its input (probably that the stream ends exactly).
- A flipped bit in the pixel stream changes pixels only for a bounded span of lines after it (about
  40 to 75 lines, e.g. a flip at 0x1000 changes lines 33-74, one at 0x8000 lines 227-299), not the
  rest of the picture. Flips in 0x1c-0x38 recolour or scramble the whole picture, so the packed
  palette part is not fixed width: single flips at 0x20 and 0x31 change only 4 and 40 pixels.
- Nothing about the entropy model (Huffman, arithmetic or LZ) has been found. A next attempt could
  start from the other 8 samples in `corpus/extra/msx-japanese/kawaii-dake-na-no/` (same header,
  different sizes) and from synthetic streams, once the stream end rule is known.

## Wave 5b: Japanese

Two more NEC formats, both found by black-box probing of `recoil2png` on mutated copies (kept
outside the corpus) and pixel-identical to it on every sample. No RECOIL or GPL source was read.
Prose only: the ftz article on note.com about KTY/KT4
(<https://note.com/ftz/n/n84d9dd98c1e2>), which says the format paints rectangles filled with one
repeated 4x2 tile. The `rururutan/ifkty` plug-in source was not read (licence unverified).

- **KT4 "Kitty"** (`nec_pc/kt4.rs`, no magic, 3 samples under `extra/msx-japanese/sembiance/kt4/`):
  640x400, the 8 digital colours. The picture is 160 x 100 cells of two stacked 4x2 tiles (three
  bytes each: blue, red, green plane, row-major, MSB first). The file is a run of segments: a byte
  `n` (0 refused, `0xFF` starts the raw tail), then one tile triple (`n == 1`, used for both halves)
  or two (any other `n`), then area entries up to `0xFF`, then single cells up to `0xFF`. Entry
  kinds come from the top two bits of the first byte, with the low 6 bits and the next byte giving
  a cell number `row * 160 + column`: `01` horizontal run plus a last column, `10` vertical run plus
  a last row, `00` rectangle plus last column and last row; `11` was never seen. Single cells are
  (column, row) pairs. Later segments overwrite earlier ones (RECOIL agrees on about 1200 random
  byte mutations of the samples, checked against a scratch decoder). The tail is six bytes for
  every cell no segment painted, raster order, and RECOIL rejects any byte more or less after it,
  which is why most single mutations of an area entry are rejected: they change the painted-cell
  count. Registered by extension only (no magic).
  **KTY** (PC-88, 640x200, the same scheme per the article) was not registered: no sample. The
  likely differences (200 rows, so 100 x 100 cells of 4x2, or a tile height of 2 pixels) are
  untested.
- **ARTV `ARV`** (`nec_pc/arv.rs`, signature `SS_SIF    0.`, 1 sample `TAIHO (from Gr.lzh).ARV`):
  the 16-colour sibling of ArtMaster88, reusing its run-length plane packing
  (`artmaster88::unpack_plane`). Version digit at 0x0D ignored. `"IRB"` at 0x10 and `"BRG"` at 0x13,
  640 and 400 at 0x18/0x1A (0x16/0x17 ignored; 0x12 other than `B` gives a different layout that
  was not decoded). A chain of three length-prefixed records at 0x28: the second holds the palette
  (16 x three LE16 values, R, G, B, 0-15, scaled by 17); the first and third do not affect the
  picture. Then four packed planes of 80 x 400 bytes (blue, red, green, intensity); trailing bytes
  are ignored. RECOIL also accepts a 200-line header (0x1A = 200) but the plane data of this
  sample does not parse as 200 lines, so only 400 is accepted; heights 1, 2, 100, 399, 401, 480
  are rejected by RECOIL. RECOIL rejects the `.arv` files under `corpus/hostile/`, and so do
  we.
