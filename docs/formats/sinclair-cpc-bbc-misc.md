# Sinclair, Amstrad CPC, BBC Micro and misc. 8-bit formats: documentation survey

Clean-room research notes for these platforms: ZX Spectrum (plus Profi, ULAplus, ZX Evolution, Next), ZX81,
Timex 2048, SAM Coupé, Amstrad CPC, BBC Micro, Oric, Electronika BK, Electronika MC 0515 and Vector-06C.
No RECOIL code and no GPL decoder source was read while collecting these notes. Format/extension list taken from
<https://recoil.sourceforge.net/formats.html> (fact list only).

Researched 2026-10-01. `fileformats.archiveteam.org` / `justsolve.archiveteam.org` refused connections that day.
Pages on that wiki are cited below only where a web search returned them, and their content was **not** checked.
`cpcwiki.eu` sits behind a Cloudflare challenge, so for CPC hardware facts we used its mirror `cpctech.cpcwiki.de`.

## 1. Summary

63 formats in total:

| Docs quality | Count | Formats |
|---|---|---|
| **Spec** | 33 | Spectrum: ATR, BMC4, BSC, BSP, CH$, CH8, HLR, IFL, IMG, MC, MG1, MG2, MG4, MG8, MLT, SCR, STL, ZXP · ULAplus SCR · SXG · NXI · ZX81 P · Timex HRG, SCR hi-colour, SCR hi-res · SAM SS1, SS2, SS3, SS4/SCS4 · CPC SCR+PAL, WIN+PAL, SGX · BBC BBG |
| **Partial** | 12 | Spectrum 3, RGB, CH4, CH6, CHX · Profi GRF · SAM LCE, SSX · CPC CM5+GFX, FNT, PPH+ODD+EVE · Vector-06C SPR |
| **Hardware-only** | 10 | CPC HGB · BBC BB0/BB1/BB2/BB4/BB5 · Oric CHS, HIR/HRS · BK PIC · MC 0515 SCR |
| **None** | 8 | Spectrum FLF, SEV, ZXS · ZX81 RAW, ZP1 · CPC FLF · BBC FLF · BK BKS |

**Best umbrella sources:**

