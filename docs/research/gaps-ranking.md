# Format gaps: merged ranking

Merged from the five family surveys:
[Commodore](gaps-commodore.md), [Atari](gaps-atari.md),
[Amiga and 8-bit](gaps-amiga-8bit.md), [PC and Japan](gaps-pc-japan.md),
[others](gaps-others.md). Each survey has the spec links, sample sources and
confidence tags (`fetched`, `snippet`, `memory`) behind these rows; check the
family file before starting a format.

Ranking weighs scene relevance, real-world frequency, a public permissive
spec, sample availability and difficulty (S, M, L). Several sites were
unreachable during the surveys, so treat snippet- and memory-based layouts as
leads, not specs.

## Scope

PCX and Targa are in scope (decided). The registry has no generic rasters
yet, and RECOIL has none, so there is no oracle to check them against;
evidence comes from other tools or by eye, as for the text-mode formats. GIF
and BMP are still open and stay marked **(scope)**.

## Top 30

| # | Candidate | Family | Size | Why |
|---|---|---|---|---|
| 1 | Self-displaying PETSCII PRG (`SYS 2061` stub) | C64 | S | 1851 files in the CSDb dump share one layout |
| 2 | Viewer-wrapped Koala-order PRGs | C64 | S per stub | about 1,100 files in three stub sizes |
| 3 | ZX SNA / Z80 / SZX snapshots (render screen memory) | ZX | S-M | most common ZX container, public specs |
| 4 | ZX TAP / TZX loading screens | ZX | S-M | pull the 6912-byte screen block |
| 5 | Shared Pack-Ice layer for ST formats, plus PowerPacker (PP20) for Amiga | Atari, Amiga | S-M | extends the Pack-Ice we already have; PP20 is the likeliest Amiga packer. How common packed pictures are is unmeasured: 5 of 3,318 corpus files, all `.TRP` we already decode |
| 6 | PCX | PC | S | most common DOS picture, about 35 samples |
| 7 | Autodesk Animator PIC / CEL, FLI/FLC first frame | PC | S-M | DOS staple, complete spec, 21 samples |
| 8 | BBC Mode 7 teletext (EP1, TTI, raw) | BBC | M | active scene; glyph font provenance to decide |
| 9 | PrintMaster / Print Shop `.GRA` | C64 | S | 58 real samples, GoDot documents the header |
| 10 | Cyber Paint `.SEQ` | Atari ST | S | spec complete, `.CEL` half already done |
| 11 | Amiga RGBN / RGB8 | Amiga | S | full public spec |
| 12 | ZX Next variants (SL2, SLR, SHC, SHR, 320x256 and 640x256 NXI) and +3DOS-headered SCR | ZX | S | a 7040-byte +3DOS SCR is rejected today |
| 13 | Dr. Halo CUT / PAL / PIC | PC | S | simple RLE, 22 samples, fits Companions |
| 14 | Signum `.IMC` | Atari ST | M | full reverse-engineered spec |
| 15 | PCPaint / PICtor PIC, CLP | PC | S-M | first DOS standard, 23 samples |
| 16 | ColoRIX RIX / SCx | PC | M | scene-heavy 256-colour art, 13 samples |
| 17 | CoCo 3 CM3, MGE, HRS, RAT, VEF | CoCo | S-M | RECOIL lacks them; MIT-licensed KAOS docs |
| 18 | Petmate `.petmate` (JSON) | C64 | S | current PETSCII standard, needs a small JSON reader |
| 19 | CharPad CTM v6-v9, SpritePad SPD v2 | C64 | S-M | we read CTM v5 and SPD v1 only |
| 20 | CPC SNA snapshots | CPC | M | render from saved Gate Array pens and CRTC state |
| 21 | Game Boy and NES art: GB Camera SAV, CHR/NAM, GBTD GBR/GBM | consoles | S | active homebrew scene, easy samples |
| 22 | FM Towns ICN / HEL, register existing Pi | Japan | S | closes the only FM Towns row in `coverage.md` |
| 23 | Check `ilbm.rs` accepts form type `BBM ` | Amiga/PC | XS | PC Deluxe Paint files; maybe a one-line fix |
| 24 | Targa | PC | S | common in DOS and raytracing output |
| 25 | GIF87/89, Fractint FRA **(scope)** | PC | S | universal, easy |
| 26 | DreamGrafix, packed Apple II Hi-Res / DHR | Apple | M | best-documented Apple gaps (CiderPress II docs) |
| 27 | GEOS geoPaint / Photo Scrap / Photo Album | C64/C128 | M | major format RECOIL lacks; needs a VLIR spec and samples |
| 28 | Self-displaying XEX pictures (RastaConverter output) | Atari 8-bit | L | highest-value 8-bit item for the modern scene |
| 29 | STOS Picture Packer `.PP1`-`.PP3` | Atari ST | M | very common; compression undocumented, needs reverse engineering |
| 30 | Raw ST screen dumps (32000 / 32034 bytes), extension-gated | Atari ST | S | common in demo trees; size sniffing alone gives false positives |

## Next just below the cut

Imploder, Crunchmania, StoneCracker and RNC pictures via the Ancient algorithms
(BSD-2 source, re-implemented): wait until real packed samples turn up, since
the local corpus and CSDb dump contain none.
Self-extracting crunched C64 pictures (a generic 6502 stub emulator would
cover them all, L), BMP RLE4/RLE8 and OS/2 **(scope)**, TI-Artist, PICT bitmap
subset, Newsroom, XRay64 IFLI, OS4 GlowIcons (needs the Atari inflate moved to
shared code first), NeoDesk icons, NEOchrome `.ANI`, Animatic `.FLM`.

## Blocked on a spec or samples

Yanagisawa PIC2, FM Towns TK4, Towns Paint II, ECC and about twenty PC-98
extensions have samples but no layout. The twelve Commodore editors already
recorded as missing still have no samples. Enterprise, Sharp MZ/X1, Dragon
paint programs and Apple Print Shop have no usable public spec.

## Do not read (clean-room)

RECOIL and any GPL decoder source. The surveys name specific hazards: TipTools
(GPL), sdo-tool's CLI (AGPL), GrafX2 PKM (GPL-only spec), the Alice Soft VSP
example code (derived from xsystem35, GPL), Martine, format198x, ep128emu and
coco-tools.

## Cheap first batch

Items 1, 2, 6, 9, 10, 11, 12, 13, 22 and 23 are all S or smaller, and have either
real samples in hand or a complete spec. They fit one working session.
