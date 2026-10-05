# Format gaps: merged summary of the 2026-10-05 survey

Merges five reports written the same day. Each keeps the detail, the source links and the
confidence tags:

| Report | Area |
|---|---|
| [gaps-nintendo.md](gaps-nintendo.md) | NES, SNES, Game Boy, GBA, DS, 3DS, GameCube/Wii, N64, Virtual Boy |
| [gaps-consoles.md](gaps-consoles.md) | Sega, NEC, SNK, Atari consoles, 3DO, CD-i, Sony, Xbox |
| [gaps-computers-extra.md](gaps-computers-extra.md) | Home computers, DOS and PC, Unix rasters, handhelds, printers |
| [gaps-amiga-extra.md](gaps-amiga-extra.md) | Amiga: ILBM fixes, new Amiga formats |
| [gaps-sample-sources.md](gaps-sample-sources.md) | Where to find samples, Wayback and archive.org mechanics, scripts |

No RECOIL source and no GPL/LGPL decoder code was read by any of the five. Section 8 lists what each
agent read and what to avoid.

## 1. Short version

- RECOIL covers none of the new console or Unix formats. Its only console row is PlayStation TIM.
  Nintendo, Sega, NEC, SNK, 3DO and the Unix rasters therefore have no `recoil2png` oracle. Every
  candidate names another check (a documented screenshot, a reference PNG, an MIT tool run as a black box).
- Amiga: five small bugs in code we already have, all measured on real files. Two files that RECOIL
  decodes fail today (two ACBM files and a DeluxePaint ST animation); three more are wrong in rare cases.
  Each fix is a few lines.
- The most ready new formats (spec read, real samples decoded, size S): SGX/SVG (Amiga), DS/DSi
  ROM icons, 3DS SMDH/3DSX icons, KiSS CEL+KCF, Flipnote PPM thumbnails, Amstrad PCW pictures, Sixel,
  Dreamcast VMS icons, First Publisher ART, Game Boy Printer captures and `.nes` CHR sheets.
- Most wanted by modders and archivists: Dreamcast PVR/GVR, PS2 TIM2, 3DO IMAG/CEL (size S-M to M,
  all with good specs and samples), plus the DOS clip-art and self-displaying-picture families.
- Four shared pieces should exist before the decoders (your "refactor first" rule): a planar tile
  helper, an XPK layer in `codec/`, a GameCube texture block codec, and a Morton/twiddle helper.
  Section 6.
- Eleven decisions are open, mostly scope and provenance. Section 5.
- Thin spots: the shared 200-call web-search budget ran out in every agent. N64, Virtual Boy, the
  Atari 2600/5200/7800/Jaguar, ColecoVision, Intellivision, Saturn and the East European computers are
  recall-level or negative-evidence only. Section 10.

## 2. Evidence levels used below

| Level | Meaning |
|---|---|
| **V** | A real sample file was decoded or parsed and checked (by eye, by exact size, or against a reference image) |
| **S** | The primary spec was read, no decode tried |
| **M** | Snippet or memory only: a lead, not a spec |

"Size" is S (a day or less), M (a few days or one risky detail), L (reverse engineering). XS is a few lines.
"Ref" points to the section of the report that holds the layout facts.

## 3. Candidates by area

### 3.1 Amiga (gaps-amiga-extra.md)

Fixes to decoders we already have. Each was measured with `recoil2png` and the release CLI.

| Fix | Size | RECOIL | Evidence | Ref |
|---|---|---|---|---|
| Keep reading past a FORM length that is shorter than the data. `cover` and `map1` (ACBM) fail today | XS | decodes both | V | 3.1 |
| `FORM ANIM`: skip chunks (`ANNO`) before the first `FORM ILBM` (Brilliance 1.0 files) | XS | rejects too | V | 3.2 |
| `FORM DPST` (DeluxePaint ST animation): decode the first frame; `vdat.rs` exists | XS | decodes | V | 3.3 |
| CAMG-less 6-plane files: 16 colours is HAM6, 32 is EHB, anything else indexed. We are wrong for five CMAP sizes | XS | differs | V | 3.4 |
| HAM flag on 5 or 7 planes: HAM6 and HAM8 with the missing top plane as 0. Exact match measured. No real file seen | XS | differs | V | 3.5 |

New formats:

| Format | Size | Evidence | Samples | Notes | Ref |
|---|---|---|---|---|---|
| SGX / SVG (SuperView Graphics) | S | V | 4 | `codec/inflate.rs` already does the zlib. Printed offsets in the spec are two bytes too high. Raw and LZ77 sample pair check each other. The `.svg` samples are XPK | 4.2 |
| IFF-RGFX | M | S | 16 | all 16 samples are XPK (15 MASH, 1 NUKE), so it needs the XPK layer. Spec file `rgfx.h` is "Freeware. All Rights Reserved" | 4.1 |
| Amiga bitmap fonts (`.font` + size files) | M | V (headers) | 23 | hunk parsing plus loc/space/kern tables; font sheet layout is a design call | 4.5 |
| YAFA animation, first frame | S (M with XPK) | S | 13 | first-hand spec; the scene shipped many 1995-98 animations this way | 4.3 |
| CDXL, first frame | S-M | V | 10 | headerless; `ffmpeg -frames:v 1` is a black-box oracle (LGPL decoder: run only) | 4.4 |
| Packers around pictures: Imploder, Crunch-Mania, RNC, StoneCracker, ByteKiller, Pack-Ice footer | M each | S | few | Ancient (BSD-2) covers all. 9 pictures among 128 packer samples; the 156-file scene sample had none | 4.6 |
| Deep ILBM variants: 12, 21 (NewTek order), 32, 48, 64 planes | S each | S | none | no real sample seen in about 280 files | 3.7 |
| Palette-only ILBM as a swatch grid | XS | V | 3 | design decision, not a decoder gap | 3.7 |
| CFAST (Disney Animation Studio `.cft`) | M | partial | 14 | plane RLE needs reverse engineering | 4.7 |
| 16-plane `PLTP` ILBM | L | V (headers) | 4 | layout unknown, looks like a sampled signal; RECOIL rejects too | 3.7 |
| YUVN | S | S | none | complete spec, untestable | 4.8 |
| `LIST`/`PROP`/`CAT`, `ANBM` | S | S | none | no real picture file found | 4.9 |
| Bare AMOS picture blocks | XS-S | partial | 11 | palette location unknown | 4.10 |

Rejected: Imagine and Turbo Silver textures (executable procedural modules), Sculpt, Real 3D, LightWave
and Pixel 3D (3D data), DR2D, AMFF, DrawStudio, PageStream (vector or document), Fantavision (no
still), PMBC (proprietary), SSA/VAXL/FILM/MovieSetter/Toaster (no public layout), Scala, Blitz shapes,
datatype-only formats, QRT, hardware sprite dumps, OS4/AROS PNG icons (blocked). Digi-View, Digi-Paint,
Photon Paint, DPaint, ADPro, Brilliance and Personal Paint all write plain ILBM or ANIM; their private
chunks (DGVW, BHBA, BHCP, BHSM, DPXT, BRNG) do not change the picture. The chunk status table is in
gaps-amiga-extra.md section 2. Video Master `.flm/.vid/.vsq` belongs to the Atari work.

### 3.2 Nintendo (gaps-nintendo.md)

| Format | Platform | Size | Evidence | Samples | Notes | Ref |
|---|---|---|---|---|---|---|
| ROM banner icon | DS, DSi (`.nds`, `.dsi`, `.srl`) | S | V | 2 real ROMs | two CRC-16 values give a strong signature; DSi animated icon is S-level only. NitroPaint (BSD-2) | A1 |
| SMDH / 3DSX / CIA icon | 3DS | S (CIA M) | V | 1 real 3DSX | RGB565 Morton tiles, GBATEK complete. Shares tile code with CLIM | A2 |
| iNES / NES 2.0 ROM as CHR tile sheet, UNIF | NES | S | V (headers) | 263 test ROMs, about half CHR-RAM | reuses `nes::tile_pixel`; CHR-RAM games stay unrecognised | A4 |
| Flipnote Studio PPM | DSi | S (frames M) | V | 56 PPM in MIT repos | 64x48 thumbnail first; `flipnote.js` (MIT) is an oracle. Docs wiki is GPL-2 (prose only) | D1 |
| rgbgfx outputs `.2bpp .1bpp .tilemap .attrmap .pal` | Game Boy | S/M | S | about 60 + 36 + 32 + 42 | RGBDS fixtures (MIT) give a pixel-exact PNG oracle. Tilemap has no width; `.pal` collides | B1 |
| Game Boy Printer capture logs (text) | Game Boy | S | V | about 10 | two text dialects, 0 pixel difference on 2 captures. GPL-3 emulator repo: data only | D2 |
| GameCube/Wii TPL (and BTI) | GC, Wii | M | S | none free | shares the GX pixel codec with Dreamcast GVR | C3 |
| DS NCLR/NCGR/NSCR (NCBR) | DS | M | S | none free | fits `Format::with_companions`. GBATEK and NitroPaint disagree on the NCGR header; trust the code and check | C1 |
| 3DS CLIM, CTPK, CTXB | 3DS | M (+M for ETC1) | S | none free | skip FLIM | C4 |
| DS BTX0 / TEX0 textures | DS | M | S | none free | contact sheet output | C2 |
| GameCube BNR, memory-card GCI | GC | S/M | S | none free | YAGCD bit table for GCI conflicts with other sources | A3 |
| GBA/DS raw tiles and BIOS-compressed streams (`.lz`) | GBA, DS | S-M | S | n/a | no signature; `.4bpp` means linear on GBA, planar on SNES | B2 |
| SNES planar and Mode 7 dumps | SNES | S plus options | S | n/a | only as part of a shared planar codec | B3 |
| BizHawk `.State` framebuffer | many | S | S | none | ZIP with a BMP; tiny audience | D3 |

Variants of formats we have (all S): NES `.chr` of any size from 1 to 256 KiB (V1), NES `.pal`
master-palette companion (V2), NES Screen Tool `.rle` nametables (V3), Pin Eight `.pb53`/`.pkb` (V4),
Game Boy Camera `.srm` alias plus a contact sheet and the 32x32 thumbnails (V5), binary NSS (V6, no sample).

