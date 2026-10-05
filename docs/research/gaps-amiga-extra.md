# Amiga gap survey: IFF variants and non-ILBM formats we do not decode yet

Research only, written 2026-10-05. No decoder code was written and nothing was committed.
This file extends [gaps-amiga-8bit.md](gaps-amiga-8bit.md) and [next-amiga-pc.md](next-amiga-pc.md) for the
Commodore Amiga family (OCS, ECS, AGA, plus third-party graphics cards, CD32/CDTV and AmigaOS-era formats).
It lists what is still missing, with evidence from real files where I could get them.

RECOIL is the baseline, not the ceiling. Some findings below are files RECOIL decodes and we reject; others are
files both reject.

## 0. How to read this file

Confidence tags:

- `fetched`: I read the primary source this session.
- `probed`: I measured it myself this session with a script, `recoil2png`, the release `retro-image` CLI
  or `ffmpeg` (all used as black boxes) on files fetched this session. Probe files and downloads were kept in
  the session scratchpad. Nothing was added to `corpus/`.
- `snippet`: from a search-result snippet or an index line only.
- `memory`: from my own recall, not checked.

Difficulty: XS (a few lines), S (a day or less), M (a few days, a codec or several variants), L (reverse
engineering or a large codec). "Oracle" means how a new decoder can be checked.

Caveats:

- The shared web-search budget (200 calls) was already used up by the team, so I made one search and did the
  rest with direct fetches (WebFetch and curl). Lists I could only get from search snippets are marked.
- `fileformats.archiveteam.org` refused WebFetch but answered curl at `justsolve.archiveteam.org` (CC0 text).
- One Aminet download made with a custom `User-Agent` came back mangled (5,654 bytes instead of 9,715). Plain
  `curl` returned the right bytes every time.
- The first row of each dexvert directory listing was dropped by my first parser, so early counts were one short.
  The counts in this file are the corrected ones.

### Existing docs that are stale on Amiga

- `gaps-amiga-8bit.md` lists RGBN/RGB8 and PowerPacker as gaps. Both are in the registry now
  (`amiga/rgbn.rs`, `codec/powerpacker.rs`, `codec/pack_ice.rs`). `gaps-ranking.md` rows 5 (PP20), 11 (RGBN/RGB8)
  and 23 (BBM) are done too.
- `amiga-apple-misc.md` says libilbm and amigazen/ifftools have unchecked licences. I checked: libilbm and
  libiff are MIT (`COPYING`, Sander van der Burg), amigazen/ifftools is BSD-2 (`LICENSE.md`).

## 1. What we decode today (checked against the source)

`crates/retro-image/src/platform/amiga.rs` dispatches `FORM` kinds `ILBM`, `BBM `, `PBM `, `ACBM`, `RGBN`, `RGB8`,
`DEEP`, `TVPP` and `ANIM` (first frame only), after one layer of PowerPacker or Pack-Ice. Plus AMOS banks (AmSp, AmIc,
Pac.Pic.), icons (classic, NewIcon, GlowIcon), DCTV, HAM-E, SHAM, DHR/DR/MP/BEAM (PCHG, SHAM, CTBL, BEAM), FLF.
ILBM handles compression 0, 1 and 2 (VDAT), HAM6, HAM8, EHB, 24 planes, masks as skipped planes, and CAMG hires,
lace and super-hires scaling. `Image` has no alpha channel (icon transparency is flattened to the Workbench grey).

RECOIL's Amiga list (formats page, `fetched`): ABK, ACBM, DEEP, FLF, HAM/HAM6, HAM8, IFF/256, INFO, LBM/ILBM,
DHR/DR/MP/BEAM, RGB8, RGBN, SHAM, DCTV, HAM-E, and "Atari ST/STE IFF". Nothing else, so every new format below
has no RECOIL oracle unless stated.

## 2. ILBM chunk and FORM table

Status: **supported** (read and used), **ignorable** (does not change the pixels we show), **unsupported**
(would change the picture or hides a picture). Counts are chunk counts in the 97 local FORM files (`corpus/`, 72 of them ILBM)
and in a header sample of 156 files from the scene.org Amiga mirror (see 6.2).