- **zx-image README** (zxart.ee's converter, **CC0-1.0**): <https://github.com/moroz1999/zx-image>.
  - One-line layouts with exact sizes for about 30 Spectrum-family formats: standard, ULA+, gigascreen, LCE,
    multicolor 8x1/8x2/8x4, MC vs MLT, BSC/BSP/BMC4, chr$, tricolor, Timex hi-res/HRG, stellar, NXI, SAM
    mode 3/4, SXG and GRF.
  - The code is CC0 too, so it may be read if needed. See the provenance caveat in section 4.
- **SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md`** (**MIT**, repo created 2026-01):
  <https://github.com/Bedazzle/SpectraLab> (file `ZX_SPECTRUM_GRAPHICS_GUIDE.md` at repo root).
  - Byte-offset specs for SCR, ULA+, ULANext, ATR/53c, BSC, BMC4, BSP (incl. border RLE), IFL, MLT, RGB3/`.3`,
    gigascreen, MGH/MultiArtist (all 4 modes), HLR, STL, NXI, ZXP, chr$ and SpecSCII text streams.
  - Also has a palette and a format-detection-by-size section.
- **SAM Coupé:** the Technical Manual and obo's SSX post.
  - Manual PDF: <https://sam.speccy.cz/systech/sam-coupe_tech-man_v3-0.pdf>
  - Manual as Markdown (no license stated): <https://github.com/stefandrissen/sam-coupe-technical-manual>
  - obo's SSX post: <https://spectrumcomputing.co.uk/forums/viewtopic.php?t=1926>
- **CPC:** Kevin Thacker's cpctech pages.
  - OCP Art Studio: <https://cpctech.cpcwiki.de/docs/artstud.html>
  - Gate Array: <https://cpctech.cpcwiki.de/docs/garray.html>
  - Pixel format: <https://cpctech.cpcwiki.de/docs/graphics.html>
  - AMSDOS header: <https://cpctech.cpcwiki.de/docs/allhead.html>
- **SymbOS SGX:** the official format page in the SymbOS wiki: <https://github.com/Prodatron/symbos-wiki/wiki>
  (page `Format-SGX-(Graphic)`). The wiki repo is GPL-3.0, but this page is documentation, not decoder code.
- **BBC LdPic:** <https://nerdoftheherd.com/projects/libbeebimage/ldpic/>. This is a prose spec. The library
  behind it (libbeebimage) is GPL-3 and was not read.

---

## 2. ZX Spectrum (48K/128K, Pentagon-class)

### Hardware references
- Screen memory layout:
  - <http://www.breakintoprogram.co.uk/hardware/computers/zx-spectrum/screen-memory-layout>
  - Spectrum FAQ, file formats: <https://rk.nvg.ntnu.no/sinclair/faq/fileform.html>
  - BASin help, `.scr`: <https://documentation.help/BASin/format_scr.html>
- Video modes and tricks:
  - <https://en.wikipedia.org/wiki/ZX_Spectrum_graphic_modes>
  - ZX tweaked/extended modes overview (gigascreen, multigigascreen, 256x384 interlace, 512x384):
    <http://users.atw.hu/zxspectrum/zx_gfx_modes_en.htm>
  - Graphics FAQ (Russian): <https://hype.retroscene.org/blog/graphics/320.html>
- Timing, for multicolor and border formats. A line is 224 T-states and the first screen pixel is at
  T = 64*224+4 on a 48K:
  - <https://github.com/kosarev/zx/blob/master/test/screen_timing/SCREEN_TIMING.md>
  - <https://www.cs.hmc.edu/~oneill/spectrum-timings.html>
  - Border effects: <https://github.com/alfishe/speccy-bootcamp/blob/main/05_development/05_display_and_timing/border_effects.md>
  - Border and multicolor techniques (Russian e-zine): <https://zxpress.ru/eng/ezines/zx-review/7-8-9-10/techniques-for-creating-border-effects-and-multicolor-graphics-on-zx-spectrum-detailed-explanation>
- Gigascreen and 3-colour display (frame blending):
  - Gigascreen is 2 frames alternating at 50 Hz. A viewer should average the two RGB frames.
  - Tricolor (`.3`/`.RGB`) shows R, G and B bitmaps on 3 successive frames.
  - Blend demo/tests: <https://github.com/dotkoval/flickering_test>
  - Gamma-aware LUT blending plugin: <https://github.com/dotkoval/spectaculator-gigascreen-noflick> (check license before reading code)
- Character set: <https://sinclair.wiki.zxnet.co.uk/wiki/Character_set>, <https://en.wikipedia.org/wiki/ZX_Spectrum_character_set>

### Palette
- 8 colours x BRIGHT. Bright black equals black, giving 15 distinct colours.
- No canonical RGB exists. SpectraLab guide §4 uses 0xD7 for normal and 0xFF for bright and explains why values vary.
- FLASH swaps ink and paper at about 1.56 Hz (every 16 frames). Static decoders usually ignore it.
- Attribute byte: `F B PPP III`.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SCR | Standard screen | **Spec** | [SpectraLab guide](https://github.com/Bedazzle/SpectraLab); [zx-image](https://github.com/moroz1999/zx-image); [BASin .scr](https://documentation.help/BASin/format_scr.html); [breakintoprogram](http://www.breakintoprogram.co.uk/hardware/computers/zx-spectrum/screen-memory-layout); [justsolve SCR (ZX Spectrum)](http://fileformats.archiveteam.org/wiki/SCR_(ZX_Spectrum)) | 6912 = 6144 interleaved bitmap + 768 attrs. 6144 = bitmap only (mono). Size-detected variants are listed under ULAplus and Timex. |
| ATR | Attributes (53c) | **Spec** | SpectraLab guide §53c/ATR; zx-image "attributes" | 768 bytes of attrs. The bitmap is a fixed tiled 8-byte pattern: checker `AA 55..` gives "53 colours", `DD 77` gives 127. Which pattern RECOIL uses for .ATR is unverified; checker is the common default. |
| HLR | Attributes Gigascreen | **Spec** | SpectraLab guide §HLR; zx-image "lowresgs" (1628 bytes) | 1628 bytes: 84 bytes Z80 loader, then an 8-byte fill pattern at 0x54, then 2x768 attrs at 0x5C and 0x35C. Display as gigascreen of 2 frames. |
| IMG | Gigascreen | **Spec** | SpectraLab guide §Gigascreen; zx-image "gigascreen" | 13824 = 2 x SCR. Average the 2 frames. |
| MG1, MG2, MG4, MG8 | MultiArtist (MGH) | **Spec** | SpectraLab guide §MGH; zx-image "multiartist"; [justsolve MultiArtist](http://justsolve.archiveteam.org/wiki/MultiArtist); [MultiArtist site](https://multiartist.untergrund.net/) | 256-byte header: `"MGH"`, version 1, mode (1/2/4/8), border1, border2. Then 2 interleaved bitmaps, then attrs for frame 1 and frame 2. Sizes: MG8 14080, MG4 15616, MG2 18688. MG1 is 19456 with a split layout: inner columns 8–23 at 8x1, outer columns at 8x8. |
| MC | Multicolor 8x1 | **Spec** | zx-image "mc"; SpectraLab guide §MLT | 12288. **Sources disagree.** zx-image says MC has a *linear* bitmap and linear attrs. SpectraLab treats MC like MLT with an interleaved bitmap. Check against samples. |
| MLT | Multicolor 8x1 | **Spec** | zx-image "mlt"; SpectraLab guide §MLT | 12288 = 6144 interleaved bitmap + 6144 linear attrs (192 rows x 32). 12352 adds a 64-byte ULA+ palette and uses Timex-interleaved attrs (SpectraLab). |
| IFL | Multicolor 8x2 | **Spec** | SpectraLab guide §IFL; zx-image "multicolor" | 9216 = 6144 interleaved bitmap + 3072 attrs (96 rows x 32, row-major). |
| BSC | Border Screen | **Spec** | SpectraLab guide §BSC; zx-image "bsc" | 11136 = SCR + 4224 border bytes. Border sections: top 64 lines x 24 B, sides 192 x 8 B (4 left, 4 right), bottom 48 x 24 B. Each byte holds two 8-px blocks: bits 0–2 left, bits 3–5 right. Canvas 384x304. |
| BMC4 | Border Screen Multicolor 8x4 | **Spec** | SpectraLab guide §BMC4; zx-image "bmc4" | 11904 = bitmap + attr bank 1 (upper 4 lines of each cell) + attr bank 2 (lower 4 lines) + 4224 BSC-style border. |
| BSP | Border Screen by Trefi | **Spec** | SpectraLab guide §BSP (incl. RLE rules); zx-image "bsp"; [justsolve BSP (ZX Spectrum)](http://fileformats.archiveteam.org/wiki/BSP_(ZX_Spectrum)) | `"bsp"` magic + config byte (bit7 giga, bit6 border) + border colour + 32-byte title + 32-byte author = 70-byte header. Then 1–2 SCRs plus optional border RLE streams (colour = `b&7`, length code = `b>>3`). In the 0xC0 variant, a u16 offset to the second border stream comes first. |
| STL | Stellar | **Spec** | SpectraLab guide §STL; zx-image "stellar"; [justsolve STL (ZX Spectrum)](http://fileformats.archiveteam.org/wiki/STL_(ZX_Spectrum)) | 3072 bytes: two 1536-byte 8x4-attr frames, interleaved in 2-byte pairs. Fixed bitmap 0x0F. Effective 64x48 gigascreen-blended. |
| 3 | Tricolor (RGB) | **Partial** | SpectraLab guide §RGB3; zx-image "tricolor" | 18432 = R, G, B bitmaps of 6144 each, interleaved. Shown on 3 successive frames and blended additively. RECOIL lists `.3` and `.RGB` separately, and why they differ (plane order?) is undocumented. |
| RGB | Tricolor (RGB) | **Partial** | zx-image "tricolor" (R,G,B order) | Same family as `.3`. Plane order for this extension is unconfirmed; verify with samples. |
| CH$ | CHR$ (Alone Coder) | **Spec** | SpectraLab guide §chr$; zx-image "chrd" | `"chr$"` + W + H (cells) + bytes-per-cell. bpc 9 is 8 bitmap bytes + attr. bpc 18 is the gigascreen pair. Cells stored row-major. Size = 7 + W·H·bpc. zx-image also lists a mono variant (probably bpc 8, unconfirmed). |
| CH8 | 8x8 font | **Spec** | [ZX-Paintbrush page](https://zx-modules.jimdofree.com/zx-modules-start/zx-paintbrush/); SpectraLab guide §Fonts | 2048 bytes = 256 chars x 8 bytes, MSB = left pixel. |
| CH6 | 6x8 font | **Partial** | ZX-Paintbrush page; [file-extensions.org ch4](https://www.file-extensions.org/ch4-file-extension-zx-spectrum-font) | 2048 bytes, 256 chars, glyphs 6 px wide. Which 6 bits are used (presumably the high 6) is unconfirmed. |
| CH4 | 4x8 font | **Partial** | same as above | 2048 bytes, 256 chars, 4 px wide. Bit position unconfirmed. |
| CHX | Big font (ZX-Editor 2nd ed. / ZX-Paintbrush) | **Partial** | ZX-Paintbrush page; [ZX-Editor](https://worldofspectrum.net/zx-modules/46) | Signature `"CHX"` + zero bytes. Up to 256 chars, each 1x1 to 4x4 cells, coloured or transparent. Body layout is not documented. Its .chm help files (inside the ZX-Editor download) might describe it; not checked. |
| ZXP | ZX-Paintbrush | **Spec** | SpectraLab guide §ZXP | Text file. Header line `ZX-Paintbrush extended image`, then H lines of `0`/`1` chars, a blank line, attribute lines of hex bytes (count gives 8x8/8x4/8x2/8x1), and an optional line with a 64-byte ULA+ palette. |
| SEV | SevenuP | **None** | [SevenuP manual (txt in repo)](https://github.com/pvmm/SevenuP) | The manual only says the format holds sprites, masks and frames. The only layout source is SevenuP's GPL-2 C++ code (**do not read**). Reverse engineer from samples. |
| ZXS | SpecSCII | **None** | [zxart SpecSCII editor](https://zxart.ee/specscii/); SpectraLab guide §SPECSCII (text stream) | SpectraLab documents a `.specscii` PRINT-stream format (control codes 0x10–0x17, 0x0D). Whether `.zxs` is that stream, a SCR, or something else is unknown. Needs samples. |
| FLF | Turbo Rascal Syntax Error | **None** | [justsolve TRSE](http://justsolve.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error); [TRSE downloads](https://lemonspawn.com/turbo-rascal-syntax-error-expected-but-begin/downloads/) | Multi-platform "fluff" image container. Only definition is TRSE's GPL-3 source (**do not read**). Same FLF container as CPC and BBC FLF. |

---

## 3. ZX Spectrum Profi

### Hardware references
- Platform overview (Russian): <https://speccy.info/Profi>. Returned 403 when fetched; found via search only.
- GRF converter thread (Russian): <https://zx-pk.ru/threads/30113-img2grf-konvertor-izobrazhenij-v-fajly-formata-grf.html>
- New-format discussion (Russian): <https://zx-pk.ru/threads/27317-novyj-format-graficheskogo-izobrazheniya-dlya-zx-sovmestimykh-kompyuterov.html>
- 512x240 mode. Monochrome or colour (8x1 attribute) depending on board revision. No English memory-map doc was found.

### Palette
Standard Spectrum colours. Colour boards use a palette (details not found).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| GRF | Profi | **Partial** | [zx-image](https://github.com/moroz1999/zx-image) ("partial support for hi-res 16 colors mode", CC0 code); [Img2Grf thread (RU)](https://zx-pk.ru/threads/30113-img2grf-konvertor-izobrazhenij-v-fajly-formata-grf.html) | 512x240 with 16 colours. Header and layout not documented in prose. The zx-image CC0 code is the cleanest reference, and the Img2Grf thread may describe the format (not fetched). |

---

## 4. ZX Spectrum ULAplus

### Hardware references
- Specification: <https://sinclair.wiki.zxnet.co.uk/wiki/ULAplus>

### Palette
- Palette bytes are GRB332 (G bits 7–5, R bits 4–2, B bits 1–0). The missing low blue bit is the OR of the two blue
  bits.
- 3-bit values expand to 8 bits by repeating the bits.
- 4 CLUTs of 16 entries each (8 ink, 8 paper). The CLUT is selected by `FLASH*2 + BRIGHT`.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SCR | ULAplus | **Spec** | [ULAplus spec](https://sinclair.wiki.zxnet.co.uk/wiki/ULAplus); SpectraLab guide §ULA+; zx-image "ulaplus" | 6976 = SCR + 64-byte palette. The spec also defines Timex hi-colour + palette (12352) and hi-res + palette (12353). SpectraLab also covers ULANext variants (6945–7426 bytes). |

---

## 5. ZX Evolution (TS-Conf)

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SXG | Speccy eXtended Graphics | **Spec** | [hype.retroscene sXg article (RU)](https://hype.retroscene.org/blog/126.html); [moroz1999/sxg writer (CC0)](https://github.com/moroz1999/sxg) and its links to [tslabs format description (RU)](http://forum.tslabs.info/viewtopic.php?f=25&t=526) and [TS-Config video modes (RU)](http://forum.tslabs.info/viewtopic.php?f=35&t=178); [PRONOM fmt/1583](https://www.nationalarchives.gov.uk/pronom/fmt/1583) | 16-byte header: `\x7F"SXG"` + version + background colour + packing (0 = none) + format (1 = 16c, 2 = 256c) + width u16 + height u16 + palette offset u16 + bitmap offset u16. Then a 512-byte palette (256 x u16) and a linear bitmap. Palette has 2 encodings (CLUT or PWM levels; the SXG writer has `SXG_PALETTE_FORMAT_CLUT`). Search snippets mention row padding and upside-down storage, which conflicts with the hype article; verify. tslabs links not fetched. |

---

## 6. ZX Spectrum Next

### Hardware references
- File formats: <https://wiki.specnext.dev/File_Formats>
- Layer 2: <https://wiki.specnext.dev/Layer_2>
- Palettes: <https://wiki.specnext.dev/Palettes>
- ULANext: <https://wiki.specnext.dev/ULANext>
- BMP to NXI/SL2 tools (**MIT**): <https://github.com/stefanbylund/zxnext_bmp_tools>

### Palette
9-bit RGB333 as 2 bytes per entry: `RRRGGGBB`, then `P000000B`.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| NXI | Layer 2 image | **Spec** | [specnext File_Formats](https://wiki.specnext.dev/File_Formats); SpectraLab guide §NXI; zx-image "nxi" | 49152 = 256x192 8bpp, no palette, use the default RGB332 palette. 49664 = 512-byte palette + pixels, row-major. SpectraLab also documents 320x256 (82432) and 640x256 4bpp (81952), both column-major. |

---

## 7. ZX81

### Hardware references
- ZX80/ZX81 video, D_FILE, charset: <https://problemkaputt.de/zxdocs.htm>
  - D_FILE pointer is at 0x400C.
  - Each of the 24 lines is preceded by HALT 0x76, plus a final HALT.
  - Expanded size is 793 bytes. In collapsed form, lines end early at a HALT.
- .P/.81 format note (kio): <https://k1.spdns.de/Develop/Projects/zasm/Info/O80%20and%20P81%20Format.txt>
  - Data loads at 0x4009, so file offset = address − 0x4009.
- Character set: <https://en.wikipedia.org/wiki/ZX81_character_set>
  - Codes 0x00–0x3F, bit 7 = inverse. Glyphs are in ROM at 0x1E00. 2x2 block graphics.

### Palette
Monochrome (black on white).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| P | Sinclair BASIC program / tape image | **Spec** | [kio P81 note](https://k1.spdns.de/Develop/Projects/zasm/Info/O80%20and%20P81%20Format.txt); [zxdocs](https://problemkaputt.de/zxdocs.htm) | Read D_FILE from offset 0x400C−0x4009 = 3, then walk the HALT-terminated lines and render with the ZX81 ROM charset. The charset bitmaps must be supplied from ROM (copyright: check whether a re-drawn font is needed). |
| ZP1 | ZXpaintyONE | **None** | [ZXpaintyONE (HTML5 editor)](http://matt.west.co.tt/apps/zxpaintyone/) (connection refused when fetched); [rsp.retrocomputacion](https://rsp.retrocomputacion.com/gfx-editors-and-utilities/) | Described as a "long string of numbers" giving character codes (32x24). Probably text; needs samples. |
| RAW | ZXpaintyONE v2.0 | **None** | same | Possibly 768 raw character codes. Unverified. |

---

## 8. Timex 2048

### Hardware references
- Timex technical info (port 0xFF):
  - <https://worldofspectrum.org/faq/reference/tmxreference.htm>
  - Mirror: <https://neuro.me.uk/projects/wos/sinclairfaq.dev/dev/reference/tmxreference.htm>
- Port 0xFF bits 0–2 select the mode: 000 screen 0 at 0x4000, 001 screen 1 at 0x6000, 010 hi-colour, 110 hi-res.
- Bits 3–5 select the hi-res ink/paper pair: black/white, blue/yellow, red/cyan, magenta/green, green/magenta,
  cyan/red, yellow/blue, white/black.
- Hi-res mode alternates 8-pixel columns from the 0x4000 and 0x6000 bitmaps.
- Hi-colour mode uses the 0x4000 bitmap with an 8x1 attribute bitmap at 0x6000, laid out in the same interleave.

### Palette
Spectrum palette. Hi-res uses the complementary pairs listed above.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SCR | Hi-color | **Spec** | zx-image "timex81"; [ULAplus spec](https://sinclair.wiki.zxnet.co.uk/wiki/ULAplus); [WoS Timex ref](https://worldofspectrum.org/faq/reference/tmxreference.htm) | 12288 = interleaved bitmap + interleaved 8x1 attrs (a memory dump of 0x4000 and 0x6000). 12352 adds a ULA+ palette. |
| SCR | Hi-res | **Spec** | zx-image "timexhr"; ULAplus spec; WoS Timex ref | 12289 = 6144 (first bitmap) + 6144 (second bitmap) + 1 byte port-0xFF value (colour in bits 3–5). zx-image says the first block holds odd columns and the second even ones; the WoS ref gives 0x4000/0x6000 alternation. Verify order. 12353 adds a ULA+ palette. 512x192, often shown as 512x384 with rows doubled. |
| HRG | Hi-res Gigascreen | **Spec** | zx-image "timexhrg"; bitplane/datatypes PR #43 (24578 bytes) | 24578 = 2 x hi-res screens of 12289 each. Blend the 2 frames. |

---

## 9. SAM Coupé

### Hardware references
- Technical Manual v3.0: <https://sam.speccy.cz/systech/sam-coupe_tech-man_v3-0.pdf>
  - Markdown version: <https://github.com/stefandrissen/sam-coupe-technical-manual> (no license stated)
  - Unofficial technical manual: <https://sam.speccy.cz/systech/unofficial_tech-man_v1-0.pdf>
- Screen modes and SCREEN$ layout (annotated): <https://www.worldofsam.org/products/screen-modes>
- Converter write-up: <https://www.martinfitzpatrick.com/writing-a-sam-coupe-screen-converter-in-python/>
  (also <https://mfitzp.com/article/sam-coupe-image>). Code has no stated license; use the prose only.
- SSX discussion (obo, SimCoupe author): <https://spectrumcomputing.co.uk/forums/viewtopic.php?t=1926>
- Modes:
  - Mode 1: Spectrum layout.
  - Mode 2: 6144 bitmap + 6144 attrs at 8x1, both linear. In memory there is a 2048-byte gap between them.
  - Mode 3: 512x192 at 2bpp.
  - Mode 4: 256x192 at 4bpp, high nibble = left pixel.

### Palette
- 128 colours, 7-bit value. Bits: 0 BLU0, 1 RED0, 2 GRN0, 3 BRIGHT (half-intensity LSB on all channels),
  4 BLU1, 5 RED1, 6 GRN1. Source: Tech Manual "Colour" table.
- 16 CLUT entries per frame. Palettes A and B alternate for FLASH.
- Line interrupts can rewrite CLUT entries per scan line.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SS1 | Mode 1 | **Spec** | [obo SSX post](https://spectrumcomputing.co.uk/forums/viewtopic.php?t=1926) | 6928 = 6144 + 768 + 16 CLUT. A SAM BASIC SCREEN$ dump has a 41-byte palette block instead (see SS4). |
| SS2 | Mode 2 | **Spec** | obo SSX post; worldofsam | 12304 = 6144 bitmap + 6144 attrs + 16 CLUT. |
| SS3 | Mode 3 | **Spec** | obo SSX post; [flatduckrecords/ssx README](https://github.com/flatduckrecords/ssx) (README only; repo is GPL-3) | 24580 = 24576 + 4 CLUT. The size identifies mode 3. Full SCREEN$ dumps are longer (see SS4). |
| SS4, SCS4 | Mode 4 | **Spec** | obo SSX post; [mfitzp article](https://www.martinfitzpatrick.com/writing-a-sam-coupe-screen-converter-in-python/); [worldofsam](https://www.worldofsam.org/products/screen-modes); [justsolve SAM Coupé Mode 4](http://fileformats.archiveteam.org/wiki/SAM_Coup%C3%A9_Mode_4) | 24592 = 24576 + 16 CLUT (SSX style). The SAM BASIC SCREEN$ style is 24576 + 16 pal A + 4 + 16 pal B + 4 + line-interrupt records + 0xFF. Each line-interrupt record is 4 bytes: y stored as 172−y, CLUT index, colour A, colour B. Minimum 24617 bytes. SCS4 is believed to be the SCREEN$-style variant (unconfirmed). |
| SSX | SimCoupe screenshot | **Partial** | obo SSX post | Fixed sizes as in SS1–SS4 above, plus a "raw" variant of 98304 = 512x192 palette indices, used when the mid-frame palette changes. RECOIL says "up to 512x192", which matches. No header was described. Raw pixels might be indices into the 128-colour palette; verify. |
| LCE | 256x384 interlace | **Partial** | zx-image "lce"; [ZX modes page](http://users.atw.hu/zxspectrum/zx_gfx_modes_en.htm) ("256x384 Interlace") | 13824 = 2 x Spectrum SCR. Screen 1 gives even lines and screen 2 odd lines, with no flicker blend. Per zx-image, pixels are doubled horizontally in 512x384 output. RECOIL files this under SAM. The SAM-specific variant, if any, is unknown. |

---

## 10. Amstrad CPC

### Hardware references
- Kevin Thacker's docs index: <https://cpctech.cpcwiki.de/docs.html>
  - Gate Array (inks, mode select, hardware colour numbers): <https://cpctech.cpcwiki.de/docs/garray.html>
  - Pixel bit packing for modes 0/1/2: <https://cpctech.cpcwiki.de/docs/graphics.html>
  - Screen addressing: <https://cpctech.cpcwiki.de/docs/screen.html>, <https://cpctech.cpcwiki.de/docs/scraddr.html>,
    <https://cpctech.cpcwiki.de/docs/32kscreen.html>
  - CRTC: <https://cpctech.cpcwiki.de/docs/crtcnew.html>
  - CPC Plus (ASIC 12-bit palette): <https://cpctech.cpcwiki.de/docs/cpcplus.html>
- Screen line address: `(line & 7) * 0x800 + (line >> 3) * 80`, at 0xC000 by default.
- Modes: 0 is 160x200x16, 1 is 320x200x4, 2 is 640x200x2.
- Overscan and other modes (cpcwiki, Cloudflare-blocked when tried):
  - <https://www.cpcwiki.eu/index.php/Programming:Overscan>
  - <https://www.cpcwiki.eu/index.php/Video_modes>
- AMSDOS header: <https://cpctech.cpcwiki.de/docs/allhead.html>; also <https://www.cpcwiki.eu/index.php/AMSDOS_Header>
  - 128 bytes. Type at 18, load address at 21, length at 24, exec address at 26, 24-bit length at 64.
  - Checksum at 67–68 is the u16 sum of bytes 0..66. A header is present if and only if the checksum matches; skip
    it when it is.

### Palette
- 27 colours: 3 levels (0, 50%, 100%) per R/G/B.
  - Hardware colour table (32 codes, with duplicates): garray.html
  - <https://www.cpcwiki.eu/index.php/CPC_Palette>
- Firmware colour number = R*3 + G*9 + B, using levels 0/1/2.
- Files store either hardware numbers (OCP .PAL stores `0x40 | hw`) or firmware numbers, depending on the program.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SCR + PAL | Advanced OCP Art Studio | **Spec** | [cpctech artstud.html](https://cpctech.cpcwiki.de/docs/artstud.html); [cpcwiki Format page](https://www.cpcwiki.eu/index.php?mobileaction=toggle_view_mobile&title=Format:Advanced_OCP_Art_Studio_File_Formats); [justsolve Advanced Art Studio](http://fileformats.archiveteam.org/wiki/Advanced_Art_Studio) | **SCR:** a 16 KB screen dump at 0xC000 layout, optionally MJH-compressed. **MJH** blocks are `"MJH"` + u16 unpacked length, then RLE: `01 count byte` repeats (count 0 = 256) and other bytes are literal. **PAL** is 239 bytes: mode, animation flag, delay, then 16 pens x 12 animation frames of hw colour (`0x40|n`), 12 border bytes, 16 excluded inks and 16 protected inks. Files usually carry an AMSDOS header. |
| WIN + PAL | Advanced OCP Art Studio window | **Spec** | same | Linear pixel data (raw or MJH). The last 5 bytes are: width in mode-2 pixels (u16), height, reserved. The doc says height is in "lines/2"; verify. |
| FNT | 8x8 font | **Partial** | artstud.html (OCP FNT) | The OCP FNT is 768 bytes = 96 chars (ASCII 32–127) x 8 bytes, MSB = left pixel. Not confirmed that RECOIL's CPC FNT is the OCP format. Other CPC font files exist (e.g. 2048-byte full sets). |
| SGX | SymbOS graphic | **Spec** | [SymbOS wiki: Format-SGX-(Graphic)](https://github.com/Prodatron/symbos-wiki/wiki); [conv2SGX README](https://github.com/salvogendut/conv2SGX) (no license; README only) | A sequence of chunks. **Simple chunk:** byte0 = width in bytes (1–63, bit 7 = ZX0 packed), width px, height px, then CPC mode-1 data. **Extended chunk:** byte0 = 64 or 192, type (0 = 4c, 5 = 16c), u16 bytes per line, u16 width, u16 height; 16-colour data is MSX nibble order with high nibble left. **Other chunks:** 255 = line feed, 0 = EOF. No palette is stored. Fixed 4-colour greys (white, black, light grey, dark grey). The fixed 16-colour palette is given in CPC-Plus RGB. Compression is ZX0 ([BSD-3](https://github.com/einar-saukas/ZX0)) with a SymbOS 6-byte wrapper (last 4 bytes stored first). |
| HGB | HinterGrundBild (FutureOS wallpaper) | **Hardware-only** | [cpcrulez FutureOS HGB](https://cpcrulez.fr/GamesTest//applications_graphic-futureos_wallpaper_hgb.htm); [cpcwiki.de forum (DE)](https://cpcwiki.de/forum/index.php/topic,71.0.html); [cpcwiki forum (DE)](https://www.cpcwiki.eu/forum/anwendungen/ein-wallpaper-hintergrundbild-(hgb)-fur-futureos-erstellen/) | A 16 KB screen file, 512x256, monochrome. This means a mode-2 screen with CRTC set to 64 bytes x 256 lines. Line addressing is presumably the standard `(y&7)*0x800 + (y>>3)*64`; verify with samples. May carry an AMSDOS header. |
| CM5 + GFX | Mode 5 (SyX) | **Partial** | [cpcwiki CM5](https://www.cpcwiki.eu/index.php?title=CM5&redirect=no) (search snippet only) | Per cpcwiki, one screen file in normal Amstrad format plus a `.CM5` file holding the palette, which changes over time (rasters/split rasters on mode 1). RECOIL gives 288x256 with 27 colours, i.e. an overscan screen. The exact CM5 record layout is not documented. |
| PPH + ODD + EVE | Perfect Pix (Batman Group, 2016) | **Partial** | [Perfect Pix manual PDF (cpc-power)](https://www.cpc-power.com/extra_lire_fichier.php?extra=notice&fiche=13139&slot=1&part=B&type=.pdf); [archive.org](https://archive.org/details/CPC-PerfectPix); [justsolve Perfect Pix](http://justsolve.archiveteam.org/wiki/Perfect_Pix) | Per the manual, `.PPH` is a header (mode R/B0/B1, size, palette), and `.ODD`/`.EVE` are *linear raw* data, one line after another, for the 2 frames. Mode R is up to 384x272 with 16 of 27 colours, interlace and half-pixel shift. B0/B1 are 2 mode-0/1 frames flickered at 50 Hz. The PPH byte layout is not documented. |
| FLF | Turbo Rascal Syntax Error | **None** | [justsolve TRSE](http://justsolve.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error) | See Spectrum FLF. Only defined by GPL-3 TRSE code. |

---

## 11. BBC Micro

### Hardware references
- Screen formats (modes, memory, pixel packing): <https://www.dfstudios.co.uk/articles/retro-computing/bbc-micro-screen-formats/>
  - Crypt magazine article: <https://www.dfstudios.co.uk/crypt/Online/47/bbc_screens.html>
- Annotated MOS 1.20 disassembly (mode dimensions, VideoULA control register, pixel masks):
  <https://tobylobster.github.io/mos/mos/S-s4.html>
- Graphics conversion utilities (J.G. Harston): <https://mdfs.net/Apps/Graphics/Conversion/>
- Image format family write-up: <https://nerdoftheherd.com/projects/libbeebimage/> (prose; library is GPL-3)
- Screen sizes: modes 0/1/2 are 20 KB at 0x3000, modes 4/5 are 10 KB at 0x5800.
- Memory is arranged as 8x8 character cells, 8 consecutive bytes per cell, cells left to right, then the next
  character row.
- Pixel packing:
  - 2-colour: MSB = left pixel.
  - 4-colour: pixel n is bits (7−n, 3−n).
  - 16-colour: pixel 0 is bits 7, 5, 3, 1 and pixel 1 is bits 6, 4, 2, 0.
  - These bit positions are interpreted from dfstudios' 1-indexed wording. Check them against the MOS pixel-mask
    tables in tobylobster §11.

### Palette
- 8 physical colours (BGR 1-bit) plus 8 flashing pairs.
- Default logical-to-physical mappings:
  - 2-colour: black, white
  - 4-colour: black, red, yellow, white
  - 16-colour: 0–7 solid, 8–15 flashing
- Screen dumps carry no palette, so use the defaults.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| BB0 | Mode 0 screen dump | **Hardware-only** | dfstudios; tobylobster MOS | 20480 bytes, 640x256, 1bpp, cell layout, 80 cells per row. |
| BB1 | Mode 1 screen dump | **Hardware-only** | same | 20480 bytes, 320x256, 2bpp. |
| BB2 | Mode 2 screen dump | **Hardware-only** | same | 20480 bytes, 160x256, 4bpp. Physical 8–15 are flashing colours; show the first phase. |
| BB4 | Mode 4 screen dump | **Hardware-only** | same | 10240 bytes, 320x256, 1bpp, 40 cells per row. |
| BB5 | Mode 5 screen dump | **Hardware-only** | same | 10240 bytes, 160x256, 2bpp. Short files (screen saved from a scrolled base or truncated) should be checked against samples. |
| BBG | LdPic / SvPic (Acorn User, Oct 1986) | **Spec** | [nerdoftheherd LdPic spec](https://nerdoftheherd.com/projects/libbeebimage/ldpic/); [justsolve LdPic](http://justsolve.archiveteam.org/wiki/LdPic) | The file is a bitstream, and multi-bit values have their bits reversed. Header fields: A = bits per stored byte (8), B = mode (8), C = 16 x 4-bit logical-to-physical mapping, colour 15 first (64), D = address step (8), E = repeat-count width (8). Then RLE: a 1 flag means count(E) + value(A) and a 0 flag means value(A), with a step-and-wrap address walk. |
| FLF | Turbo Rascal Syntax Error | **None** | [justsolve TRSE](http://justsolve.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error) | GPL-3 TRSE only. |

---

## 12. Oric

### Hardware references
- Oric coding part 7 (HIRES at 0xA000, 8000 bytes = 40 x 200): <https://www.defence-force.org/computing/oric/coding/part_7/index.htm>
- Oric graphics in detail: <https://osdk.org/index.php?page=articles&ref=ART9>
  - Serial attributes; ink/paper reset to white on black at the start of each line.
- HIRES colour attributes: <http://thespider.oric.org/oric_hires_colour.html>
- HIRES mode: <http://twilighte.oric.org/twinew/hiresmode.htm>; graphics overview: <http://twilighte.oric.org/twinew/graphics.htm>
- Serial attribute notes: <https://github.com/xahmol/oricdemo2026/blob/main/docs/oric.md>, <https://github.com/xahmol/oricdemo2026/blob/main/docs/hires.md> (check license)
- Atmos manual: <https://defence-force.org/computing/oric/library/lib_manual_oric/files/manual_atmos.pdf>
- Byte decoding:
  - `(b & 0x60) == 0` means a serial attribute that renders as 6 paper pixels:
    - 0–7 ink
    - 8–15 text attributes, incl. double-height and blink
    - 16–23 paper
    - 24–31 video mode
  - Otherwise bits 0–5 are 6 pixels with MSB on the left.
  - Bit 7 inverts the cell (colour XOR 7).

### Palette
8 fixed colours, RGB 1-bit each: black, red, green, yellow, blue, magenta, cyan, white.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| HIR, HRS | HIRES screen (240x200) | **Hardware-only** | defence-force part 7; OSDK ART9; thespider; [justsolve Oric HIRES screen](http://justsolve.archiveteam.org/wiki/Oric_HIRES_screen) | A raw dump of 8000 bytes (0xA000–0xBF3F). Some files may carry an Oric tape header or extra bytes up to 0xBFFF; check sizes against samples. |
| CHS | 6x8 font | **Hardware-only** | defence-force part 7; OSDK ART9 | Oric charset RAM holds 96 chars x 8 bytes = 768 bytes (standard set at 0xB400; a second set follows). Only the low 6 bits are used. Exact size and whether the alternate set is included: verify with samples. |

---

## 13. Electronika BK (BK-0010 / BK-0011M)

### Hardware references
- Platform overview: <https://en.wikipedia.org/wiki/Electronika_BK>; Russian: <https://ru.wikipedia.org/wiki/БК_(семейство_компьютеров)>
- Hardware overview with screen notes (Russian): <https://alemorf.github.io/retro_computers/computer.html?id=BK0010>
- Documentation sets (Russian):
  - BK0010-01 docs, screen chapter: <https://github.com/prcoder-1/bk0010-01-docs/blob/main/09-%D1%8D%D0%BA%D1%80%D0%B0%D0%BD-%D0%B8-%D0%BA%D0%BB%D0%B0%D0%B2%D0%B8%D0%B0%D1%82%D1%83%D1%80%D0%B0.md>.
    Seen in search; returned 404 when fetched. Its snippet confirms colour bitmasks, green = 0xAAAA and blue = 0x5555.
  - Emuverse user manual: <https://www.emuverse.ru/wiki/БК-0010_-_Руководство_пользователя>
  - Emulator documentation index: <https://gid.pdp-11.net/docstable.html>
- Operating manuals (Russian):
  - <https://archive.org/details/BK0010OperatingManual>
  - <https://archive.org/download/bk0010-bk0010-1-instruction-manual/BK0010%20%26%20BK0010-1%20Instruction%20manual.pdf>
- Screen RAM is 16 KB at octal 040000 (0x4000), 64 bytes per line, 256 lines, and supports hardware scroll.
- Mono mode is 512x256. Colour mode is 256x256 at 2bpp. The PDP-11 is little-endian, so the LSB of each word is the
  leftmost pixel (to verify).

### Palette
- BK-0010 colour mode: 0 black, 1 blue, 2 green, 3 red (to verify against the screen chapter above).
- BK-0011M adds 16 selectable 4-colour palettes. No table of these was found in the fetched sources.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PIC | 256x256 4-colour screen | **Hardware-only** | sources above | Presumably 16384-byte raw screen dumps, possibly with a BK 4-byte address/length header. Check sizes with samples. |
| BKS | BK screen (up to 512x256, 1–2 frames) | **None** | [bk-utils (converter)](https://github.com/zakirov-net/bk-utils); [BK software catalog](https://kalininskiy.github.io/bk-catalog/) | No format description found. "1–2 frames" suggests a BK-0011 two-page variant. Check bk-utils' license before reading it. |

---

## 14. Electronika MC 0515

### Hardware references
- Emuverse (Russian, CC-BY-SA): <https://emuverse.ru/wiki/Электроника_МС_0515>
  - Modes are 640x200 at 2 colours and 320x200 with 8 colours, 2 per 8-pixel octet (Spectrum-like).
  - The palette is the Spectrum's, with a border.
- Russian Wikipedia: <https://ru.wikipedia.org/wiki/Электроника_МС_0515>
- Museum page: <http://www.leningrad.su/museum/show_calc.php?n=267>
- Video memory layout and bit order were **not** found in any fetched source.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SCR | 640x200 mono screen | **Hardware-only** | emuverse; ru.wikipedia | Expected 16000 bytes (80 x 200). Line order and bit order are unknown; derive from samples or emulator docs. |

---

## 15. Vector-06C

### Hardware references
- Video system overview: <https://en.wikipedia.org/wiki/Vector-06C>
  - 4 bit planes.
  - Each plane is 32 columns of 8-pixel bytes, running bottom-to-top within a column, columns left to right.
  - Modes are 256x256 with 16 of 256 colours, or 512x256 with 4 colours.
- Hardware overview (Russian): <https://alemorf.github.io/retro_computers/computer.html?id=Vector_06C>
- Sprite output (Russian, CC-BY-SA): <https://www.emuverse.ru/wiki/Вектор-06Ц/Вывод_спрайта>
- FPGA replica: <https://svofski.github.io/vector06cc/>

### Palette
- 256 colours from an 8-bit value. Commonly cited as BBGGGRRR; the source was not fetched, so verify.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| SPR | Vector-06C graphics file | **Partial** | [drilnet/vector-06c-spr2bmp Readme (RU)](https://github.com/drilnet/vector-06c-spr2bmp/blob/master/Readme.RUS.UTF8.txt) | The readme says `Info SPR.7z` explains "how an SPR file is built" and bundles an unpacker in Vector assembler. So SPR is likely compressed, with 16 colours. **The repo is GPL-3.** The archive mixes prose with GPL asm. Have one person extract only the prose description, or reverse engineer from the sample archives in the same repo (sample data is not code). |

---

## 16. Sample file sources (URLs only, nothing downloaded)

- **Spectrum, Timex, SAM, Next, Profi, SXG, ZX81 art:** ZX-Art <https://zxart.ee/>.
  zx-image exists to render zxart uploads, so every format it handles has samples there.
- **Spectrum software archives:**
  - World of Spectrum <https://worldofspectrum.org/>
  - Spectrum Computing <https://spectrumcomputing.co.uk/>
- **MultiArtist:** <https://multiartist.untergrund.net/>
- **SpectraLab repo:** may include example images; not checked.
- **CPC:**
  - CPC-Power <https://www.cpc-power.com/> (Perfect Pix entry and manual linked above)
  - Perfect Pix: <https://archive.org/details/CPC-PerfectPix>
  - FutureOS wallpapers: see the HGB links.
- **SGX:** conv2SGX `samples/` directory <https://github.com/salvogendut/conv2SGX>
- **BBC:**
  - Stardot forum tools thread <https://stardot.org.uk/forums/viewtopic.php?t=33208>
  - mdfs <https://mdfs.net/Apps/Graphics/Conversion/>
  - justsolve LdPic page mentions "dexvert samples"
- **Oric:** defence-force <https://www.defence-force.org/>
- **BK:** BK catalog <https://kalininskiy.github.io/bk-catalog/>
- **Vector-06C:** SPR sample archives in <https://github.com/drilnet/vector-06c-spr2bmp>
  (the `Graphics Files Vector-06C (SPR).*.7z` files; data, not code)

---

## 17. Permissive references (may be read)

| Project | License | Covers | Caveat |
|---|---|---|---|
| [moroz1999/zx-image](https://github.com/moroz1999/zx-image) | CC0-1.0 | Most Spectrum-family formats, SAM 3/4, Timex, NXI, GRF, SXG | Long-lived zxart code. We cannot rule out that it was cross-checked against RECOIL, though nothing suggests so. |
| [moroz1999/sxg](https://github.com/moroz1999/sxg) | CC0-1.0 | SXG writer | — |
| [Bedazzle/SpectraLab](https://github.com/Bedazzle/SpectraLab) | MIT | Spectrum formats (guide + JS code) | Created 2026-01. Provenance of its format knowledge is unknown, so prefer the guide's prose over its code. |
| [bitplane/datatypes](https://github.com/bitplane/datatypes) (PR #43 zxscr) | MIT | Spectrum extended screens by size | Provenance unknown. |
| [stefanbylund/zxnext_bmp_tools](https://github.com/stefanbylund/zxnext_bmp_tools) | MIT | NXI/SL2 | — |
| [einar-saukas/ZX0](https://github.com/einar-saukas/ZX0) | BSD-3-Clause | ZX0 decompression (SGX) | — |
| [ha1tch/zentools pkg/scr](https://pkg.go.dev/github.com/ha1tch/zentools/pkg/scr) | Apache-2.0 | SCR only | — |

## 18. To avoid (GPL code)

Do not read the source of any of these:

- **RECOIL** (all languages and ports).
- **TRSE / Turbo Rascal Syntax Error**, GPL-3.0: <https://github.com/leuat/TRSE> and forks. This is the only FLF definition.
- **SevenuP**, GPL-2.0: <https://github.com/pvmm/SevenuP>. This is the only SEV definition. Its `SevenuP.txt` user
  manual was skimmed and contains no layout.
- **GrafX2**, GPL-2.0: its `cpcformats.c`, and the doxygen pages generated from it
  (<https://grafx2.gitlab.io/grafX2/doxygen/html/group__cpcformats.html>). Not opened.
- **martine** (CPC converter), GPL-2.0: <https://github.com/jeromelesaux/martine>.
- **libbeebimage**, GPL-3.0: <https://github.com/ribbons/libbeebimage>. Its website prose spec (LdPic) was used; the
  code was not.
- **SimCoupe**, GPL-2.0: <https://github.com/simonowen/simcoupe>. It is the reference SSX writer.
- **flatduckrecords/ssx** (SAM viewer for Next), GPL-3.0. Only the README was read.
- **vector-06c-spr2bmp**, GPL-3.0: <https://github.com/drilnet/vector-06c-spr2bmp>, including the asm in `Info SPR.7z`.
- **SymbOS repos** (Prodatron/*), GPL-3.0. The symbos-wiki *documentation* page for SGX was used; no code was read.
- **symbosvm**, GPL-2.0.

Not license-checked, so treat as tainted until checked:

- zakirov-net/bk-utils
- dotkoval/* flicker tools
- xahmol/oricdemo2026
- mfitzp/scrimage (no license file)
- conv2SGX (no license; only its README was read)

## 19. Findings from implementation (wave 1)

Checked against corpus samples and `recoil2png` output (black box). These settle open questions above.

- **Spectrum palette:** 0xCD normal, 0xFF bright. Mono 6144-byte SCR is white on black.
- **Frame blending:** gigascreen frames (IMG, HLR, STL, MG*, BSP, CH$ bpc 18, HRG) are averaged per channel, rounded
  down. Tricolor (`.3`, `.RGB`) is additive at 0xFF, not averaged.
- **`.3` plane order** is blue, red, green; **`.RGB`** is red, green, blue.
- **ATR** pattern is `55 AA` (the first row starts with paper).
- **MC** has a linear bitmap and linear 8x1 attributes (zx-image is right). MLT has the interleaved bitmap.
- **Border formats** (BSC, BMC4, BSP) render 384x304 with the screen at (64, 64); border colours are non-bright.
- **Timex hi-res** uses bright colours and doubles rows to 512x384. **ULAplus** blue is the 2-bit value times 0x55.
- **SXG** palette/bitmap offsets count from the end of their own field. Palette entries without bit 15 are 25-level
  indices: `level * 255 / 24`, rounded down.
- **NXI** RGB333 widens by bit repetition.
- **SAM `SCREEN$`** files: screen memory (mode 2 attributes at offset 8192), 40-byte palette table, 4-byte line
  interrupt records `(line, entry, colour, flash colour)` up to `FF`. Changes apply from `line + 1`. In mode 3
  `SCREEN$` files, pixel values 1 and 2 map to CLUT entries 2 and 1 (SSX does not swap). Mode 3 and raw SSX
  output rows are doubled (512x384). LCE is two mode 4 `SCREEN$` files interlaced to 512x384.
- **BBC LdPic:** the address step stays constant; each pass starts one byte lower until offset 0.
- **Oric** HRS/CHS samples carry an Oric tape header (`16 16 16 24`, 9 header bytes with big-endian end/start
  addresses, zero-terminated name). CHS loads at 0xB500 (769 bytes).
- **CPC HGB** is the standard line layout with 64-byte rows, rows doubled to 512x512. **SGX** levels map 4-bit
  `8` to 0x80; greys are 0xAA/0x55.
- **Electronika BK PIC:** 16384 bytes, lowest bit pair leftmost. **MC 0515 SCR:** 16000 bytes linear, MSB left,
  rows doubled.

## 20. Findings from implementation (wave 2)

Checked against corpus samples and `recoil2png` output (black box).

- **Content detection** (`.signature()`): SXG, BSP (border runs must use up their data exactly), MultiArtist,
  CH$, CHX, ZX-Paintbrush, Profi GRF (fixed 10-byte header), Oric tape HIRES screens, LdPic (the bit stream ends
  in the file's last byte in every sample) and AMSDOS-headed files whose header names them `.SCR`. Not marked: SGX
  (no magic), Oric charset blocks (most program tapes carry one), SAM `SCREEN$` bodies and all headerless dumps.
- **AMSDOS headers**: the 24-bit length at 64 excludes the disk record padding that files copied off disk images
  keep; RECOIL counts the padding and rejects such files.
- **CPC SCR/WIN + PAL**: SCR is 16384 or 16336 bytes, or four MJH blocks of 4096. MJH block: `MJH`, u16 unpacked
  length, `01 count value` runs (count 0 = 256), other bytes literal. WIN trailer (last 5 bytes): width in mode 2
  pixels as u16 at -4, height at -2. RECOIL needs a PAL of exactly 239 bytes, mode 0-2, and the first colour of
  every pen in 0x40-0x5F; it rejects WIN with a mode 1/2 PAL. Output: mode 0 pixels doubled horizontally, mode 2
  rows doubled. Without a PAL we show mode 1 with the power-on inks (RECOIL rejects; recorded as divergences).
- **CPC Mode 5 (CM5 + GFX)**: GFX is 256 linear lines of 72 bytes (288 mode 1 pixels). CM5 is the pen 3 colour,
  then per line pen 2, pen 1 and six pen 0 colours, one per 48-pixel band. RECOIL also rejects colours stored
  without bit 6.
- **Perfect Pix (PPH + ODD + EVE)**: PPH is kind (3 = R, 4 = B0, 5 = B1), width in mode 1 pixels and height
  (u16), zone count, then per zone 16 (mode 0) or 4 (mode 1) firmware colours plus a line count (none after the
  last). ODD/EVE are linear frames, blended. In mode R the second frame is shifted half a pixel left on even
  lines and the first on odd lines; the gap is black.
- **CPC FNT**: 768 bytes, shown as a 32x3 sheet, white on black.
- **ZX81 P**: RECOIL doesn't show the saved display file (blank in every sample). It runs picture programs:
  optional FAST/CLS/CLEAR/SLOW, then PRINT lines with string literals, `AT r,c` and `;`, ended by STOP, PAUSE
  or GOTO. ZXpaintyONE programs also put a 64-character `A$` on the bottom two lines. Other programs are
  rejected. Glyphs come from a ZX81 ROM dump (archive.org `ts1000-roms`).
- **ZX80/ZX81 S80/S81**: RECOIL rejects them; not on its list.
- **SpecSCII ZXS**: RECOIL rejects SpectraLab's `.specscii` text streams and simple test streams; its ZXS layout
  is unknown, so it is not implemented.
- **Profi GRF**: 128-byte header (RECOIL accepts only `00 02 F0 00 04 00 80 00 01 13...`), a GRB332 palette of 16
  at offset 10, then 240 lines of 64 (bitmap, attribute) pairs. Ink is bits 2-0 plus bit 6, paper is bits 5-3
  plus bit 7. Rows doubled.
- **CHX**: `CHX`, 256 u16 file offsets at 5. Each character: flag (0 = attribute after each cell, 1 = none),
  width and height in cells, then the cells row by row. Drawn 16 per row in slots of the largest size over a
  black/0xCDCDCD checkerboard.
- **BK BKS**: 16384 bytes mono (512x256, LSB left, rows doubled), plus one palette byte per screen for colour
  (BK-0011M palettes 0-15, colours observed from RECOIL); 2 screens blended.
- **Vector-06C SPR**: compressed and in a scheme not recognised from the samples. Its only description sits in a
  GPL archive, so it is not implemented. (Solved from samples in wave 3, see below.)

## 21. Findings from implementation (wave 3)

Checked against corpus samples and `recoil2png` output (black box, including hand-made probe files).

- **Vector-06C SPR**: 16 palette bytes (`BBGGGRRR`), then a run-length stream read *backwards* from the last byte
  of the file: `0x80 | n` repeats the byte before it `n` times, `n` < 0x80 takes the `n` bytes before it. Output
  fills the 32768-byte screen from its end, so the file's first plane holds the highest index bit. Each plane is
  32 columns of 256 bytes, rows bottom to top. Decoding stops when the screen is full: bytes between the palette and
  the stream (2 in every sample) and trailing zero padding (literals of 0) are ignored; RECOIL rejects a stream that
  needs the palette bytes. Colours: red `round(r * 255 / 7)`, green `g * 36`, blue `b * 85`. 256x256 output.