Rejected: emulator save states (Mesen, FCEUX, Snes9x, bsnes, DeSmuME, VBA, Dolphin), N64, Virtual Boy,
Wii U/Switch textures, 3DS MPO, Flipnote KWZ, Pokemon sprite compression, ROM header logos, e-Reader,
FDS, Tile Studio, NFTR/BCFNT fonts, NCER/NANR cells. Reasons are in section 10 of that report.

### 3.3 Other consoles (gaps-consoles.md)

| Format | Platform | Size | Evidence | Samples | Notes | Ref |
|---|---|---|---|---|---|---|
| PVR / PVP / PVM, GVR / GVM | Dreamcast, GameCube | M | V | 12 | PuyoTools (MIT) and `gvrtex` (MIT). Twiddle axis order and VQ mipmap offsets worked out from samples (some docs are wrong). Length fields cannot be trusted | 3.1 |
| TIM2 (`.tm2`) | PS2 | S-M | V | 11 spec variants + 6 real | CSM1 8-bit CLUT needs the bit 3/4 swap; real files are linear, not GS-swizzled; opaque alpha is 0x80 | 3.2 |
| 3DO IMAG / CEL / ANIM | 3DO | M | V (IMAG) | about 365 in one repo | pixel order 1 interleaves row pairs; packed cel decoder not yet tried | 3.3 |
| VMS / ICONDATA_VMS icons | Dreamcast VMU | S | V | 1 | CRC-16 is the only signature for data files | 3.4 |
| GIM | PSP, PS3 | M | M | 12 | spec evidence is search snippets only; PuyoTools (MIT) | 3.5 |
| Neo Geo C-ROM pairs, CD `.SPR`, `.FIX` | SNK | S-M | V (partly) | 4 | public-domain wiki; needs `c1`/`c2` companions; `.spr` already claimed by five formats | 3.6 |
| Mega Drive Nemesis tile art | Sega | M | S | 100+ | CC BY spec; grey sheet without a palette companion | 3.7 |
| PS1 memory-card icons | PlayStation | S (M with wrappers) | S | none | 16x16 icons, small payoff | 3.8 |
| Raw VRAM tile sheets: SMS/GG, MD, PCE, WonderSwan, NGPC | various | S each | S (PCE, NGPC M) | none | nothing to detect them with; extension practice unknown | 3.9 |
| CD-i IFF images (DYUV, CLUT4, RL7) | Philips CD-i | M-L | partial | 11 | spec only partly public; RL7 undocumented | 3.10 |
| Atari Lynx `.spr` + `.pal` (sprpck) | Lynx | M | M | none | no size or depth in the file | 3.11 |
| TIC-80 `.tic`, PICO-8 `.p8` | fantasy consoles | S | S (TIC), M (PICO-8) | many | outside the brief; `.p8.png` needs a PNG decoder | 3.12 |

Rejected or not searched: Atari 2600/5200/7800/Jaguar, ColecoVision, SG-1000, Intellivision, Odyssey2,
Channel F, Vectrex, Pokemon Mini, Supervision, Game.com, Saturn, Sega CD/32X, PC-FX/PCE-CD (recall only:
no standalone image file known); Xbox XPR, PS1 PXL/CLT, Yuke's TXC, emulator save states, KallistiOS
tool formats.

### 3.4 Computers, PC, Unix (gaps-computers-extra.md)

| Format | Platform | Size | Evidence | Samples | Notes | Ref |
|---|---|---|---|---|---|---|
| KiSS CEL + KCF | KiSS (PC-98, Amiga, others) | S | V | 22 | public-domain spec; companion palette via `Companions`; CNF composition optional (M) | C1 |
| Amstrad PCW: MDA, MDP, CUT, GRF, SPC | Amstrad PCW (new) | S | V (MD2 only) | 5 | author spec; 1-bit; PCW pixels are about twice as tall as wide | C2 |
| Sixel | DEC VT340 | S | S | 22 | DEC manual, libsixel (MIT) as oracle | C3 |
| Unix rasters: Sun, SGI, XWD, XBM/XPM, PNM, farbfeld, Utah RLE | Unix | S each | S | 10-21 each | needs the scope decision; Pillow is an oracle | C4 |
| GRASP GL, first picture | DOS | S | V | 47 | embedded PCPaint PIC/CLP, which we already decode; weak signature | C5 |
| PFS First Publisher ART | DOS | S | V | 17 of 17 by exact size | high-res variant uses PackBits | C6 |
| Print Shop DAT/POG, PrintMaster SHP/SDR, PrintPartner GPH, Print Shop GS | DOS, IIGS | S each | V (SHP, POG) | 40+ | multi-picture libraries need a policy; DAT size rule fails on one real file | C7 |
| Self-displaying DOS pictures: PIXIT, GIFEXE, PCX2EXE, OPTIKS | DOS | S-M | V (PIXIT, GIFEXE) | 60+ | stub fingerprint plus fixed payload offset; do not scan EXEs for `GIF8` | C8 |
| BSAVE dumps and raw CGA/EGA/MCGA screens | DOS | M | S | 23 | `FD` + length is the only check; palette guesses | C9 |
| TI-Artist `_P` + `_C` | TI-99/4A (new) | S | S | none found | same pixel model as MSX Screen 2; needs a stem-plus-suffix companion | C10 |
| Inset PIX | DOS | M | M | 45 | spec link is dead; two incompatible versions | C11 |
| Small PC batch: DCX, OS/2 icon and pointer, Win 1.x ICO/CUR, HP LX ICN, DGI, IBM KIPS, Lumena CEL | DOS | S each | V (KIPS, ICN) | 5-10 each | DCX is XS | C12 |
| IBM Storyboard PIC/CAP | DOS | M | partial | 28 | Deark calls most of it experimental | C13 |
| Apple II set: Print Shop clip art, IIgs Finder icons, LZ4FH, Paintworks ANI, lo-res | Apple II, IIGS | S each | S | 11-13 | identification needs the ProDOS file type our API never sees | C14 |
| EPOC MBM, Sketch, AIF | Psion Series 5 | M | partial | 10 | UID-word signature | C15 |
| PC-98 plane dumps B1/R1/G1/E1, BLK, FRM | NEC PC-98 | S | M | none | blocked on samples and spec | C16 |
| WPG, Palm bitmap/TealPaint, ESC/P bit images, SSTV `.HRZ` | misc | M, M, M, XS | M | 19, 10, none, 1 | low value, not ranked | C17-C20 |