| Chunk | Where | Status | Notes |
|---|---|---|---|
| BMHD | all | supported | planes, masking, compression read. Ignored: x, y, transparentColor, aspect, page size (CAMG is used for scaling) |
| CMAP | ILBM | supported | 4-bit expansion rule as in the spec |
| CAMG | ILBM | supported, with gaps | HAM, EHB, lace, hires, super-hires, monitor ID read. Gaps: HAM flag on 5 or 7 planes, CAMG-less 6 planes (see 3.4, 3.5). DPF (0x400) and SPRITES (0x4000) ignored; no DPF in the 277 CAMG chunks seen |
| BODY / ABIT | ILBM, PBM / ACBM | supported | |
| BODY with VDAT | compression 2 | supported | Atari ST DeluxePaint |
| PCHG | ILBM | supported | 12-bit, 32-bit, Huffman. USE_ALPHA ignored. 3 corpus files |
| SHAM, CTBL, BEAM | ILBM | supported | corpus files: SHAM 2, CTBL 5, BEAM 2 |
| DYCP, MPCT | next to CTBL | ignorable | DYCP is 8 bytes (`00000001 00100000` in the one I dumped, `Seascape.dr`); MPCT is 4 zero bytes in both `.mp` files. CTBL carries the data |
| CRNG, CCRT, DRNG, BRNG | ILBM, PBM | ignorable | colour cycling. Chunk counts: CRNG 60 in the corpus and 422 in the scene header sample. BRNG is Brilliance's range chunk (seen in `SWERVE10`, `shotanim`) |
| GRAB, SPRT, DEST | ILBM | ignorable | GRAB: 1 chunk in the corpus, 6 in the scene sample. No SPRT or DEST seen |
| ANNO, AUTH, (c), NAME, TEXT, CHRS, FVER, UTF8, JUNK | any | ignorable | |
| DPI, DPPV, DPPS, DPXT, DPAN | ILBM | ignorable | DPaint and Digi-View chunks. DPPS (ILBM chunks: 7 in the corpus, 68 in the scene sample) and DPXT (2) are not in the registry |
| TINY, PRVW, XS24 | ILBM, PBM, DEEP | ignorable | thumbnails. TINY: 18 chunks in the 156-file scene sample. XS24 is a 24-bit thumbnail in some `.iff24` and DEEP files (Deark note) |
| CLUT | ILBM | unsupported, rare | intensity or colour LUT, "may be found in deep ILBMs" (spec). No sample seen |
| CMYK, CNAM, XBMI, EPSF, XSSL, 3DCM, 3DPA, VTAG, TMAP, HLID | ILBM | ignorable | registry names; no samples |
| DGVW | ILBM | ignorable | private NewTek DigiView chunk |
| BHBA, BHCP, BHSM | ILBM | ignorable | private Photon Paint chunks; BHCP and BHSM in 1 corpus file |
| IMRT | RGBN, RGB8 | ignorable | in 5 of 14 Impulse files, meaning unknown |
| ICCP, GAMA, CHRM, SRGB | ILBM | ignorable | ILBM64 colour-management chunks (Kleinert); no sample seen |
| PLTP | ILBM | unsupported | 16-plane variant, see 3.7 |
| ANHD, DLTA, DPAN, ANSQ | ANIM frames | first frame only | ANHD of frame 1 ignored |
| RAST | NEOchrome Master | handled in Atari ST | |

FORM kinds beyond the ones we dispatch:

| FORM | Status | Notes |
|---|---|---|
| `DPST` | unsupported, RECOIL decodes the first frame | 3.3 |
| `RGFX` | unsupported | 4.1 |
| `YAFA` | unsupported | 4.3 |
| `YUVN` | unsupported | spec `fetched`, no sample found, 4.8 |
| `FAXX` | unsupported | needs a G3/G4 decoder, already in `gaps-amiga-8bit.md` |
| `ANBM` | unsupported | `LIST ILBM` with `PROP`. No sample found, 4.9 |
| `CHBM`, `RGB4`, `RGBX`, `AHAM`, `DCTV`, `DCCL`, `DCPA`, `PLBM` | unsupported | registry names with no public spec. `DCTV` ILBMs are handled; the FORM `DCTV` is not |
| `SSA `, `VAXL`, `FANT`, `FILM`, `MOVI`, `VDEO`, `ROXN` | unsupported | animation or movie FORMs, see section 5 |
| `CAT `, `LIST` at top level | unsupported | no real picture sample found in the 280 or so files looked at |

## 3. ILBM-family gaps that real files show

These are small fixes in code we already have. Each was found by running real or synthetic files through
`recoil2png` and the release `retro-image` CLI. Sample names are from the dexvert sample set unless noted.

### 3.1 FORM length shorter than the file (XS, RECOIL decodes) `probed`

`iff::form` clamps the walk to the FORM length in the header. `cover` and `map1` (both ACBM, 320x256, 4 planes, no
CAMG; `iffILBM/` in the dexvert set) declare FORM length 40988 but the file holds 41056 bytes after the 8-byte
header. The ABIT chunk is 40960 bytes and ends 68 bytes past the declared end, so it is cut and the file is
rejected. `recoil2png` decodes both. Patching only the FORM length to `file size - 8` makes our CLI decode both
pixel-identical to RECOIL. Two of the 127 files in that set. Probably a writer bug of the time, but the files are
valid otherwise. Fix: when the declared length is shorter than the data and the last chunk is cut off, keep going
to the end of the file.

### 3.2 ANIM with a chunk before the first frame (XS, RECOIL rejects) `probed`

`decode_form` for `ANIM` takes only the first child and requires it to be `FORM`. `shotanim` (iffANIM) is
"Written by Brilliance 1.0" and starts with an `ANNO` chunk, then the first `FORM ILBM` (128x64, 5 planes, DPAN,
BRNG chunks, valid BODY of 5120 bytes). RECOIL rejects the file too. Both decoders decode the extracted `FORM ILBM`.
Fix: skip non-FORM children until the first `FORM ILBM`. Other files written by Brilliance 1.0 may start the same way;
I have one sample.

### 3.3 `FORM DPST`: DeluxePaint ST animation, first frame (XS, RECOIL decodes) `probed`

