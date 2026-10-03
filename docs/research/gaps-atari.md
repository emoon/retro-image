# Atari 8-bit and ST/STE/TT/Falcon: formats we do not decode yet (gap survey, clean-room)

Scope: candidates for the Atari 8-bit and Atari ST/STE/TT/Falcon families that no registered decoder claims. That covers formats RECOIL lists and we skipped, and formats RECOIL lacks. Surveyed 2026-10-03 against `docs/formats.md`, `docs/coverage.md` and the two existing Atari research notes ([atari-8bit.md](atari-8bit.md), [atari-st-tt-falcon.md](atari-st-tt-falcon.md)). **No GPL/LGPL decoder source and no RECOIL source was opened.** Only prose docs, wiki pages and permissively licensed repositories were read. Licences are stated where known.

## 0. Method and caveats

- Format lists came from the [AtariForumWiki ST Picture Formats index](https://temlib.org/AtariForumWiki/index.php/ST_Picture_Formats) (81 entries; per-page raw wikitext at `index.php?title=<Page>&action=raw`), the [AtariWiki file suffix list](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix), the [Mad Team utilities page](https://madteam.atari8.info/index.php?prod=uzytki) and the [abydos supported-formats page](https://snisurset.net/code/abydos/supported.html). I read only the prose of those pages. abydos is a third-party viewer with an unstated licence, so its code was not read.
- Just Solve (fileformats.archiveteam.org / justsolve.archiveteam.org) refused connections (ECONNREFUSED) for the whole session. Facts attributed to Just Solve come from web-search snippets only and are marked "(snippet)".
- "How common" is an estimate. I could not query Demozoo, Atari Legend, Atarimania or scene.org counts. Treat the ratings (high/medium/low/rare) as informed guesses, not measurements.
- Signature column: **magic** = fixed bytes at a known offset; **size** = only a fixed file size; **ext** = extension only; **none** = no reliable way to tell from content.
- Difficulty: S = a day or less from a spec, M = a few days, L = needs reverse engineering or an emulator-style approach.
- The AFW page texts carry no licence statement beyond what [atari-st-tt-falcon.md](atari-st-tt-falcon.md) section 1 records (Pursell/Wessels additions public domain, original Baggett text redistributable non-profit). Using them for facts is fine.
- Already covered, so not repeated: everything in the `formats.md` Atari tables. That includes Spectrum 512 (SPU/SPC/SPS/SPX), NEOchrome and NEOchrome Master, DEGAS and DEGAS Elite, Tiny, CrackArt, Dali, Doodle, Photochrome, QuantumPaint, MPP, Canvas, Art Director, DUO and KID, Falcon true colour (TRP, TCP, FTC, XGA and others), TT DEGAS, and the 8-bit Mad Studio, APAC, HIP, TIP, CIN, RIP, MIC, XLP and G2F families.
- Searched and found nothing: "Rainbow Graphics" as an Atari ST format (no hit; may be a mix-up with Rainbow Arts or Rainbow Painter, which is C64 and already covered), Falcon "Rendition", "Photon" and "ZView" file formats (no format docs found). I record these as non-findings, not as proof that they do not exist.

## 1. Atari ST/STE: candidates

| Candidate | Ext | How common | Diff. | Spec / source | Samples | Signature |
|---|---|---|---|---|---|---|
| Whole-file packers on any ST picture (JAM Packer, StoneCracker, Atomik 3.5, Pack-Ice outside the few decoders that call it) | any | high (demos, disk mags, cracktro menus; most scene PI1/NEO/PC1 files were crunched before release) | M | see 1.1 | plentiful, any Demozoo ST graphics release | magic |
| STOS packed screen / Picture Packer | PP1, PP2, PP3, and screen banks inside MBK | high (STOS games and slideshows) | M-L | see 1.2 | Atari Legend STOS games, STOS disks | magic (but resolution flag unreliable) |
| STOS memory bank, sprite bank | MBK | medium | M | see 1.2 | STOS disk collections | magic |
| Cyber Paint Sequence | SEQ | medium (Cyber Paint animations; also used by some demos and slideshows) | M | see 1.3 | Atari Legend, textfiles.com ST CDs | magic |
| NEOchrome Animation | ANI | low-medium | S | see 1.4 | NEOchrome and Animator ST disks | weak (magic reportedly ignored) |
| Raw ST screen dumps | none, PIC, SCR, RAW, DAT, BIN | high in demo packs, but unlabelled | S for decode, hard for detection | see 1.5 | demo source trees, scene.org | size only |
| Signum! compressed image | IMC (also I__, sometimes PAC) | low-medium (German DTP, scanned logos) | M | see 1.6 | sdo-tool test files | magic |
| NeoDesk icon file | NIC | medium among ST users (icon packs) | S-M | see 1.7 | Atari-Forum "NeoDesk 4 Icon Disk" thread | magic for v3 only, size for v1/v2 |
| Animatic Film | FLM | low | S | see 1.8 | few | magic (`27182818`) |
| Lexicor Film, Video Master | FLM, VID, VSQ | rare | L | see 1.8 | few | ext |
| Animaster sprite bank | ASB | rare | S | see 1.9 | rare | none |
| ColorBurst II | BST | rare | L | see 1.9 | rare | weak |
| GFA Raytrace | SUL, SCL, SAL, SAH (SUH is plain DEGAS high) | low | L | see 1.9 | gfa.atari-users.net | magic (6-byte ASCII id) |
| TruePaint animation | TPA | rare | L | see 1.9 | rare | magic (`$54504100`) |
| DEGAS-style IFF from Deluxe Paint ST | IFF, LBM | medium | S | the IFF decoder exists on the Amiga side; RECOIL lists it under ST/STE too | common | magic |
| Ani ST scripts | SCR, STR | rare | L (vector script) | skip | rare | n/a |
| GEM metafile | GEM | low | L (vector) | skip | common but not bitmap | magic |
| STAD variants (PIC, SEQ) | PIC, SEQ | unclear | ? | abydos lists "STAD (.pac, .pic, .seq)". I found no layout for the last two. | none | ? |
| Signum! fonts | E24, P24, P09, L30 | low | M | only the Rust parser prose in sdo-tool; layout not read | few | magic |

### 1.1 Transparent packers

- Evidence from this repo: `pack_ice.rs` is called by only three decoders (`falcon.rs` TRP path, `blend.rs`, `spectrum.rs`). Pack-Ice's picture mode is implemented. Other pictures (PI1, NEO, PC1, TNY and so on) are not unpacked first, which is a gap, not a format.
- JAM Packer and StoneCracker 2.69 to 4.10 are in [Ancient](https://github.com/temisu/ancient) (BSD-2-Clause), which we already credit. Pack-Ice clones `SHE!`, `TMM!` and `TSM!` are listed there too.
- Atomik 3.5 (`ATM5`, `ATOM`): prose on the [AtariForumWiki Atomik Packer page](https://temlib.org/AtariForumWiki/index.php/Atomik_Packer) and in the [Packer/Depacker overview](https://temlib.org/AtariForumWiki/index.php/Packer/Depacker). The SourceForge project [atari-icedpck](https://sourceforge.net/projects/atari-icedpck/) has C depackers, but I did not find its licence. Treat it as all rights reserved until checked.
- Design note, not code: a single "unwrap known packer, then re-sniff" step in the container layer would fix every ST format at once instead of per decoder.
- Un-surveyed: Automation, Thunder, LSD and Pompey Pirate Packer variants. No permissive source seen. Mark as out of reach unless docs turn up.

### 1.2 STOS

- Packed screen: magic bytes `06 07 19 63`. The image is cut into slices and each slice is compressed separately. The encoder tries a few slice sizes and keeps the best. Bitplanes are stored separately, so the data is de-interleaved first. Per-resolution shuffling is described only in outline. Source: the [abydos Picture Packer page](https://snisurset.net/code/abydos/picturepacker.html). That page states no licence. abydos itself has no stated licence here, so use the page for facts only and do not read the program's code.
- **The compression algorithm itself is not specified in any page I read.** Expect reverse engineering from samples (difficulty M-L, not S).
- Picture Packer files (`.PP1`/`.PP2`/`.PP3`) are STOS packed screens without the MBK wrapper ([AFW page](https://temlib.org/AtariForumWiki/index.php?title=Picture_Packer_file_format&action=raw)). The STOS screen header's resolution flag is set to medium for every `.PP?` file, so the resolution must come from the extension.
- MBK sprite bank: header magic `$19861987`, three resolution parameter blocks (low, medium, high), frames with width in words, height, hotspot and a monochrome mask, plus a `PALT` block of 16 words ([AFW page](https://temlib.org/AtariForumWiki/index.php?title=STOS_Memory_Bank_file_format&action=raw)). Frames are often in semi-random order. AFW notes no compression. The raw-wikitext page does not mention the other bank types or the `06071963` screen magic, so the bank-type table is a gap in this survey. Also see the [Just Solve STOS memory bank page](http://fileformats.archiveteam.org/wiki/STOS_memory_bank) (could not be fetched).
- Uncertain: whether the MBK magic in the AFW page matches all STOS sprite banks (STOS banks may carry a different tag). Check against samples.

### 1.3 Cyber Paint Sequence (SEQ)

- Source: [AFW Cyber Paint Sequence page](https://temlib.org/AtariForumWiki/index.php?title=Cyber_Paint_Sequence_file_format&action=raw), also in the [EGFF Atari summary](https://www.fileformat.info/format/atari/egff.htm).
- 128-byte file header: magic `$FEDB` or `$FEDC`, version, frame count, speed in vblanks per frame. Each frame has a 128-byte header with palette, x/y offset, size, operation (copy or XOR), storage method and data length. Delta frames use a "change box". The control-word scheme is negative = that many literal words, positive = repeat count. Bitplanes are stored as vertical columns.
- A decoder could render a contact sheet of frames, as the existing multi-frame decoders do. We already decode the CEL sibling.

### 1.4 NEOchrome Animation (ANI)

- Source: [AFW page](https://temlib.org/AtariForumWiki/index.php?title=NEOchrome_Animation_file_format&action=raw); Just Solve NEOchrome Animation (snippet).
- 22-byte header: long `$BABEEBEA` (reportedly ignored), width in bytes, height in lines, size+10, x-1, y-1, frame count, speed, long reserved. Frames follow as raw screen-memory words.
- **No palette is stored.** Both sources say it is unclear where the palette comes from. A decoder would need a default palette or a sibling `.NEO` file. This lowers the value, so mark S but "palette unknown".

### 1.5 Raw ST screen dumps

- Demo sources and tools often hold plain 32000-byte low/medium/high-res dumps with no palette (palette in a separate file or in code), 32034-byte DEGAS-without-magic, and overscan or STE-extended dumps. Sizes for overscan are not in any doc I read. I did not find a page enumerating them.
- Value: high in raw demo disks. Risk: false positives (any 32000-byte file "decodes"), the same trap the C64 notes describe in the Wave 5 sample hunt in [README.md](README.md). Needs extension gating and a grey-ramp fallback palette. Recommend registering as a low-priority, extension-gated format, not content-sniffed.

### 1.6 Signum! IMC

- Source: the reverse-engineered [bimc0002 spec at sdo.dseiler.eu](https://sdo.dseiler.eu/formats/bimc) (no licence on the page) and the [AFW IMC page](https://temlib.org/AtariForumWiki/index.php?title=Signum!_file_format&action=raw), which says "no description of the compression exists". Header magic `bimc0002`, then compressed size, dimensions, chunk counts, bit-stream and byte-stream sizes, a final XOR value. Image is cut into 16x16 chunks in raster order. A bit-stream says which chunks exist. Four chunk strategies (direct 32 bytes, or 8x8 sub-chunk serialisation with optional single or rolling 4-byte XOR). A final row-wise XOR. Standard size 640x400 mono.
- Repository: [Xiphoseer/sdo-tool](https://github.com/Xiphoseer/sdo-tool). The `signum` parsing crate is MIT OR Apache-2.0 (readable). The CLI and web parts are AGPL-3.0 (do not read).
- Extension clash: IMC bitmaps sometimes use `.PAC`, the same extension as STAD. Magic `bimc` separates them.

### 1.7 NeoDesk icons (NIC)

- Source: Just Solve NeoDesk icon (snippet). Three versions. v1: exactly 2088 bytes = 9 icons of 232 bytes, no header. v2: any count of 244-byte icons, no header. v3: ASCII `.NIC` signature. I did not see the internal layout of an icon (mask, image, label, drive letter). It is not in any page I read.
- Per-icon layout therefore needs the Gribnif manual or samples. Rated S-M only if the layout is found.
- abydos supports it (licence unchecked, code not read).

### 1.8 Films

- Animatic FLM: 32-word header (frame count, 16-word palette, speed 0-99, direction flag, end action, size, version, magic `27182818`, three reserved words). Low resolution only, frames as raw screen-memory words ([AFW page](https://temlib.org/AtariForumWiki/index.php?title=Animatic_Film_file_format&action=raw)).
- Lexicor FLM and Video Master (FLM, VID, VSQ) share the `.FLM` extension. I did not read their pages, so the layout is unknown here. FLM is therefore ambiguous between three programs; do not register the extension until the header magics are checked.
- Cybermate/CAD-3D Delta (`.DLT`): headerless stream of XOR deltas against an unknown base screen. Not renderable on its own. Skip.
- Stereo CAD-3D / Imagic films: Imagic (`IMDC`) is already covered as IC1-IC3.

### 1.9 Remaining small items

- ASB: 38-byte header with frame count, maximum size and a 16-word palette, then per-frame width-1, height-1, plane count (always 4) and screen words ([AFW](https://temlib.org/AtariForumWiki/index.php?title=Animaster_Sprite_Bank_file_format&action=raw)). Easy but rare.
- BST (ColorBurst II): 200 palettes of 16 words (non-standard bit order 3210), flags word, compressed data. Versions 1.2 and 1.3 use different, undocumented compressions ([AFW](https://temlib.org/AtariForumWiki/index.php?title=ColorBurst_file_format&action=raw)). Needs reverse engineering.
- GFA Raytrace: 6-byte ASCII id plus CR/LF, 3-byte size factor (1, 2, 4, 8). Uncompressed sizes 50408, 25208, 12608 and 6308 bytes. Compression and raster layout are undocumented. 68K decompression code is said to be at <http://gfa.atari-users.net> (licence not checked, so do not read it) ([AFW](https://temlib.org/AtariForumWiki/index.php?title=GFA_Raytrace_file_format&action=raw)).
- TPA (TruePaint animation, Falcon): 128-byte header (ID `$54504100`, version, palette size, size, planes, compression always 0, image size, speed, length), palette, full first frame, then incremental frames whose structure is unknown ([AFW](https://temlib.org/AtariForumWiki/index.php?title=TruePaint_Animation_file_format&action=raw)). The first frame alone could be shown (S), at the risk of looking like a still. Falcon sample availability is poor.

## 2. Atari Falcon and TT

Searches for Falcon-native formats beyond the 26 covered rows found nothing new with a public spec. Related notes:

- Apex Media / Apex Animator (Black Scorpion, 1994/95) mostly produces standard FLI/FLC files (that is the PC family; [Atarimania](https://www.atarimania.com/utility-atari-st-apex-image-viewers_45258.html)).
- A search result named a DuneGraph "DGU" extension next to DG1; I found no confirmation. Uncertain, check the DuneGraph disk.
- TT: only the three DEGAS-TT rows. No TT-specific paint format found. TT high-resolution (1280x960 mono) raw dumps exist by hardware; not evidenced as a file format.
- Transparent packers (section 1.1) matter here too: Falcon TRP is already Ice-aware.

## 3. Atari 8-bit: candidates

### 3.1 RECOIL-listed formats still open

These are already tracked in [atari-8bit.md](atari-8bit.md), listed here only to give one place to see the whole backlog. Per the Wave 5 notes, RM0/RM1/RM3 and PIX have no samples and A4R, PGR and HPM have unresolved packer details.

| Format | Ext | Status per existing notes | Diff. |
|---|---|---|---|
| Rambrandt | RM0, RM1, RM3 | Koala-style header; no samples | S-M |
| Graphics Magician Picture Painter (Atari) | SPC | vector command stream; Atari layout differs from Apple II | M |
| Grass' Slideshow | HPM | `00 v n` runs unpack; unresolved | M |
| PowerGraphics | PGR | memory image from `$8206`, `PowerGFX` at offset 8 | M |
| Anime 4ever | A4R | packer unresolved | M |
| Graph2Font vertical scroll | VSC+G2F | text list plus companions; API cannot express it | M |
| Atari 8-bit PIX | PIX | probably a GR15 dump; size unknown | S |

### 3.2 Beyond RECOIL

| Candidate | Ext | How common | Diff. | Spec / source | Samples | Signature |
|---|---|---|---|---|---|---|
| HiRes Player Missile | HPM | low | S-M | Just Solve (snippet): files are exactly 19203 bytes; abydos supports it. I found no layout page. Possibly a 320-pixel mode drawn with player/missile overlay (uncertain). [Hardware refs](atari-8bit.md) cover PMG layout. | dexvert sample folder (via Just Solve) | size |
| Self-displaying XEX pictures (RastaConverter output and similar) | XEX | high in the modern scene (Demozoo graphics; RastaConverter emits 20-22 KB executables) | L | [RastaConverter](https://github.com/ilmenit/RastaConverter) (GPL? licence not checked, do not read); [Ironman guide](https://atariwiki.org/wiki/Wiki.jsp?page=Ironman+Atari). Needs the display list, DLIs and per-line colour writes. Realistic only with a small 6502+ANTIC/GTIA model written from the Altirra manual. The Altirra emulator is GPL, so only the PDF is usable. | Demozoo, atari.org, AtariAge | weak (generic binary-load header `FF FF`) |
| TIP Animator animation | ANM, ANM.DAT, DICXX.DAT | rare | L | [Mad Team page](https://madteam.atari8.info/index.php?prod=uzytki): FLI/FLC-like compression, GR9/10/11. The format is not described there. | Mad Team | unknown |
| RIP compressed variants | RIP | rare | M-L | Visage 2.7 reads RIP files compressed with Lempel-Ziv-Szymanski or Shannon-Fano ([Mad Team](https://madteam.atari8.info/index.php?prod=uzytki)). The compression is not described. Existing notes list RIP as Partial; check which variants the corpus holds. | Mad Team | magic `RIP` |
| Extension aliases G15, GR15, GRA, DGS, RLE | as named | low | S | [AtariWiki file suffix list](https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix): G15 = Graphics 15 160x192x4 raw, GRA = APAC 80x96 (same as APC). These extensions are not registered. DGS ("Degas", unspecified) and RLE ("variable, compressed") are not specified there. | easy | size |
| VBXE-native pictures | ? | rare | ? | Searches found only G2F VBXE and DAP, which are covered | none | none |

Notes on "beyond RECOIL" for 8-bit: this family is already almost exhausted by RECOIL's list. Wave 4 and wave 5 took 124 of 143 rows. The real remaining value is XEX pictures and the shrinking list of packer-bound formats (SFDN family is covered per `formats.md`).

## 4. To avoid (adds to the existing lists)

- **Atari800 / Altirra source** (GPL): the Altirra Hardware Reference Manual PDF is fine, the emulator is not.
- **RastaConverter source**: licence not checked, treat as unusable. Its help text and README were read for facts only.
- **sdo-tool CLI** (AGPL-3.0): only its `signum` parsing crate (MIT OR Apache-2.0) is readable.
- **abydos** (<https://snisurset.net/code/abydos/>): licence not stated on the pages read. Prose pages only.
- **atari-icedpck** (SourceForge): licence not found. Prose on Atomik from AFW instead.
- **GrafX2 Atari ST picture-format docs** (appeared in search results; GrafX2 is GPL-2): not read.
- **TipTools** (epi/TipTools): GPL-2.0 (checked on COPYING). Do not read.
- **gfa.atari-users.net 68K sources**: licence unchecked.

## 5. Top-10 ranking for this family

1. **Universal transparent-packer layer for ST pictures (Pack-Ice everywhere, plus JAM and StoneCracker from Ancient BSD-2).** One container-level change unlocks packed versions of dozens of already-decoded formats, and scene releases are mostly crunched. Atomik needs a source that is not yet vetted.
2. **STOS packed screens, PP1-3 and MBK screen banks.** STOS games and slideshows are very common, and the structure is partly documented. The compression itself needs reverse engineering from samples.
3. **Cyber Paint Sequence (SEQ).** A complete public spec with magic `$FEDB/$FEDC`, and we already have the CEL half. A contact-sheet render is straightforward.
4. **Signum! IMC (bimc0002).** A full reverse-engineered spec exists and a permissive parser crate can be read for cross-checks. It is a single-resolution mono format, so the output is easy to verify by eye.
5. **Raw ST screen dumps, extension-gated.** Very common in demo trees and trivial to decode, but only safe behind extension gating because size sniffing gives false positives.
6. **STOS sprite banks (MBK).** Documented header and `PALT` palette. Frame ordering and bank-type coverage are uncertain, so validate with samples first.
7. **NeoDesk icons (NIC).** Sizes and v3 magic are known and icon packs exist, but the per-icon layout still has to be found.
8. **NEOchrome Animation (ANI).** The header is fully specified and the decoder is S, but the missing palette limits usefulness.
9. **HiRes Player Missile (HPM, 19203 bytes).** A cheap fixed-size 8-bit format that nobody covers, but no layout page was found, so it depends on samples.
10. **Self-displaying XEX pictures.** The highest-value 8-bit item for the modern scene, but L difficulty because it needs a small display-list and colour-write model.

Runners-up: Animatic FLM (S, spec known, rare), extension aliases G15/GRA (S, free), compressed RIP variants (M-L), GFA Raytrace and ColorBurst (L, compression undocumented), TPA first frame (S, poor sample supply).