Rejected: Geneve MyArt/YAPP, ThunderScan, RISC OS Clear, Lotus PIC, AutoCAD SLD, GEM Draw, Image
Alchemy and similar scientific formats, GEM IMG from PC Ventura (same file as the Atari one), Sierra
AGI/SCI, phone logos, MegaZeux, KYG, PIC2, Q0, TK4, Towns Paint II, TMS9918 dumps for Sord, Memotech,
Coleco ADAM and others, CoCo/Dragon PMODE (header gap only), disk wrappers.

Systems with no picture format worth supporting (about 40, negative evidence from Just Solve, Deark,
dexvert and PictureFan): Sharp X1/MZ, FM-7, PC-6001/8001, PET, Dragon, Jupiter Ace, Aquarius, Lynx,
Hector, P2000, Enterprise, Kaypro, Lisa, Star, the Soviet and Eastern European machines, and others.
Table in gaps-computers-extra.md section 6. The Eastern European rows are the weakest.

## 4. Combined ranking

Ranked on: real files that fail today, how often the format turns up in archives, spec quality, samples
in hand, size, and how few decisions it needs. Batches are in order; inside a batch the order is a guide.

### Batch 0: Amiga ILBM correctness (all XS)

| # | Item | Why |
|---|---|---|
| 1 | Short FORM length, DPST first frame, CAMG-less 6-plane rule, HAM on 5/7 planes, ANIM chunk before first frame | files that RECOIL decodes and we reject, or wrong colours; a few lines each |

### Batch 1: new formats with spec, samples and no open decision

| # | Item | Size | Samples | Needs |
|---|---|---|---|---|
| 2 | SGX / SVG | S | 4 | signature gate against the CPC `SGX` row |
| 3 | DS/DSi ROM banner icon | S | 2 ROMs | transparent-colour fill |
| 4 | 3DS SMDH / 3DSX icon (CIA later) | S | 1 | shares Morton helper |
| 5 | KiSS CEL + KCF | S | 22 | platform label |
| 6 | Flipnote PPM thumbnail | S | 56 | platform label |
| 7 | NES ROM CHR sheet + UNIF, NES `.chr` any size | S | 263 | planar helper; sheet height cap |
| 8 | Amstrad PCW pictures | S | 5 | platform label; aspect ratio |
| 9 | Sixel | S | 22 | platform label |
| 10 | Dreamcast VMS / ICONDATA_VMS | S | 1 | transparent-colour fill |
| 11 | PFS First Publisher ART | S | 17 | extension collision handling |
| 12 | Game Boy Printer captures | S | about 10 | content-only detection |

### Batch 2: the larger, high-demand formats

| # | Item | Size | Samples | Needs |
|---|---|---|---|---|
| 13 | PS2 TIM2 | S-M | 17 | alpha fill; "confidential" banner call |
| 14 | Dreamcast PVR/PVP/PVM and GVR/GVM, with GameCube TPL on the same GX codec | M | 12 (TPL: none free) | GX codec, Morton helper, PVP companion |
| 15 | 3DO IMAG / CEL / ANIM | M | about 365 | tag-based detection (`.cel` collides) |
| 16 | Print Shop, PrintMaster, PrintPartner libraries | S each | 40+ | multi-image policy |
| 17 | GRASP GL first picture, self-displaying DOS pictures, BSAVE/CGA dumps | S, S-M, M | 47, 60+, 23 | palette choices for CGA |
| 18 | Amiga bitmap fonts | M | 23 | sheet layout |
| 19 | XPK layer (MASH, NUKE) then IFF-RGFX, YAFA, SGX/SVG XPK variants | M | 16 + 13 + 2 | `rgfx.h` provenance call |
| 20 | YAFA (unpacked frames) and CDXL first frames | S-M | 13, 10 | animation policy; `ffmpeg` oracle |
| 21 | rgbgfx family (`.2bpp`, `.tilemap`, `.attrmap`, `.pal`) | S/M | fixtures | width heuristic for `.tilemap` |
| 22 | Unix rasters (Sun, SGI, XBM, PNM first) | S each | 10-21 each | scope decision |
| 23 | DS NCLR/NCGR/NSCR | M | none free | companions; samples from own cartridges |
| 24 | PSP/PS3 GIM | M | 12 | spec is the weak part |
| 25 | Neo Geo CD `.SPR`/`.FIX`, cartridge C pairs | S-M | 4 | companions; `.spr` collisions |
| 26 | Mega Drive Nemesis | M | 100+ | needs a palette companion to be useful |

