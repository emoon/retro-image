# Atari 8-bit, VBXE and Portfolio image formats: documentation survey (clean-room)

Scope: the Atari 8-bit (143 entries), Atari 8-bit VBXE (2) and Atari Portfolio (2) formats on RECOIL's format list. Every source below is a prose spec, a manual, a wiki page, a forum thread, a hardware document or the original program's own documentation. **No GPL/LGPL decoder source was opened.** When permissively licensed code exists, its license is stated.

Resolutions and colour counts in the Notes column come from the RECOIL [formats list](https://recoil.sourceforge.net/formats.html), which is a plain list of facts. File sizes and magic bytes come from the Just Solve pages unless another source is given.

## 1. Summary

The table rows group extensions the way RECOIL groups them. Counts are per table row (e.g. "AP3/APV/DGI/DGP/ESC/ILC/PZM" is one row; "RM0-RM4" is one row).

| Platform | Rows | Spec | Partial | Hardware-only | None |
|---|---|---|---|---|---|
| Atari 8-bit | 139 | 17 | 60 | 5 | 57 |
| Atari 8-bit VBXE | 2 | 0 | 1 | 1 | 0 |
| Atari Portfolio | 2 | 2 | 0 | 0 | 0 |
| **Total** | **143** | **19** | **61** | **6** | **57** |

Overall, Atari 8-bit is much worse documented than the C64. Most scene formats (Polish/Czech/Hungarian 1990s interlace editors) only have a file size or a magic string on Just Solve, plus a prose description of how the display mode works. Many "Partial" rows are fixed-size raw screen dumps where only the colour-register tail is uncertain, so they are cheap to finish against samples. The hard ones are the formats with an undocumented packer: the "SFDN" compression shared by APP/APS/G9S/HPS/ILS/INS/PLS/SFD, plus the CCI, CPI, CPR, XLP and MAX packers.

### Best umbrella sources (read these first)

1. **Just Solve the File Format Problem, [Atari graphics formats](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats)**. This index lists every 8-bit format with links to per-format pages, which give sizes, magic bytes, the original program's download page and a dexvert sample folder. *Caveat:* the [AP*](http://fileformats.archiveteam.org/wiki/AP*) page cites RECOIL source as its reference for file sizes. Use such sizes as plain facts only and do not follow the reference. (The site only answers over plain `http://`.)
2. **[Mad Studio file formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf)** by the author of Mad Studio. It gives full byte tables for GR0, AN2, GR1, GR2, AN4, AN5, GR3, TL4, SPR, MPL and MSL. The Mad Studio repository is **MIT** ([github.com/Gury8/Mad-Studio](https://github.com/Gury8/Mad-Studio)), so the original program is also a permissible reference.
3. **[Graph2Font manual (English)](https://g2f.atari8.info/instrukcja_eng.html)** gives the MIC layout (7680/9600/11520 bytes, last 4 bytes = colour registers 712, 708, 709, 710), plus FNT, SCR, TAB, COL, PMG, INV and ALL as G2F uses them. It also has [A Graphician's Guide (PDF)](https://g2f.atari8.info/graph2font_a_graphicians_guide.pdf).
4. **[Koala picture files by Jiri Bernasek (BEWESOFT)](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/KOALA%20PICTURE%20FILES.txt)**: a full spec of the Koala/Micro Illustrator `.PIC` header and its 3 compression methods.
5. **[XL-Paint 1.9 MaX info (Polish)](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt)** gives the XLP, RAW, MIC and MAX layouts, and confirms INP = 16004 bytes and MIC = 7684 bytes.
6. **Explanations of the software display modes** (APAC, HIP, RIP, TIP, CIN, Super IRG, ColorView and others):
   - [Atari Software Graphic Modes (atari-owner.com)](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/)
   - Wikipedia's "Software-driven graphics modes for the Atari 8-bit computers" ([mirror](https://en-academic.com/dic.nsf/enwiki/5875720))
   - [AtariWiki: APAC Graphics Mode](https://atariwiki.org/wiki/Wiki.jsp?page=APAC+Graphics+Mode)
   - [Mad Team article on TIP/HIP (Polish)](https://madteam.atari8.info/index.php?atarynka=tip)
7. **Extension lists with a resolution, colour count and compression flag per extension**: [AtariWiki: File Suffix](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix) and the [Atarimania FAQ on filename extensions](http://www.atarimania.com/machines/atari-400-800-xl-xe/faq/86).

## 2. Atari 8-bit

### Hardware references

- **[Altirra Hardware Reference Manual](https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf)** (Avery Lee; a free PDF, prose only). It is the most precise ANTIC/GTIA reference: modes, display list, DMA, PMG, GTIA modes 9/10/11, the GTIA mode-10 half-pixel shift that HIP depends on, NTSC/PAL artifacting and colour generation. Altirra the emulator is GPL, so read only the manual, not the emulator source.
- **De Re Atari** (Atari, 1981):
  - [Ch. 2 ANTIC and the display list](https://www.atariarchives.org/dere/chapt02.php) has the mode table: bytes per line, scan lines and colours per ANTIC mode.
  - [Ch. 3 Graphics indirection](https://www.atariarchives.org/dere/chapt03.php) covers colour registers and character sets.
  - [Ch. 4 Player-missile graphics](https://www.atariarchives.org/dere/chapt04.php).
  - [App. D Artifacting](https://www.atariarchives.org/dere/chaptD.php).
  - [App. E GTIA modes 9/10/11](https://www.atariarchives.org/dere/chaptE.php).
- **[Mapping the Atari, Appendix 15: XL/XE graphics modes](https://www.atariarchives.org/mapping/appendix15.php)** gives OS shadow register addresses: 708-712 = COLOR0-4 (PF0-PF3, BAK) and 704-707 = PCOL0-3. Many file formats store these addresses in that order.
- [Atari Player-Missile Graphics in BASIC, ch. 2](https://www.atariarchives.org/pmgraphics/chapter2.php) covers the PMG memory layout, which the PLA, MIS, 4PL, SPR, MPL, MSL and APL formats depend on.
- [Wikipedia: CTIA and GTIA](https://en.wikipedia.org/wiki/CTIA_and_GTIA). Also [atari800xl.eu KB: CTIA/GTIA](https://www.atari800xl.eu/docs/kb/kb-hardware-0001-atari-8bit-ctia-gtia.html) and [atari800xl.eu KB: 256 colours](https://www.atari800xl.eu/docs/kb/kb-hardware-0002-atari-8bit-256-colors.html).

**Fonts:** character-mode formats (GR0, AN2, AN4, AN5, GR1, GR2, ART, ASC, SGE, DLM, MAP) often store screen codes only and rely on the OS ROM font. That font is Atari copyright. A clean decoder must either ship its own glyphs or need the font from the user, so check this before shipping.

### Palette

- The palette is 16 hues × 16 luminances. Outside GTIA mode 9 (and the modes built on it), luminance bit 0 is ignored, which leaves 128 distinct colours.
- The colour-generation section of the [Altirra Hardware Reference Manual](https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf) describes how NTSC and PAL colours are produced, so a palette can be computed from the documented model instead of copied.
- [Mad Studio `palettes/`](https://github.com/Gury8/Mad-Studio/tree/master/palettes) has `.act` files: `laoo`, `altirra`, `jakub`, `g2f`, `atari800winplus`, `OlivierN/P`, `Xformer` and `real.pal`. The repository is MIT, but these palettes were collected from elsewhere and their own provenance and licences are unknown.
- [Lospec: Atari 8-bit GTIA palette](https://lospec.com/palette-list/atari-8-bit-series-gtia) (no licence stated). [Wikimedia: Atari 8-bit GTIA NTSC palette](https://commons.wikimedia.org/wiki/File:Atari_8_bit_GTIA_NTSC_palette.png): check its licence on the file page.
- AtariAge measurement and discussion threads: [Atari 128 colour palettes](https://forums.atariage.com/topic/243369-atari-128-color-palettes/), [PAL GTIA hardware details](https://forums.atariage.com/topic/203957-pal-gtia-hardware-details/), [RGB values for PAL and NTSC colours](https://forums.atariage.com/topic/331917-rgb-values-for-pal-and-ntsc-colours/). The "laoo" palette is described there as the more accurate one.
- Emulator palettes from Atari800 and Altirra are GPL code. Do not lift tables from their source.

### Sample files

- dexvert sample folders, one per format, are linked from each Just Solve page, e.g. [image/apStar](http://sembiance.com/fileFormatSamples/image/apStar), [image/hip](http://sembiance.com/fileFormatSamples/image/hip), [image/madStudio](http://sembiance.com/fileFormatSamples/image/madStudio), [image/koalaMicroillustrator](http://sembiance.com/fileFormatSamples/image/koalaMicroillustrator), [image/xlPaint](http://sembiance.com/fileFormatSamples/image/xlPaint), [font/nlq](http://sembiance.com/fileFormatSamples/font/nlq). The folder names follow the Just Solve page names.
- [Atari Graphics Archive](http://gfa.atari-users.net/gfx/).
- [pigwa.net Atari Forever tools collection](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/): the original editors with sample pictures, as ATR images and loose files (e.g. InterPainter `.INT`, Marco Pixel Editor `.CPI`, `ATARI.SIF`).
- [umich Atari 8-bit graphics collections](http://www.umich.edu/~archive/atari/8bit/Graphics/Collections/) is behind a Cloudflare challenge.
- [whatis.rest7.com/Atari](http://whatis.rest7.com/Atari).
- [Mad Studio `examples/`](https://github.com/Gury8/Mad-Studio) (MIT repository).
- The atarionline.pl utility pages linked in the table usually include sample pictures.

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| 256, AP2 | Paint 256 | Partial | [Just Solve AP*](http://fileformats.archiveteam.org/wiki/AP*), [AtariWiki File Suffix](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix), [AtariWiki APAC](https://atariwiki.org/wiki/Wiki.jsp?page=APAC+Graphics+Mode) | 80x96, 256 colours (APAC). Sizes 7680/7684/7720. AP2 is listed as "APAC-2 80x192x256 noncompressed". Hue/lum line order to verify against samples. |
| 4MI | AtariTools-800 4 missiles | None | [Just Solve AtariTools-800](http://fileformats.archiveteam.org/wiki/AtariTools-800), [homepage](https://ataritools.fr.gd/ATARITOOLS_800--k1-8_BIT-k2-.htm) | No layout or size published. Probably a PMG data dump. |
| 4PL | AtariTools-800 4 players | None | same as above | As for 4MI. |
| 4PM | AtariTools-800 4 players + missiles | None | same as above | As for 4MI. |
| A4R | Anime 4ever slideshow | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Anime_4ever_slideshow), [pouet](http://www.pouet.net/prod.php?which=71261) | 80x256, 16 greys. |
| ACS | AtariTools-800 font | Partial | [Just Solve AtariTools-800](http://fileformats.archiveteam.org/wiki/AtariTools-800) | Exactly 7690 bytes. 4 colours. Internal layout unknown. |
| AGP | AtariTools-800 graphic | Partial | same as above | Exactly 7690 bytes (≈7680 screen + 10?). |
| AGS | Atari Graphics Studio | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats) | Nothing found. |
| ALL | Graph | Partial | [Just Solve Graph](http://fileformats.archiveteam.org/wiki/Graph), [G2F manual](https://g2f.atari8.info/instrukcja_eng.html) | 160x192, 5 colours. G2F's save-only "ALL" = FNT+SCR+TAB in one file. Whether this is the same format is unconfirmed. |
| AN2 | Mad Studio ANTIC 2 | Spec | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) | maxX, maxY, then up to 960 screen codes. No font is stored. |
| AN4 | Mad Studio ANTIC 4 | Spec | same | maxX, maxY, then 5 colours (COLOR4, 0-3), then up to 960 screen codes. No font is stored. |
| AN5 | Mad Studio ANTIC 5 | Spec | same | Same as AN4, up to 480 screen codes. |
| AP3, APV, DGI, DGP, ESC, ILC, PZM | 80x192 APAC-interlace family (Digi Paint, EscalPaint, Pryzm Artist, ...) | Partial | [Just Solve Digi Paint](http://fileformats.archiveteam.org/wiki/Digi_Paint), [AtariWiki File Suffix](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | 15360 or 15362 bytes (AP3: 15872). 2 frames of GR9/GR11, 256 colours. Uncompressed. Frame order unverified. |
| APA, APC, PLM | 80x96 APAC | Partial | [Just Solve AP*](http://fileformats.archiveteam.org/wiki/AP*), [AtariWiki File Suffix](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix) | APC = "80x96x256 noncompressed". 7680/7684/7720 bytes. GR9/GR11 lines alternate. |
| APL | Atari Player Editor (Playsoft) | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats), [SprEd thread](https://forums.atariage.com/topic/330217-spred-new-atari-sprite-editor/) | Multi-frame PMG animation. SprEd can load it. 48-line player height is the default. |
| APP | Apac3 Linker-Viewer | Partial | [Just Solve Apac3 APP](http://fileformats.archiveteam.org/wiki/Apac3_APP) | Starts with ASCII `S101`. "SFDN" compressed (packer reverse engineered, see section 8). 80x192, 2 frames. |
| APS | Any Point, Any Color | Partial | [AtariWiki APAC](https://atariwiki.org/wiki/Wiki.jsp?page=APAC+Graphics+Mode) | 80x96 APAC, SFDN compressed (undocumented). |
| ART | Ascii-Art Editor | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Ascii-Art_Editor), [atarionline.pl](https://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Ascii-Art%20Editor) | Up to 64x24 characters, mono. |
| ART | Artist by David Eaton | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Artist_(David_Eaton)) | 160x80, 4 colours. The extension is arbitrary. |
| ART | (mono, unknown origin) | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | Mono. Nothing else found. |
| ASC | ASCII maker | Hardware-only | [De Re Atari ch. 2](https://www.atariarchives.org/dere/chapt02.php) | 40x24 characters, mono. Probably 960 ANTIC-2 screen codes (compare Mad Studio GR0). Needs a font. |
| BG9, G09 | 160x192 16-grey | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats) | 160x192, 16 greys, 1 frame. Layout unknown. |
| BGP | Bugbiter APAC239i | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Bugbiter_APAC239i), [AtariAge thread](https://atariage.com/forums/topic/216997-apac-256-color-mode-80x240-interlaced-has-this-been-done/) | Starts with ASCII `BUGBITER_APAC239I_PICTURE`. 80x239, 256 colours, 2 frames. |
| BKG | Movie Maker background | Partial | [Just Solve Movie Maker](http://fileformats.archiveteam.org/wiki/Movie_Maker), [ANTIC: Rapid Graphics Converter](https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html) | 3856 bytes = 3840 (160x96 GR7) + 16 bytes. 4 colours. |
| CCI | Champions' Interlace (compressed) | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Champions%27_Interlace) | Compressed CIN. Packer undocumented. |
| CHR | Blazing Paddles font | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats), [Blazing Paddles manual (archive.org)](https://archive.org/details/BlazingPaddlesAtariSupplementManualBaudville) | Mono. |
| CIN | Champions' Interlace | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Champions%27_Interlace), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/), [atarionline.pl](https://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Champions%27%20Interlace) | 15360, 16004 or 16384 bytes. GR15 and GR11 alternate per scanline. Up to 160x200, 2 frames. |
| CPI | Marco Pixel Editor | Partial | [MPE.DOC](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/MARCO%20PIXEL%20EDITOR%202.1/MPE.DOC), [archive.org](https://archive.org/details/a8b_Marco_Pixel_Editor_v2.1_1995_11_15_Marco) | 160x192, 4 colours. The doc says CPI = compressed save and PIC = plain save. The packer is undocumented. Samples are in the same pigwa dir. |
| CPR | Trzmiel | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Trzmiel) | 320x192 mono, compressed. |
| CUT | Cut Creator | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 96x99 mono. |
| DIN | DIN | None | [Just Solve](http://fileformats.archiveteam.org/wiki/DIN), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | 320x192, 10 colours, 2 frames. A Super-IRG-style text-mode variant. |
| DIT | DrawIt (Antic Software) | Hardware-only | [Just Solve](http://fileformats.archiveteam.org/wiki/DrawIt_(Atari)), [Atarimania](http://www.atarimania.com/utility-atari-400-800-xl-xe-drawit_30485.html) | Exactly 3845 bytes = 3840 (160x96 GR7) + 5 colour bytes (assumed). |
| DLM | Dir Logo Maker | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Dir_Logo_Maker), [atarionline.pl](http://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Dir%20Logo%20Maker) | Exactly 256 bytes, starts with `B`. 11x16 characters, mono. |
| DRG | Atari CAD | Hardware-only | [Just Solve](http://fileformats.archiveteam.org/wiki/AtariCAD) | Exactly 6400 bytes = 320x160 GR8 raw. |
| F80 | The Last Word font | None | [The Last Word page](https://atari8.co.uk/the-last-word/), [manual PDF](https://atari8.co.uk/wp-content/uploads/2015/03/The-Last-Word-2.1.pdf) | An 80-column editor font. The manual does not give the layout. |
| FGE | Floor Designer | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Floor_Designer), [atarionline.pl](http://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Floor%20Designer) | 64x40, 16 greys. |
| FN2 | Atari FontMaker | Partial | [matosimi Atari FontMaker](http://matosimi.websupport.sk/atari/atari-fontmaker/), [github](https://github.com/matosimi/atari-fontmaker) | "Dual font": 2 × 1024-byte charsets (inferred). The GitHub repo has **no licence**, so do not read or copy its code. |
| FNT | 8x8 font | Spec | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html), [De Re Atari ch. 3](https://www.atariarchives.org/dere/chapt03.php) | 1024-byte raw charset, 128 chars × 8 bytes. It may carry a 6-byte DOS binary header. |
| FWA | Fun with Art | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Fun_with_Art), [ANTIC: Rapid Graphics Converter](https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html) | "Slightly longer than 62 sectors" = GR15 screen plus per-line colours (DLI). 160x192, 128 colours. |
| G10 | Graphics 10 | Partial | [Just Solve GR*](http://fileformats.archiveteam.org/wiki/GR*), [De Re Atari App. E](https://www.atariarchives.org/dere/chaptE.php) | 7689 bytes = 7680 + 9 colour registers (704-712). Up to 80x240. |
| G11 | Graphics 11 | Hardware-only | [De Re Atari App. E](https://www.atariarchives.org/dere/chaptE.php) | Raw GTIA mode 11 dump. Up to 80x240. Height from size/40. |
| G2F | Graph2Font | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Graph2Font), [G2F manual](https://g2f.atari8.info/instrukcja_eng.html), [G2F site](http://g2f.atari8.info/) | Starts with `G2FZLIB` (zlib container). The components (FNT/SCR/TAB/COL/PMG) are documented, but the container layout is not (reverse engineered, see section 8). Up to 336x240. |
| G9S, SFD | Graphics 9 (SFDN compressed) | Partial | [De Re Atari App. E](https://www.atariarchives.org/dere/chaptE.php) | GR9 data is documented. The SFDN packer is not. |
| GED | GED | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/GED) | 11302 bytes, starts `FF FF` (DOS binary-load header). 160x200, 128 colours (per-line colours). |
| GHG | Gephard Hires Graphics | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Gephard_Hires_Graphics) | Up to 320x200 mono. |
| GR0, ASC, SCR | Mad Studio Graphics 0 | Spec | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) | 960 bytes of screen codes (40x24). No font is stored. |
| GR1 | Mad Studio Graphics 1 | Spec | same | 480 screen bytes + COLOR4, COLOR0-3 = 485 bytes. |
| GR2 | Mad Studio Graphics 2 | Spec | same | 240 screen bytes + 5 colours = 245 bytes. |
| GR3 | Mad Studio Graphics 3 | Spec | same | 240 bytes + COLOR4, COLOR0-2 = 244 bytes. 40x24, 4 colours. |
| GR7 | Graphics 7 | Partial | [Just Solve GR*](http://fileformats.archiveteam.org/wiki/GR*), [AtariWiki File Suffix](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix) | 3844 bytes = 3840 + 4 colours. Up to 160x120. |
| GR8 | Graphics 8 | Partial | same | 7680/7682/7684 bytes. Up to 320x240. Tail = colours. |
| GR9 | Graphics 9 | Partial | same | 7680/7682/7684 bytes. Up to 80x240, 16 greys. |
| GR9P | Graphics 9+ | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 80x60, 16 greys (a GR9 variant with 4-line pixels?). |
| HCI, HR2 | 320x200 interlace | Partial | [Just Solve HCI](http://fileformats.archiveteam.org/wiki/HCI) | HCI is exactly 16006 bytes (2 × 8000 + 6?). 5 colours, 2 frames. |
| HCM | Hard Color Map | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 128x192, 9 colours. |
| HIP | Hard Interlace Picture | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Hard_Interlace_Picture), [Mad Team TIP/HIP article](https://madteam.atari8.info/index.php?atarynka=tip), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/), [Altirra HW manual](https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf) | GR9 and GR10 frames alternate. GR10 is shifted by ½ pixel. 160x200, 30 greys. About 16 KB. No byte layout is published. |
| HPM | Grass' Slideshow | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Grass%27_Slideshow), [slideshow page](http://atari.fox-1.nl/atari-400-800-xl-xe/grass-slideshow/) | 160x192, 4 colours. Do not confuse with HiRes Player Missile `.hpm` (19203 bytes, [Just Solve](http://fileformats.archiveteam.org/wiki/HiRes_Player_Missile)). |
| HPS | Hard Interlace Picture (SFDN) | Partial | as HIP | HIP data + undocumented SFDN packer. |
| HR | 256x239 | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 256x239, 3 colours, 2 frames. |
| ICE | Interlace Character Editor font | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Interlace_Character_Editor_font), [AtariAge ICE beta](http://atariage.com/forums/topic/171666-new-beta-of-ice-font-editor/), [atarionline forum](http://atarionline.pl/forum/comments.php?DiscussionID=450) | 2-frame font. |
| ICN | ICE CIN | None | [Just Solve ICE](http://fileformats.archiveteam.org/wiki/ICE_(Atari)), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | 160x192, 80 colours, 2 frames (charset-flip CIN). |
| IGE | Interlace Graphics Editor | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Interlace_Graphics_Editor), [atarionline.pl](https://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Interlace%20Graphics%20Editor#Interlace%20Graphics%20Editor) | 128x96, 16 colours, 2 frames. |
| ILD | Interlace Logo Designer | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Interlace_Logo_Designer), [ILD.DOC](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/INTERLACE%20LOGO%20DESIGNER%201.0/ILD.DOC) | 128x128, 7 colours. The bundled doc has key bindings only. |
| ILS | APACVIEW (SFDN) | None | [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | 80x192, 256 colours, 2 frames. SFDN packed. |
| IMN | ICE MIN | None | [Just Solve ICE](http://fileformats.archiveteam.org/wiki/ICE_(Atari)) | 160x192, 80 colours, 2 frames. |
| ING | ING 15 | None | [Just Solve](http://fileformats.archiveteam.org/wiki/ING_15) | 160x200, 7 colours, 2 frames. |
| INP | InterPainter | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/InterPainter), [XL-Paint doc](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/), [gury.atari8.info](http://gury.atari8.info/software/1523.php) | Usually 16004 bytes = 2 × 8000 (160x200 GR15 frames, flipped per VBI) + 4 colours (inferred). |
| INS | InterPainter (SFDN) | Partial | as INP | INP + undocumented SFDN packer. |
| INT | INT95a | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/INT95a) | Starts with ASCII `INT95a`. Up to 160x239, 16 colours, 2 frames. Samples: [pigwa InterPainter dir](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/InterPainter/) (its `.INT` files). |
| IP2 | ICE PCIN+ | None | [Just Solve ICE](http://fileformats.archiveteam.org/wiki/ICE_(Atari)) | 160x192, 45 colours, 2 frames. |
| IPC | ICE PCIN | None | same | 160x192, 35 colours, 2 frames. |
| IR2 | Super IRG 2 | Partial | [SIFE.TXT (Bill Kendrick)](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/Super%20IRG%20Font%20Editor/SIFE.TXT), [AtariAge Super IRG modes](https://forums.atariage.com/topic/186653-super-irg-modes-using-graphics-1/) | 160x192, 25 colours. Charset and colour registers flip per VBI. File layout unknown. |
| IRG | Super IRG | Partial | same, [Just Solve](http://fileformats.archiveteam.org/wiki/Super_IRG) | 160x192, 15 colours. 2 charsets flip per VBI, ANTIC mode 4. File layout unknown. |
| IST | Atari Interlace Studio | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Atari_Interlaced_Studio), [Mad Team tools](http://madteam.atari8.info/index.php?prod=uzytki), [AtariAge AIS thread](https://forums.atariage.com/topic/148954-ais-atari-interlace-studio/) | Exactly 17184 bytes. 160x200, 2 frames. |
| JGP | Jet Graphics Planner | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Jet_Graphics_Planner), [archive.org](https://archive.org/details/a8b_Jet_Graphics_Planner_v1.0_Small_1993_Zolna_Dariusz_pl) | Exactly 2054 bytes. 4 colours (tiles/charset?). |
| KPR | Kompresor do Animatora | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Kompresor_do_Animatora), [atarionline.pl](http://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Kompresor%20do%20grafiki%20z%20Animatora#Kompresor%20do%20grafiki%20z%20Animatora) | 4 colours, compressed. |
| KSS | KSS-Paint | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 160x160, 4 colours. The extension is arbitrary. |
| LDM | Ludek Maker | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Ludek_Maker), [atarionline.pl](https://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=Ludek%20Maker#Ludek%20Maker) | Starts with inverse-ATASCII "Ludek Maker data file". 4 colours. |
| LEO | Larka Edytor Obiektów | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Larka_Edytor_Obiekt) | ANTIC 4 objects, 5 colours. |
| LUM+COL | Technicolor Dream | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Technicolor_Dream), [gury.atari8.info](http://gury.atari8.info/software/511.php) | 2 files, 4766 bytes each. `.lum` starts with `04`. 80x119, 256 colours. |
| KFX | KFX | None | [Just Solve](http://fileformats.archiveteam.org/wiki/KFX_(Atari_graphics_format)) | 56x60 mono. |
| MAP | Envision | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Envision), [Envision site](http://www.user.dccnet.com/dschebek/envision.htm) | Character map up to 512x512. Even the EnvisionPC author did not know this layout. |
| MAP | EnvisionPC | Partial | [EnvisionPC manual](http://ftp.pigwa.net/stuff/collections/holmes%20cd/Holmes%202/PC%20Atari%20Programming%20Utils/EnvisionPC%20V0.5/envision.txt), [EnvisionPC page](http://ftp.pigwa.net/stuff/collections/holmes%20cd/Holmes%202/PC%20Atari%20Programming%20Utils/EnvisionPC%20V0.5/index.html) | ANTIC mode (1), width (2, LE), height (2, LE), PF0-PF4 (5), "...", map (w×h), font (1024). The "..." gap is unexplained. The zip includes C source (licence unstated). |
| MAX | XL-Paint MAX | Partial | [XL-Paint doc](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt), [Just Solve](http://fileformats.archiveteam.org/wiki/XL-Paint) | `XLPM`, then 8 × 193-byte per-line tables (colour 0-3, luminance 0-3), then a 193-byte palette table, then compressed data (packer undocumented). 160x192, 2 frames. |
| MBG | Mad Designer | Hardware-only | [Just Solve](http://fileformats.archiveteam.org/wiki/Mad_Designer), [gury.atari8.info](http://gury.atari8.info/software/1330.php) | Exactly 16384 bytes = 512x256 mono bitmap (64 bytes/line). |
| MCH | Graph2Font | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Graph2Font), [G2F manual](https://g2f.atari8.info/instrukcja_eng.html) | Up to 336x240. Layout not published; reverse engineered, see section 8. |
| MCP | McPainter | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/McPainter), [McPainter page](http://east.atari8.info/mcp/index.htm) | Exactly 16008 bytes (2 × 8000 + 8 colours?). 160x200, 16 colours, 2 frames. |
| MCPP | Paradox | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Paradox_(graphics)), [Demozoo](https://demozoo.org/productions/111562/) | Exactly 8008 bytes. 160x100, 16 colours. |
| MCS | MCS | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/MCS) | Exactly 10185 bytes. 160x192, 9 colours. |
| MGA | 80x96 | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 80x96, 256 colours. |
| MGP | Magic Painter | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Magic_Painter), [gury.atari8.info](http://gury.atari8.info/software/1550.php) | 3845 bytes, starts `F4 0E 36 00`. 160x96 (GR7-size body). |
| MIC | Micro Illustrator / Graphics 15 (Micro-Painter) | Spec | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html), [Atarimania FAQ](http://www.atarimania.com/machines/atari-400-800-xl-xe/faq/86), [ANTIC: Rapid Graphics Converter](https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html) | 7680/9600/11520 bytes of ANTIC E data + 4 bytes of colours (712, 708, 709, 710). Up to 160x240. |
| MIC | AtariGraphics | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/AtariGraphics) | 7680 or 7684 bytes. 160x192. The extension is arbitrary. |
| MIC+COL | Graph2Font | Partial | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html) | COL = 5 × 256 bytes of per-line colours. Register order within COL to confirm. 160x240, 128 colours. |
| MIC+PMG+RP+RP.INI | RastaConverter | Partial | [help.txt](https://github.com/ilmenit/RastaConverter/blob/master/help.txt), [repo](https://github.com/ilmenit/RastaConverter), [AtariWiki](https://atariwiki.org/wiki/Wiki.jsp?page=Rastaconverter) | RP = text "raster program" of per-line register writes, executed by the Generator display kernel. MIC = bitmap, PMG = sprite data. **The repo has no licence**: docs only, no code. |
| MIS | AtariTools-800 missile | None | [Just Solve AtariTools-800](http://fileformats.archiveteam.org/wiki/AtariTools-800) | 2x240 mono. Probably a raw missile dump. |
| MPL | Mad Studio multi-color player | Spec | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) | Height, 4 X positions, 4 colours, 4 sizes, a 3rd-colour flag, then 4 player blocks. Max 174 bytes. The PDF's per-player offset formula looks garbled, so check it against samples. |
| MSL | Mad Studio missile | Spec | same | Height, colour, 34 bytes of data = 36 bytes. |
| NLQ | Daisy-Dot font | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Daisy-Dot_font), [PRONOM fmt/1547](https://www.nationalarchives.gov.uk/PRONOM/fmt/1547), [Daisy-Dot III User's Guide](https://ia601000.us.archive.org/20/items/DaisyDotIIIUsersGuide/Daisy_Dot_III_Users_Guide.pdf), [AtariAge: Daisy-Dot source](https://atariage.com/forums/topic/291455-daisy-dot-source-code/) | DD I/II signature is `'3' 0x9B`. Some DD III files start with `DAISY-DOT NLQ FONT`. **monobit (MIT)** has a Daisy-Dot reader: [github.com/robhagemans/monobit](https://github.com/robhagemans/monobit). |
| ODF | OD Font Editor | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | Mono font. |
| PGR | PowerGraphics | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/PowerGraphics) | ASCII `PowerGFX` at offset 8. Up to 336x240, 256 colours. |
| PI8 | up to 320x192 | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | Nothing found. |
| PI9 | PI9 | None | same | Nothing found. |
| PIC | Graphic Arts Department | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats), [program ATR (pigwa)](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/Graphics%20Art%20Department/) | 160x96, 128 colours. |
| PIC | Koala MicroIllustrator | Spec | [Bewesoft KOALA PICTURE FILES](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/KOALA%20PICTURE%20FILES.txt), [Just Solve](http://fileformats.archiveteam.org/wiki/Koala_MicroIllustrator) | Magic `FF 80 C9 C7`, 28-byte header (window, 5 colours), method 0/1/2. Method 1 is vertical/interleaved order. RLE with bit 7 = literal run and a 16-bit length escape. 160x192. |
| PIC | PIC v2 | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | Up to 320x192. |
| PIC | Visualizer | Partial | [ANTIC: Rapid Graphics Converter](https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html) | "About 31 sectors". 160x79, 4 colours. |
| PIX | 160x192 | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 160x192, 4 colours. Probably a GR15 dump. Size unknown. |
| PLA | AtariTools-800 player | None | [Just Solve AtariTools-800](http://fileformats.archiveteam.org/wiki/AtariTools-800) | 8x240 mono. Probably a raw player dump. |
| PLS | Plama 256 (SFDN) | None | [Just Solve AP*](http://fileformats.archiveteam.org/wiki/AP*) | 80x96 APAC, SFDN packed. |
| PMD | PMG Designer (H. Karpowicz) | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/PMG_Designer), [atarionline.pl](http://atarionline.pl/v01/index.php?ct=utils&sub=2.%20Grafika&tg=PMG%20Designer) | Starts with `F0 ED E4`. |
| PSF | Print Shop | Partial | [AtariAge: Print Shop graphics](https://forums.atariage.com/topic/324752-print-shop-atari-related-graphics/), [Just Solve: The Print Shop](http://justsolve.archiveteam.org/wiki/The_Print_Shop) | 88x52 mono. Print Shop's generic graphic is a 572-byte raw bitmap (11 × 52). The Atari PSF is extracted from the data-disk directory (32-byte records). |
| RAP | Vidig Paint | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Vidig_Paint), [gury.atari8.info](http://gury.atari8.info/software/1647.php) | Exactly 7681 bytes = 7680 + 1. 80x192, 16 colours (GR9/GR11 style). |
| RAW | XL-Paint MAX | Spec | [XL-Paint doc](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt), [Just Solve](http://fileformats.archiveteam.org/wiki/XL-Paint) | `XLPB` + 7680 + 7680 bytes (2 frames), with no colours stored (defaults needed). Just Solve also reports 792-byte variants. |
| RGB | ColorViewSquash | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/ColorViewSquash), [atari-owner modes (ColrView)](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | Starts with `RGB1`. R/G/B frames (3 frames). Up to 160x192. The squash packer is undocumented. |
| RIP | Rocky Interlace Picture | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Rocky_Interlace_Picture), [atari-owner modes](https://atari-owner.com/club/articles/atari-software-graphic-modes.17/) | Starts with `RIP`. A colour extension of HIP. Up to 320x239, 1 or 2 frames. |
| RM0-RM4 | Rambrandt | None | [Just Solve](http://fileformats.archiveteam.org/wiki/RAMbrandt), [gury 476](http://gury.atari8.info/software/476.php), [gury 962](http://gury.atari8.info/software/962.php), [Rambrandt docs ATR (pigwa)](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/RambRant/) | RM0 160x96/99 colours, RM1 80x192/256, RM2 80x192/104, RM3 80x192/128, RM4 160x192/99. There is a docs disk, which was not read (an ATR image). |
| RYS | Mamut | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Mamut) | 160x96, 4 colours. |
| SG3 | Standard Graphics 3 | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Standard_Graphics_3_(Atari)), [AtariAge GR3 planner](https://atariage.com/forums/topic/225305-graphics-3-planner-early-beta-version/) | 40x24, 4 colours (GR3 = 240 bytes + colours?). |
| SGE | Semi-Graphic logos Editor | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Semi-Graphic_logos_Editor) | 40x24 characters, mono. |
| SHC | SAMAR Hi-res Interlace | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/SAMAR_Hires_Interlace), [AtariAge discussion](https://atariage.com/forums/topic/138640-640-x-200-x-2-color-mode-discussion/?tab=comments#comment-1675241) | Exactly 17920 bytes. 320x192 with a colour map, 2 frames (layout reverse engineered, see section 8). |
| SHP | Blazing Paddles shape table | None | [Blazing Paddles manual (archive.org)](https://archive.org/details/BlazingPaddlesAtariSupplementManualBaudville), [Just Solve](http://justsolve.archiveteam.org/wiki/Blazing_Paddles) | Mono vector/shape table. |
| SHP | Movie Maker shapes | Partial | [Just Solve Movie Maker](http://fileformats.archiveteam.org/wiki/Movie_Maker), [Wikipedia](https://en.wikipedia.org/wiki/Movie_Maker_(Reston_Publishing)) | 1024 or 4384 bytes. 160x96, 4 colours. |
| SIF | Super-IRG Font | Spec | [SIFE.TXT](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/Super%20IRG%20Font%20Editor/SIFE.TXT) | 2 × 1024-byte ANTIC 4 charsets, flipped per VBI (2048 bytes). Colours are not stored. Sample `ATARI.SIF` is in the same directory. |
| SKP | Sketch-PadDles | Spec | [Sketch-PadDles page (with BASIC listing)](https://www.vitoco.cl/atari/10liner/SKETCH/), [Just Solve](http://fileformats.archiveteam.org/wiki/Sketch-PadDles) | A raw `BPUT` of 40×192 = 7680 bytes of GR15 screen, with no header. Just Solve says files start with `tm89PS`, which is probably pixel data, not a header (verify). |
| SPC | The Graphics Magician Picture Painter | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/The_Graphics_Magician_Picture_Painter), [Apple II Picture Painter disassembly (A. McFadden)](https://6502disassembly.com/a2-graphics-magician/), [Wikipedia](https://en.wikipedia.org/wiki/Graphics_Magician) | Vector command stream: high nibble = op, 0-2 argument bytes. Documented for Apple II only. The Atari port (160x192, 128 colours) may differ. |
| SPR | Mad Studio sprite | Spec | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) | Height, colour, 40 bytes of data = 42 bytes. |
| SPR | SprEd | None | [AtariAge SprEd thread](https://forums.atariage.com/topic/330217-spred-new-atari-sprite-editor/) | Tri-colour sprites, up to 128 lines and 256 frames. No layout published. |
| SXS | 16x16 font | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats) | Mono. |
| TIP | Taquart Interlace Picture | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/Taquart_Interlace_Picture), [Mad Team TIP article](https://madteam.atari8.info/index.php?atarynka=tip) | `TIP` + version byte `01`. Up to 160x119, 2 frames, GR9/10/11. The article says no formal file format was ever defined. TipTools is GPL (avoid). |
| TL4 | Mad Studio ANTIC 4 tile | Spec | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) | Width, height, then 9 bytes per char (8 glyph bytes + an inverse flag). Max 4x5 chars. |
| TX0 | Texture Maker0 | None | [Just Solve](http://fileformats.archiveteam.org/wiki/Texture_Maker0) | 16x16, 16 colours. |
| TXE | Texture Editor by Mikey | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats) | 80x96, 16 greys. |
| TXS | TXS | None | [Just Solve](http://fileformats.archiveteam.org/wiki/TXS) | 16x16, 16 greys. |
| VSC+G2F | Graph2Font vertical scroll | Partial | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html) | G2F container + scroll file. Layout unknown. |
| VZI | VertiZontal Interlacing | Partial | [Just Solve](http://fileformats.archiveteam.org/wiki/VertiZontal_Interlacing) | Exactly 16000 bytes = 2 × 8000 (160x200 frames, inferred). 31 greys. |
| WND | Blazing Paddles window | None | [Just Solve list](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats), [Blazing Paddles manual](https://archive.org/details/BlazingPaddlesAtariSupplementManualBaudville) | Up to 160x192, 4 colours. |
| XLP | XL-Paint | Partial | [XL-Paint doc](http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt), [archive.org XL-Paint MaX](https://archive.org/details/a8b_XL_Paint_v2.3MaX_2003_06_08_Stanley_TeBe_pl) | `XLPC` + 4 colour bytes + compressed 2-frame data (packer undocumented). Up to 160x200, 7 colours. |
| ZM4 | Zoom-4 graphics editor | None | [RECOIL formats list](https://recoil.sourceforge.net/formats.html) | 64x64, 16 greys. |

## 3. Atari 8-bit VBXE

### Hardware references

- [VBXE FPGA core "FX" Programmer's Manual (T. Piórek)](https://www.mathyvannisselroy.nl/VBXE/VBXE%20fx_en.pdf) covers the XDL, overlay modes (including 8 bpp), palettes and colour registers. Also available: [FX core v1.20 (Scribd)](https://www.scribd.com/document/24580323/VideoBoard-XE-FX-Core-Version-1-20) and an [early vbxe.pdf (atariarea)](https://atariarea.krap.pl/pliki/rozne/vbxe.pdf).
- [AtariAge: VBXE tutorials index](https://forums.atariage.com/topic/346667-vbxe-tutorials-summary-of-all-vbxe-topics/) and [VBXE tutorial: the XDL](https://forums.atariage.com/topic/346498-vbxe-tutorial-the-xdl/).

### Palette

VBXE palettes are 7-bit-per-channel RGB values that the program loads, so every file must carry its own palette. There is no fixed hardware palette.

### Sample files

[dexvert image/dap](http://sembiance.com/fileFormatSamples/image/dap). [Mad Team VBXE page](https://madteam.atari8.info/index.php?prod=vbxe).

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| DAP | SlideShow for VBXE | Hardware-only | [Just Solve](http://fileformats.archiveteam.org/wiki/SlideShow_for_VBXE), [VBXE FX manual](https://www.mathyvannisselroy.nl/VBXE/VBXE%20fx_en.pdf) | Exactly 77568 bytes = 320×240 8-bpp (76800) + 768-byte palette (256 × RGB). The split is inferred from the size. Palette position and bit depth need checking against samples. |
| G2F | Graph2Font (VBXE) | Partial | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html), [Just Solve](http://fileformats.archiveteam.org/wiki/Graph2Font) | The `G2FZLIB` container as above, with VBXE extensions. Up to 336x240, 256 colours. |

## 4. Atari Portfolio

### Hardware references

- [atari-portfolio.co.uk: graphics library page](http://www.atari-portfolio.co.uk/library/pages/tx-graphics.html), [PGF image notes](http://www.atari-portfolio.co.uk/library/pages/tx-pgf-img.html), [PGC image notes](http://www.atari-portfolio.co.uk/library/pages/tx-pgc-img.html).
- The screen is 240x64 monochrome LCD, 1 bpp, 30 bytes/row ([PGC spec](http://www.textfiles.com/programming/FORMATS/pgcspec.txt)).

### Palette

Two levels (LCD on/off). Pick a pleasant LCD green/grey or plain black/white.

### Sample files

[dexvert image/pgc](http://sembiance.com/fileFormatSamples/image/pgc), [dexvert image/portfolioGraphics](http://sembiance.com/fileFormatSamples/image/portfolioGraphics), [dexvert image/pgx](http://sembiance.com/fileFormatSamples/image/pgx) (PGX animations are not in scope).

### Formats

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| PGC | Portfolio Graphics Compressed | Spec | [PGC spec (Don Messerli, 1991)](http://www.textfiles.com/programming/FORMATS/pgcspec.txt), [PGCSPEC.ZIP](http://cd.textfiles.com/blackphiles/PHILES/CODING/SPECS/PGCSPEC.ZIP), [Just Solve](http://fileformats.archiveteam.org/wiki/PGC_(Portfolio_Graphics_Compressed)) | Magic `50 47 01`. Index byte: bit 7 set = repeat the next byte (low 7 bits) times; clear = copy (low 7 bits) literal bytes. Decodes to 1920 bytes. |
| PGF | Portfolio Graphics | Spec | [Just Solve](http://fileformats.archiveteam.org/wiki/PGF_(Portfolio_Graphics)), [PGC spec](http://www.textfiles.com/programming/FORMATS/pgcspec.txt) | No header, exactly 1920 bytes = 64 rows × 30 bytes, 1 bpp, MSB = leftmost (verify bit polarity). |

## 5. To avoid (GPL / unclear-licence code)

Do not open the source of any of these:

- **RECOIL** (GPL), in any language port or fork, including the `recoil.ci` file that Just Solve's AP* page cites.
- **TipTools** by epi ([github.com/epi/TipTools](https://github.com/epi/TipTools)), **GPL-2.0**. It has TIP/HIP conversion and viewer code.
- **Atari800 emulator** (GPL). Avoid its palette tables and any picture loaders too.
- **Altirra** emulator source (GPL). The Hardware Reference Manual PDF is fine.
- **ScummVM** (GPL). Its Comprehend engine renders Graphics Magician pictures.

Repositories with no licence, or a licence not verified, are all-rights-reserved by default. Read their docs but not their code:

- RastaConverter ([ilmenit/RastaConverter](https://github.com/ilmenit/RastaConverter)): no licence file. Only `help.txt` was read.
- Atari FontMaker ([matosimi/atari-fontmaker](https://github.com/matosimi/atari-fontmaker)): no licence file.
- EnvisionPC v0.5: the zip bundles C source with no licence stated.
- Daisy-Dot source code thread on AtariAge: the original program's source, licence unknown.
- [PNGCrushCS (Hawkynt)](https://github.com/Hawkynt/PNGCrushCS): claims reading of Graph2Font scrolls and other formats. Licence and provenance are unverified, and it may be derived from RECOIL.
- abydos ([snisurset.net/code/abydos](http://snisurset.net/code/abydos/)): listed by Just Solve for AtariCAD and HiRes Player Missile. Licence not checked.
- Konvertor, Tom's Editor and XnView: closed source. Their Atari support may wrap RECOIL.
- dexvert: a converter framework that calls RECOIL. Use only its sample files.

Permissive references confirmed:

- **Mad Studio** (MIT): [github.com/Gury8/Mad-Studio](https://github.com/Gury8/Mad-Studio), Free Pascal source plus the formats PDF.
- **monobit** (MIT): [github.com/robhagemans/monobit](https://github.com/robhagemans/monobit). It reads Daisy-Dot `.nlq/.nl2/.nl3/.nl4` and raw 8x8 fonts.

## 6. Implementation notes (wave 1)

Decoders live in `crates/retro-image/src/platform/atari8/`. Conventions observed from
`recoil2png` output (black box, default PAL):

- **Palette**: every channel is `clamp(base[hue] + 0x11 * luminance)`; the 16 per-hue bases
  were read back from synthetic MIC files covering all 128 even colours, and the odd
  luminances were confirmed with GR9 (greys) and APAC/TIP samples (hues). Outside GTIA
  mode 9 the luminance bit 0 is ignored. GTIA mode 9 ORs the pixel into the background
  register.
- **Canvas**: 320 pixels wide for 40-byte modes (160-pixel modes drawn 2 wide, GTIA modes
  4 wide); GR7 lines are drawn twice.
- **Interlace**: two frames are shown as the per-channel average of their RGB values,
  rounded down. APAC/CIN/TIP hue lines take the averaged luminance of their neighbours;
  HIP/VZI/TIP frames sit 1 output pixel left/right of the 4-pixel grid.
- **Defaults** when a file stores no colours: GR8 `00/0E`, GR15 greys `00 04 08 0C`,
  OS colours `00 28 CA 94` (SG3, TL4), G11 luminance 6.

Implemented: GR3, SG3, GR7, DIT, BKG, MGP, GR8, DRG, MBG, PSF, GR9, RAP, G10, G11, MIC,
SKP, AGP, PIC (Koala, Visualizer), 256/AP2, APA/APC/PLM, AP3/APV/DGI/DGP/ESC/ILC/PZM, CIN,
HIP, VZI, TIP, INP, INT, HCI/HR2, IST, MCP, MCPP, FNT, FN2, SIF, ACS, JGP, NLQ (Daisy-Dot II),
SPR/MPL/MSL/TL4 (Mad Studio), DAP (VBXE), PGF/PGC (Portfolio).

## 7. Implementation notes (wave 2)

New since wave 1 (all checked against `recoil2png` unless listed as a divergence in
`crates/retro-image/tests/divergences/atari8.tsv`):

- **ROM font**: text modes use the standard character set of the Atari XL/XE OS ROM
  (rev. 2, `ATARIXL.ROM`, MD5 06daac977823773a3eea3422fd26a703, from archive.org's
  `atari-8-bit-bios-files`), at $E000. `recoil2png` renders this set, not the
  international one at $CC00. Embedded in `atari8/rom_font.rs`.
- **Text formats**: GR0/ASC/SCR/SGE (960 screen codes; we also take 24-30 lines),
  AN2, GR1, GR2, AN4, AN5 (Mad Studio PDF layouts), DLM (16 DOS directory entries, the
  11 name bytes are ATASCII, converted to screen codes).
- **Companion files**: MIC+COL (a 1024- or 1280-byte G2F `.COL` gives per-line colours,
  table = pixel value, entry = line, but only for 240-line MIC files), LUM+COL
  (Technicolor Dream: hue/luminance scanline pairs like TIP; the luminance alone in
  greys). Technicolor Dream's own disk holds run-length packed LUM/COL files
  ((value, count) pairs after the 6-byte header); the packed HAYWAIN pair unpacks to
  RECOIL's sample byte for byte, so we unpack them too (RECOIL rejects them).
- **Other new formats**: Daisy-Dot III NLQ (monobit's MIT reader; RECOIL rejects),
  Magic Painter saved as `.PIC` (no rainbow flag, screen at offset 5), G09, TXE, ZM4,
  TX0, WND, SXS (16x16 font), ODF (8x10 font), F80 (4x8 font), PLA, MIS; JGP at any
  load address; HIP as two 192-line binary-load frames.
- **Content detection** (`.signature()`): INT95a, TIP, NLQ, PGC. Not Koala (Rambrandt
  RM0-RM4 files start with a Koala header) and not JGP (a generic binary-load header).
- **SFDN**: solved in wave 3 (section 8).
- **Not attempted** (MCH, G2F, SHC since done in wave 3, section 8): G2F/MCH/VSC (the G2F container is undocumented and needs raster
  and PMG emulation), SHC (the colour map is a list of mid-line register writes),
  Blazing Paddles CHR (proportional glyphs behind a pointer table), RastaConverter
  (no samples).

## 8. Implementation notes (wave 3)

- **SFDN** (APP/APS/G9S/HPS/ILS/INS/PLS/SFD), `atari8/sfdn.rs`. Reverse engineered by
  feeding `recoil2png` hand-made `.G9S` files (GR9 shows each nibble as a grey):
  `S101`, unpacked length (LE16), a 16-byte table of nibble deltas (most frequent
  first; only the low nibble counts), then an MSB-first bitstream. The first nibble
  is 4 raw bits; each next nibble is the previous minus `table[rank]` (mod 16), where
  the rank is coded as `k` one bits, a zero, and one more bit: `rank = 2k + bit`
  (`00`, `01`, `100`, `101`, `1100`, ... `111111101`). Eight ones in a row are
  rejected. Nibbles fill bytes high nibble first. Each extension takes exactly one
  unpacked length (probed): G9S/SFD 7680 (GR9), PLS 7680 (interleaved APAC), APS 7720
  (interleaved APAC), APP 15872 and ILS 15360 (interlaced APAC), INS 16004
  (InterPainter), HPS 16009 (HIP with registers). Every corpus sample consumes its
  bitstream to the last byte and matches RECOIL. We also take G9S/SFD that unpack to
  7684 bytes (the MGV12 disk's GIRL1/GIRL2; RECOIL rejects them). No `.signature()`:
  the header doesn't say which picture format is inside (7680 bytes is GR9 or PLS).
- **MCH** (Graph2Font), `atari8/graph2font/mch.rs`; renderer in `atari8/graph2font.rs`, GTIA logic in `atari8/gtia.rs`.
  Exactly 30833 bytes (40 columns) or 32993 (48). 30 rows of 9-byte cells (code byte,
  then the 8 bytes shown), then 20 per-scanline tables of 240 bytes (COLBK, COLPF0-3,
  COLPM0-3, HPOSP0-3, HPOSM0-3, SIZEP0-3 packed, SIZEM, PRIOR), GRAFM per scanline,
  and 4 × 256 bytes of player memory (scanline y at byte 16 + y). The rest (13856
  bytes, then a 113-byte tail) has no effect in RECOIL. The first code byte's low 6
  bits are the mode: 01 ANTIC 2, 05 ANTIC 4, 09/19/29 ANTIC 2 + GTIA 9/10/11. The
  picture is 336x240. RECOIL emulates GTIA: the priority equations ORing every
  surviving colour register (checked for all 64 PRIOR values), PRIOR bits 6-7 ORed
  into the header's GTIA mode per scanline, GTIA 10 delayed 2 pixels, and so on (the
  module doc lists every rule). 150 random synthetic MCH files render identically to
  `recoil2png`. Mode 07 (RastaConverter conversions made with G2F's `rc2mch`) is
  rejected by RECOIL; it probably uses the ignored region for mid-line register
  changes, and is not decoded.
- **G2F** (Graph2Font), `atari8/graph2font/g2f.rs`, with a clean-room zlib/DEFLATE
  decoder written from RFC 1950/1951 in `atari8/inflate.rs` (nothing else in the crate
  needs it yet; it can move to `codec` if something does). `G2FZLIB`, then a zlib
  stream of the editor's memory (160-330 KB inflated). Susanne's G2F and MCH render
  identically in RECOIL, so the MCH tables could be located inside the G2F; the rest
  came from probing RECOIL with modified, recompressed files. The full layout is in
  the module doc. Highlights: width and font count in the header, screen codes, the
  fonts, one font per row, 256-byte colour tables, 512-byte per-object (X, size)
  tables for P0, M0, P1, M1, ... whose size-byte flags give each scanline's PRIOR
  (player 0's flags pick 4/2/1/8/0; player 1's give the fifth player and multicolour
  bits), player memory with missile graphics in the top bits of its second half, and
  at fixed offsets after that: options (split inverse, ANTIC 4 inverse colour), 30
  row modes (ANTIC 2, ANTIC 4, GTIA 9/10/11 picked by header byte 1, blank), a VBXE
  flag and the bottom-half inverse map. RECOIL reads nothing else in the 140-300 KB
  that follow. All 13 corpus samples without VBXE attributes match, and 100 random
  synthetic G2F files render identically.
- **SHC** (SAMAR Hires Interlace), `atari8/interlace.rs`. Exactly 17920 bytes: two
  7680-byte Graphics 8 frames, then for each frame 1280 bytes with 6 colours per
  scanline (192 × 6, the last 128 unused). Each colour is COLPF2 (the background) for
  a fixed span of the scanline, i.e. mid-line register writes: frame 1 changes at
  pixels 94, 166, 214, 262, 306, frame 2 at 46, 142, 190, 238, 286. Set pixels show the
  background hue at luminance 0. Frames are averaged. Probed with hand-made files; all
  8 corpus samples and 15 random files match `recoil2png`.
- **Not done**: G2F with VBXE colour attributes (flag at end+146753 = 1: athena,
  sergeantseymour-robotcop, Blinkys; 12-byte records per character column and row
  from end+146754), VSC (a text list of G2F file names, which the companion API, keyed
  by extension, can't fetch).

## 9.1 Wave 4: interlace and multi-frame bitmaps

IGE, ILD, ING, HR, MGA, BGP, CCI, RGB, RIP, RM2 and RM4. Every layout was found by
reading the corpus samples and probing `recoil2png` with hand-made files (one register
or one byte at a time, size scans, bit flips). The layouts are also in each module's doc
comment (`interlace2.rs`, `colorview.rs`, `rip.rs`, `rambrandt.rs`). All corpus samples
match `recoil2png`; no divergences recorded. Just Solve was unreachable while this was
written, so none of its pages were read.

- **IGE** (`interlace2.rs`): exactly 6160 bytes. The binary-load header
  `FF FF F6 A3 FF BB` and `FF 5F` are checked, then 4 colour registers per frame (frame
  1, frame 2, indexed by pixel value), then two 128x96 two-bit frames (32 bytes per
  line), drawn 2 wide and averaged.
- **ILD**: exactly 8195 bytes, no header. Two 128x128 two-bit frames in greys 0, 6, 2, 10
  (by pixel value), 2 wide, averaged. The last 3 bytes are not read.
- **ING**: two 160x200 frames, then 4 shared colour registers. Anything after is ignored
  (the sample has a screen-code text there).
- **HR** (Atari): exactly 16384 bytes, two 1-bit frames of 256 lines x 32 bytes, 239 lines
  shown, averaged into black, grey and white. No corpus sample: the `.hr` files in the
  corpus are TRS-80. The size and layout come from probing alone.
- **MGA**: exactly 7856 bytes; 80x96 APAC with alternating luminance and hue lines (the
  reverse of APA), 176 unread bytes.
- **BGP**: `BUGBITER_APAC239I_PICTURE_V1.0`, `FF 50 EF`, 4 unread bytes, a 16-bit title
  length and the title, the plane size 9560 (`58 25`), then 239 luminance lines and 239
  hue lines, drawn like interlaced APAC. `Scanlines`, `apac_80x96`, `deinterleave` and
  `nibble` in `apac.rs` are now `pub(super)`.
- **CCI** (packed CIN): `CIN 1.2 ` and four chunks. Each has a 16-bit length (counting
  the next field), a 16-bit field that is never read, and run-length tokens: below 0x80 a
  literal block of n + 1 bytes, from 0x80 one byte repeated n + 1 times. They unpack to
  the 16384-byte CIN: the Graphics 15 columns of even lines (3840 bytes), of odd lines
  (3840), the hue plane by column (7680) and the per-line colour tables (1024). Found by
  unpacking SUNV2.CCI and matching it to THESUNV2.CIN. `recoil2png` only accepts the
  1024-byte table variant and ignores data after the fourth chunk.
- **RGB** (ColorViewSquash, `colorview.rs`): `RGB1`, title length and title, mode 9 or 15,
  width in 4-pixel units (even, 2-80), height (1-192), the byte 1. The picture is a
  column-major list of pixels, each three 4-bit values (one per frame), stored as a
  nibble stream: tokens 1-7 repeat a triple 2-8 times, `0 N` repeats it N + 8 times, 9-15
  hold 1-7 literal triples, `8 N` holds N + 7. The frames use the hues 3, 12 and 7. Mode 9
  takes the value as luminance, mode 15 reads two 2-bit pixels per value (colours 0, 4, 10
  and 14 of the hue, value 0 black). The screen colour is the average of the three frames.
  Missing data is an error.
- **RIP** (`rip.rs`): header `RIP`, 4 version bytes, a mode byte, 16-bit big-endian fields
  (0/1, header length that is not read, width in units, height, title length), `T:`, the
  title, a tab, `CM:` and 9 colour registers. Modes: `0e` one Graphics 15 frame, `1e` two
  averaged, `10` two frames whose register sets swap on every line, `20` HIP with the
  frames swapped (mode 10 first), `30` mode 10 with per-line-pair colour tables (8 bytes
  per two lines after the frames) plus a mode 9 frame. The data is packed when it starts
  with `PCK`. The packer is LZ77 with Huffman codes: 13 unread bytes, three canonical
  Huffman tables of 4-bit lengths (64 symbols for match lengths, 256 for distances, 256
  for literals), then tokens of a flag bit followed by a literal, or by a distance symbol
  + 2 and a length symbol + 2. Found by probing with uniform tables, where the bit layout
  shows directly, and confirmed by decoding GOSTBUST byte for byte. A truncated stream
  leaves the rest blank, like RECOIL. `hip.rs` gained a `width` argument on
  `half_pixel_pair` and a `pub(super)` `nibble`. Modes `0f` (Graphics 8) and the other
  mode bytes `recoil2png` accepts are not decoded; no sample uses them.
- **RM2 / RM4** (`rambrandt.rs`): RM2 is exactly 8192 bytes: a Graphics 10 screen, the 9
  registers, 119 unread bytes and three 128-byte change tables. RM4 is a Koala file
  (Graphics 15) plus the 9 registers 464 bytes before the end and the same tables as the
  last 384 bytes. The tables hold line codes, register numbers and colours per pair of
  lines; the module doc explains how line codes map to lines. `koala.rs` gained a `parse`
  function and a `Pic` struct so RM4 can reuse the unpacker. RM0, RM1 and RM3 are not
  done (no samples). The Koala PIC decoder is still not a content-detection format.
