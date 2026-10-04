# Next batch: Amiga packers, PC raster, Apple, Atari ST/8-bit pictures

Research only, written 2026-10-04. No decoder code was written and nothing was committed.
Scope from the lead: GIF and BMP are in, then Amiga packers around pictures, Autodesk
Animator, Dr. Halo, PCPaint/PICtor, ColoRIX, Fractint, DreamGrafix and packed Apple II
Hi-Res/DHR, STOS Picture Packer, Signum IMC, Cyber Paint SEQ, raw ST dumps and RastaConverter
XEX pictures. Read first: [CLEANROOM.md](../../CLEANROOM.md), [README.md](README.md),
[amiga-apple-misc.md](amiga-apple-misc.md), [gaps-amiga-8bit.md](gaps-amiga-8bit.md),
[gaps-pc-japan.md](gaps-pc-japan.md), [gaps-atari.md](gaps-atari.md),
[gaps-ranking.md](gaps-ranking.md), [../adding-a-format.md](../adding-a-format.md).

Clean-room position: no RECOIL source and no GPL/LGPL decoder code was read. Code that was read
is BSD-2 (Ancient, only its headers and detection lines), MIT (Deark module list and licence) or
Apache-2.0 (CiderPress II notes and file headers). Everything else is prose specs.

## 1. Registry check (what is already done)

Checked with `retro-image --list-formats`, `rg` over `crates/retro-image/src` and a run of the
release CLI over every new sample.

| Item | Status | Evidence |
|---|---|---|
| PCX, Targa | done | `platform/pc/pcx.rs`, `tga.rs` |
| Dr. Halo CUT + PAL | done | `platform/pc/halo.rs` (companions). All 14 sample CUTs decode. Its header says PIC is not implemented |
| Amiga RGBN / RGB8 | done, with one gap | 9 real samples (`corpus/extra/next-amiga-pc/sembiance/iffILBM-rgbn/`) decode pixel-identical to `recoil2png`. Two real RGB8 files are rejected, see 3.1 |
| IFF form `BBM ` | done | `platform/amiga.rs:104` maps it to ILBM. Only a synthetic unit test. No real BBM file in 127 ILBM-family samples, so the claim "PC Deluxe Paint writes BBM" stays unverified |
| Cyber Paint SEQ | done | `atari_st.rs:110` (`signature`). 10 samples already in `corpus/extra/atari-st/cyberPaintSeq` |
| Apple II DHR (unpacked) | done | `apple.rs` (`dhgr`, `dhr`) |
| Pack-Ice (ST) | partly done | `atari_st/pack_ice.rs`, private to `atari_st`, only the `Ice!`/`ICE!` header form. See 3.2 |
| Not registered | GIF, BMP, PowerPacker and every other Amiga packer, Animator PIC/CEL, FLI/FLC, Dr. Halo PIC, PCPaint PIC/CLP, GLPaint PIC2, ColoRIX, FRA, DreamGrafix, packed HGR/DHR, STOS PP1-PP3, Signum IMC, raw ST dumps, RastaConverter XEX | none of the 26 RastaConverter XEX, 14 STOS, 15 Signum, 25 PCPaint, 21 Animator, 34 FLI/FLC, 43 GIF, 41 BMP or 13 RIX samples decode today, and none is claimed by a wrong decoder (all fail with "unknown file format") |

[gaps-ranking.md](gaps-ranking.md) is stale on rows 10 (Cyber Paint SEQ), 11 (RGBN/RGB8), 23
(BBM) and 24 (Targa): all four are in the registry.

## 2. Samples obtained

Group `corpus/extra/next-amiga-pc/` (git-ignored, about 90 MB), `MANIFEST.tsv` in the usual
four columns (`recoil_size` is `-` everywhere: RECOIL decodes none of these except RGBN/RGB8).
Nearly all come from dexvert's sample set at `sembiance.com/fileFormatSamples/`.

