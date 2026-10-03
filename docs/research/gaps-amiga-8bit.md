# Gap survey: Amiga, ZX Spectrum, Amstrad CPC, MSX and BBC Micro

Clean-room research notes, written 2026-10-03. This file lists formats retro-image does not
decode yet, for the Amiga, ZX Spectrum (and Timex/Next), Amstrad CPC, MSX and BBC Micro families.
It lists only public specifications, vendor documentation and permissively licensed references. No RECOIL
code and no GPL/LGPL decoder code was read. Starting points were `docs/formats.md`, `docs/coverage.md`,
`docs/sources.md`, `docs/research/README.md`, `amiga-apple-misc.md`, `sinclair-cpc-bbc-misc.md`,
`msx-japanese.md` and `CLEANROOM.md`. Our own registry (`retro-image --list-formats`) and a few
synthetic probe files were used to confirm what is already decoded.

RECOIL is the baseline, not the ceiling. Two kinds of gap are listed:

- **Baseline gaps**: on RECOIL's list, still missing here (`coverage.md`): Amiga RGB8/RGBN, MSX2 SRI+PL7.
- **Beyond-RECOIL gaps**: formats RECOIL doesn't list. Most are containers (snapshots, tapes, packers)
  or screen variants that real archives are full of.

## Docs quality legend

Same as `README.md`: **Spec**, **Partial**, **Hardware-only**, **None**. Difficulty: S (a day or less, a
fixed layout), M (a few days, a codec or several variants), L (reverse engineering or a big codec).
"Signature" says whether content detection is possible without the extension.

## Summary

| Family | Gaps listed | Baseline | Beyond RECOIL | Comment |
|---|---:|---:|---:|---|
| Amiga | 11 | 2 | 9 | Mostly wrappers around IFF (packers) and rare IFF FORMs |
| ZX Spectrum / Timex / Next | 9 | 0 | 9 | Snapshot/tape screen extraction and Next variants are the big wins |
| Amstrad CPC | 4 | 0 | 4 | SNA snapshots; overscan and CPC Plus files are poorly documented |
| MSX | 4 | 1 | 3 | Few real gaps; MSX2 is already well covered |
| BBC Micro | 3 | 0 | 3 | Mode 7 teletext, Mode 3/6 dumps |

Already covered, so not repeated below (checked against the registry): ILBM/PBM/ACBM, HAM6/HAM8, EHB,
PCHG/SHAM/CTBL/BEAM palettes, DCTV, HAM-E, DEEP, ANIM (first frame), Atari ST VDAT, AMOS banks and
Pac.Pic., icons (classic/NewIcons/GlowIcons), SCR/ATR/IMG/MLT/MC/IFL/BSC/BMC4/BSP/MG*/STL/3/RGB/CH$/CHX/
ZXP/SEV, Timex SCR/HRG, ULAplus SCR, NXI (256x192 only), SXG, Profi GRF, OCP SCR/WIN/FNT, CM5, Perfect Pix, HGB,
SGX, MSX SC2-SCC/GRP/GE*/SR*/GL*/SH*/MAG/MAKI/PI/PIC/MIG/MIF/CMP/PCT/G9B, BBC BB0-BB5/LdPic.

## Sources checked in this pass

