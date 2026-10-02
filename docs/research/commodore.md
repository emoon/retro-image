# Commodore image formats: documentation survey (clean-room)

Scope: the Commodore 64 / VIC-20 / C16-116-Plus4 / C128 formats on RECOIL's format list. Every source below is a prose spec, a manual, a wiki page or a hardware document. **No GPL/LGPL decoder source was opened.** When a permissively licensed implementation exists, its license is stated.

## 1. Summary

| Platform | Formats | Spec | Partial | Hardware-only | None |
|---|---|---|---|---|---|
| Commodore 64 | 108 | 54 | 24 | 4 | 26 |
| VIC-20 | 4 | 1 | 0 | 0 | 3 |
| C16/116/Plus4 | 3 | 1 | 1 | 0 | 1 |
| C128 | 3 | 3 | 0 | 0 | 0 |
| **Total** | **118** | **59** | **25** | **4** | **30** |

Most "None" entries are obscure scene editors such as the Super Hires family, X-FLI, Flimatic, CFLI and Boogie Down Paint. Decoding them would mean reverse-engineering CSDb sample files against the known VIC-II modes.

### Best umbrella sources (read these first)

1. **Codebase64 "C64 Graphics File Format Specs"** (based on groepaz's grafix specs list v0.03, extended by later editors). It gives memory maps (load address and the offset of every bitmap, screen RAM, colour RAM, `$D021` table and sprite block) for about 40 formats: hires, multicolour, interlace, AFLI, FLI, IFLI, UFLI, SHFLI, SHIFLI, UIFLI, EAFLI and FLI-Profi. It also describes several RLE packers (Doodle/Koala `$FE`, Amica `$C2`, Zoomatic `$03`, Drazlace, Hires Manager backwards RLE, FLI Graph backwards RLE).
   Live: [codebase.c64.org](http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03). Archive: [web.archive.org snapshot 2023-12-03](https://web.archive.org/web/20231203175456/https://codebase64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03).
   **Warning:** the old `codebase64.org` domain now 301-redirects to an unrelated, suspicious domain. Use `codebase.c64.org` or the archive.
2. **GoDot loader/saver documentation** by Arndt Dettke, in German. Each loader page has a "Dateiformat" box giving the load address, the size and order of chunks, the background-colour position and the packing scheme. **GoDot is MIT-licensed**, with 6502 source at [github.com/godot64/GoDot](https://github.com/godot64/GoDot), so it is a usable permissive reference. Index pages: [format table](https://www.godot64.de/german/formats.htm), [all loaders/savers](https://www.godot64.de/german/lstab.htm), [GoDot's own file formats](https://godot64.de/german/4bitformate.htm). It covers C64 formats plus Plus/4 Botticelli, VIC-20 MiniPaint and C128 BASIC 8, IPaint and VBM.
3. **Peter Schepers, "Standard C64 BITMAP files" (BITMAP.TXT)**: a table of load address, length and bitmap/screen/colour/background offsets for 10 classic formats. [ist.uwaterloo.ca/~schepers/formats/BITMAP.TXT](http://ist.uwaterloo.ca/~schepers/formats/BITMAP.TXT)
4. **Just Solve the File Format Problem**: [Commodore graphics formats](http://fileformats.archiveteam.org/wiki/Commodore_graphics_formats). Its per-format pages mostly give file sizes, magic bytes and CSDb links, with links to sample files.
5. **C64 OS "Image File Formats"** article by Greg Naçu: full specs for PETSCII BOT (`.pbot`), the C64 OS screenshot `.pet` v0/v1/v2 and Commodore Grafix (`.cgx`, RIFF/CGFX). [c64os.com/post/imageformats](https://c64os.com/post/imageformats)

### Explanations of how the modes are displayed (FLI bug, interlace, sprite underlays)

- [Description of C64 graphic modes (studiostyle.sk / Dmagic)](http://www.studiostyle.sk/dmagic/gallery/gfxmodes.htm): hires, multicolour, MCI (interlace with 1-pixel shift), FLI and IFLI.
- [C64-Wiki: Graphics Modes](https://www.c64-wiki.com/wiki/Graphics_Modes)
- [Codebase: UFLI](http://codebase.c64.org/doku.php?id=base:ufli): UFLI, UIFLI, MUFLI, MUIFLI and NUFLI lineage. Underlay of six X-expanded sprites plus one sprite over the FLI bug; FLI on every second line.
- [C64-Wiki: NUFLI](https://www.c64-wiki.com/wiki/NUFLI): layout of the sprite underlay, FLI-bug handling and "sprite crunch".
- [GoDot NuFLI loader page](https://www.godot64.de/german/l_nufli.htm) (German): a detailed explanation of NUFLI.
- [Pasi Ojala, "BFLI - New graphics modes 2"](http://www.antimon.org/dl/c64/code/bfli.txt): linecrunch and Big FLI, including display code that shows where the data sits in memory.
- [GoDot TruePaint loader](https://www.godot64.de/german/l_trupnt.htm): MCI interlace (two multicolour frames, one shifted by a pixel).
- [NUFLIX Studio manual](https://github.com/cobbpg/nuflix-studio/blob/main/manual/manual.md) (**MIT**): NUFLI(X) structure. It can load `.nuf` saved as the memory range `$2000-$79FF`.

## 2. Commodore 64

### Hardware references

- Christian Bauer, *The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64*: [cebix.net/VIC-Article.txt](https://www.cebix.net/VIC-Article.txt). This is the canonical reference for badlines, FLI, `$D011`/`$D016`/`$D018` and sprite timing.
- [Commodore 64 Programmer's Reference Guide (archive.org)](https://archive.org/details/Commodore_64_Programmers_Reference_Guide_1983_Commodore): bitmap and multicolour layout, plus the colour codes in Appendix D.
- [zimmers.net chipdata](https://www.zimmers.net/anonftp/pub/cbm/documents/chipdata/): 6567 preliminary datasheet, VIC-Article, 656x luminance tables.
- [C64-Wiki: VIC](https://www.c64-wiki.com/wiki/VIC), [C64-Wiki: Graphics Modes](https://www.c64-wiki.com/wiki/Graphics_Modes)

### Palette

- **Pepto (Philip Timmermann), "Calculating the color palette of the VIC II"**: [pepto.de/projects/colorvic](https://www.pepto.de/projects/colorvic/). It covers both the 2001 "Pepto" palette ([2001 page](https://www.pepto.de/projects/colorvic/2001/)) and the newer **Colodore** model. **No license is stated on the page.** The values are measured and calculated data, and they are widely reused. Justsolve reproduces the RGB table: [Commodore 64 color palette](http://fileformats.archiveteam.org/wiki/Commodore_64_color_palette).
- **Colodore**: [colodore.com](https://www.colodore.com/) is an interactive palette generator for VIC, VIC-II and TED. **No license is stated.** The page derives RGB from a luma/chroma model (16 hue angles × 32 luma levels), so we can reimplement the published model instead of copying tables.
- GoDot's colour analysis: [godot64.de/german/epalet.htm](http://www.godot64.de/german/epalet.htm).
- VICE ships `.vpl` palettes, but VICE is GPL, so do not use its files. The view64 manual documents the `.vpl` format as 16 lines of R G B intensity in hex.

### Sample files

- [CSDb](https://csdb.dk/): every editor's release page has demo pictures. The CSDb release IDs linked in the table come from Justsolve.
- [zimmers.net /pub/cbm/c64/graphics/pictures/](https://www.zimmers.net/anonftp/pub/cbm/c64/graphics/pictures/), including the [FLI](http://www.zimmers.net/anonftp/pub/cbm/c64/graphics/pictures/FLI/) and [PrintFox](http://www.zimmers.net/anonftp/pub/cbm/c64/graphics/pictures/PrintFox/) folders.
- Dexvert sample corpus, one folder per format: for example [sembiance.com/fileFormatSamples/image/koalaPaint/](https://sembiance.com/fileFormatSamples/image/koalaPaint/). The Justsolve pages link the matching folder names (`image/afl`, `image/gunpaint` and so on).
- [Tom's retro gallery (multicolour)](https://tomseditor.com/gallery/?platform=commodore&format=multicolor), [cbmfiles GEnie HiRes listing](http://cbmfiles.com/genie/HiResGraphicsListing.php), [GoDot 4Bit samples](http://www.godot64.de/download/4bits/), [C64Gfx.lha](http://www.zimmers.net/anonftp/pub/cbm/crossplatform/graphics/Amiga/C64Gfx.lha), which contains example BFLI/FFLI files plus `man/bfli.doc` and `man/ffli.doc`.

Abbreviations: CB = Codebase64 grafix spec (link above), GD = GoDot loader page, JS = Justsolve page, BT = Schepers BITMAP.TXT. Addresses are C64 memory addresses. The file offset is the address minus the load address, plus 2 for the load-address header.

### Commodore 64 format table

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| 4BT | GoDot 4Bit | **Spec** | [GoDot formats](https://godot64.de/german/4bitformate.htm), [GoDot 4Bit](https://www.godot64.de/german/4bit.htm), [JS](http://fileformats.archiveteam.org/wiki/GoDot) | No load address. Magic `GOD0`, then 32000 bytes of 4-bit pixels (320×200, one nibble per pixel, stored in 8×8 tiles), RLE `$AD count byte` (count 0 = 256). Minimum 380 bytes. MIT reference source. |
| 64C | 8×8 font | **Partial** | [agon_64cfontloader](https://github.com/eightbitswide/agon_64cfontloader), [C64-Wiki Character set](https://www.c64-wiki.com/wiki/Character_set) | 2-byte load address, then 8 bytes per character (up to 256 characters = 2048 bytes). Render as a glyph sheet. |
| A | SEUCK sprites | **Partial** | [JS SEUCK](http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit), [C64-Wiki SEUCK](https://www.c64-wiki.com/wiki/S.E.U.C.K.) | Exactly 8130 bytes, which matches 2 + 127×64 (127 sprites in 64-byte slots). Multicolour sprites. The colours are not stored in the file, so a fixed default palette is needed. |
| A64, WIG | Wigmore Artist 64 | **Spec** | CB, [GD Artist64](https://www.godot64.de/german/l_artist.htm), BT, [JS](http://fileformats.archiveteam.org/wiki/Wigmore_Artist_64) | Load `$4000`, 10242 bytes. Bitmap `$4000`, screen `$6000`, colour `$6400`, background `$67FF`. Same as Blazing Paddles apart from the load address and the background position. |
| AAS, ART | Art Studio (hires) | **Spec** | CB, BT, [GD OCP](https://www.godot64.de/german/l_ocp.htm), [JS](http://fileformats.archiveteam.org/wiki/Art_Studio) | Load `$2000`, 9002/9003/9009 bytes. Bitmap `$2000`, screen `$3F40`, border at `$4328`. |
| AFL | AFLI-editor | **Spec** | CB, [GD AFLI](https://www.godot64.de/german/l_afli.htm), [JS](http://fileformats.archiveteam.org/wiki/AFLI-Editor) | v2.0: load `$4000`, 8 screen RAMs `$4000-$5FE7`, bitmap `$6000`. Hires FLI with a 3-column FLI bug. |
| AMI | Amica Paint | **Spec** | CB, [GD Amica](https://www.godot64.de/german/l_amica.htm), [JS](http://fileformats.archiveteam.org/wiki/Amica_Paint) | Packed Koala-like data with RLE `$C2,count,byte`; `$C2,$00` ends the file. Unpacks to 10257 bytes (Koala layout plus a 256-byte colour-cycle table). CB gives load `$6000`, GD gives `$4000`. |
| BDP | Boogie Down Paint | **None** | – | No information found. |
| BFL, BFLI | Big FLI | **Partial** | [JS BFLI](http://fileformats.archiveteam.org/wiki/BFLI), [Ojala bfli.txt](http://www.antimon.org/dl/c64/code/bfli.txt), C64Gfx.lha `man/bfli.doc` | Exactly 33795 bytes, starts `FF 3B 62`. 2-screen-tall FLI shown with linecrunch. The display code in bfli.txt shows where the data lives in memory. |
| BML, FLI, FLG | FLI Graph (Blackmail) | **Spec** | CB, [GD FLI Graph](https://www.godot64.de/german/l_funetfli.htm), [JS](http://fileformats.archiveteam.org/wiki/FLI_Graph) | Unpacked: load `$3B00`, 17474 bytes. Per-line `$D021` table `$3B00` (200 used of 256), colour `$3C00`, 8 screens `$4000`, bitmap `$6000`. Packed: load `$38F0`, header (escape, end addresses), RLE applied backwards. `.fli` variant is 17409 bytes. |
| BS | Printfox screen | **Spec** | [GD PFoxSelect](https://www.godot64.de/german/l_pfoxs.htm), [JS Printfox](http://fileformats.archiveteam.org/wiki/Printfox_bitmap) | No load address. First byte `B` (`$42`), then 8000 bytes of 320×200 hires, RLE `$9B count(word) byte`. Black on white (inverted compared with a C64 bitmap). |
| CDU | CDU-Paint | **Spec** | CB, BT, [GD CDU](https://www.godot64.de/german/l_cdupnt.htm), [JS](http://fileformats.archiveteam.org/wiki/CDU-Paint) | Load `$7EEF`, 10277 bytes. 273-byte viewer, then the Koala layout: bitmap `$8000`, screen `$9F40`, colour `$A328`, background `$A710`. |
| CFLI | CFLI Designer | **None** | – | No information found. |
| CGX | Commodore Grafix (C64 OS) | **Spec** | [C64 OS Image File Formats](https://c64os.com/post/imageformats) | RIFF container with media ID `CGFX`, chunks FRMT (12 bytes: matrix, animation, data info), META (128 bytes), DATA. Supports multiple frames and several video modes. |
| CHE | Cheese | **Partial** | [JS Cheese](http://fileformats.archiveteam.org/wiki/Cheese) | Exactly 20482 bytes; nothing else known. 20480 = 2 × 10240, which suggests two 10K multicolour blocks. Must be verified against samples. |
| CLE | Centauri Logo-Editor | **Partial** | [JS](http://fileformats.archiveteam.org/wiki/Centauri_Logo_Editor), [CSDb](https://csdb.dk/release/?id=87045) | Only sample known is 8194 bytes. Layout unknown; probably a charset/logo. |
| CLP | GoDot 4Bit clip | **Spec** | [GoDot formats](https://godot64.de/german/4bitformate.htm) | Magic `GOD1`, row/column/width/height in tiles, then packed 4-bit tiles, RLE `$AD`. |
| CTM | CharPad | **Spec** (v4) | [CTM V4 format doc](https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt), [CharPad](https://subchristsoftware.itch.io/charpad-c64-free) | Magic `CTM` plus version byte. Contains charset, tiles, map and colours. v5-v9 exist but were not checked, and no spec for them was found. CharPad ships a format doc in its `Docs` folder; the repo is NOASSERTION, so use it for the docs only. |
| CWG | Create with Garfield | **Partial** | CB (Koala section) | A Koala variant: load `$8000` with 4 extra bytes at the end. |
| DD, DDP | Doodle | **Spec** | CB, BT, [GD Doodle](https://www.godot64.de/german/l_doodle.htm), [JS](http://fileformats.archiveteam.org/wiki/Doodle!_(C64)), [C64-Wiki](https://www.c64-wiki.com/wiki/Doodle) | Load `$5C00`, 9218 bytes. Screen `$5C00`, bitmap `$6000` (CB lists `$7000`; verify with BT, whose bitmap offset is 1024). Also loads at `$1C00`. C= Hacking #11 covers the format. |
| DOL, BED | Dolphin Ed | **None** | [JS list](http://fileformats.archiveteam.org/wiki/Commodore_graphics_formats) | Listed only. |
| DRL, DLP | Drazlace | **Spec** | CB, [GD Draz](https://www.godot64.de/german/l_draz.htm), [JS](http://fileformats.archiveteam.org/wiki/Drazlace) | Load `$5800`. Colour `$5800`, screen `$5C00`, bitmap 1 `$6000`, background `$7F40`, `$D016` flag `$7F42`, bitmap 2 `$8000`. Packed files start with `DRAZLACE! 1.0` plus an escape byte; RLE is ESC,count,byte. Interlace: two bitmaps share one screen and colour RAM. |
| DRZ, DRP | Drazpaint | **Spec** | CB, [GD Draz](https://www.godot64.de/german/l_draz.htm), [JS](http://fileformats.archiveteam.org/wiki/Drazpaint) | Load `$5800`. Colour `$5800`, screen `$5C00`, bitmap `$6000`, background `$7F40`. v1.3 ends at `$7FFE`, v2.0 at `$7F40`. Packed variant has a `DRAZPAINT` header and RLE. |
| ECI | ECI Graphic Editor | **Spec** | CB, [JS](http://fileformats.archiveteam.org/wiki/ECI_Graphic_Editor) | Load `$4000-$BFFF`. Bitmap 1 `$4000`, screens 1 `$6000`, bitmap 2 `$8000`, screens 2 `$A000`. Interlaced AFLI. |
| ECP | ECI Graphic Editor (packed) | **Partial** | [JS](http://fileformats.archiveteam.org/wiki/ECI_Graphic_Editor) | Packed ECI; packing scheme undocumented. |
| EMC | EMC-editor (Magic Disk) | **Spec** | [GD MagicDiskEMC](https://www.godot64.de/german/l_mdisk.htm) | Load `$4000`. 8×1024 video RAMs, 8192 bitmap, 1024 colour; background always black. Multicolour FLI, unpacked. Same layout as M.C.S. apart from the pack header. |
| ESH | Extend Super Hires Interlace Editor | **None** | – | No information found. |
| FBI | Flip (FLI Painter) | **Spec** | [GD Flip](https://www.godot64.de/german/l_flipr.htm) | Unpacked: load `$3C00` (68 blocks). Colour 1024, 8×1024 video RAMs, 8192 bitmap; background black. Packed: load `$38F0` with a 16-byte header (indicator, end addresses) and backwards RLE. |
| FCP, FPT | Face Painter | **Partial** | [JS](http://fileformats.archiveteam.org/wiki/Face_Painter) | Exactly 10004 bytes, so Koala-sized with 1 extra byte. Offsets not documented. |
| FD2 | FLI Designer 1.1/2.0 | **Spec** | CB, [GD FLI-Designer](https://www.godot64.de/german/l_flidesgn.htm) | Load `$3C00`. Colour `$3C00`, 8 screens `$4000`, bitmap `$6000`; background black. |
| FED | FLI Editor | **None** | – | Nothing beyond the name. |
| FFL, FFLI | Flash FLI | **Partial** | [JS FFLI](http://fileformats.archiveteam.org/wiki/FFLI), C64Gfx.lha `man/ffli.doc` | Exactly 26115 bytes, starts `FF 3A 66`. Flickering FLI (two frames blended). ffli.doc not yet read. |
| FLF | Turbo Rascal Syntax Error | **None** | [JS](http://fileformats.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error) | TRSE's own image format. The only description is TRSE's GPL source (avoid). |
| FLI | FLI (FLI Designer 2 / generic) | **Spec** | CB ("FLI Designer 2") | Load `$3FF0`, 17409 bytes. Colour `$3FF0`, 8 screens `$43F0`, bitmap `$63F0`. |
| FLM | Flimatic | **None** | – | No information found. |
| FP | Fuckpaint | **None** | – | No information found. |
| FUN, FP2 | Funpaint II | **Spec** | CB, [GD IFLI](https://www.godot64.de/german/l_ifli.htm), [JS](http://fileformats.archiveteam.org/wiki/Funpaint) | Load `$3FF0`, header `FUNPAINT (MT) `, packed flag `$3FFE`, escape `$3FFF`. Screens 1 `$4000`, bitmap 1 `$6000`, 100×`$D021` `$7F48`, colour `$8000`, screens 2 `$83E8`, bitmap 2 `$A3E8`, 100×`$D021` `$C328`. RLE is ESC,count,byte. CB warns that the addresses may be inexact. IFLI. |
| FPR | FLI Profi | **Spec** | CB, [JS](http://fileformats.archiveteam.org/wiki/FLI_Profi) | Load `$3780`. Y-expanded sprite data `$3780`/`$37C0`/`$38C0`/`$3900` (sprites cover the FLI bug), colour `$3C00`, screens `$4000`, bitmap `$6000`. |
| G | SEUCK font | **Hardware-only** | [JS SEUCK](http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit) | Font file: multicolour charset (SEUCK chars live at `$F800`). Exact size not documented. |
| GB | Printfox large picture | **Spec** | [GD PFoxSelect](https://www.godot64.de/german/l_pfoxs.htm), [JS](http://fileformats.archiveteam.org/wiki/Printfox_bitmap) | First byte `G`, 640×400 = 32000 bytes, RLE `$9B count(word) byte`. |
| GCD, MON | Giga-CAD / Mono Magic (320×200) | **Hardware-only** | [GD formats](https://www.godot64.de/german/formats.htm), [JS Giga Cad](http://fileformats.archiveteam.org/wiki/Giga_Cad), [JS Mono Magic](http://fileformats.archiveteam.org/wiki/Mono_Magic) | GoDot loads Giga-CAD with the plain HiresBitmap loader: a raw hires bitmap with no colour. |
| GG | Koala (compressed) | **Spec** | CB, [GD Koala](https://www.godot64.de/german/l_koala.htm) | Load address, then RLE with escape `$FE`, value, count. Decompresses to the Koala layout. |
| GR, CS | Star Painter | **Spec** | [GD StarPntr](https://www.godot64.de/german/l_starp.htm), [JS](http://fileformats.archiveteam.org/wiki/Star_Painter) | No load address. 2-byte header (width, height in 8×8 cells; `.gr` max 80×89, `.cs` 32×21), then hires bitmap stored cell by cell. |
| GUN, IFL | Gunpaint | **Spec** | CB, [GD IFLI](https://www.godot64.de/german/l_ifli.htm), [JS](http://fileformats.archiveteam.org/wiki/Gunpaint) | Load `$4000`, 33603 bytes. Screens 1 `$4000` (header text `GUNPAINT (JZ)` at `$43E8`), bitmap 1 `$6000`, 177×`$D021` `$7F4F`, colour `$8000`, screens 2 `$8400`, 20×`$D021` `$87E8`, bitmap 2 `$A400`. |
| HBM, HIR, HPI, FGS | Hires-Bitmap | **Spec** | [JS](http://fileformats.archiveteam.org/wiki/Hires-Bitmap), [GD HiresBitmap](https://www.godot64.de/german/l_hibmap.htm) | 8002 bytes: load address plus 8000-byte hires bitmap, no colour (monochrome). |
| HCB | HCB-editor | **None** | – | No information found. |
| HED | Hi-Eddi | **Spec** | CB, [GD Hi-Eddi](https://www.godot64.de/german/l_hieddi.htm), [JS](http://fileformats.archiveteam.org/wiki/Hi-Eddi) | Load `$2000`, 9218 bytes. Bitmap `$2000`, screen `$4000`. |
| HET | Hires-Editor (Topaz Beerline) | **None** | CB ("undocumented") | Listed as undocumented. |
| HFC, HFD | Hires FLI Designer | **Spec** | CB, [JS](http://fileformats.archiveteam.org/wiki/Hires_FLI_Designer) | Load `$4000`. Bitmap `$4000`, 8 screens `$6000-$7FE7`. AFLI. |
| HIM | Hires Manager | **Spec** | CB, [GD HiManRaw](https://www.godot64.de/german/l_hmanr.htm), [JS](http://fileformats.archiveteam.org/wiki/Hires_Manager) | Unpacked: load `$4000`, bitmap `$4000` (`$4001`=`$FF` marks unpacked), screens `$6000`. Packed: backwards RLE (escape `$00`) with a buggy literal mode, described in CB. 192 lines tall. |
| HLE | Hireslace Editor (Hires-Lace v1.5) | **Spec** | CB ("Hires-Lace v1.5 by Oswald", ext `.hil`) | Load `$4000`. Bitmap 1 `$4000`, screen 1 `$6000`, screen 2 `$8000`, bitmap 2 `$A000`. Hires interlace. |
| HLF, HIE | Hires Interlace (Feniks) | **Spec** | CB, [JS](http://fileformats.archiveteam.org/wiki/Hires_Interlace) | Bitmap 1, screen 1 `$4400`, screen 2 `$4800`, bitmap 2 `$6000`. CB's load address (`$4000`) and its bitmap-1 address (`$2000`) are inconsistent; check against samples. |
| HPC | Hi-Pic Creator | **Partial** | [JS](http://fileformats.archiveteam.org/wiki/Hi-Pic_Creator), CB (undocumented) | Load `$6000`, 9003 bytes (= 2 + 8000 + 1000 + 1, a hires layout). |
| IHE | Interlace Hires Editor | **None** | [JS](http://fileformats.archiveteam.org/wiki/Interlace_Hires_Editor) | Name and release link only. |
| ILE | Interlaced Logo Editor | **None** | – | No information found. |
| IPH, GIG, HPI, HRE | Interpaint hires | **Spec** | CB, [JS](http://fileformats.archiveteam.org/wiki/Interpaint), [GD formats](https://www.godot64.de/german/formats.htm) | Load `$4000`, 9002 bytes. Bitmap `$4000`, screen `$5F40`. |
| IPT, LRE | Interpaint lores | **Spec** | CB (Koala note), [GD formats](https://www.godot64.de/german/formats.htm) | Koala layout at load `$4000`. |
| ISH | Image System (hires) | **Spec** | CB, BT, [GD ImageSystem](https://www.godot64.de/german/l_imgsys.htm) | Load `$4000`, 9194 bytes. Bitmap `$4000`, screen `$6000`. |
| ISH | Interlace-Super-Hires Painter | **None** | – | Same extension as Image System hires; tell them apart by size. No information found. |
| ISM | Image System (multi) | **Spec** | CB, BT, [GD ImageSystem](https://www.godot64.de/german/l_imgsys.htm) | Load `$3C00`, 10218 bytes. Colour `$3C00`, bitmap `$4000`, background `$5FFF`, screen `$6000`. |
| JJ | Doodle (compressed) | **Spec** | CB, [GD Doodle](https://www.godot64.de/german/l_doodle.htm) | RLE: escape `$FE`, value, count. Decompresses to the Doodle layout. |
| GIG, KLA, KOA | Koala Painter | **Spec** | CB, BT, [GD Koala](https://www.godot64.de/german/l_koala.htm), [JS](http://justsolve.archiveteam.org/wiki/KoalaPainter), [C64 OS](https://c64os.com/post/imageformats) | Load `$6000`, 10003 bytes (sometimes padded). Bitmap 8000, screen 1000, colour 1000, background 1. The same layout appears at `$4400` (Amica) and `$4000` (Interpaint, Gigapaint). |
| LP3 | Logo Painter 3/3+ | **None** | [JS](http://fileformats.archiveteam.org/wiki/Logo-Painter) | Name and release links only. |
| MCI | True Paint | **Spec** (unpacked) | CB, [JS True Paint I](http://fileformats.archiveteam.org/wiki/True_Paint_I), [GD TruePaint](https://www.godot64.de/german/l_trupnt.htm) | Load `$9C00`, 19434 bytes. Screen 1 `$9C00`, background `$9FE8`, bitmap 1 `$A000`, bitmap 2 `$C000`, screen 2 `$E000`, colour `$E400`. The packed variant (self-running, 9 pack flags, packed backwards) is undocumented. MCI interlace. |
| MIL | Micro Illustrator | **Spec** | [GD MIllustr8or](https://www.godot64.de/german/l_millu.htm), [JS](http://fileformats.archiveteam.org/wiki/Micro_Illustrator) | Load `$18DC`, 10022 bytes. 20-byte header: magic `FF 80 69 67`, header length, compression 0/1/2, background, chunk lengths, width 40, height 200. Then screen, colour and bitmap. GD describes the RLE variants. |
| MLE | Multi-Lace Editor | **None** | [JS](http://fileformats.archiveteam.org/wiki/Multi-Lace_Editor) | Release link only. |
| MUF | MUFLI Editor | **Partial** | [Codebase UFLI](http://codebase.c64.org/doku.php?id=base:ufli), [CSDb MUFLI Editor](https://csdb.dk/release/?id=35737) | Mode described (multicolour FLI plus sprite underlay with colour splits). No file layout found. |
| MUI | MUIFLI Editor | **Partial** | [Codebase UFLI](http://codebase.c64.org/doku.php?id=base:ufli), [Forum64 Mufflon thread](https://www.forum64.de/index.php?thread/38462-mufflon-v1-0-released-crest-s-nufli-muifli-konverter/) | Mode described only. |
| MUP | MUFLI Editor (packed) | **Partial** | as MUF | Packing undocumented. |
| MWI, MWIN | Art Studio window | **None** | – | Art Studio v1.2b window (clip) files. No layout found. |
| NUF | NUFLI Editor | **Partial** | [C64-Wiki NUFLI](https://www.c64-wiki.com/wiki/NUFLI), [GD NuFLI](https://www.godot64.de/german/l_nufli.htm), [pynuvie FORMAT.md](https://github.com/anarkiwi/pynuvie/blob/main/docs/FORMAT.md) (**Apache-2.0**), [NUFLIX manual](https://github.com/cobbpg/nuflix-studio/blob/main/manual/manual.md) (**MIT**) | Unpacked: load `$2000`, data to `$7A00` (91 blocks), viewer at `$3000`. The full spec is in a Forum64 thread "Nufli File Specs?", linked from C64-Wiki (it returned 403 to us). pynuvie decodes the bitmap, FLI screens and six underlay sprites under a permissive license. |
| NUP | NUFLI Editor (packed) | **Partial** | as NUF | Packed form undocumented. |
| OCP, MPI, MPIC | Advanced Art Studio | **Spec** | CB, BT, [GD OCP](https://www.godot64.de/german/l_ocp.htm), [JS](http://fileformats.archiveteam.org/wiki/Advanced_Art_Studio) | Load `$2000`, 10018 bytes. Bitmap `$2000`, screen `$3F40`, border `$4328`, background `$4329`, colour `$4338`. |
| P64, FLY | Picasso 64 / Flying Colors | **Partial** | [JS Picasso 64](http://fileformats.archiveteam.org/wiki/Picasso_64), CB (`.fly` undocumented) | Load `$1800`, 10050 bytes (same size as Vidcom). Offsets not documented. |
| PBOT | PETSCII BOT | **Spec** | [C64 OS](https://c64os.com/post/imageformats) | 70 bytes (5×7) or 384 bytes (12×16). Colour codes, then screen codes, using the ROM uppercase charset. |
| PDR | PetDraw64 | **None** | – | No information found. |
| PET | C64 OS screenshot v2 | **Spec** | [C64 OS](https://c64os.com/post/imageformats) | `PET` (`$D0 $C5 $D4`) plus version `0`/`1`/`2`, three 17-byte strings, 1000 screen codes, 1000 colours, border, background; v2 adds a 2048-byte charset. |
| PET | PETSCII Editor | **Spec** | [GD PETSCII](https://www.godot64.de/german/l_petscii.htm), [Marq's PETSCII](https://www.kameli.net/marq/?page_id=2717) (**WTFPL** source) | Header: width, height, border, background, charset (1 byte each), then screen RAM and colour RAM (1000 bytes each for 40×25). |
| PG | Pagefox | **Spec** | [GD PFoxSelect](https://www.godot64.de/german/l_pfoxs.htm), [JS](http://fileformats.archiveteam.org/wiki/Printfox_bitmap) | First byte `P`, height/width in tiles, `K` plus layout contour data up to `$00`, then bitmap in tiles. RLE `$9B count(byte) byte`. Up to 640×800. |
| PI, BPL | Blazing Paddles | **Spec** | CB, BT, [GD BlPaddles](https://www.godot64.de/german/l_blpad.htm), [JS](http://fileformats.archiveteam.org/wiki/Blazing_Paddles), [manual](https://archive.org/details/BlazingPaddles/page/n27/mode/2up) | Load `$A000`, 10242 bytes. Bitmap `$A000`, background `$BF80`, screen `$C000`, colour `$C400`. |
| PMG | Paint Magic | **Spec** | CB, [GD PaintMagic](https://www.godot64.de/german/l_pmagic.htm) | Load `$3F8E`. Display code, bitmap `$4000`, background `$5F40`, single colour-RAM byte `$5F43`, border `$5F44`, screen `$6000`. |
| PP | Pixel Perfect | **Spec** | [GD IFLI](https://www.godot64.de/german/l_ifli.htm), [GD saver](https://www.godot64.de/german/s_pixperfect.htm) | Load `$3C00`. Colour 1024, then for each of the two frames 8×1024 screens plus a bitmap. Background byte at `$7F7F`. IFLI. |
| PPP | Pixel Perfect (packed) | **Spec** | [GD saver](https://www.godot64.de/german/s_pixperfect.htm) | Load `$3BFC`, 4-byte header (`$10 $10 $10` plus indicator), RLE indicator,count-1,byte. |
| RP | Rainbow Painter | **Partial** | [JS](http://fileformats.archiveteam.org/wiki/Rainbow_Painter) | Exactly 10242 bytes (Blazing Paddles/Artist 64 size). Offsets unverified. |
| RPH | Run Paint (hires) | **Partial** | [JS RUN Paint](http://fileformats.archiveteam.org/wiki/RUN_Paint), [RUN #63](https://archive.org/details/run-magazine-63) | Hires variant. The original magazine listing on archive.org could be used to derive the layout. |
| RPM | Run Paint (multi) | **Spec** | CB | Identical to Koala (load `$6000`). |
| RPO, GIH | Run Paint mono / Gigapaint hires | **Partial** | [JS Hires-Bitmap](http://fileformats.archiveteam.org/wiki/Hires-Bitmap), [GD formats](https://www.godot64.de/german/formats.htm) | Mono hires bitmap; GoDot reads Gigapaint with HiresBitmap. |
| SAR | Saracen Paint | **Spec** | CB, [GD SaracenPaint](https://www.godot64.de/german/l_saracenp.htm), [JS](http://fileformats.archiveteam.org/wiki/Saracen_Paint) | Load `$7800`. Screen `$7800`, background `$7BF0`, bitmap `$7C00`, colour `$9C00`. |
| SCR + COL | PETSCII Editor (raw) | **Hardware-only** | [PETSCII Editor](https://petscii.krissz.hu/) (exports raw streams) | Pair of raw 1000-byte screen-code and colour files. Needs the ROM charset. |
| SH1 | Super-hires Editor I | **None** | – | No information found. |
| SH2 | Super-hires Editor II | **None** | – | No information found. |
| SHE | Super Hires Editor | **None** | – | No information found. |
| SHE | Super Hires Editor 2 | **None** | – | No information found. |
| SHF | Super Hires FLI Editor | **Partial** | CB (SHFLI) | Unpacked: 8 video RAMs `$4000-$5FE7` with sprite pointers, bitmap `$6000`. How the sprite data is stored is unknown ("How are the sprites stored?"). Packed form unknown. |
| SHI | Super Hires Interlace Editor | **Partial** | CB (SHIFLI) | Runnable `$0801` file: picture 1 data at `$095C` is moved to `$4000`, picture 2 at `$475C` is moved to `$C000`. Sprite storage unknown. |
| SHP | Loadstar SHP | **Spec** | [GD Loadstar](https://www.godot64.de/german/l_loadstar.htm), [JS](http://fileformats.archiveteam.org/wiki/SHP_(Loadstar)) | Load `$4000`. New format: mode byte (`$80` hires, `$00` multi), pack indicator, background; then bitmap, video and colour chunks, each RLE'd with its own indicator. The old format (`$A8`/`$E8` mode, height in cells) is also documented. |
| SHS | Super Hires Studio | **None** | – | No information found. |
| SHX | SHF-XL Edit | **Partial** | CB (SHFLI-XL) | Load `$4000`. 8 video RAMs with sprite pointers, bitmap `$6000`. Sprite storage unknown. |
| SIF | Super Hires Interlace FLI Editor | **None** | – | No information found. |
| SPD | SpritePad | **Spec** | [CSDb forum: SPD format](https://csdb.dk/forums/?roomid=7&topicid=125812), [GD SpritePad](https://www.godot64.de/german/l_spritepad.htm), [spritemate](https://github.com/Esshahn/spritemate) (**MIT**) | Magic `SPD` plus version, sprite count-1, animation count-1, background, MC1, MC2; then 63 bytes of data plus 1 flag byte per sprite; then animation tables. Versions 1 and 2 (tiles). |
| UFL | UFLI-editor | **Partial** | CB (UFLI), [Codebase UFLI](http://codebase.c64.org/doku.php?id=base:ufli) | Memory map known (60 sprites `$4000`, sprite colour `$4FF0`, 4 screen RAMs, bitmap `$6000`), but the load address is unknown. |
| UIF | UIFLI-editor | **Partial** | CB (UIFLI) | Memory map for both frames known; load address unknown. |
| VHI | Vertical Hires Interlace Editor | **None** | – | No information found. |
| VIC | Generic C64 (up to 320×200) | **Hardware-only** | [JS list](http://fileformats.archiveteam.org/wiki/Commodore_graphics_formats) | Generic memory dumps. Identify them by load address and length. |
| VID | Vidcom 64 | **Spec** | CB, BT, [GD VidCom](https://www.godot64.de/german/l_vidcom.htm), [JS](http://fileformats.archiveteam.org/wiki/Vidcom_64) | Load `$5800`, 10050 bytes. Colour `$5800`, screen `$5C00`, background `$5FE8`, bitmap `$6000`. |
| XFL | X-FLI Editor | **None** | – | No information found. |
| ZOM | Zoomatic | **Spec** | CB | Load `$6000`, Koala layout. RLE where the escape `$03` comes *after* byte,length (length 0 = 256). |
| ZS | Star Painter font | **None** | [JS Star Painter](http://fileformats.archiveteam.org/wiki/Star_Painter) | Only the extension is listed. |

## 3. Commodore VIC-20

### Hardware references

- [zimmers chipdata](https://www.zimmers.net/anonftp/pub/cbm/documents/chipdata/): `VIC-I.txt` and the `6560.zip` datasheet.
- [DenialWIKI: MOS Technology VIC](http://sleepingelephant.com/denial/wiki/index.php?title=MOS_Technology_VIC), [C64-Wiki: VIC-20](https://www.c64-wiki.com/wiki/VIC-20).
- [GoDot MiniPaint loader](https://www.godot64.de/german/l_minipaint.htm): explains the VIC-20 mixed hires/multicolour cells and the pixel aspect ratio (5:3).

### Palette

- [Colodore](https://www.colodore.com/) models the VIC (6560/6561) too. No license is stated.
- [Wikipedia: VIC-20](https://en.wikipedia.org/wiki/VIC-20) lists the 16 colours as YPbPr values; derive RGB from those.
- [Lospec: Commodore VIC-20 palette](https://lospec.com/palette-list/commodore-vic-20) (provenance unclear).

### Samples

- [zimmers /pub/cbm/vic20/graphics/](https://www.zimmers.net/anonftp/pub/cbm/vic20/graphics/), [editors](https://www.zimmers.net/anonftp/pub/cbm/vic20/graphics/editors/index.html), CSDb.

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| BP | Best Paint | **None** | [Denial thread on VIC-20 picture formats](http://sleepingelephant.com/~sleeping/ipw-web/bulletin/bb/viewtopic.php?t=1649&start=45) | By Andreas Dietmair (1990), needs +16K RAM. File layout not documented. |
| FLF | Turbo Rascal Syntax Error | **None** | [JS](http://fileformats.archiveteam.org/wiki/Turbo_Rascal_Syntax_Error) | Only described by TRSE's GPL source (avoid). |
| MG | MINIPAINT (Minigrafik) | **Spec** | [GD MiniPaint](https://www.godot64.de/german/l_minipaint.htm) | Load `$10F1`. 15-byte header (offset 14 = ink `$900E`, offset 15 = paper/border `$900F`), 3840-byte bitmap 160×192 in vertical 8-pixel strips, 120-byte colour RAM, then a 120-byte display routine. |
| PIC0 + PIC1 | Picasso (VIC-20) | **None** | – | Two-file format. No documentation found. |

## 4. Commodore 16 / 116 / Plus/4

### Hardware references

- [Plus/4 Service Manual (pagetable.com)](https://www.pagetable.com/docs/ted/Service%20Manual%20Model%20Plus%204%20Computer.pdf)
- [C= Hacking #12: Talking to TED](http://mclauchlan.site.net.au/scott/C=Hacking/C-Hacking12/gfx.html)
- [Wikipedia: MOS Technology TED](https://en.wikipedia.org/wiki/MOS_Technology_TED): colour byte = bits 0-3 hue, bits 4-6 luminance, giving 121 colours. [The TED Chip (retroisle)](https://www.retroisle.com/general/tedchip.php).

### Palette

- [Colodore](https://www.colodore.com/) covers TED: 15 hues × 8 luminances plus black. No license is stated.

### Samples

- [Plus/4 World](https://plus4world.powweb.com/) (Botticelli and Multi Botticelli downloads), and the Dexvert `image/p4i` folder (linked from [JS P4I](http://fileformats.archiveteam.org/wiki/P4I)).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| P4I | Botticelli (hires) | **Partial** | [Plus/4 World Botticelli](http://plus4world.powweb.com/software/Botticelli), [GD Botticelli](https://www.godot64.de/german/l_botticelli.htm) | Hires `G.` files: presumably the same `$7800` luma/chroma/bitmap layout without the `MULT` tag. GoDot doesn't support them. |
| P4I | Multi Botticelli | **Spec** | [Plus/4 World Multi Botticelli](http://plus4world.powweb.com/software/Multi_Botticelli), [GD Botticelli](https://www.godot64.de/german/l_botticelli.htm) | Load `$7800`. Luminance `$7800` (1000 bytes, padded to 1024), colour `$7C00` (1024), bitmap `$8000` (8000). The last 6 bytes of the luma block (`$7BFA`) hold `MULT` plus the `$FF16` and `$FF15` colours, nibble-swapped. Prefix `m.`. |
| P4I | 128×64 | **None** | [JS P4I](http://fileformats.archiveteam.org/wiki/P4I) | Justsolve says "little is known". |

## 5. Commodore 128

### Hardware references

- [Wikipedia: MOS Technology 8563 (VDC)](https://en.wikipedia.org/wiki/MOS_Technology_8563), [c128lib Reference](https://c128lib.github.io/Reference/), [VDC tips (mirkosoft)](http://commodore128.mirkosoft.sk/vdc.html), [VDC 256×200 bitmap setup gist](https://gist.github.com/ytmytm/265d6ae1f5b1df7bd7ef0ba67bdaa816). Bitmap attributes are a 4-bit foreground and a 4-bit background per cell.
- [Wikipedia: BASIC 8](https://en.wikipedia.org/wiki/BASIC_8)

### Palette

- VDC is digital RGBI (CGA-like, 16 colours). No measured palette source found. Use the standard RGBI mapping.

### Samples

- [zimmers /pub/cbm/c128/graphics/](https://www.zimmers.net/anonftp/pub/cbm/c128/graphics/), [VBM samples (csbruce)](http://csbruce.com/cbm/ftp/images/vbm/), [VBM viewers](https://www.zimmers.net/anonftp/pub/cbm/c128/graphics/viewers/vbm/index.html).

| Extension(s) | Program/format | Docs quality | Sources | Notes |
|---|---|---|---|---|
| BM, VBM | VDC BitMap (ACE, Craig Bruce) | **Spec** | [Craig Bruce's comp.sys.cbm post](http://csbruce.com/cbm/postings/csc19950906-1.txt), [GD VBM](https://www.godot64.de/german/l_vbm.htm), [JS](http://fileformats.archiveteam.org/wiki/VBM_(VDC_BitMap)) | `B M $CB` plus version 2/3, width and height (big-endian). v3 adds an encoding byte, 5 RLE code bytes and a comment. Monochrome. Reference converter `pbmtovbm.c` (license not checked). |
| BRUS, PICT | BASIC 8 | **Spec** | [GD Basic8Mode1 saver](https://www.godot64.de/german/s_b8mode1.htm), [GD Basic8Select](https://www.godot64.de/german/l_b8select.htm) | Magic `brus`, type 4, start column/row, pack flag, colour mode 0-4, width (tiles), height (lines), 3 unused colours. Then a bitmap chunk and a `colr` chunk with per-line-pair fg/bg nibbles (interleaved for the interlace field). RLE: bit 7 = repeat, bits 0-6 = count. |
| IP | IPaint | **Spec** | [GD IPaint saver](https://www.godot64.de/german/s_ipaint.htm), [GD Basic8Select](https://www.godot64.de/german/l_b8select.htm) | Same `brus` container as BASIC 8. Default colour mode 2 (8×4 attributes per interlace field), 640×400 or 320×200. |

## 6. To avoid (GPL/LGPL code; do not read)

| Project | License | Notes |
|---|---|---|
| RECOIL | GPL | Project policy: never open. |
| [view64 / libview64](https://view64.sourceforge.net/) (soci) | **GPLv2** (SourceForge metadata) | Decodes 70+ C64 formats. The manual's format/extension list is factual and was seen; **do not open the source**. |
| [abydos](https://snisurset.net/code/abydos/) | **LGPL-2+** | Image library with many C64 loaders. Its site claims "specifications for many C64 graphic formats"; those spec pages were not opened, and their relation to the code is unclear. |
| [TRSE (Turbo Rascal Syntax Error)](https://github.com/leuat/TRSE/) | **GPL-3.0** | The only known description of `.flf`. |
| VICE | GPL | Emulator; its `.vpl` palette files ship with GPL code. Derive palettes from Pepto/Colodore instead. |
| Dexvert | not checked | Converter that wraps RECOIL. Use only its sample-file corpus. |

### Permissive or other implementations found (for reference)

- **GoDot**: MIT. [github.com/godot64/GoDot](https://github.com/godot64/GoDot), 6502 loaders for about 60 of the formats above.
- **Marq's PETSCII editor**: WTFPL ([kameli.net](https://www.kameli.net/marq/?page_id=2717)).
- **pynuvie**: Apache-2.0, NUFLI decoding ([github](https://github.com/anarkiwi/pynuvie)).
- **NUFLIX Studio**: MIT ([github](https://github.com/cobbpg/nuflix-studio)).
- **spritemate**: MIT, SPD ([github](https://github.com/Esshahn/spritemate)).
- Closed-source viewers, mentioned only: XnView, Konvertor, Tom's Editor.

## 7. Implementation notes (checked against samples and `recoil2png`)

- Palette: Pepto 2001 for the VIC-II; TED (128 entries), VIC-20 and VDC RGBI levels were read from `recoil2png` output of synthetic files.
- Output: multicolour pixels doubled to 320 wide; FLI drops the leftmost 24 pixels (296 wide); interlace frames are averaged per channel, the shifted frame moves one pixel right with black entering at the left.
- Doodle: bitmap at `$6000` (BT is right, CB's `$7000` is wrong); `.jj` may end right after the bitmap.
- Hires-Interlace (HLF): bitmap `$2000` pairs with screen `$4800`, bitmap `$6000` with screen `$4400`.
- Hires Manager: shown from the second character row, 192 lines. Packed: the stored unpacked end is exclusive; literal byte `n` is followed by `n-1` bytes.
- Gunpaint/Funpaint: background black; second frame shifted right. Pixel Perfect background from `$7F7F`.
- BFLI: bottom 200 lines continue the video matrix/colour RAM/bitmap counters at 1000/8000, wrapping at 1024/8192 (linecrunch).
- Loadstar old format: low 6 bits of the mode byte are the width in cells; the `$FF` byte sits between screen and colour chunks.
- CharPad v5: 20-byte header (16-bit tile count), 16-bit tile cells and map, no cell attributes; flags bit 2 = all characters multicolour.
- BASIC 8 `brus`: bitmap and colours packed separately with a literal `COLR` between them.
- Plus/4 Botticelli: set/`01` pixels = high colour nibble + low luminance nibble, clear/`10` = low colour nibble + high luminance nibble, `00` = `$FF15`, `11` = `$FF16`.
- VIC-20 MiniPaint: colour RAM packed two cells per byte, low nibble first, 20×12 cells of 8×16.
- C64 character ROM: 901225-01 (MD5 `12a4202f5331d45af846af6c58fba946`), taken from the ROM dump `chargen` in `1.4.0/Firmware.zip` of the archive.org item `bizhawk-firmware-files_20250516`. In this ROM the reversed `@` is not an exact inverse of `@` (row 5 is `$99`, not `$9D`). `recoil2png` draws the exact inverse, so `gary.pet` differs by one pixel (recorded divergence).
- PETSCII pictures (ROM upper case/graphics set unless noted, set pixels in the colour RAM colour):
  - PETSCII Editor `.pet`: 2026-byte dump of `$3000-$37E7`, with screen codes at `$3000`, background at `$33E9` and colour RAM at `$3400`.
  - `.scr` + `.col`: two 1002-byte files (load addresses ignored), black background. `recoil2png` rejects the `.scr` alone, and so do we.
  - PETSCII BOT `.pbot`: 70 or 384 bytes, black background.
  - C64 OS `.pet` versions 0/1 use the ROM upper/graphics and lower/upper sets. `recoil2png` rejects them; the C64 OS sample files (`thirtytwo.pet`, `colors.pet`, `dndroller.pet` on c64os.com) render correctly but aren't in the corpus.
- VIC-20 Picasso: `.pic0` (load `$0D00`) holds 22×11 characters of 8×16 pixels in screen order, followed by the VIC registers `$9000-$900F` (`$900E`/`$900F` give the colours). `.pic1` is colour RAM. `recoil2png` needs both files and checks `$9002`=`$96`, `$9003`=`$17` and `$9005`=`$8C`.
- VIC-20 Best Paint: load `$1100`, bitmap in vertical 8-pixel strips (like MiniPaint), 240 bytes of colour RAM, then `$900F`. Hires only; `recoil2png` rejects colour RAM with bit 3 set.
- Picasso 64: Vidcom's layout at `$1800` with the background at `$1FFF`.
- Cheese: load `$8000`, bitmap `$8000`, screen `$C200`, colour `$C800`, background `$CFFD` (probing `recoil2png` confirms it).
- Rainbow Painter: load `$5C00`, screen `$5C00`, bitmap `$6000`, colour `$8000`. No byte sets the background; `recoil2png` shows black.
- CFLI Designer: load `$4000`, eight FLI screen RAMs and no bitmap. The picture is hires FLI over a fixed `$AA` bitmap.
- Logo Painter 3: load `$1800`, 40×50 screen codes, character set at `$2000`, always multicolour. 4174-byte files (with the viewer appended) keep `$D021`, colour RAM, `$D022` and `$D023` at `$1FFB`. 4098-byte files have no colours, and `recoil2png` uses black, light red, red and white.
- Star Painter font (ZS): load `$F0B0`, then 9-byte records (a width byte and 8 rows), shown as a 128-character sheet.
- Super Hires Interlace (SHI): 15355 bytes, load `$7FFF` with a zero byte there; bitmaps (12×25 cells) at `$8000` and `$8960`, sprites of the two frames at `$92C0` and `$A6C0` (ten 21-line bands of 4 upper + 4 lower sprites, one per 24-pixel column), shared screen RAM `$BAC0`, upper/lower sprite colours `$BBEC`/`$BBF0`. Upper layer over lower layer over bitmap; 96×200. `recoil2png` decodes other sizes or a nonzero `$7FFF` some other way (no samples). The runnable `$0801` form described by CB was not seen.
- Super Hires Interlace FLI (SIF): two packed sections (one per frame), then the four sprite-layer colours. A section starts with `$9400` + its length and an escape byte; the data is packed backwards (`value count escape`, count 0 = 256) and unpacks to exactly 8176 bytes: four 2048-byte planes of 167 lines × 12 bytes (upper sprite layer, lower sprite layer, bitmap, a colour byte per cell per line); 96×167.
- Super Hires FLI (SHF) / SHF-XL (SHX), unpacked (15874 / 15362 bytes, load `$4000`): hires FLI (screens `$4000`, bitmap `$6000`) with the sprite block changing every line as if the pointers were read from the previous line's screen RAM; picture line `y` shows sprite row `y % 21`. The blocks follow the editors' fixed pointer layout; `recoil2png` ignores the pointer bytes in the file. SHF: 26 columns from column 14, bitmap lines 1-167, sprites 0-3 (colour `$43E8`) over 4-7 (`$43E9`); the 64-byte blocks live in unused bitmap and screen areas. SHX: 18 columns from column 11, lines 0-167, sprites 1-6 (colour `$43E9`).
- Packed SHF (any other size): 2 ignored bytes, escape, forward `ESC count value` RLE to the four planes of one SIF frame (at least `$1FEA` bytes); layer colours at `$1FE8`/`$1FE9`. The editor saves names as 12 characters padded with spaces plus `.SHF`. `recoil2png` paints both layers in the first colour; Crest's editor (run in VICE) uses the second for sprites 4-7 (recorded divergence).
- Packed SHX (any other size): 2 ignored bytes, data packed backwards (`value count ESC`) with the escape as the last byte, unpacking to 9168 bytes: three 3072-byte planes of 168 lines × 18 bytes (sprite layer, bitmap, colour per cell per line), sprite colour at `$BD1`. Extra data in front is ignored.
- NUFLI (`.nuf`, 23042 bytes, load `$2000`, layout from pynuvie, Apache-2.0): bitmap `$6000` (first 5120 bytes) + `$3400`; one screen RAM row per line pair (`$5C00`...`$4000` cycling for lines 0-127, `$2000-$2C00` below); six X-expanded underlay sprites at x 24-311 behind the bitmap's clear pixels, plus a hires and a multicolour sprite in the FLI-bug columns 0-23. Sprite data comes through the pointers in each line pair's screen RAM; `recoil2png` rejects the file when a needed fetch falls outside `$2000-$79FF`. Colour tables `$2400/$2480/$2800/$2880/$2C00/$2C80` (101 entries; entry 0 = initial colour, a nonzero high nibble keeps the previous colour; entry `k` colours lines `2k-1`,`2k`). High nibbles 7/5/6/E switch the FLI-bug colours (hires, multicolour `01`/`11`/`10`; initial values `$3FF7/$3FF1/$3FF0/$3FF6`) from line `2k`, or `2k-1` when in the first table. In the FLI-bug columns lines 2-7 of each character row show light grey ($F) for both bitmap colours. Timing details checked by probing `recoil2png` with random data.
- UFLI-editor (`.ufl`): 16194 bytes = unpacked `$4000-$7F3F`; any other size is packed like SHF (2 ignored bytes, escape, forward `ESC count value`) to at least `$3F40` bytes. Hires FLI with screen RAM `$5000 + $400 * (y/2 % 4)`, bitmap `$6000` (columns 3-38 shown, 288 wide), six X-expanded sprites behind the bitmap's clear pixels in colour `$4FF0`: line `y` shows row `(y+1)/2 % 21` of block `$4000 + $300*(y/40) + $180*(y/2 % 2) + $40*column` (the file's sprite pointers are not used).
- FLI Profi (`.fpr`, 18370 bytes, load `$3780`): multicolour FLI (colour `$3C00`, screens `$4000`, bitmap `$6000`, black background) at the full 320 width. The FLI-bug columns 0-23 are covered by a multicolour sprite read from two streams of 21-row blocks: lines `y % 4` = 0/3 from `$3780`, 1/2 from `$38C0`. Sprite pairs `01` = per-line colour `$3A00+y`, `10` = `$3BC8`, `11` = `$3BC9`; under it the bitmap shows `00` black, `01`/`10` light grey, `11` the high nibble of `$3B00+y`.
- Content detection (`.signature()`): packed Drazpaint/Drazlace, Funpaint II, GoDot 4Bit/clip, SpritePad v1, Commodore Grafix, CharPad, C64 OS `.pet`, VDC BitMap, BASIC 8 `brus`.
- Not implemented: packed ECI (ECP), Dolphin Ed (one sample, black background, so the background byte can't be located), Face Painter, Hires Editor (HET), Centauri Logo Editor, M.C.S. (the `.mcs` samples have no load address and may belong to another platform), Interlace Hires Editor, the remaining Super Hires editors (SH1, SH2, SHE, SHS, ESH and the Interlace-Super-Hires ISH; no samples in the corpus), packed NUFLI (NUP), UIFLI, MUFLI/MUIFLI, Multi-Lace Editor, Botticelli 128×64 and PetDraw64 (no samples). Petmate `.pet` (header with width, height, colours and charset) is documented on GoDot's PETSCII loader page, but `recoil2png` rejects it and there are no samples.

### Corrections found while reviewing hostile samples

- **BASIC 8 / IPaint (C128 VDC):** the alternate-line attribute layout (rows 0/2 and 1/3
  sharing attributes) applies only to interlaced pictures taller than 200 lines. In pictures of
  200 lines or fewer, each attribute row covers its own 8-line cell. GoDot's mode-1 page
  describes the alternate-line layout for 200-line pictures too, and `recoil2png` renders that
  way, but the samples show colour fringes at every shape edge unless each row covers its own
  cell (`biplane.pict`'s signature "b.kane 1986" only reads cleanly this way).
- **Drazpaint:** packed files can carry the header `DRAZPAINT 2.0` as well as `1.4`; the packing
  is the same.
- **Saracen Paint:** a 10018-byte variant ends at `$9F1F`, five rows of colour RAM short of the
  full file; the missing tail is zero.

## Wave 5: C64

Eight more formats, all reverse engineered by mutating bytes in copies of the samples and watching `recoil2png`, then matched against it (corpus oracle, every sample pixel-exact). No divergences recorded.

- Centauri Logo-Editor (`.cle`, 9 samples): 8194 bytes, load `$6000`. An 8000-byte multicolour bitmap in the usual cell order, then three colour bytes at offsets 8002-8004: bit pair `01` = high nibble of the first, `10` = its low nibble, `11` = low nibble of the second, background `00` = low nibble of the third. The remaining 189 bytes change nothing. There is no screen or colour RAM, so every picture uses four colours in total.
- Face Painter (`.fcp`, `.fpt`, 3 samples): 10004 bytes, load `$4000`. Koala layout (bitmap, screen, colour, background at `$6710`) plus one trailing byte that changes nothing.
- Dolphin Ed (`.dol`, 1 sample; `.bed` registered by extension only): 10242 bytes, load `$5800`. Drazpaint's map (colour RAM `$5800`, screen `$5C00`, bitmap `$6000`) with the background at `$5FE8` (offset 2026, low nibble). The earlier attempt missed it because the sample's own background is black; changing the byte moves the whole background.
- ECI Graphic Editor packed (`.ecp`, 1 sample): load `$4000`, then the escape byte (`$F3` in the sample, varied freely when repacking), then `ESC count value` runs and literals, unpacking to the 32768 bytes of the unpacked ECI layout. Count 0 is an empty run (not 256, unlike the other escape-RLE formats here). Extra bytes after the 32768th are ignored (3 in the sample); a stream that ends early is rejected, even by 68 bytes.
- Hires-Editor (`.het`, 1 sample): 9217 bytes, load `$5C00`. Doodle's screen (`$5C00`) and bitmap (`$6000`), one byte shorter than Doodle's file. Bytes after the bitmap change nothing.
- Interlace Hires Editor (`.ihe`, 1 sample): 16194 bytes, load `$2000`. Two bare bitmaps at `$2000` and `$4000` (192 unused bytes between them); no screen RAM, set bits are black and clear bits grey (`$0C`) in both frames, averaged.
- Multi-Lace Editor (`.mle`, 1 sample): 4098 bytes, load `$2000`. Two 2048-byte multicolour bitmaps at `$2000` and `$2800`, each covering 256 cells (six cell rows and 16 cells of the seventh; the rest of the 320x56 image is background). Fixed colours: `00` black, `01` brown (9), `10` orange (8), `11` green (5). The first frame is shown one pixel to the right (`recoil2png` fills black at the left edge), the second is not.

Left over from the Wave 4 list: M.C.S., the remaining Super Hires editors, NUP, UIFLI, MUFLI/MUIFLI, Botticelli 128x64 and PetDraw64. Not touched here. Apart from CLE and Face Painter, each format rests on a single sample. Other CLE file sizes, ECP files with other escape values and long runs, and `.bed` files are unverified.