| Directory | Files | Contents |
|---|---:|---|
| `sembiance/gif` | 43 | 9 GIF87a, 18 GIF89a (6 animated), 15 MacBinary-wrapped, 1 with 4 junk bytes before `GIF8`, 2 interlaced, 3 without trailer |
| `sembiance/bmp` | 41 | 38 `BM` (see 4.2), 2 headerless DIBs (`abydos.dib`, `input.dib`), 1 MacBinary-wrapped |
| `sembiance/animatorPICCEL` | 21 | original Animator PIC/CEL/RAW |
| `sembiance/fli`, `sembiance/flc` | 15, 19 | FLI `AF11`, FLC `AF12`, one FLH `AF44` |
| `sembiance/drHalo` | 20 | 14 CUT, 2 PAL, 4 PIC |
| `sembiance/pcPaint` | 25 | 14 PIC (`34 12`), 11 CLP |
| `sembiance/glPaintPIC` | 14 | `PIC2` magic, no spec found |
| `sembiance/rix` | 13 | ColoRIX `RIX3`, 12 uncompressed (storage byte 0), 1 with storage byte 4 (`DATA.DAT`, 931909 bytes, meaning unknown). No compressed file |
| `fractint/` | 4 | Wikimedia Commons GIF89a files carrying Fractint application extensions |
| `sembiance/powerPack` | 20 | PP20 / PX20 |
| `sembiance/crunchMania` | 15 | CrM!, Crm!, CrM2, Crm2 |
| `sembiance/fileImploder` | 12 | `IMP!`, `CHFI`, `M.H.` |
| `sembiance/rnc` | 13 | RNC method 1 and 2 |
| `sembiance/packIce` | 14 | `Ice!` and footer-form Pack-Ice |
| `sembiance/atomikCruncher` | 10 | ATM5 (Atari ST packer) |
| `sembiance/picturePacker`, `picturePackerDAJ` | 23, 10 | PP1/PP2/PP3 and DAJ |
| `sembiance/signumBitmap` | 15 | all `bimc0002` (`.IMC`, `.I01`, `.I02`, `.I04`, `.PAC`) |
| `sembiance/a2HighRes`, `a2eDoubleHighRes` | 2, 5 | no packed files |
| `sembiance/iffILBM-rgbn` | 14 | 9 RGBN/RGB8 plus 5 other ILBM-family oddities |
| `apple2gs-dreamgrafix` | 15 | 10 DreamGrafix (3 x 256-colour, 7 x 3200-colour, all packed), 4 plain 3200, the `.shk` they came from. Source: Trenco archive `dreamgrafix.shk`, unpacked with Deark's `nufx` module |
| `atari8-rastaconverter` | 26 + 4 PNG | 26 RastaConverter XEX from the repo's `examples/`, plus 4 reference renders |

Not found: StoneCracker samples (none in the dexvert set), any compressed ColoRIX file, any packed
Apple II HGR/DHR file,
a real BBM file, a Dr. Halo PIC outside CGA (all 4 are board 0x01), RNC/CrM pictures with
a header (the "pictures" are headerless screens, see 3.1).

Two filenames in `signumBitmap` are percent-encoded twice at the source (`MA%25E2...`); harmless.

## 3. Cross-cutting findings

### 3.1 Evidence for the packer layer

Black-box check with the Ancient CLI (BSD-2, built in the scratchpad, `ancient d in out`) over
the 128 packer samples above. Pictures that unpack to something the registry already decodes:

| Packed file | Packer | Unpacks to |
|---|---|---|
| `yo&trashcan.ham8.pp` | PP20 | FORM ILBM (HAM8), 349894 bytes |
| `hotachy1.anim.pp`, `shad-art.anim`, `MJ2.ANIM` | PP20 | FORM ANIM (first frame is ILBM) |
| `G12.GFX` | RNC1 | FORM ILBM, 63902 bytes |
| `Texture.DAT` | Imploder | FORM ANIM |
| `WARNSIGN.SFX` | Pack-Ice | FORM (ILBM), 2090 bytes packed |
| `GAME-OVER.DAWN` | CrM2 | AMOS bank (`AmBk`), which `amiga/abk.rs` reads |
| `00SCREEN.DAT`, `33SCREEN.DAT` | Pack-Ice v1.1 | raw 32000-byte ST screens |

That is 9 pictures out of 128 samples, and the dexvert sets are curated for packer coverage, so
real frequency is lower. The rest are modules, executables and headerless bitmaps (`bansheepic`,
`introscreen`, `*.bmp` CrM files unpack to 164864 and 163840 bytes of raw bitplanes with no
header). Headerless Amiga screens are not a format; skip them.

Not counted by any survey so far: Pack-Ice also covers Amiga IFF files, so the "shared layer
for ST" and PowerPacker belong in one module.