### Batch 3: low value, blocked, or without samples

TI-Artist (no samples), Inset PIX (spec link dead), Storyboard, Apple II set (ProDOS type), EPOC MBM,
PC-98 plane dumps, 3DS CLIM/CTPK, DS BTX0, GameCube BNR/GCI, GBA/SNES raw tiles and BIOS decompression,
raw VRAM tile sheets (SMS, PCE, WonderSwan, NGPC), CD-i, Lynx, PS1 memory-card icons, TIC-80 and PICO-8,
Amiga packers (Imploder, Crunch-Mania, RNC, StoneCracker), deep ILBM variants, palette-only ILBM, CFAST,
PLTP, YUVN, ANBM, bare AMOS pictures, WPG, Palm, ESC/P, SSTV, BizHawk states.

## 5. Decisions needed

1. ROM and executable containers as image formats. DS banner, 3DS icon and `.nes` CHR give an icon
   or a tile sheet, not a standalone picture. Precedent: Game Boy Camera SAV, AMOS banks, Amiga icons.
   Recommendation: yes.
2. Generic Unix rasters (Sun, SGI, XWD, XBM/XPM, PNM, farbfeld, Utah RLE). The registry already has
   GIF, BMP, ICO, Targa and PCX. Not retro hardware; low retro value. Recommendation: yes, but last.
3. Transparency. `Image` has no alpha, and DS/3DS/GBA colour 0, PVR, TIM2, VMS and GIM carry it.
   Pick one fill (light grey; the Workbench icon grey and `fm_towns/icn.rs` already composite onto grey)
   and record it once. Alpha-only fonts will show as solid squares.
4. Files with several pictures (PVM/GVM, multi-picture TIM2, 3DO ANIM, VMS frames, Print Shop and
   PrintMaster libraries, GRASP GL, KiSS sets, animations). Suggested policy: first picture for archives
   of unrelated pictures and for animations; a fixed-grid sheet for icon frames and small clip-art
   libraries. One rule for all.
5. Platform names. RECOIL's list has none of these. Proposed: Nintendo DS, Nintendo 3DS, GameCube,
   Wii, Super Nintendo, Game Boy Advance, Dreamcast, PlayStation 2, PSP, 3DO, Neo Geo, Mega Drive, KiSS,
   Amstrad PCW, TI-99/4A, DEC VT340, Unix. `RETRO_IMAGE_PLATFORMS` filtering depends on them.
6. Provenance calls:
   - TIM2 spec carries a "confidential" banner (public for years); CD-i technical notes carry "not to be
     duplicated".
   - `rgfx.h` is "Freeware. All Rights Reserved" and the readme names it as the chunk spec. The Amiga agent
     already read it. Treat it as spec prose, or limit work to the three text files plus the samples.
   - Pin Eight's `pilbmp2nes.py` is under the GNU all-permissive licence, not on the MIT/BSD/zlib/Apache/CC0 list.
   - GBATEK, YAGCD, nesdev wiki and similar have no licence: facts only.
   - The Flipnote docs wiki is GPL-2 prose.
7. Deark as a black-box oracle. MIT, builds from source with a C compiler, already used for BMP and
   PCX. Several candidates have no other oracle. Recommendation: yes.
8. Animations and movies (ANIM, YAFA, CDXL, CFAST, Flipnote): first frame only, as ANIM today.
9. Palette-only ILBM: show a swatch grid, or leave unrecognised.
10. `.sgx` on two platforms: keep the CPC row and add an Amiga row with a signature gate, or merge.
11. A second discovery pass once the web-search budget is available again, aimed at the thin spots in
    section 10. The question is whether it is worth it before building batches 0 to 2.

## 6. Shared infrastructure to build first

The four pieces below each serve several candidates. Your "refactor to the principle first, then change
behavior" rule applies: do these before the decoders that need them.

| Piece | Serves | Where it lives today |
|---|---|---|
| Planar tile helper (bpp, plane order, row interleave, nibble order, tile size) | NES ROM CHR, rgbgfx, Game Boy sheets, GBA/SNES dumps, Neo Geo, MD, SMS, PCE tiles | separate loops in `nes::tile_pixel` and `game_boy/camera.rs` |
| XPK layer in `codec/` (container, MASH, NUKE; Ancient is BSD-2) | IFF-RGFX, YAFA, SGX/SVG XPK files, XPK-wrapped ILBM | not present; sits next to `powerpacker.rs` |
| GameCube GX block codec (I4, I8, IA4, IA8, RGB565, RGB5A3, ARGB8888, CI4/8, CMPR) | TPL, GVR/GVM, BNR, GCI | not present |
| Morton / twiddle index | PVR, 3DS tiles, TIM2 if swizzled files appear | not present |