- AmigaOS wiki: [IFF FORM and Chunk Registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry),
  [ILBM](https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap),
  [YUVN](https://wiki.amigaos.net/wiki/YUVN_IFF_YUV_Image_Data),
  [FAXX](https://wiki.amigaos.net/wiki/FAXX_IFF_Facsimile_Image),
  [RGBN/RGB8](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data).
- [Ancient](https://github.com/temisu/ancient) (BSD-2-Clause) format list; [MultimediaWiki IFF](https://wiki.multimedia.cx/index.php/IFF).
- [Deark formats.txt](https://github.com/jsummers/deark/blob/master/formats.txt) (MIT) for what a permissive reference already covers.
- [SpectraLab README](https://github.com/Bedazzle/SpectraLab) (MIT), [zx-image README](https://github.com/moroz1999/zx-image) (CC0),
  [SpecNext wiki: File Formats](https://wiki.specnext.dev/File_Formats), [World of Spectrum Z80 format](https://worldofspectrum.org/faq/reference/z80format.htm),
  [WoS TZX format](https://worldofspectrum.net/TZXformat.html), [WoS TAP format](https://worldofspectrum.net/zx-modules/fileformats/tapformat.html).
- [CPC SNA format (cpctech)](https://cpctech.cpcwiki.de/docs/snapshot.html), [cpcwiki SNA](https://www.cpcwiki.eu/index.php?title=Format:SNA_snapshot_file_format),
  [cpctech artstud.html](https://cpctech.cpcwiki.de/docs/artstud.html).
- [BeebWiki MODE 7](https://www.beebwiki.mdfs.net/MODE_7), [MRG TTI page format (PDF)](https://zxnet.co.uk/teletext/documents/ttiformat.pdf),
  [mdfs.net BBC conversion utilities](https://mdfs.net/Apps/Graphics/Conversion/), [Aminet util/dtype](https://aminet.net/util/dtype).
- Not reachable this session: fileformats.archiveteam.org and justsolve (connection refused), cpcwiki.eu pages (HTTP 403),
  msx.org wiki (HTTP 403). Facts taken only from search snippets are marked "(snippet)".

---

## Amiga

| Ext | Format | Docs | Sources | In the wild / samples | Diff | Signature |
|---|---|---|---|---|---|---|
| RGBN, RGB8 (FORM) | Impulse Turbo Silver / Imagine 12- and 24-bit RGB (baseline gap) | **Spec** | [AmigaOS wiki](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data), Deark `ilbm` (MIT) as cross-check | Rare. Aminet has only the datatype (`RGBx_DT.lha`, README.md wave 5). No real sample found. Synthetic files can be fed to `recoil2png` as an oracle, as was done for HAM-E | S | `FORM....RGBN` / `RGB8` |
| (packed) | PowerPacker PP20, Imploder IMP!/ATN!/CHFI, Crunchmania, StoneCracker, Bytekiller, RNC wrapping an ILBM | **Partial** | Ancient's [format list](https://github.com/temisu/ancient) (BSD-2, readable with notice); PP20 layout summarised from a search snippet; [amiga-stuff decrunchers](https://www.amiga-stuff.com/decrunchers.html). Avoid format198x (GPL) and ipr/PowerPacker-decrunch (no licence checked) | **High** on Aminet, ADA and old demo/cracktro disks (pictures were often stored as `.pp`/`.imp`/`.iff.pp`). Samples would be easy to produce with the era tools or by repacking | M | Magic at offset 0 (`PP20`, `IMP!`, `CrM!`, `S404`...), then the inner `FORM` |
| (XPK) | XPK-wrapped ILBM (`XPKF` header, 60+ sub-packers) | **Partial** | Ancient covers XPK sub-formats (BSD-2) | Medium. Only the commonest sub-packers (NUKE, SQSH, HUFF, RLEN, FAST, BLZW, ELZX) are worth doing | L | `XPKF` |
| (any) | Digi-View 21-bit pictures (NewTek) | **None** | Product pages only: [Big Book of Amiga Hardware](https://bigbookofamigahardware.com/bboah/product.aspx?id=307), [amiga.resource.cx](https://amiga.resource.cx/exp/digiview) | Rare. Mostly saved as IFF/HAM. The proprietary 21-bit format is mentioned in snippets only; no layout found | L | Unknown |
| YUVN (FORM) | MacroSystem YUV image (Video Machine, 1992) | **Spec** | [AmigaOS wiki YUVN](https://wiki.amigaos.net/wiki/YUVN_IFF_YUV_Image_Data): `YCHD`, `DATY`, `DATU`, `DATV`, modes 400/411/422/444 and lores 200/211/222, CCIR-601 ranges, no compression defined | Rare. An Aminet `ycbcrdatatype.lha` exists ("TBCPlus YCrCb"), which may be a different format (uncertain). Not on RECOIL's list | S | `FORM....YUVN` |
| FAXX (FORM) | IFF fax page (`FXHD`, `PAGE`, `FLOG`) | **Spec** | [AmigaOS wiki FAXX](https://wiki.amigaos.net/wiki/FAXX_IFF_Facsimile_Image): MH, MR, MMR (T.4/T.6) or none | Very rare (a 1991 fax-card spec). Needs a G3/G4 decoder, so cost is high for little reward | M | `FORM....FAXX` |
| (ARGB chunk) | OS4 / PNG GlowIcons (`FORM ICON` with `ARGB` chunk, zlib) | **Partial** | Not in Stöcker's spec (see `amiga-apple-misc.md`). Chunk layout would need reverse engineering from samples | **High**: OS4Depot and Aminet icon packs (uncertain, not counted). Today these icons show only the older image. Needs a shared inflate first (it lives in the Atari 8-bit module) | M | Already detected as `.info` |
| ANIM frames 2..n | IFF ANIM ops 5/7/8 (delta frames) | **Spec** | [AmigaOS wiki ANIM](https://wiki.amigaos.net/wiki/ANIM_IFF_CEL_Animations) (not fetched this pass), Deark `anim` (MIT) | Common on Aminet, but they are animations. The first frame is shown already. Skip unless frame export is wanted | M | Detected |
| (any) | Amiga bitmap fonts (`.font` plus numbered files, hunk format) | **Partial** | Not fetched. [monobit](https://github.com/robhagemans/monobit) (MIT) may have a reader; unchecked | Common (every Workbench). Fonts, not pictures. Only relevant if fonts count | M | Hunk header `$000003F3` |
| (any) | AmigaBASIC shape files | **Partial** | Only an AmigaFFH (R package) man page seen in a search result; the package's licence was not checked, so treat it as prose only | Very rare | S | None |
| CHBM, ANBM (FORM) | Chunky bitmap (CHBM) and Framer/Deluxe Video animated bitmap (ANBM) | **None** | Names only, in the [registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry) and [MultimediaWiki IFF](https://wiki.multimedia.cx/index.php/IFF) | Practically none | L | `FORM....CHBM` / `ANBM` |
| (ILBM chunks) | `TINY`/`PRVW` thumbnails, `DPPS`/`DPPV` | **Partial** | Registry; ILBM page lists PRVW only | Present in many files, but only useful for thumbnails; the main body already decodes. Not worth a format | S | n/a |

Notes for the Amiga rows:

- **Lightwave, Imagine and Real3D** do not add 2D image formats. Their objects and scenes (LWOB, TDDD/IOB, REAL)
  are 3D data (FORM types seen in search results). Their rendered pictures are IFF ILBM (24-bit) or RGB8, which
  we already or soon cover. The framebuffer cards in [amiga.resource.cx](http://amiga.resource.cx/search.pl?cat=fb) list
  "IFF24, IFF21, RGB8, TIFF, REND" (snippet); IFF24/21 are ILBMs with 24/21 planes (assumed, unverified).
- **Not distinguishable in the wild:** "Rendition" was named in the task as an ILBM variant. Searches found no
  chunk or FORM with that name; treat it as unknown until someone supplies a file.
- **Dynamic HiRes (`DYCP`)**: `docs/formats.md` already lists DHR/DR/MP/BEAM; whether `DYCP` is read inside
  `multi_palette.rs` was not checked. Worth a quick look by whoever does the Amiga work.
- **Raw bitplane dumps** from demos have no header. Nothing can be said about a signature; they are not a
  format until a platform tool defines one.

## ZX Spectrum, Timex and Next

| Ext | Format | Docs | Sources | In the wild / samples | Diff | Signature |
|---|---|---|---|---|---|---|
| SNA, Z80, SZX | Emulator snapshots: render the screen memory (48K/128K, Timex port 0xFF mode, ULA+ chunk in SZX) | **Spec** | [WoS Z80 format](https://worldofspectrum.org/faq/reference/z80format.htm) (v1/v2/v3, `ED ED xx yy` RLE, memory pages); SNA is a 27-byte header + 48K (49179 bytes) or 131103 for 128K ([SpectraLab README](https://github.com/Bedazzle/SpectraLab)); SZX spec not fetched | **Very high**: every emulator, WoS, zxart.ee screens and demos. The screen is in RAM at 0x4000 (bank 5, or bank 7 when bit 3 of port 0x7FFD is set; from general knowledge, verify). Beyond RECOIL | S-M | Z80 has no magic (header heuristics); SNA by size; SZX `ZXST` |
| TAP, TZX | Tape images: extract the loading screen (6912-byte CODE block at 0x4000, headered or not) | **Spec** | [TZX spec](https://worldofspectrum.net/TZXformat.html), [TAP format](https://worldofspectrum.net/zx-modules/fileformats/tapformat.html) | **Very high** (the whole WoS/TOSEC archive). Only standard (0x10) and turbo (0x11) blocks give a clean screen; custom loaders are out of reach. `corpus/extra/*/tosec-*` hold tape-era collections | S-M | TZX `ZXTape!\x1a`; TAP by block structure (heuristic) |
| SCR (+3DOS) | SCR with 128-byte +3DOS header (7040 bytes), also SL2 `PLUS3DOS` | **Spec** | [SpecNext wiki](https://wiki.specnext.dev/File_Formats): "headered (7,040 bytes) and headerless" | **High**: Next software, ZX-Paintbrush, Next emulators. Probe: a 7040-byte `.scr` is rejected today | S | `PLUS3DOS\x1a` |
| SL2, SLR, SHC, SHR, NXI (wide) | Next screens: SL2 L2 256x192 (49152/49280 bytes), SL2 320x256 / 640x256 (81920), SLR lores 128x96 (12288), SHC Timex hi-colour, SHR Timex hi-res, NXI at 82432/81952 bytes (320x256 / 640x256) | **Spec** | [SpecNext wiki](https://wiki.specnext.dev/File_Formats), [SpectraLab README](https://github.com/Bedazzle/SpectraLab) sizes. Our `decode_nxi` accepts only 49664 bytes | Medium-high on Next sites and Remy Sharp's tools (snippet). The 640x256 layout is 4 bpp (assumed; check) | S | By size and extension |
| SCR (variants) | SCR + ULANext palette (6945-7426 bytes), 2/3 mono (4096) and 1/3 mono (2048) screens | **Spec** | [SpectraLab README](https://github.com/Bedazzle/SpectraLab) | Low-medium. ULANext variable palette length needs the SpectraLab guide section | S | Size only |
| SCA | SpectraLab animation (type 0 full frames, type 1 attribute-only) | **Partial** | [SpectraLab README](https://github.com/Bedazzle/SpectraLab) (names only; layout in the guide, not read here) | Low. Show the first frame | S | Unknown |
| BTILE, WTILE | Nirvana 8x2 multicolour tile sets | **Partial** | SpectraLab README (name only) | Low (game-dev tooling) | M | Unknown |
| ATMEGA (zx-image) | ATM Turbo 2+ EGA mode, 32128 bytes, 320x200 16 colours | **Partial** | [zx-image README](https://github.com/moroz1999/zx-image) (CC0, a facts source only; size listed, layout not) | Low (Russian clones; nedopc.org and zxart.ee may hold samples; uncertain) | M | Size |
| MC4 (zx-image "multicolor4") | Multicolor 8x4, 7680 bytes (6144 bitmap + 1536 attrs) | **Partial** | zx-image README lists size; the name of the extension is unconfirmed | Low-medium (zxart.ee). Sibling of IFL/BMC4 | S | Size |
| (compressed) | `.scr.zx0`, `.zx7`, `.mlz` and similar packed screens | **None** | n/a | Common in modern demos, but no magic bytes. Only by extension | S each | No |

The gap that matters most is the container family. A Spectrum thumbnailer that handles `.scr` only misses
nearly every file people exchange.

## Amstrad CPC

| Ext | Format | Docs | Sources | In the wild / samples | Diff | Signature |
|---|---|---|---|---|---|---|
| SNA | CPC snapshot v1/v2/v3: render screen RAM with the saved Gate Array pens, mode and CRTC start address | **Spec** | [cpctech snapshot.html](https://cpctech.cpcwiki.de/docs/snapshot.html), [cpcwiki SNA](https://www.cpcwiki.eu/index.php?title=Format:SNA_snapshot_file_format) (chunked v3, CPC+ ASIC palette is 32 x 2 bytes) | **High**: demos and games ship as SNA; `MV - SNA` header gives a clean screenshot. Beyond RECOIL. Raster effects will not be reproduced | M | `MV - SNA` |
| SCR (overscan) | OCP/ConvImgCPC overscan screens (screen larger than 16K, CRTC reprogrammed) | **Partial** | [Overscan](https://www.cpcwiki.eu/index.php/Programming:Overscan) (403 here), cpctech artstud.html does **not** cover it (checked). Sizes and the position of the CRTC data are unknown to me | Medium (CPC-Power, Demozoo). Cannot be told from a normal SCR except by size/AMSDOS header (uncertain) | M | Size heuristic |
| PAL (CPC+) / INK | 12-bit ASIC palettes next to SCR, `.INK` (Martine, GPL-2: do not read its code) | **None** | cpctech artstud.html: no Plus support. Hardware side: [cpcplus.html](https://cpctech.cpcwiki.de/docs/cpcplus.html) | Medium among modern CPC+ art; layout would have to come from samples | M | No |
| (any) | CPC+ hardware sprite files (16x16, 4 bpp, 12-bit palette) | **Partial** | [cpcwiki Plus sprite format](https://www.cpcwiki.eu/index.php/Programming:Amstrad_CPC_plus_sprite_format) (403 here; title seen in a search) | Low | S | No |

## MSX

MSX2 is the best covered platform in the repository. These gaps remain:

| Ext | Format | Docs | Sources | In the wild / samples | Diff | Signature |
|---|---|---|---|---|---|---|
| SRI + PL7 | Graph Saurus interlaced screen 7 (baseline gap) | **Spec** | [GSRLE readme](https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE) (already used for SR5-SRS) | Popular per the msx.org BLS page (snippet), but wave 5 found no sample file. Decoder is the SR7 path with interlace | S | `FE`/`FD` BSAVE header |
| GRA | QLD, 8 fixed colours, used on Japanese BBSes | **None** | Only a one-line mention on [msx.org BLS (Seiga)](https://www.msx.org/wiki/BLS_(Seiga)) (snippet; page 403 here) | Rare | L | Unknown |
| SC0, SC1 | Text-mode VRAM dumps (Screen 0/1 name/pattern/colour tables) | **Spec** | [MSX2 Technical Handbook, Appendix 5](https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html) | Low as pictures; they are text screens with a font | M | BSAVE start |
| (compressed) | Pletter, ZX0, ZX7, MegaLZ-packed screens, plus `.PL5` palettes alone | **None** | n/a | Common in demos, no magic | S each | No |

Other findings:

- BLS (Seiga) lists the formats MSX users actually viewed: `.SCx`, Graph Saurus `.SRx+.PLx` and `.SRI+.PL7`,
  `.SHx`/`.GLx` and QLD `.GRA` (snippet). We cover all but SRI and GRA.
- No GPL-free spec was found for newer MSX paint formats (anything outside Graph Saurus, Maki-chan, Pi, PIC,
  Dynamic Publisher and Dot Designer's Club). "Super Pi" and similar names turned up nothing.
- MGSDRV, mentioned in some lists, is a sound driver, not a picture format.

## BBC Micro

| Ext | Format | Docs | Sources | In the wild / samples | Diff | Signature |
|---|---|---|---|---|---|---|
| (BB7, `.m7`) | MODE 7 teletext screen (1000 bytes shown, 1024 at &7C00) | **Spec** | [BeebWiki MODE 7](https://www.beebwiki.mdfs.net/MODE_7); control codes and glyphs from the SAA5050 datasheet and ETSI EN 300 706 (not fetched); [J.G. Harston's M7toBMP](https://mdfs.net/Apps/Graphics/Conversion/) is described as incomplete | **High** in the teletext-art scene (Teefax, Ceefax archives, edit.tf); BBC games and utilities save 1024-byte dumps. Beyond RECOIL. Needs the 5x9 SAA5050 glyph set and the Level 1 rules (colour, mosaic, hold, double height, flash). Glyph provenance has to be settled, as for the ZX81 ROM | M | No magic; size 1000/1024 |
| TTI | MRG teletext page (text, `PN`/`DS`/`OL` command lines, control codes mapped to high ASCII) | **Spec** | [MRG TTI format (PDF)](https://zxnet.co.uk/teletext/documents/ttiformat.pdf) | **High** among teletext archives; renders with the same Mode 7 engine | S after Mode 7 | `PN,` / `DE,` / `OL,` lines |
| BB3, BB6 | MODE 3 (80x25 text, 2 colours) and MODE 6 (40x25 text) dumps; Electron equivalents | **Hardware-only** | [dfstudios](https://www.dfstudios.co.uk/articles/retro-computing/bbc-micro-screen-formats/) and [MOS disassembly](https://tobylobster.github.io/mos/mos/S-s4.html) already in our notes | Text-screen memory dumps (20 KB / 8 KB). The BBC font is needed to read them as text. Little use | S | Size |
| ScrLoad / ScrSave | J.G. Harston's RLE screen files | **None** | [mdfs.net JGH library](https://mdfs.net/Mirror/Image/JGH/) lists the commands only; the [libbeebimage](https://nerdoftheherd.com/projects/libbeebimage/) prose names the format (its code is GPL-3) | Medium-low; layout would come from samples | M | Unknown |

---

## Top 10 for this family

| # | Candidate | Why |
|---|---|---|
| 1 | ZX Spectrum SNA / Z80 / SZX snapshot screens | Nearly every Spectrum file in circulation is a snapshot; specs are public and complete, difficulty S-M. |
| 2 | ZX Spectrum TAP / TZX loading screens | Whole WoS/TOSEC archive becomes thumbnailable; shares the 6912-byte screen path with #1. |
| 3 | Amiga packer wrappers (PP20, Imploder, Crunchmania, StoneCracker, RNC) around ILBM | Large share of old Amiga scene pictures; Ancient is BSD-2 and covers every variant. |
| 4 | Amiga RGBN / RGB8 | Baseline gap, full public spec, small; testable with synthetic files through `recoil2png`. |
| 5 | ZX Spectrum Next: SL2/SLR/SHC/SHR, 320x256 and 640x256 NXI, +3DOS-headered SCR | Current NXI only accepts 49664 bytes; probes confirm a 7040-byte headered SCR fails; all fixed layouts (S). |
| 6 | BBC MODE 7 teletext (screen dumps and TTI) | Big, active scene and archives; fully specified; blocked only by deciding where the SAA5050 glyphs come from. |
| 7 | Amstrad CPC SNA snapshots | Demos ship as SNA; spec is complete; screenshot via saved pens and CRTC start. |
| 8 | OS4 / PNG GlowIcons (`ARGB` chunk) | Icon packs are plentiful, and today such icons fall back to the old image; needs a shared inflate. |
| 9 | Amiga YUVN | Complete small spec and unique to the Amiga; rare, so low risk and low reward. |
| 10 | MSX2 Graph Saurus SRI+PL7 | Baseline gap, popular in its day; trivial on top of SR7, held back only by lack of a sample. |

Runners-up: ZX SCR size variants (ULANext, 2/3 and 1/3 mono, MC4, SCA first frame), CPC overscan SCR and CPC+ palettes
(layouts unknown), FAXX (needs a T.4/T.6 codec), packed `.zx0`/Pletter screens (no signature).