### 3.2 Pack-Ice gaps in the existing code

`pack_ice::is_packed` accepts only an `Ice!`/`ICE!` tag at offset 0. Ancient's detection (BSD-2)
also lists `TMM!`, `TSM!`, `SHE!` as headers and the v1.1-v1.14 files whose `Ice!` tag is a
footer. Both samples with the footer form (`00SCREEN.DAT`, `33SCREEN.DAT`) are real ST
screens that we reject today. Moving `pack_ice.rs` to `codec/` and adding these tags is a
small step that serves ST and Amiga.

### 3.3 Real RGB8 files RECOIL and we reject

`Vogel_Kamera.24` (148x262) and `WorldMap2.24` (224x118) are `FORM RGB8` files whose BMHD
says compression 3, not the 4 the AmigaOS wiki requires. The BODY is exactly width x height x 4
bytes, every entry is RGB plus a last byte of `0x01` (repeat count 1), so the data is
valid RGB8 with a mislabelled compression field. `recoil2png` reports "file decoding error", so
this is a "decode what RECOIL rejects" item (see
[recoil-is-baseline-not-ceiling](../../../../.claude/projects/-home-emoon-code-projects-retroimg/memory/recoil-is-baseline-not-ceiling.md)).
`rgbn.rs:40` rejects any `compression != 4`. Fix: also accept 3 (the BODY layout is
identical), and record both files in `tests/divergences/` with the byte-count evidence.
No registration change is needed: RGBN files without a known extension (`Clouds`,
`Ringbump`) already decode through the IFF signature path, so `.24` reaches the same
decoder.

### 3.4 MacBinary and junk prefixes

16 of 43 GIFs and 1 of 41 BMPs are MacBinary I files (128-byte header: byte 0 is 0, byte 1 the
name length, resource-fork length at 87, data-fork length at 83). One GIF has 4 junk bytes. The
existing `apple/macpaint.rs::decode_mac_binary` is private to the Apple module. A shared
`crate::bytes`-style `strip_macbinary(&[u8]) -> &[u8]` would let GIF, BMP and any later raster
format accept these, at the cost of moving the helper out of `apple`. For GIF only, scanning the
first few bytes for `GIF8` is the cheap rule; do not scan further.

### 3.5 Oracle

None of these formats are in RECOIL, so the oracle test cannot check them. Black-box
references that work today:

- **Deark** (MIT, `git clone https://github.com/jsummers/deark && make`, built in the scratchpad):
  decodes `gif` (27/43, no MacBinary), `bmp` (38/41), `fli`/`flc` (33/34), `animator_pic`
  (19/21), `pcpaint` (25/25, PIC and CLP), `drhalopic` and `drhalocut` (17/20), `colorix`
  (12/13), `stos` incl. Picture Packer PP1-PP3 and DAJ (33/33).
  Not covered: GLPaint PIC2, Signum IMC, DreamGrafix, Apple packed HGR/DHR, XEX.
- **Python PIL** (installed): GIF 24/43, BMP 39/41, FLI 14/15, FLC only 4/19, no MacBinary.
  Good enough for GIF and BMP, not for FLC.
- **Ancient CLI**: ground truth for every packer (`ancient d`, `ancient v`).
- **CiderPress II** (Apache-2.0, needs .NET): oracle for DreamGrafix and Apple packed formats.
- **RastaConverter reference PNGs**: 4 renders paired with XEX files.

This follows the existing text-mode practice: a divergence/evidence file per format, no RECOIL.

### 3.6 Extension and signature map

`.pic` now has five claimants: Animator (`19 91` at 0, plus `w*h+800 == len`), PCPaint
(`34 12` at 0, byte 11 `0xFF`), Dr. Halo (`AH`, byte 6 == 2), GLPaint (`PIC2`), and
the existing Atari/MSX/PC-98/QL/Psion rows. All five new ones can carry `.signature()`.
`.clp` clashes with GoDot and CoCo `clp` already registered: PCPaint CLP has no magic, but its first
word equals the file length, and that is checked tightly. `.cel` clashes with Cyber Paint CEL
(no magic): Animator is `19 91`. `.pal` Dr. Halo is `AH` with byte 6 == `0x0A`.

## 4. Per item