Other registry notes from the reports:

- Alpha, multi-picture policy, platform names: decisions 3 to 5 above.
- Companions fit PVR+PVP, Neo Geo `c1`+`c2`, DS NCGR+NCLR+NSCR, KiSS CEL+KCF, rgbgfx files and
  palette files for tile sheets. Every main file must still decode alone with a grey ramp.
  TI-Artist `_P`/`_C` and KiSS `.cnf` palettes need stem-plus-suffix or `get_named`.
- Detection (`.signature()`) is safe for: DS banner (two CRC-16), SMDH/3DSX, `NES\x1A`, `PARA`, Game
  Boy Printer packets, NCLR/NCGR/NSCR chunk magics, `00 20 AF 30`, PVR/GVR/TIM2/GIM/3DO tags, KiSS, MDA/MDP,
  Sixel, Sun, SGI, PrintMaster, PrintPartner, EPOC. Not for headerless tile dumps, size-only formats, `.cel`
  without a tag, Print Shop DAT, FP ART (exact size only) or raw CGA.
- Extension collisions: `.cel`, `.chr`, `.spr` (five claimants), `.art`, `.cut`, `.grf`, `.spc`, `.pix`,
  `.pic`, `.icn`, `.pal`, `.sgx`, `.shp`, `.dat`, `.pog`, `.4bpp`/`.8bpp`. Tag-checked formats go first;
  headerless extension-gated formats go last. Full table in gaps-computers-extra.md section 8.
- Output size: `.nes` CHR and DS BTX0 can make very tall sheets; cap them or widen.
- Palette choices to record, as for the NES master palette: SMS 2-bit expansion, Mega Drive DAC ramp,
  PCE 9-bit expansion, Neo Geo dark bit, TIM2 alpha 0x80 = opaque, VMU ARGB4444 `v * 17`, CGA palette,
  Print Shop GS colours, Sixel register defaults.
- Divergence files: per `adding-a-format.md`, record each reviewed sample with evidence for formats
  RECOIL lacks.

## 7. Spec conflicts the agents found (check before coding)

| Where | What |
|---|---|
| SGX spec (Aminet) | printed offsets are two bytes too high; use the sample-checked list in gaps-amiga-extra.md 4.2 |
| PVR | the SmokesGrass document lists pixel formats 5/6 and data types 5-8 wrongly; the Ikaruga guide has the twiddle axes the wrong way round (gives a transposed picture) |
| PVR length fields | `GBIX` and `PVRT` lengths can be 0 in real files; size the data from width, height and format |
| TIM2 | opaque alpha is 0x80, not 0xFF; CSM1 8-bit CLUT needs the bit 3/4 swap; real files are linear |
| NCGR | GBATEK reads the first two u16 as "size in KB, always 0x20"; NitroPaint (BSD-2) reads tilesY and tilesX |
| NSCR | GBATEK lists width at +8 (4 bytes) and height at +0xA, which overlap; read as two u16 |
| GameCube BNR | YAGCD says RGB5A1, its own footnote and mkwiiki say RGB5A3 |
| GCI | YAGCD bit table for the image key conflicts with Dolphin-style banner formats; check with a real file |
| DS icon | low nibble is the left pixel (the doc does not say; verified on two ROMs) |
| ILBM | no-CAMG 6-plane rule: spec says HAM; RECOIL and Just Solve give 16 = HAM6, 32 = EHB, else indexed |
| Print Shop DAT | "size is a multiple of 572" fails on one real file (26,368 bytes) |
| CDXL | the MultimediaWiki field layout does not fit every sample (`optologo.cdxl`, one `Maku.XL` size) |

## 8. Clean-room position

Read by the agents, by licence:

| Licence | Read |
|---|---|
| MIT / BSD / Apache / CC0 / zlib / WTFPL, code read | Deark, libsixel, PuyoTools, NitroPaint (BSD-2), RGBDS, SameBoy (headers only), `flipnote.js`, KallistiOS (BSD-like, except `utils/pvrtex`), CiderPress II (Apache-2.0), Kaitai `ines.ksy`, TIC-80 format page (MIT) |
| Permissive, licence checked but code not read | libilbm and libiff (MIT; libilbm README read), amigazen/ifftools (BSD-2), bitplane/datatypes, oxideav-iff, Ancient (BSD-2; file sizes only), sprpck (Apache-2.0), SuperFamiconv, PVSnesLib, `gvrtex` |
| Prose only (no licence or restricted) | GBATEK, YAGCD, mkwiiki, nesdev and SNESdev wikis (CC0), Pan Docs (CC0), 3dbrew, Shonumi, psx-spx, Marcus Comstedt's Dreamcast pages, John Elliott's MicroDesign page, DEC VT340 manual, Ninerpedia (CC BY-NC-SA), Just Solve (CC0), AmigaOS wiki, Aminet docs, MultimediaWiki |