`abydos.dpst.iff` (iffANIM dir): `FORM DPST`, a 24-byte `DPAH` chunk (`0006 0010` then zeros), a full `FORM ILBM`
(320x200, 4 planes, BMHD compression 2 = VDAT, pad1 = 7, four CRNG) and then sixteen `FORM VDLT` frames, each with
four `ADAT` chunks (one per plane); the last one ends exactly at the end of the file. `recoil2png` shows the first frame
(its format list has "Atari ST/STE IFF"). We reject the FORM type. Cutting the inner `FORM ILBM` out and decoding it with
our CLI gives a result pixel-identical to RECOIL's (diff bounding box empty). `vdat.rs` already exists, so dispatching
`DPST` to its first child is all that is missing. No public spec; layout from this one sample. `next-blocked.md` ("Atari ST
IFF") says every IFF in the corpus decodes; this file is not in the corpus, so that claim still holds for what was tested.

### 3.4 CAMG-less 6-plane pictures: the 16/32 colour rule (XS, RECOIL differs from us) `probed`

The ILBM spec appendix says "if no CAMG chunk is present and the image is 6 planes deep, assume HAM". Just Solve
says 16 colours means HAM6. We use `colors <= 32` for EHB and HAM for every other count. I took `flag_b24.iff` (6 planes,
HAM, 16 colours), removed CAMG and resized CMAP, and compared the `recoil2png` and CLI outputs with the same file
rendered with each explicit CAMG:

| CMAP entries, no CAMG | RECOIL draws | We draw |
|---|---|---|
| 16 | HAM6 | EHB (differs) |
| 17, 24 | plain 6-plane indexed | EHB (differs) |
| 32 | EHB | EHB (same) |
| 33, 64 | plain 6-plane indexed | HAM6 (differs) |

Rule to adopt: 16 colours is HAM6, 32 is EHB, anything else is indexed. Rare in practice: 1 CAMG-less 6-plane file
in the 17 local 6-plane files (an EHB test file, which we already get right) and 0 in 19 scene files. Fix size XS.

### 3.5 HAM flag on 5 or 7 planes (XS, RECOIL differs from us) `probed`

`Mode::detect` treats HAM only for 6 and 8 planes; a HAM-flagged 5- or 7-plane file falls back to indexed colours.
Just Solve (`fetched`) says HAM6 files have "6 planes (rarely 5)" and HAM8 "8 planes (rarely 7)", and Deark (`ilbm.c`, MIT)
decodes 5 as HAM6 and 7 as HAM8. `recoil2png` does too. I set CAMG 0x800 on `AH_Eye.iff` (5 planes) and
`Bookworm_7plane.ilbm` (7 planes, both in `corpus/`) and fitted the output:

- 5 planes = HAM6 with the missing top plane read as 0, so control is `v >> 4` (only 0 or 1) and data is `(v & 15) * 17`. Exact
  match, 0 differing pixels.
- 7 planes = HAM8 with the missing top plane read as 0, control `v >> 6`, data `v & 63` expanded as in HAM8. Exact match.

No real HAM5 or HAM7 file was found (0 in 277 CAMG chunks), so this is correctness work, and nothing I saw needs it today.

### 3.6 What I checked and found fine

- 70 more random `.iff`/`.lbm` files (3.7 MB) from the scene.org Amiga mirror: accepted by both decoders, 70 of 70
  (acceptance only; I did not compare pixels).
- Ten of the 15 files in the dexvert `iffANIM` set (the rest are the two failing ones above, plus three I did not test)
  show the same first frame in both decoders.
- Header scan of 156 scene files: ILBM 141, PBM 15; planes 3-8 and 24 only (7 planes in 4%); BMHD masking 2 in 12%
  (DPaint brushes and stencils; we skip the colour key); compression 0 or 1 only; no CAT or LIST; no 12/16/21/32 plane files.
- DCTV: `batbabe.dct` and `vase.dct` decode in both. The unexplored DCTV variants stay as in `amiga-apple-misc.md`.
- Scaling for double-scan monitors (DblNTSC 0x91000, DblPAL 0xA1000) and Super72 uses CAMG only; BMHD aspect is not
  consulted. Just Solve notes the aspect field is wrong in a significant minority of files, so CAMG is a defensible choice.
  Not a decoding gap; RECOIL parity is the check.

### 3.7 Unsupported variants seen in real files

| File(s) | What | RECOIL | Status |
|---|---|---|---|
| `rainbow2`, `spicy`, `spir`, `woodpan` (iffILBM; 752x480 and 752x888) | 16-plane ILBM, BMHD compression 1, `PLTP` chunk of 32 bytes: 16 `(type, bit)` byte pairs `06 07, 06 06 ... 06 00, 07 07 ... 07 00`. No CMAP, no CAMG | rejects | L, unidentified. `probed`: with planes 0-7 as one byte (plane 0 the MSB, as the PLTP pairs suggest) and planes 8-15 as another, both bytes give the same swirl texture, one pixel apart in the top row, and the values oscillate with a period of about 4 pixels. The picture is not a plain 16-bit RGB word. It looks like an encoded or sampled signal, in the spirit of DCTV. I found no documentation for `PLTP`. The 752x480 size matches a digital NTSC full frame, but that is only a guess about where the files came from |
| `data1.fcy`, `data2.fcy` | 14 concatenated `FCY!`-tagged ILBMs: the `FORM` id replaced by `FCY!`, length field as normal. Replacing `FCY!` by `FORM` in the first gives a normal 640x512 ILBM that RECOIL and our CLI decode | rejects | game data, not worth a format. Mention only |
| `4.pal`, `Game2.pal`, `Red_Green16.pal` | palette-only ILBMs (no BODY; one has no BMHD; `Game2.pal` has a 384-entry CMAP with DRNG and BRNG) | rejects | XS if we want to show a swatch grid. Design decision, not a decoder gap. The spec calls nPlanes = 0 with a CMAP "the recommended way to store a color map" |
| deep ILBM with 12, 15, 21, 32, 48 or 64 planes | spec'd variants (ILBM spec appendix: 12 and 24; 21-plane NewTek order `R7 G7 B7 R6 ...`; ILBM64: 32 = RGBA, 48 = RGB16, 64 = RGBA16, `mskHasAlpha`) | not checked | S each; `fetched`. No real sample seen in the 280 or so files looked at, so frequency is unknown |

Sources: ILBM spec <https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap> (`fetched`), ILBM64 by Andreas
Kleinert <https://aminet.net/docs/misc/ILBM64.readme> (`fetched`, v1.2, 2010-01-07), Just Solve ILBM
<http://justsolve.archiveteam.org/wiki/ILBM> (`fetched`, CC0).

## 4. Formats we do not decode yet

### 4.1 IFF-RGFX (`FORM RGFX`)

| | |
|---|---|
| Magic, extensions | `FORM....RGFX`; `.rgfx`, `.rgx` (the samples have no extension) |
| Pixels | `RGHD` header (13 big-endian longs: edges, size, page, depth, pixel bits, bytes per line, compression, aspect, bitmap type), `RSCM` view mode (replaces CAMG), `RCOL` 256 x RGB palette plus transparent colour, optional `RTRN` per-colour alpha, `RBOD` body. Bitmap types: unaligned planar 8, chunky 8, RGB15, ARGB16, RGB24, ARGB32, RGB48, ARGB64, float RGB96, float ARGB128. Compression: 0 none, 1 XPK, 2 zlib (`RBOD` starts with a 4-byte uncompressed size) |
| Spec | Andreas R. Kleinert, Aminet `dev/misc/IFF-RGFX` v4.1 (2012-08-31): `IFF-RGFX.readme`, `RGFX-Remarks.txt`, `RGFX-Chunks.txt`, `rgfx.h`. `fetched` <https://aminet.net/dev/misc/IFF-RGFX.zip>. Also <http://justsolve.archiveteam.org/wiki/RGFX> |
| Licence | `rgfx.h` says "Freeware. All Rights Reserved". It holds struct definitions and comments only. The readme tells readers to use `rgfx.h` as the chunk spec, so I read it. Decision for the lead: treat it as spec prose, or limit work to the three text files plus the samples |
| Samples | dexvert `image/rgfx/` 16 files (`1-shot`, `_belanna`, `_kira`, `crew`, `boundless_WB2.rgfx` and so on). I read the header of all 16 (`probed`): 15 are 160x128 chunky 8-bit, `boundless_WB2.rgfx` is 800x600 RGB24; all 16 use compression 1 (XPK): 15 MASH, 1 NUKE. So the container is only useful with an XPK layer |
| Reference | XPK container and MASH, NUKE decompressors: Ancient (BSD-2), `src/XPKMain.cpp` (11 KB), `src/MASHDecompressor.cpp` (3 KB), `src/NUKEDecompressor.cpp` (3 KB). Sizes only; I did not read the code. Deark has "very limited" RGFX support (Just Solve) |
| XPK header | `XPKF` at 0, packed length at 4, sub-packer id at 8, unpacked length at 12, as in the samples (`58504b46 000010dc 4d415348 00005000` is `XPKF`, 4316, `MASH`, 20480 = 160 x 128) `probed` |
| Size | S for the container, M with an XPK layer (MASH, NUKE) |
| Oracle | none. Check by eye against the Star Trek crew portraits in the dexvert set. The raw and zlib forms would need synthetic files |
| Relevance | medium: the format of the late-1990s OS3 and AROS RTG era; XPK was already deprecated by its author. The XPK layer is shared with YAFA and SGX/SVG (4.2, 4.3) |

### 4.2 SGX / SVG: SuperView Graphics (Andreas Kleinert)

| | |
|---|---|
| Magic, extensions | 18 bytes `SGX Graphics File` or `SVG Graphics File` plus a NUL; `.sgx`, `.svg` (not W3C SVG). Our registry already has an `SGX` row for the SymbOS format on Amstrad CPC, so this needs the signature gate |
| Pixels | header then data. `GfxDataOffset` points at the data. Bits and planes: 1 bit x 1-8 planes (unaligned planar, <= 256 colours), 8 bit chunky, 24 RGB, 32 RGBA, 48, 64. 256 x RGB palette in the header (8-bit values). Data is raw, `LZ77` + 4-byte length + a zlib stream (SGX), or `XPK`/`PP20` (SVG, deprecated) |
| Spec | `SGX-Specs.lha` on Aminet `docs/misc` (FormatSpecs v4.2, 2009-04-06). `fetched` <https://aminet.net/docs/misc/SGX-Specs.lha>. Just Solve <http://justsolve.archiveteam.org/wiki/SGX> |
| Correction | the offsets in the printed spec are two bytes too high. `probed` on `testimg.sgx`: version word at 18, `GfxDataOffset` at 20, left 24, top 28, width 32, height 36, colour depth 40, `ViewMode32` 44, pixel bits 48, planes 49, bytes per line 50, colour map 54 (768 bytes). `GfxDataOffset` is 822 = 54 + 768 |
| Samples | dexvert `image/sgx/`, four files, all fetched: `testimg.sgx` (288x442, 8-bit, raw), `testimg-lz77.sgx` (same image, LZ77), `abydos.rlen.svg` (800x600, 32-bit, XPK RLEN) and `NUKE.SVG` (640x512, 8-bit chunky, 16 colours, ViewMode 0xA9004, XPK). The raw file decodes to a clean photo of an orange flower when read as palette plus chunky bytes (`probed`), and the zlib payload of the LZ77 file is byte-identical to the raw one |
| Reference | `abydos` is GPL and listed on Just Solve: do not read. `akSGX-dt` (Aminet `util/dtype`) is a datatype binary |
| Size | S (the `codec/inflate.rs` zlib code exists). The XPK `.svg` variant needs the XPK layer |
| Oracle | the two `testimg` files must match each other. Otherwise by eye |
| Relevance | low-medium. It is the predecessor of RGFX and rare, but complete and small, and the spec is first-hand |

### 4.3 YAFA animation (`FORM YAFA`), first frame

| | |
|---|---|
| Magic | `FORM....YAFA`; `.yafa` |
| Pixels | `INFO` 7 words (width, height, depth, speed, frames, frame type 0 planar / 1 planar XPK / 3 chunky XPK / 4 chunky, flags: HAM, palette per frame, delta type), `DRGB` a `LoadRGB32` table, optional `PROF`, `TTBL`, `DLTX`, `ANNO`, then `BODY` with the frames. The first two frames of a delta-compressed file are stored uncompressed |
| Spec | Michael Henke (WK-Artworks and Infect), "YAFA-doc" v1.0, 1996-05-26, Aminet `docs/misc/YAFA-doc.lha`. `fetched` |
| Samples | dexvert `video/iffYAFA/` 13 files. Aminet `demo/tp95` has about 35 DATAWORLD `.yafa` files (2-6 MB), `pix/anim` `creamed.lha` and `wuerfel.lha`, player `gfx/show/yp.lha` (`snippet`, listing seen). From the two YAFA heads I read: `a.yafa` is 80x64, 5 planes, 36 frames, planar with the delta flag set; `t15.yafa` is 176x140, 6 planes, 60 frames, frame type 3 (chunky XPK) |
| Size | S for frame type 0 and 4, M once the XPK layer exists |
| Oracle | none; by eye |
| Relevance | medium for a demoscene archive: YAFA is how many 1995-98 scene animations ship |

### 4.4 CDXL (Commodore CD-ROM video), first frame

| | |
|---|---|
| Magic | none. 32-byte frame header repeated per frame; `.cdxl`, `.xl` |
| Layout | `fetched` MultimediaWiki <https://wiki.multimedia.cx/index.php/CDXL>: type byte, info byte (bits 0-2 encoding RGB / HAM / YUV / AVM-DCTV; bit 3 stereo; bits 5-7 plane arrangement: bit planar, byte planar, chunky, bit line, byte line), current and previous chunk size, frame number, width, height, planes, palette size in bytes, sound size, then the 12-bit palette words, video, 8-bit audio |
| Probe | `probed`: for `Fruit.CDXL` (128x80x4) and `callnow.xl` (176x110x6, HAM) the video size (chunk size - 32 - palette - sound) equals the planar size computed from the header. For `Maku.XL` (176x128x8, frame number 0) it is 24 bytes larger, unexplained. `optologo.cdxl` (info 0x81, bit-line arrangement, "planes" word 0x0108) does not fit the page's field layout. The page is community-written, so expect to need the samples |
| Samples | dexvert `video/cdxl/` 10 files; FFmpeg samples <https://samples.mplayerhq.hu/cdxl/> (index reachable, contents not listed here) |
| Reference | none permissive found; FFmpeg's decoder is LGPL: do not read, run only |
| Oracle | `ffmpeg -i file.cdxl -frames:v 1` works (`probed`: 176x110 and 128x80 PNGs from two samples). It is a black-box oracle only |
| Size | S-M (a header scan and three arrangements, HAM reuses our HAM6) |
| Relevance | medium for CD32 and CDTV collections; the first frame is a thumbnail of a video. Headerless, so extension plus a header sanity check |

### 4.5 Amiga bitmap fonts (`.font` plus one file per size)

| | |
|---|---|
| Magic | `.font` starts `0F 00` (FCH_ID) or `0F 02` (TFCH_ID), then an entry count and one 260-byte `FontContents` record per size (256-byte path, YSize, style, flags). `emerald.font` is 524 bytes = 4 + 2 x 260. Each size is a file named after the size inside a directory named after the font; it begins with a hunk header `00 00 03 F3`. `probed` on `emerald.font` and one size file |
| Layout | `fetched` AmigaOS wiki, Graphics Library and Text, "Composition of a Bitmap Font on Disk" <https://wiki.amigaos.net/wiki/Graphics_Library_and_Text>: FontContentsHeader, FontContents, TFontContents; each size file is a `DiskFontHeader` hidden inside a loadable hunk; `TextFont` with `tf_CharData`, `tf_CharLoc`, `tf_CharSpace`, `tf_CharKern`; ColorFonts (`FSF_COLORFONT`, `ColorTextFont` with up to 8 planes and a 12-bit colour table) |
| Samples | dexvert `font/amigaBitmapFont/` 13 fonts with their sizes, `amigaBitmapFontContent/` 10 size files; Aminet `text/bfont` and `text/font` (listing only) |
| Reference | monobit (MIT) lists "Amiga font" (`amiga`) and "Amiga Font Contents" (`amiga-fc`) in its README; I did not locate the source file in this pass |
| Size | M. Rendering a glyph sheet needs hunk parsing plus the loc, space and kern tables. The pointers inside `TextFont` are offsets that the DOS loader relocates (`memory`: not checked on a real file). The size file decodes alone; the `.font` file only lists sizes |
| Relevance | medium. Fonts are on every Workbench disk; the registry already has fonts for other platforms, so the scope question is settled. One decision: sheet layout and the sample text |
| RECOIL | no |

### 4.6 Remaining packers around pictures

Status after this survey: PowerPacker (PP20) and Pack-Ice are done. Not done: Imploder (`IMP!`, `ATN!`, `CHFI`, `M.H.`...),
Crunch-Mania (`CrM!`, `Crm!`, `CrM2`, `Crm2`), Rob Northen RNC (`RNC\1`, `RNC\2`), StoneCracker (`S404`...), ByteKiller
clones, the footer-form Pack-Ice (`next-amiga-pc.md` 3.2). Ancient (BSD-2, `fetched` README list) covers all of them.
Evidence for frequency stays as in `next-amiga-pc.md` 3.1: 9 pictures among 128 packer samples. In the local corpus the
packers appear as `ICE!` 17, `PP20` 16, `RNC1` 12, `CrM2` 10, `IMP!` 8 files, mostly not pictures. The 156-file scene header
sample had no packed pictures at all: scene pictures were stored as plain `.iff` inside LhA archives. Rank stays low and
M per packer. An XPK layer (4.1, 4.3) is a new shared dependency; XPK-wrapped plain ILBM files are plausible
(`XPKF` at offset 0) but I found none (`memory`).

### 4.7 CFAST: Disney Animation Studio (`.cft`), first frame

Community spec on Just Solve (`fetched`, CC0): `GUCF`, `LOCK` or `STDY` signature (the latter two followed by a text),
image size, screen size, plane count (1-5), two colours, an extra block, frame count, then per frame, per plane a size and
run-length-coded data, palette and cycle ranges. `probed` on `PATH.CFT` (320x200, 1 plane, 8 frames): the header fields
line up. The plane run-length code is not specified, and the `.sec` files (14 dexvert samples in the directory, `.cft` and
`.sec` together) start with a different header (`SSFFANM0`...) that nothing documents. M, needs reverse engineering. Rank
low.

### 4.8 YUVN (`FORM YUVN`)

Complete spec `fetched` from the AmigaOS wiki <https://wiki.amigaos.net/wiki/YUVN_IFF_YUV_Image_Data> (MacroSystem, 1992):
`YCHD`, then `DATY`, `DATU`, `DATV`; modes 400, 411, 422, 444 and the lores 200, 211, 222; CCIR-601 ranges (Y 16-235,
U and V 16-240), no compression defined. Samples: none found. The Aminet `YUVNdt.lha` carries a viewer and no pictures.
`ycbcrdatatype.lha` is a different thing: TBCPlus 24-bit YCrCb dumps. S, but untestable with real data. Rank low.

### 4.9 Containers: LIST, PROP, CAT, ANBM

The EA IFF standard lets a `LIST ILBM` share `BMHD` and `CMAP` through a `PROP ILBM`. `ANBM` (Deluxe Video,
spec `fetched` <https://wiki.amigaos.net/wiki/ANBM_IFF_Animated_Bitmap>) uses exactly that: `FORM ANBM`, `FSQN`, then
`LIST ILBM` with a `PROP` and one `FORM ILBM` per frame. libilbm (MIT) documents extracting several ILBMs from one file. I
found no real file with a top-level `CAT`/`LIST` of pictures: the dexvert `iffCAT` samples are `CAT EFCT` effect data, not
pictures. S when a sample turns up. Rank low.

### 4.10 Odd one: bare AMOS picture blocks

dexvert `image/amosPicturePacker/` (11 files, mostly `.Bin`) begin with the picture id `06 07 19 63` and no `AmBk` bank header or screen
header (`probed`: two heads). `amiga/pac_pic.rs` decodes the same block inside an `AmBk`. Whether the palette is
in the file, and where, is unknown. XS-S, unverified.

## 5. Rejected, or nothing to do

| Candidate | Why |
|---|---|
| Imagine texture `.itx`, Imagine objects (TDDD, `.iob`), Turbo Silver objects | Imagine's texture format is an executable procedural module (the 3.0 texture document is a C source and a Phar Lap `.REX` build recipe: `fetched`, textfiles.com `t3d_doc2.txt`). Objects are 3D. RGBN and RGB8 pictures are done |
| Sculpt-Animate 4D, Sculpt 3D (`SC3D`), Real 3D, LightWave objects, Pixel 3D | 3D scene data. Their pictures are ILBM, RGB8 or 24-bit ILBM. `SC3D` is private (registry) |
| DR2D, Amiga Metafile (`AMFF`), DrawStudio (`DSDR`), Professional Draw clips, PageStream and ProPage documents, IntelliFont and other outline fonts | vector or document formats; no renderer in scope. 7-11 dexvert samples exist for each |
| FANT (Fantavision movies) | polygon animation; the spec is complete (`fetched`) but there is no still picture to show |
| PMBC (Black Belt Systems, ImageMaster) | the registry page is a 1991 announcement ("the technology will remain proprietary"), no layout (`fetched`) |
| IFF-SSA (ClariSSA), VAXL, FILM, MovieSetter, Magic Lantern DIFF, Video Toaster `.rtv`, Deluxe Video `VDEO` | animation or movie formats with no public layout found. dexvert has 19 SSA, 18 VAXL, 7 DIFF and 3 RTV samples. Heads read: SSA is `FORM SSA ` + `ANIM` + `FORM DLTA` (`DSCR`, `COST`, `BEST`, `DAST`); VAXL has `VXHD`, `TMCD`, `COLS`, `BMAP`, `SAMP`. Layout of the pixel chunks is unknown |
| Video Master `.flm`, `.vid`, `.vsq` (Microdeal, Atari ST and Amiga) | Just Solve has a header spec (`VMAS`, 16 palette words and 8000 bytes per frame). 10 dexvert samples. Belongs to the Atari researcher; mentioned so it is not lost |
| Digi-View, Digi-Paint, Photon Paint, DeluxePaint brushes and stencils, ADPro, XiPaint, Brilliance, Personal Paint, Opal | save IFF ILBM, ANIM or DEEP; only private chunks differ (DGVW, BHBA, BHCP, BHSM, DPXT, BRNG), all ignorable. Digi-View's own deep order is the 21-plane NewTek ILBM of 3.7 |
| Scala (EX, BRD), Vista, Video Toaster framestores and Toaster Paint | no public layout found, no samples (`memory` for the Toaster) |
| Blitz Basic and AmigaBASIC shape or object files | no spec found (search budget gone). `memory`: Blitz saves IFF brushes; AmigaBASIC `GET`/`PUT` objects are a small header plus planes. Already tracked in `gaps-amiga-8bit.md` |
| Imagine / Turbo Silver `RGBN`, `RGB8` | done |
| `debox.library` pictures (CDTV and CD32 Kickstart), F1GP `.bkg`, Digital Almanac II `.map`/`.map24`, TBCPlus YCrCb, KlondikeAGA "REKO" card sets | datatype-only formats (Aminet `util/dtype` readmes `fetched`); no format document, only binary datatypes. Single-title or single-card niches |
| QRT / POV-Ray dump (`QRT_DT`) | generic raytracer output, not Amiga-specific |
| Hardware sprite dumps, copper-list-driven formats, raw bitplane dumps | headerless; not formats until a tool defines them. The per-line palette chunks already cover the copper cases |
| OS4 and AROS PNG icons (`ARGB` chunk) | still unspecified. The Stöcker document has no OS4 section (`fetched`); the OS4 and AROS sources are not permissive. Stays as in `gaps-amiga-8bit.md` |
| Amiga GL | GRASP GL animation with 4 extra bytes; 1 sample in an LZH. Nothing to add |

## 6. Sample sources

Samples belong to their artists and authors. They go only into the git-ignored corpus with a MANIFEST. Nothing was
added to `corpus/` in this pass.

### 6.1 dexvert sample set, <https://sembiance.com/fileFormatSamples/>

An open directory index (Sembiance). The dexvert project itself has no licence statement on GitHub (SPDX
`NOASSERTION`), so I used only its directory names and no code. `fetched`. About 125 files in full (counting the
scene.org ones in 6.2) and about 210 header ranges in total this session, spaced out.

| Directory | Files | Note |
|---|---:|---|
| `image/iffILBM` | 127 | about half already in `corpus/` by name. Found the 3.1 and 3.7 cases here |
| `video/iffANIM` | 15 | `shotanim` (3.2), `abydos.dpst.iff` (3.3). Nine others tested decode the first frame in both decoders |
| `image/rgfx` | 16 | 16 of 16 XPK |
| `image/sgx` | 4 | two raw or zlib, two XPK |
| `video/iffYAFA`, `iffSSA`, `iffVAXL` | 13, 19, 18 | large (up to 20 MB); I read heads only |
| `video/cdxl` | 10 | sizes from 0.7 to 15 MB |
| `video/disneyCFAST` | 14 | `.cft` and `.sec` |
| `font/amigaBitmapFont`, `amigaBitmapFontContent` | 13 fonts, 10 size files | |
| `image/amosPicturePacker` | 11 | 4.10 |
| `archive/xpk` | 65 | mostly modules and data; not looked at |

### 6.2 scene.org Amiga mirror

<https://ftp.scene.org/pub/mirrors/amigascne/>, with the file list `amigascne-index.txt` (5.9 MB, one download). `fetched`.
`Gfx/` holds 10,019 files: 2,853 `.iff`, 1,387 `.lbm`, 2 `.ilbm`, 758 GIF, 3,107 PNG, 297 JPEG, 54 LhA. This is the best
real-world frequency source for scene art. I read the first 2 KB of 156 random `.iff`/`.lbm`/odd-name files and fetched
70 more in full (3.7 MB). Pictures are copyrighted scene art. `Groups/` (55,000 files), `Packdisks/` (11,600) and
`Slideshows/` are also there and I did not look into them. `ftp.back2roots.org` from Just Solve's list is down (HTTP 526 on https).

### 6.3 Others

- Aminet (<https://aminet.net/>): readmes and docs `fetched`. Picture directories `pix/*`, `gfx/*`, `demo/tp95` (YAFA),
  `text/bfont`, `text/font`, `util/dtype` (about 230 datatypes listed by file name). Search by name works at
  `aminet.net/search?query=`; downloads work with a plain `curl`.
- Amiga Graphics Archive <https://amiga.lychesis.net/>: reachable, a curated gallery of screenshots and logos. Good for checking a
  decode by eye, but not original files.
- archive.org "DCTV + RGB Converter" (already in `amiga-apple-misc.md`).

## 7. Ranked top 15

Ranking weighs: real files that fail today, RECOIL parity, size of the fix, and spec and sample quality.

| # | Item | Size | RECOIL | Why |
|---|---|---|---|---|
| 1 | Keep walking past a short FORM length (3.1) | XS | decodes | 2 of 127 files fail, valid otherwise, fix is a few lines |
| 2 | ANIM: skip chunks before the first `FORM ILBM` (3.2) | XS | rejects | files written by Brilliance 1.0 (one sample), beyond RECOIL |
| 3 | `FORM DPST` first frame (3.3) | XS | decodes | closes the "Atari ST/STE IFF" coverage row; `vdat.rs` exists |
| 4 | CAMG-less 6 planes: 16 is HAM6, 32 is EHB, else indexed (3.4) | XS | differs | spec and RECOIL agree; we are wrong for five CMAP sizes |
| 5 | HAM flag on 5 and 7 planes (3.5) | XS | differs | exact fit measured; rare but spec'd by Just Solve |
| 6 | SGX / SVG SuperView Graphics (4.2) | S | no | complete first-hand spec, zlib already in the crate, four real samples, a self-check pair |
| 7 | IFF-RGFX with an XPK layer: MASH and NUKE (4.1) | M | no | 16 of 16 samples need XPK; the XPK layer also helps 6 (two `.svg` files) and 9; spec is first-hand |
| 8 | Amiga bitmap fonts (4.5) | M | no | primary spec, samples on Aminet and dexvert, fonts are in scope |
| 9 | YAFA first frame (4.3) | S, M with XPK | no | first-hand spec; many scene files; unpacked frames are easy |
| 10 | CDXL first frame (4.4) | S-M | no | `ffmpeg` is an oracle, 10 samples, CD32 collections |
| 11 | Remaining packers: Imploder, Crunch-Mania, RNC, StoneCracker, ByteKiller, Pack-Ice footer (4.6) | M each | no | Ancient is BSD-2; frequency on pictures is low |
| 12 | Deep ILBM plane variants: 12, 21 (NewTek), 32, 48, 64 planes (3.7) | S each | not checked | spec'd, but no real sample found |
| 13 | Palette-only ILBM as a swatch grid (3.7) | XS | rejects | 3 samples; design decision |
| 14 | CFAST `.cft` first frame (4.7) | M | no | 14 samples, the plane RLE needs reverse engineering |
| 15 | 16-plane `PLTP` ILBM (3.7) | L | rejects | 4 real samples, layout unknown |

Below the cut: YUVN (spec but no samples), `LIST`/`PROP`/`ANBM` (spec, no samples), bare AMOS pictures (unverified), `FCY!`
files, OS4 and AROS icons (blocked).

## 8. Validation notes

- Items 1 to 5 have an oracle: `recoil2png` (items 1, 3, 4, 5) and the corpus oracle test. For item 2, RECOIL fails, so record a
  divergence with the evidence in 3.2 (`shotanim` and its extracted frame).
- Items 6 to 9 have no RECOIL oracle. SGX has the raw-versus-LZ77 pair; RGFX and YAFA only by eye against known pictures;
  CDXL has `ffmpeg`.
- `ffmpeg` also decodes many plain ILBMs (HAM, EHB, PBM, RGB8, RGBN) and is a second black-box check.

## 9. Sources read

Permissive, read: Deark (MIT) `formats.txt` and parts of `modules/ilbm.c`, for chunk and variant coverage only; libilbm
`README.md` and `COPYING` (MIT); Ancient (BSD-2) README and file sizes, no decoder code; monobit (MIT) README; Just Solve (CC0
text); the AmigaOS wiki (public documentation). Licence checked, nothing read: libiff (MIT), amigazen/ifftools (BSD-2),
bitplane/datatypes (MIT), oxideav-iff (MIT).

Prose docs read: Kleinert's `SGX-Specs`, `IFF-RGFX` text files and `ILBM64`, Henke's `YAFA-doc`, MacroSystem's YUVN text, the
Impulse 3.0 texture note, the MultimediaWiki CDXL page, Aminet datatype readmes.

To avoid (code): RECOIL in any form; abydos (GPL, SGX, AMFF, ILBM); FFmpeg decoders (LGPL, run the binary only); the OS4 and AROS
icon libraries; dexvert (no licence); `rgfx.h` only if the lead decides it is code and not spec (4.1).

## 10. Decisions and open questions for the lead

1. `rgfx.h`: spec or code? (4.1)
2. Animations and movies (YAFA, CDXL, CFAST, ANIM): show the first frame only, as for ANIM today. Agree?
3. Palette-only ILBMs: show a swatch or leave them unrecognised.
4. `.sgx` gets a second decoder under another platform. Keep the CPC row and add an Amiga row with a signature, or merge them?
5. The XPK layer lives in `codec/` next to `powerpacker.rs`, like the other depackers.