Difficulty: S (fixed layout or one small codec), M (several variants or a codec plus
containers), L (reverse engineering or a CPU model). All platform names follow the existing
registry: `"PC"` for GIF, BMP and the DOS formats (matches PCX and Targa), `"Amiga"`, `"Atari ST"`,
`"Atari 8-bit"`, `"Apple IIGS"`/`"Apple II"`.

### 4.1 GIF87a / GIF89a, plus FRA

- Spec: [GIF89a, W3C](https://www.w3.org/Graphics/GIF/spec-gif89a.txt) (fetched). LZW patents expired
  2003-2004. Cross-check: Deark `gif.c` (MIT, 1317 lines).
- Fractint: versions 5-13 write GIF87a with parameters appended after the trailer; later versions
  use a GIF89a application extension ([Just Solve FRA](http://fileformats.archiveteam.org/wiki/FRA_(Fractint)),
  fetched). The four Commons samples carry `fractint001`/`fractint003` application extensions.
  A GIF decoder that stops at `0x3B` already reads FRA; no separate format, just register
  extension `fra`.
- Samples: 43 + 4 (section 2). Variants to support: interlace, local palettes, transparency
  index (ignore), first frame only, missing trailer (3 files), MacBinary wrapper (16), junk prefix (1),
  logical screen larger than frame (composite the first frame onto the background colour as Deark does).
- Difficulty S-M. Core is about 200 lines (LZW with clear/end codes, 12-bit cap, deferred clear);
  the wrappers need the shared MacBinary helper (3.4).
- Risk: `image/gif` is claimed by every system thumbnailer. The CLI emits MIME XML and a
  `.thumbnailer`; decide before registering whether GIF and BMP are exported there or
  only decodable by name. Flag to the lead.
- Detection: `GIF87a`/`GIF89a` at 0 is a reliable `.signature()`.

### 4.2 BMP / DIB / OS/2 BMP

- Spec: [Microsoft bitmap storage](https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-storage)
  (fetched) and the BITMAPINFOHEADER / V4 / V5 pages; OS/2 `BITMAPCOREHEADER` (12 bytes) and the 64-byte
  OS/2 2.x header. Cross-check: Deark `bmp.c` and `os2bmp.c` (MIT).
- Sample header census (38 `BM` files): header 12 x 4 (4 and 8 bpp, OS/2), 40 x 29 (1 bpp x 5,
  4 bpp x 4 incl. 1 RLE4, 8 bpp x 10 incl. 2 RLE8, 24 bpp x 9, 32 bpp bitfields x 1), 64 x 2, 108 x 1, 124 x 2.
  So V4/V5 headers, bitfields, RLE4 and RLE8 and OS/2 all occur. Not present: 16 bpp,
  top-down, Huffman 1D, JPEG/PNG-in-BMP.
- Headerless DIB (2 files) begins with the info header (`28 00 00 00`): accept by extension
  with a strict header check, no `.signature()`.
- Difficulty S. About 250 lines with RLE, bitfield masks and padding rules.
- Detection: `BM` plus a size field that matches or is zero, header-size in
  {12, 40, 52, 56, 64, 108, 124}: tight enough for `.signature()`.
- Ico/cur: out of scope here (separate decision).

### 4.3 Amiga packers around pictures

- Specs: no prose spec for any of them. The permissive reference is
  [Ancient](https://github.com/temisu/ancient) (BSD-2-Clause, Teemu Suutari). The registry already
  re-implements Pack-Ice from it with the licence text in the file header
  (`atari_st/pack_ice.rs`); repeat that pattern. Sizes in Ancient: `PPDecompressor` 486 lines
  (includes XPK glue), `CRMDecompressor` 259, `IMPDecompressor` 296, `RNCDecompressor` 468,
  `StoneCrackerDecompressor` 652. Rust cost is roughly 150-400 lines each. Avoid `format198x`
  (GPL) and `PowerPacker-decrunch` (no licence checked).
- Magic: `PP20` (also `PX20` encrypted, `PP11`), `CrM!`/`Crm!`/`CrM2`/`Crm2`, `IMP!` (plus
  `ATN!`, `BDPI`, `CHFI`, `EDAM`, `M.H.`, `RDC9` clones), `RNC\x01` / `RNC\x02`, `S404`/`S310`/`S400`
  (StoneCracker, none in samples). PP20's tail holds the 24-bit unpacked length.
- Samples (section 2): PP20 20, CrM 15, Imploder 12, RNC 13. Pictures among them: section 3.1.
- Plan, in order:
  1. Move `pack_ice.rs` to `codec/`, add the `TMM!`/`TSM!`/`SHE!` tags and the footer form (3.2).
  2. Add `codec::pp20` (S-M; one bit reader, offset table in the header). Gains 4 of the
     9 sample pictures.
  3. Add a front step to `decode_iff` (and the ANIM first-frame path): if the data starts with a
     packer magic, unpack, then decode the result. Also register `pp` as an extension with a
     signature so `picture.iff.pp` and `x.pp` work. Guard recursion depth to 1.
  4. RNC, Imploder, CrM only when more real samples turn up: the sets have 1, 1 and 1 pictures.
     StoneCracker: no sample yet; wait.
  5. XPK (`XPKF`) is L and not worth starting.
- Atomik Cruncher 3.5 (`ATM5`, Atari ST) is not in Ancient. The 10 samples include
  `LOADING.PI1` (a packed DEGAS picture), but no permissive source was found. Mention only.
- Difficulty: PP20 S-M, rest M each.

### 4.4 Amiga RGBN / RGB8 and BBM

Done, see section 1 and 3.3. The remaining work is one line (accept compression 3) plus two
divergence rows. Spec: [AmigaOS wiki](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data).
Cross-check: Deark `ilbm.c` (MIT) supports RGBN and RGB8.

### 4.5 Autodesk Animator PIC/CEL and FLI/FLC

- Specs: [CompuPhase FLIC](https://www.compuphase.com/flic.htm) (fetched, all chunk types incl. the
  original `AF11` FLI), [Jim Kent, Dr. Dobb's 1993](https://jacobfilipp.com/DrDobbs/articles/DDJ/1993/9303/9303a/9303a.htm)
  (fetched, BYTE_RUN, DELTA_FLI, DELTA_FLC, COLOR_64/256), [fileformat.info CEL](https://www.fileformat.info/format/cel/corion.htm)
  (original Animator PIC == CEL), [Just Solve Animator PIC/CEL](http://fileformats.archiveteam.org/wiki/Animator_PIC/CEL)
  (CC0). Deark `fli.c` and the `animator_pic` module in `misc2.c` (MIT). The Animator Pro source on
  GitHub has no licence file ("permission granted" in the README): do not read it.
- Original PIC/CEL layout, derived from the 21 samples: `19 91`, width, height, x, y (u16 LE),
  depth byte (8), a flags byte, then padding up to a 256 x 3 palette at offset 32 (6-bit VGA values), pixels at 800.
  Every sample satisfies `len == 800 + w*h` (CEL files can be smaller than the screen). Two CELs carry
  0x91 in the flag byte yet are still raw, so the byte is not a compression flag.
  RLE-compressed originals are unverified (no sample).
- FLI/FLC: first frame only. Types needed: COLOR_64, COLOR_256, BYTE_RUN, DELTA_FLI (first
  frame can be a delta against black), DELTA_FLC, BLACK, COPY. FLH (`AF44`, 1 sample) is hi-colour: skip.
  Animator Pro PIC (`0x9500`) is a different format, no sample: skip.
- Difficulty: PIC/CEL S, FLI/FLC first frame S-M (about 250 lines for all chunk codecs).
- Detection: FLI/FLC `AF11`/`AF12` at offset 4 with a size field check is a `.signature()`.
  PIC/CEL `19 91` plus the size equation.

### 4.6 Dr. Halo PIC (and PAL)

- CUT and PAL are done. PIC: `AH` at 0, byte 6 == 2, board ID at 7 (0x01 CGA, 0x07 Hercules,
  0x15 EGA, 0x3C VGA 1-plane, 0x47 VGA 4-plane) with a board-dependent header size, then a
  byte RLE aligned to 512-byte blocks. Spec: [Just Solve Dr. Halo PIC](http://fileformats.archiveteam.org/wiki/Dr._Halo_PIC)
  (CC0, "partly reverse engineered"), Deark `drhalo.c` (MIT).
- Samples: 4 (`screen00/03/04/06.pic`, all board 0x01, CGA 320x200 4-colour). The EGA and VGA
  paths have no sample. Do CGA only, mark the rest unsupported, say so in the module doc.
- Difficulty S. Low value; only worth doing together with another PC item.

### 4.7 PCPaint / PICtor PIC and CLP

- Spec: [Encyclopedia of Graphics File Formats, Pictor](https://www.fileformat.info/format/pictor/egff.htm)
  (fetched, CC-BY, full header, block RLE, CLP and OVR layouts),
  [Just Solve PCPaint PIC](http://fileformats.archiveteam.org/wiki/PCPaint_PIC) and
  [CLP](http://fileformats.archiveteam.org/wiki/PCPaint_CLP) (CC0), Deark `pcpaint.c` (MIT).
  Do not read FFmpeg's pictor decoder (LGPL); `samples.libav.org/image-samples/pictor/` is fine for samples.
- Sample census (25): PIC with VGA 256 (palette type 4, 768 bytes) x 11, EGA 16-colour planar
  (plane info `0x31`, 16-byte palette or none) x 3, CGA 4-colour x 1, one `s`-mode text-like file
  (`EASY1.PIC`, 8 bpp, no palette), CLP x 11 (all packed). Image rows are written bottom-up in the spec text;
  verify against Deark output on the first sample.
- Not covered: 24-bit variants (no sample), text-mode PICs, OVR containers, GLPaint `PIC2`.
- Difficulty S-M: block RLE with 8-bit and 16-bit run forms, plane layouts for EGA, CGA bit packing.
- Detection: `34 12` is weak alone (two bytes), but the 17-byte header and
  `PaletteFlag == 0xFF` make it tight; CLP uses the length field.
- GLPaint `PIC2` (14 samples, `PIC2` magic, header `01 08 ...`): no spec found. L, blocked.

### 4.8 ColoRIX

- Specs: [Encyclopedia of Graphics File Formats, RIX](https://www.fileformat.info/format/rix/egff.htm)
  (fetched: `RIX3` header, palette type, storage type bits `80` compressed / `40` extension /
  `20` encrypted / `01`,`02` planar, palette 48 or 768 bytes, extension block), [Just Solve ColoRIX](http://fileformats.archiveteam.org/wiki/ColoRIX)
  (fetched: old headerless form, extension to screen-mode table, compression is segments + XOR
  filter + RLE + Huffman). The EGFF page says the codec is unpublished; **Deark `colorix.c`
  (MIT, 757 lines, 2023) implements it** (Huffman node table, segments, XOR filter), which moves
  the compressed case from L to M. No sample is compressed, so a decoder would be untested
  against real compressed data until one turns up.
- Samples: 13, all `RIX3`: 12 uncompressed (storage `00`, size equals 10 + palette + w*h; palette
  type `AF` is VGA with 768 bytes, one file has type `00`), and `DATA.DAT` with storage byte 4,
  which the spec table does not list (Deark rejects it too). The extension-to-mode table is
  only needed for the old headerless form.
- Difficulty: raw S, compressed M (read Deark's module with a notice, as for Ancient).

### 4.9 DreamGrafix and packed Apple II Hi-Res / DHR

- Spec: [CiderPress II format notes](https://ciderpress2.com/formatdoc/SuperHiRes-notes.html)
  and [HiRes](https://ciderpress2.com/formatdoc/HiRes-notes.html) /
  [DoubleHiRes](https://ciderpress2.com/formatdoc/DoubleHiRes-notes.html) (docs CC BY-SA 4.0, code
  Apache-2.0; fetched this session from the repo's `FileConv/Gfx/*.md`). `SuperHiRes_DreamGrafix.cs`
  is Apache-2.0 and based on code from the DreamGrafix co-author.
- DreamGrafix: 17-byte footer `[mode u16][height u16][width u16][len byte 0x0A]"DreamWorld"`.
  Mode 0 = 256 colours (pixels 32000 + SCB 256 + palettes 512 + 512 unused), mode 1 = 3200
  colours (pixels 32000 + 6400 colour table + 512). PNT/$8005 compresses everything but the footer with
  12-bit LZW. `Convert2dg` (Brutal Deluxe) calls it "a variation of the LZW algorithm". `PIC/$8003`
  (unpacked) is not used in practice. Footer is a reliable `.signature()`. All 10 sample files carry it.
- Packed Hi-Res/DHR: `FOT/$4000` (hi-res) and `FOT/$4001` (DHR) are PackBytes-packed screens,
  `FOT/$8066` is LZ4FH (made by `fhpack`, format "in the compression sources"). `apple/pack_bytes.rs`
  exists. There is no magic and ProDOS types do not survive in a plain file: accept by extension
  (CiderPress exports `NAME#084000`) plus unpack-to-exactly-8192/16384-bytes as the validation. Samples:
  none. Build tests by PackBytes-packing the existing `.hgr`/`.dhgr` corpus files and cross-check with CiderPress II.
- `.3201` (`APP\0` or `NRL\0` prefix) is already handled (3201 rows in the registry).
- Difficulty: DreamGrafix M (LZW variant, two layouts, 3200-colour path reuses
  the existing Brooks decoder), packed HGR/DHR S, LZ4FH S-M. Value: DreamGrafix medium, HGR/DHR packed low until a sample exists.

### 4.10 STOS Picture Packer PP1-PP3 (and DAJ), STOS packed screens

- Specs: [abydos Picture Packer page](https://snisurset.net/code/abydos/picturepacker.html) and
  [Atari Forum Wiki](https://temlib.org/AtariForumWiki/index.php?title=Picture_Packer_file_format&action=raw)
  (prose, no licence: facts only). Both say the compression is not described. **Deark `mbk.c`
  (MIT, 1092 lines) implements it**, including PP1 (low res but stored as 2 planes), PP2, PP3 and
  the DAJ medium-res variant, and decodes all 33 samples. The decompress routine is
  `fmtutil_decompress_stos_pictbank` in Deark's `fmtutil` code. Readable with a notice. The MBK
  magic is `06 07 19 63`.
- Resolution cannot come from the header (set to medium for every file); it comes from the
  extension: `.pp1` low, `.pp2` medium, `.pp3` high, `.daj` medium 4-plane.
- Samples: 23 + 10. Extension-gated, no `.signature()` unless the `06 07 19 63` magic is present
  (it is absent for bare `.pp?` files).
- Difficulty M (slice compression plus per-resolution plane shuffling). Medium-high value:
  these are common on STOS disks. Cross-check: Deark.

### 4.11 Signum IMC

- Spec: [sdo.dseiler.eu bimc](https://sdo.dseiler.eu/formats/bimc) (fetched; reverse-engineered
  from a disassembly; no licence on the page): header `bimc0002`, compressed size, width, height,
  horizontal and vertical chunk counts, bit-stream and byte-stream sizes, final XOR word. 16x16
  chunks, a bit-stream selecting present chunks, four chunk strategies (direct 32 bytes, 8x8
  sub-chunks with optional 2-byte or 4-byte rolling XOR), then a final row-wise XOR. The
  [sdo-tool](https://github.com/Xiphoseer/sdo-tool) `signum` crate is MIT OR Apache-2.0 and
  readable for cross-checks; its CLI is AGPL, do not read.
- Samples: 15, all `bimc0002`, 640x400 (width/height fields `02 80 01 90`). Extensions `.IMC`,
  `.I01`, `.I02`, `.I04`, `.PAC` (clash with STAD `.PAC`; the magic separates them).
- Deark has no module; the check is by eye plus the sdo-tool crate. Output is 1-bit, easy to judge.
- Difficulty M (spec is complete but fiddly). `.signature()` fits. Value low-medium.

### 4.12 Cyber Paint SEQ

Done. `platform/atari_st/seq.rs`, 10 samples in `corpus/extra/atari-st/cyberPaintSeq`.
Nothing to research.

### 4.13 Raw ST screen dumps

No spec to find: 32000-byte low/medium/high-res screens, no palette, plus 32034-byte DEGAS
without magic. See [gaps-atari.md](gaps-atari.md) 1.5. The only new evidence is that Pack-Ice v1.1
files (3.2) unpack to such screens: `00SCREEN.DAT` starts `0F FF 00 00`, `33SCREEN.DAT` starts with a
palette word run, so a packed dump carries the palette inline. Plan: extension-gated
(`.PI?`, `.RAW`, `.DAT` candidates), grey-ramp fallback palette, never `.signature()`. Difficulty S,
risk is false positives; do it last or skip until a demo-tree sample set exists.

### 4.14 RastaConverter self-displaying XEX pictures

- RastaConverter has no licence file: do not read its code, treat README and `help.txt` as prose.
  The repo's `examples/` hold 26 XEX files and 4 reference PNGs (downloaded, not committed).
- Structure, derived from the 26 samples (all identical in shape): `FF FF` then segments
  `$2000..$2005` (6 bytes), `$02E2` (2, DLIST pointer), `$2010` (9616 bytes: 16 + 40 x 240, a
  40-byte x 240-line bitmap), `$4800` (723 bytes, constant length), `$5400` (about 12 KB,
  length varies per picture, the per-scanline register-write kernel), `$02E0` (2, RUN address). One
  outlier `winter-beauty.xex` uses `$4400`/`$4C00`. The common prefix `FF FF 00 20 05 20 A9 FF 8D 01 D3` makes it
  signature-detectable.
- A static decode needs the register writes that the kernel performs on each of the 240 lines, so
  either a small 6502 interpreter that records writes to GTIA/ANTIC addresses per scanline (about 400
  lines: only the instructions the kernel uses, no cycle exactness), or a pattern parse of the
  generated `LDA/STA` runs after confirming the generator emits a fixed grammar. Neither is
  specified anywhere readable. Altirra is GPL: only its hardware manual may be used.
- Verification: four reference PNGs, the other 22 judged by eye against the bitmap segment.
- Difficulty L, but the 26 samples are uniform, so the kernel grammar can be reverse engineered
  from them. High value for the modern Atari scene. Start with a spike: dump the `$5400`
  segment as disassembly (permissive 6502 tables) and check how regular it is.

## 5. Order of work

Order by value over cost, using sample counts above and the clean-room constraints:

| Rank | Item | Size | Why |
|---:|---|---|---|
| 1 | RGB8 compression-3 fix, 2 divergence rows | XS | 2 real files, one-line change |
| 2 | BMP | S | 41 samples, 7 header variants, trivial oracle (PIL and Deark) |
| 3 | GIF (plus FRA extension) | S-M | 43 samples, needs MacBinary helper (3.4) |
| 4 | Pack-Ice move to `codec/`, extra tags and footer form | S | 2 real ST screens rejected today, prerequisite for PP20 |
| 5 | PP20 (then the unpack front step for IFF/ANIM) | S-M | 4 sample pictures; unlocks `.pp` |
| 6 | PCPaint PIC + CLP | S-M | 25 samples, full EGFF spec, Deark oracle |
| 7 | Animator PIC/CEL, then FLI/FLC first frame | S, S-M | 21 + 34 samples, Deark and PIL oracles |
| 8 | STOS Picture Packer PP1-PP3 and DAJ | M | 33 samples, Deark's MIT code, common on STOS disks |
| 9 | ColoRIX (raw first, compressed from Deark) | S then M | 12 raw samples, none compressed |
| 10 | DreamGrafix | M | 10 samples, CiderPress II Apache code and notes |
| 11 | Signum IMC | M | 15 samples, complete spec, no oracle |
| 12 | RastaConverter XEX | L | 26 samples, 4 reference renders |
| 13 | Dr. Halo PIC (CGA) | S | 4 samples; only with another PC item |
| 14 | RNC, Imploder, CrM | M each | wait for more real packed pictures |
| 15 | Packed HGR/DHR, LZ4FH | S | no sample; wait |
| 16 | Raw ST dumps | S | extension-gated, false-positive risk |
| 17 | GLPaint PIC2, StoneCracker, Atomik ATM5, XPK | L / blocked | no spec or no sample |

Items 1 to 4 fit one session and are the cheap batch for this list.

## 6. Open questions for the lead

1. Do GIF and BMP appear in the thumbnailer MIME list, or only decode by name? They can shadow
   the desktop's own thumbnailers (4.1).
2. Should MacBinary stripping become a shared helper in `crate::bytes` or a new `crate::container`
   (3.4)? It is needed by Apple, GIF and BMP.
3. Animations: first frame only for FLI/FLC, GIF and ANIM (consistent with the current ANIM rule)?
4. Platform label for GIF and BMP: `"PC"` as for PCX/Targa, or a separate generic label? The
   `$RETRO_IMAGE_PLATFORMS` filter in the oracle test depends on it.
5. Sample use: the corpus group is `corpus/extra/next-amiga-pc/`. Several samples are adult art
   (the GLPaint set, some GIFs and FLIs). They stay in the git-ignored corpus; pick which
   thumbnails the test output may show if any is published.