Do not read (code): RECOIL in any form; dexvert (no licence; sample folders only); the KYG parser
gist; abydos, wuimg, XV, netpbm source; Tilemap Studio (LGPL-3); grit (GPL-2); `bucanero/dcvmu-tool`,
Rainbow and Kuriimu2 (GPL); the Arduino Game Boy Printer emulator decoders (GPL-3); KallistiOS
`utils/pvrtex` (bundles LGPL FFmpeg `elbg`); FFmpeg decoders (LGPL, run the binary only); Mesen, FCEUX,
Nestopia, Snes9x, bsnes, mGBA, VBA, Gambatte, DeSmuME, melonDS, Citra, Dolphin; Tinke, ndspy, Kiwi.DS;
xoreos; the pret disassemblies and their tools (no licence); city41/neospriteviewer; GirianSeed/tim2
sample code; libogc, Wiimm's `wimgt`, BrawlCrate; YY-CHR (closed); OS4/AROS icon libraries; SPRtools.
Licences not checked and so treated as unreadable: mdcomp, clownnemesis, bmp2tilecompressors, the Sonic
disassemblies, the 3DO devkit sources.

One licence claim is second-hand: the MIT status of the GVR references (`gvrtex`, PuyoTools `Textures/Gvr`,
DolphinTextureExtraction-tool) comes from the consoles agent. The Nintendo agent did not check it. Check the
`LICENSE` files before anyone reads that code.

Where code or tables are taken from a permissive project, the module keeps its copyright notice and licence
text (existing rule).

## 9. Samples

The sample report is the practical one. Main points, all tested unless marked:

- Wayback. CDX with `filter=statuscode:200`, `filter=!mimetype:text/html` and an extension regex
  enumerates a dead site; the `id_` URL form gives unmodified bytes. Counts from sites that no longer
  resolve: `garbo.uwasa.fi` 30,564 files, `wuarchive.wustl.edu` (`/systems/amiga` 38,638, `/systems/mac`
  9,935, `/graphics` 6,273, `/systems/atari` 909), `hp.vector.co.jp` 13,577 `.lzh`/`.zip` (Japanese hobby
  pages), the CPCScene mirror on `amstrad.eu` 1,244 URLs. Several hosts assumed dead are live
  (`devrs.com`, `nesdev.parodius.com` redirects, `noname.c64.org`).
- archive.org. `archive.org/download/<id>/<big.zip or .iso>/<inner/path>` pulls one member without the
  container (one level only). A Range-request zip lister reads a 15 GB TOSEC zip's directory in 5 requests.
  Chain tested end to end: Atari ST TOSEC zip, Floppyshop picture disk, FAT12 extractor, `retro-image`
  accepted 16 of 19 files. 345 more Floppyshop Picture/Clip Art disks are in that zip.
- Direct APIs and indexes: ZXArt (19,507 Spectrum-family pictures, including `.sxg`, `.hrg`, `.specscii`,
  `.ss4`), Demozoo, 16colo.rs (about 5,500 packs), CSDb, Pouet, Aminet `INDEX` (11,889 `pix/` packages),
  asimov `site_files.txt`. rsync works for funet and scene.org (`graphics/` 8,704 files).
- Not yet mined: `ggnkua/Atari_ST_Sources` (2,452 `.PI1`, 1,147 `.ANI`, 691 `.NEO`; a blobless sparse
  clone fetched the `.NEO` files in about 5 s), Assembly64 `CSDB_discmags` / `c128` / `bbs`, 520 CDs on
  cd.textfiles.com (Atari ST and Amiga coverdisc sets), and the Sembiance folders not yet in `corpus/`.
- Pipeline: the appendix scripts go from CDX listing to throttled download, dedupe against the corpus by
  body hash, unpack, and a `retro-image` accepted/rejected sort. Manifests use the newer five-column format.

Where the new candidates' samples come from:

| Candidate | Source |
|---|---|
| SGX/SVG, RGFX, CDXL, YAFA, Amiga fonts | Sembiance `fileFormatSamples` folders; Aminet `docs/misc`, `util/dtype`, `demo/tp95`; scene.org Amiga mirror (about 4,200 IFFs) |
| DS/DSi icons, 3DS icons | GitHub release assets: GodMode9i v3.9.0 (`.nds`, `.dsi`), Universal-Updater v3.4.1 (`.3dsx`, `.cia`) |
| NES ROM CHR | `christopherpow/nes-test-roms` (263 ROMs); nesdev compo ROMs |
| Flipnote PPM | `jaames/flipnote.js` and `flipnote-player` repos (56 PPM) |
| rgbgfx | `gbdev/rgbds` `test/gfx/` (MIT) |
| Game Boy Printer | `mofosyne/arduino-gameboy-printer-emulator` capture files and PNGs (data only; repo is GPL-3) |
| PVR, TIM2, GIM, CD-i | Sembiance folders; KallistiOS examples; `GirianSeed/tim2` `tim2img_e.zip` |
| 3DO | `trapexit/3do-devkit` (about 365 files) |
| Neo Geo | freem's `helloworld_tutorial.zip`; wiki.neogeodev.org |
| Nemesis | `sonicretro/s1disasm` `artnem/` |
| KiSS | Sembiance `kissCel` and `kissCELColorPalette`; otakuworld free dolls |
| PCW | Sembiance `microDesign`; John Elliott's `mdaspec.com` on the classiccmp mirror |
| Sixel | Sembiance `sixel`; libsixel `images/` |
| Print Shop family, FP ART, PIXIT, GIFEXE, GL, BSAVE | Sembiance folders; textfiles CDs (`swinnund/disk3/CLIPART/`, `powerpakgold/GRAPHV_E/`, `psl/psl9309/DOS/PCWRITE/PIX/`) |
| Unix rasters | Sembiance folders (10-21 each), netpbm and libtiff test sets |
| GRASP GL | Sembiance `video/grasp` (47); `corpus/extra/next-amiga-pc/` already holds some derived CLP files |
| TPL, NCGR/NCLR/NSCR, 3DS CLIM/CTPK, GCI, BNR | none free found; the user's own cartridges and discs |
| TI-Artist, PC-98 plane dumps, Inset PIX with spec | none found, or no spec |

