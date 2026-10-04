# "Check" variants in coverage.md (excluding C64)

Surveyed 2026-10-04. The "Check" table in [coverage.md](../coverage.md) flags
extensions where RECOIL's format list has more rows than we register entries.
Each row was checked three ways: the registry and decoder source, the corpus
through our CLI, and `recoil2png` as a black box (pixel diff with PIL, plus
acceptance sweeps over file sizes with random and zero data). Spec sources are
the ones the decoders already cite; no new GPL or unlicensed code was read.

## Result

Of the 36 non-C64 rows, 35 are already handled (3 of them with a tiny
acceptance gap against RECOIL) and 1 is partly handled (RastaConverter). None
is missing. "Check" mostly reflects how the report counts: one registry entry per extension, several
RECOIL rows. The decoders already dispatch by size, header or companion file.

The full oracle passes for all affected platforms with 0 failures:
`MSX2,MSX2+` 261 matched, `Atari ST,Atari Falcon` 672 matched,
`Atari 8-bit` 0 failed, `Commodore 16/116/Plus4` 12 matched.

## Table

Fix size: XS under an hour, S a few hours, L a day or more.

| Variant | Status | Evidence | Spec sources and licence | Fix size | Plan |
|---|---|---|---|---|---|
| Atari 8-bit ASC "ASCII maker" | Handled, one gap | Same 960-byte layout as Mad Studio GR0: RECOIL renders `.asc` and `.gr0` identically for the same bytes. All 6 corpus `.ASC` match RECOIL. RECOIL also accepts 1024 bytes for ASC and GR0 (not SCR) and ignores the last 64 bytes (flipping any of them changes nothing). We reject 1024. | [Mad Studio formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf), MIT project. Nothing documents ASCII maker itself. | XS | Accept 1024 bytes in `text::decode_gr0` for `asc`/`gr0` only, ignoring the tail. Needs a real sample to know what the 64 bytes are. |
| Atari 8-bit GR0/ASC/SCR Mad Studio Graphics 0 | Handled | `text::decode_gr0` takes 24 to 30 lines. RECOIL takes 960 only (and 1024, above), so we are a superset. Corpus SCR/GR0/ASC match. | Mad Studio PDF (MIT). | none | none |
| Atari 8-bit MIC Micro Illustrator / Graphics 15 | Handled | Size sweep, 725 sizes around 7680, 9600, 11520 and `lines*40 + {0,4,5}` for 1 to 299 lines: acceptance and output dimensions identical to RECOIL, no differences. 11 corpus MIC files match. | [G2F manual](https://g2f.atari8.info/instrukcja_eng.html), RECOIL formats list. | none | none |
| Atari 8-bit MIC (arbitrary) AtariGraphics | Handled by MIC, can't go further | 7680 and 7684 bytes with any extension. Same decoder as MIC. | [Just Solve AtariGraphics](http://fileformats.archiveteam.org/wiki/AtariGraphics) | none | Don't register by size alone (adding-a-format.md forbids it). |
| Atari 8-bit MIC+COL Graph2Font | Handled | `test.mic` + `test.col` (240 lines, 1024-byte COL): pixel-identical to RECOIL (PIL diff, empty bbox). Behaviour probes agree with RECOIL: COL used for 240-line MIC with 1024 or 1280 bytes, ignored for 192 lines, 768 or 1025 bytes. | G2F manual. | none | none |
| Atari 8-bit MIC+PMG+RP+RP.INI RastaConverter | Partly handled | The `.mic` alone decodes (bitmap with its 4 colours), so the file opens. The raster program (`.rp`, per-line register writes) and PMG sprites are not applied. No `.rp` samples from RastaConverter in the corpus (`corpus/RainbowPainter.rp` is a C64 file). `help.txt` names the files but doesn't describe the `.rp` format. | [RastaConverter](https://github.com/ilmenit/RastaConverter) has no licence file: prose only, no code. | L | Defer. Needs RastaConverter output samples and reverse engineering of `.rp`, plus a display-kernel model. Covered by the "self-displaying XEX pictures" item in [gaps-ranking.md](gaps-ranking.md), #28. |
| Atari ST ART: Art Director, GFA Artist, Palette Master | Handled, one gap | Dispatch by size in `simple::decode_art`: 32000, 32032, 32512, 36864, 34360. 27 corpus `.ART` files match. GFA Artist "1000 colours on" (34360 B): RECOIL ignores the first header word, we require 4. Palette Master (36864): RECOIL rejects zero and random data that we accept, so we are laxer, not stricter. | [AFW Art Director](https://temlib.org/AtariForumWiki/index.php/Art_Director_file_format), [AFW GFA Artist](https://temlib.org/AtariForumWiki/index.php/GFA_Artist_file_format), [Palette Master](http://fileformats.archiveteam.org/wiki/Palette_Master). | XS | Optional: drop the `be16 == 4` check in `decode_gfa_artist_rasters` as RECOIL does (a header word 0 to 8 and 0xFFFF all render in RECOIL). Only worth it if a real file shows another value. |
| Atari ST DOO, ART Doodle | Handled | 32000 bytes, 8 `.DOO` and 7 32000-byte `.ART` match. Size sweep 31990 to 32140: same acceptance as RECOIL. | [AFW Doodle](https://temlib.org/AtariForumWiki/index.php/Doodle_file_format) | none | none |
| Atari ST NEO NEOchrome / NEOchrome Master | Handled | `.NEO` 32128 bytes (flag 0) and the 640x400 `0xBABE` canvas via `simple::decode_neo`. IFF form with a `RAST` chunk via signature (`FISH.neo`, matches). Sweep: same acceptance. | [AFW NEOchrome](https://temlib.org/AtariForumWiki/index.php/NEOchrome_file_format) | none | none |
| Atari ST NEO+RST | Handled | `discs.neo` + `discs.rst` through `rasters::with_rst`. Output differs with and without the sibling, matches RECOIL with it (oracle `+companions`). | AFW NEOchrome Master (`neochrom.txt`). | none | none |
| Atari ST CPT, CPT+HBL | Handled | `canvas::decode_cpt` takes the sibling `.HBL`. Corpus has pairs `foo`, `f_348`, `sunset`; output differs with and without the HBL and matches RECOIL. `.FUL` (HBL embedded) also handled. | Canvas 1.17 manual (cited in `docs/research/atari-st-tt-falcon.md`), layout otherwise derived from samples. | none | none |
| Atari ST OBJ ColorSTar / MonoSTar | Handled | `mono::decode_obj` handles both (palette text lines mean colour). 5 of 7 corpus OBJ match. `KOPF1.OBJ` and `SCHATZ.OBJ` (hostile/funpaint, header `005B 0083 FFDF`) are rejected by both RECOIL and us: they are a different, unidentified format, not a ColorSTar variant. | Derived from samples (no documentation). | none | none. The funpaint OBJ is beyond RECOIL; leave unless someone wants it. |
| Atari ST SPU Spectrum 512 and enhanced | Handled | `5BIT` header selects the 15-bit palette in `spectrum::decode_spu`. `enhanced.spu` (header `35424954`) matches RECOIL; 7 plain SPU match. | [AFW Spectrum 512](https://temlib.org/AtariForumWiki/index.php/Spectrum_512_file_format), [Enhanced](https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Enhanced_file_format). Bit positions observed from `recoil2png`. | none | none |
| Atari Falcon TRP EggPaint / Spooky Sprites | Handled | One decoder for `TRUP` and `tru?` (id, width, height, RGB565), and Pack-Ice packed files. 13 corpus TRP decode. The 5 packed ones RECOIL rejects are recorded divergences. | AFW EggPaint page (cited in the TRP divergence entries), Pack-Ice notes in the divergence file. | none | none |
| Plus4 P4I Botticelli, Multi Botticelli, 128x64 4-grey | Handled | Three variants by size and `MULT` tag at luminance offset `0x3FA`. 12 corpus P4I, 12 matched in the oracle, none diverge. | [GoDot Botticelli page](https://www.godot64.de/german/l_botticelli.htm), plus4world Multi Botticelli. 128x64 variant derived from `DCD.P4I`. | none | none |
| MSX2 SC5+S15, SC6+S16, SC7+S17, SC8+S18 interlaced | Handled | `screen::decode_bitmap_dump` looks for the `S1x` sibling. Corpus has all four pairs; our 512x424 output matches RECOIL's (pixel match in the oracle). `.GE5` + `.S15` pairs too, as RECOIL does. Without the sibling the plain page is shown. | [MSX2 Technical Handbook ch. 4](https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html), pairing from `recoil2png` output. | none | none |
| MSX2 SC8 under `.PIC` + S18 | Minor divergence | `FLOWER.SC8` copied to `t.pic` with `t.s18`: RECOIL shows 256x212 (no pairing), we show 512x424. No corpus pair exists, so the oracle never sees it. | Black box only. | XS | Decide: either skip pairing for `pic` or record a divergence if a pair turns up. Rare, probably neither. |
| MSX2+ SCA+S1A, SCC+S1C interlaced | Handled | Same code path (`Yae`, `Yjk` modes); corpus has `PIRATE.S1A`, `KITATEHA.S1C`, `MONSHIRO.S1C`, `UGUISU.S1C`; all match RECOIL. `.SCB` + `S1A` pairs as in RECOIL. | MSX2 Technical Handbook, `recoil2png` output. | none | none |
| MSX2 and MSX2+ single-screen rows (SC5/GE5, SC6, SC7/GE7, SC8/GE8/PIC, SCA/SCB, SCC/SRS/YJK) | Handled | All extensions are registered, `SRS`/`YJK` split between Graph Saurus and Screen 12. Rows only appear in "Check" because interlaced rows share the registry entry. | as above | none | none |
| MSX2 SRI+PL7 (listed under "Not covered", related) | Handled under another name | We register Graph Saurus interlaced as `SR0` (+`SR1`, `PL7`); RECOIL's `SRI` name has no sample anywhere. Two MSX-FAN sets match RECOIL page by page. | GSRLE readme (uniskie). | none | none |

## Gaps to act on

None of these deserves a task of its own. If someone wants one cleanup pass:

1. Accept 1024-byte `GR0`/`ASC` files (XS), but only with a real sample.
2. Relax the GFA Artist first-word check (XS), same condition.
3. Settle `.pic` + `S18` pairing (XS).
4. RastaConverter `.rp` stays deferred (L, no samples, no spec, no licence).

## Reporting fix

The "Check" table reports 36 non-C64 rows that need no work. The coverage
generator could count a row as covered when the registry entry's decoder
handles the variant through a companion or by size. That needs an alias
list per format, so I would only build it if people keep reading "Check" as
a to-do list.