All samples belong to their artists and stay in the git-ignored `corpus/`, with a manifest row giving the
source and licence (`none stated` where there is none). Some art disks (PC-98, Atari ST, 16colors) hold adult
material; keep it out of anything shown to others. Details: gaps-sample-sources.md section 6.

## 10. Thin spots and unverified claims

| Area | State |
|---|---|
| N64, Virtual Boy | recall only |
| Atari 2600/5200/7800/Jaguar, ColecoVision, Intellivision, Saturn tool formats | in the rejected list on memory; never searched |
| East European computers (Specialist, Orion, Korvet, PMD 85, Primo, KC 85, Robotron, Pravetz) | negative evidence from four catalogues; no search |
| Exelvision, P2000, VG5000, Epson QX-10, DEC Rainbow | not searched individually |
| Kosinski, Enigma, Saxman | not read in detail |
| PCE and NGPC tile byte order, real-world extension names for raw tile dumps | unconfirmed |
| GIM block layout, `.dci` byte order, PS1 memory-card wrappers, 3DO packed cels | not tried on samples |
| MD3, MDP, CUT, GRF, SPC (PCW) | spec only; only MD2 decoded |
| DSi animated banner, GCI, KWZ | spec only or none |
| ZXArt `.specscii` | 35 files available; may unblock the SpecSCII question (separate from these reports) |
| All `M`-evidence rows above | leads, not specs |

Most of this comes from the exhausted search budget. Re-running these areas with search available is
decision 11.

## 11. Stale documents

- `gaps-amiga-8bit.md` still lists RGBN/RGB8 and PowerPacker as gaps; `gaps-ranking.md` rows 5 (PP20),
  11 (RGBN/RGB8) and 23 (BBM) are done too.
- `amiga-apple-misc.md` says libilbm and amigazen/ifftools have unchecked licences. Checked: libilbm and
  libiff are MIT, ifftools is BSD-2.
- `gaps-others.md` section 2.6 (raw SNES, Mega Drive, PC Engine and Neo Geo tile dumps, ranked 18 and
  "low") is replaced by gaps-consoles.md section 3.9 and gaps-nintendo.md B2 and B3.
- `next-zx-misc.md` section 3 and `gaps-ranking.md` row 21 describe NES and Game Boy work that is done
  (PR #14).
- A separate session (`r5-amiga-pc`) added a five-line cross-reference to `next-amiga-pc.md`; the file is
  modified and uncommitted.

## 12. Decisions taken (2026-10-05)

Answers to section 5, and the status of batch 0.

| # | Decision |
|---|---|
| 1 | ROM and executable containers count as image formats: DS banner, 3DS SMDH/3DSX icon, `.nes` CHR sheet. |
| 2 | Unix rasters are in, last in batch 2. |
| 3 | Transparent pixels are composited onto one shared light grey, defined once in the crate. |
| 4 | Several pictures in one file: the first picture for archives of unrelated pictures and for animations, a fixed-grid sheet for icon frames and small clip-art libraries. |
| 5 | Platform names are the list proposed in section 5. |
| 6 | Provenance calls approved: the TIM2 spec is read as spec prose (facts only), `rgfx.h` is read as spec prose, `pilbmp2nes.py` may be read (GNU all-permissive). GBATEK, YAGCD, nesdev and similar stay facts-only; the Flipnote docs wiki stays prose-only. |
| 7 | Deark may be built from source and run as a black-box oracle. |
| 8 | Animations and movies decode the first frame only. |
| 9 | Palette-only ILBM: not decided, left for batch 3. |
| 10 | `.sgx`: keep the CPC row and add an Amiga row with a signature gate. |
| 11 | No second discovery pass before batches 0 to 2 are built. |

Batch 0 was merged as PR #18: short FORM length, ANIM and DPST first frame, CAMG-less and odd-plane-count
color modes. Sections 3.1 and 4 of this file still list those as open.

The XPK layer (section 6) is built inside the Amiga family's work rather than ahead of it, because IFF-RGFX,
YAFA and SGX/SVG are its only consumers. The other shared pieces (planar tile helper, Morton index, GX block
codec) go in with their first consumer too, since CI rejects dead code.
