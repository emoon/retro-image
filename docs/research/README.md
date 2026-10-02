# Format documentation survey

Survey of public, non-GPL documentation for the image formats RECOIL supports,
as the basis for a clean-room, permissively licensed Rust implementation.
Format list taken from <https://recoil.sourceforge.net/formats.html> (552 formats,
51 platforms — a list of facts; no RECOIL code was read). Surveyed 2026-10-01.

## Clean-room rules

- Never read RECOIL source, in any language, port or fork.
- Never read GPL/LGPL decoder code, or code derived from RECOIL (see the per-file "To avoid" lists).
- Code with no licence, or an unchecked one, counts as all rights reserved: read its prose docs, not its code.
- Running the RECOIL binary and comparing its pixel output is fine.
- Record the source of every decoder in the per-format notes.

## Docs quality legend

- **Spec**: public layout description sufficient to implement.
- **Partial**: some layout info (size, header, mode); gaps remain.
- **Hardware-only**: no file doc, but evidently a raw dump of a known video mode.
- **None**: needs reverse engineering from sample files.

## Summary

Counts are per table row; some rows group several extensions.

| File | Platforms | Rows | Spec | Partial | HW-only | None |
|---|---|---|---|---|---|---|
| [atari-st-tt-falcon.md](atari-st-tt-falcon.md) | Atari ST/STE, TT, Falcon | 126 | 99 | 15 | 3 | 9 |
| [commodore.md](commodore.md) | C64, VIC-20, C16/Plus4, C128 | 118 | 59 | 25 | 4 | 30 |
| [atari-8bit.md](atari-8bit.md) | Atari 8-bit, VBXE, Portfolio | 143 | 19 | 61 | 6 | 57 |
| [sinclair-cpc-bbc-misc.md](sinclair-cpc-bbc-misc.md) | ZX Spectrum family, ZX81, Timex, SAM, CPC, BBC, Oric, Electronika, Vector-06C | 63 | 33 | 12 | 10 | 8 |
| [msx-japanese.md](msx-japanese.md) | MSX/MSX2/2+/V9990, NEC PC-80/88/98, X68000, FM Towns | 56 | 26 | 21 | 0 | 9 |
| [amiga-apple-misc.md](amiga-apple-misc.md) | Amiga, Apple II/IIGS/Mac, PC, PlayStation, Psion, HP 48, Tandy, TRS-80/CoCo | 40 | 23 | 5 | 4 | 8 |
| **Total** | | **546** | **259** | **139** | **27** | **121** |

About 47% of the formats are fully specified. Another 30% are partly documented or derivable
from hardware docs. About 22% need reverse engineering, and more than half of those are Atari 8-bit.

## Caveats

- The shared web-search limit (200 calls) ran out near the end of several agents' runs.
  Some obscure formats rated **None** were never searched for individually, so they may be
  better documented than the rating says.
- The Just Solve wiki (fileformats.archiveteam.org), msx.org/wiki and a few other sites
  could not be fetched. Links taken only from search results are marked as such in the files.

## Cross-cutting blockers

| Blocker | Formats affected | Status |
|---|---|---|
| **FLF** (Turbo Rascal Syntax Error) | ~8 platforms | Only defined in GPL-3 TRSE source. Reverse engineer from TRSE-produced files. |
| **SFDN packer** (Atari 8-bit) | APP, APS, G9S, HPS, ILS, INS, PLS, SFD | Undocumented. One reverse-engineering effort unlocks all 8. |
| **PackBytes** (Apple IIGS) | APF, Paintworks, $C0/0001, 3201, packed DHGR | Documented in Apple IIGS Toolbox Reference vol. 1 (not found online); CiderPress II (Apache-2.0) implements it. |
| **SEV** (SevenuP, Spectrum) | SEV | Only defined in GPL-2 SevenuP source. |

## Permissive references (code may be read)

- **Deark** (MIT-style): broad coverage across Atari ST, Amiga, Apple, PC and more.
- **GoDot** (MIT): loaders and docs for ~60 C64, VIC-20, Plus/4 and C128 formats.
- **CiderPress II** (Apache-2.0 code; CC BY-SA 4.0 docs): Apple II and IIGS formats.
- **Mad Studio** (MIT): Atari 8-bit Mad Studio formats, with a formats PDF.
- **Ancient** (BSD-2): Pack-Ice and other Atari ST/Amiga packers.
- **monobit** (MIT): fonts, including Daisy-Dot.
- **bitplane/datatypes** (MIT), **oxideav-iff** (MIT, Rust), **Kaitai psx_tim.ksy** (CC0), **zx-image** README (CC0), **SpectraLab guide** (MIT).

## Decisions needed

- **bitplane/datatypes PRs #44/#46** (Atari ST/TT/Falcon loaders) were checked against
  RECOIL's *output*, and the PRs state that RECOIL's source was not read. Decide whether that
  provenance is acceptable before anyone reads that code.
- **roytam1/stb_gemras** says its extended GEM IMG coverage came from RECOIL code. Treat it as tainted.
- **PNGCrushCS** may be derived from RECOIL. Treat it as tainted until checked.
- **zx-image** (CC0) and **SpectraLab** (MIT) are the main Spectrum-family sources, but it is not
  known whether their authors consulted RECOIL. Use them for facts and check those facts against
  sample files; don't copy their code.

## Existing Rust crates

No Rust equivalent of RECOIL exists. `format198x-*` / `play198x-core` cover 4 formats and are
GPL-2.0+ (do not read). `oxideav-iff` (MIT) covers IFF/ILBM only. The crate name `retroimg`
is already taken on crates.io by an unrelated project.

## Suggested order

1. **Pipeline and test harness**: core decode API, PNG CLI, and pixel comparison against `recoil2png`.
2. **Well-specified, high-value formats**: Atari ST (DEGAS, NEOchrome, Spectrum 512, Tiny), C64 (Koala, Art Studio, FLI family),
   ZX Spectrum SCR, Amiga IFF/ILBM/HAM, Apple II/IIGS, MSX screen dumps, MAG/PI.
3. **Partial and hardware-only formats**: fill gaps by comparing against RECOIL's output.
4. **Reverse engineering**: SFDN first (unlocks 8 formats), then FLF, then the long tail of Atari 8-bit formats.

## Wave 5 sample hunt (2026-10-02)

We searched for samples of every uncovered format that had none. A file only counted if
`recoil2png` decoded it, and each one found is recorded in `corpus/extra/<group>/MANIFEST.tsv`.

Found: EMC (1, Magic Disk 64 12/1990), MWIN (5, Art Studio 2 disk), MUI (5, CSDb 93314),
MUP (8, CSDb 35737), KT4 (3, Sembiance) and 4 extra PC-98 Pi files (RECOIL ticket #12). ARV
was never missing: `TAIHO (from Gr.lzh).ARV` in RECOIL's own set is a real sample; only the
files under `hostile/` fail. Four NUFLI `.nuf` files from CSDb 93314 also exist; they weren't saved.
recoil2png rejects four other Art Studio 2 windows on the same disks (PORTAL, FLAG, WINDOW,
MOAT SECTION). They may be valid files that RECOIL fails on.

Not found, after name-scanning about 21,000 CSDb releases, archive.org's d64 collection,
funet, c64.rulez.org and Sembiance:

- C64 BDP, ESH, FED, FLM, FP, HCB, HLE, ILE, MUF, MWI, NUP, PDR, RPH, SH1/SH2/SHE/SHS, UIF,
  VHI, XFL, ZOM. Only the editors turn up, never saved pictures. Don't brute-force: recoil2png
  accepts many of these on file size alone (BDP takes any 10003-byte file, SHS any 14338-byte
  file, ILE any 4098-byte file), so random PRGs "decode". Real samples would have to be made in
  VICE with the editors. Mufflon (CSDb 93314) can write MUF/NUF/MUI, but it ships source with a
  GPL-like licence, so treat it as a black box only.
- Turbo Silver RGB8/RGBN: Aminet has only the datatype. The spec is public: the
  [AmigaOS wiki](https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data) and
  `RGBN-RGB8.doc` inside Aminet's `util/dtype/RGBx_DT.lha`. We could write a decoder from the spec,
  but there'd be nothing to test it against.
- Rambrandt RM0/RM1/RM3 (savetz/RAMbrandt and the Antic disks hold no pictures), Atari 8-bit
  PIX, Graph Saurus SRI (only the program disk), PC-88 KTY, FM Towns PI.
- SpecSCII ZXS: only SpectraLab's two `.specscii` exports, which RECOIL rejects. It's still
  unclear what RECOIL's ZXS layout is.

Prose docs found along the way: GoDot's [Magic Disk page](https://www.godot64.de/german/l_mdisk.htm)
for EMC (GoDot is MIT; its sizes give 17410 bytes, the sample has 17412), the Codebase64 grafix
spec list (Wayback) for Hireslace Editor, a [note.com article](https://note.com/ftz/n/n84d9dd98c1e2)
on Kitty (the `rururutan/ifkty` plugin has no checked licence; don't read it), and a Japanese Pi
document with a machine translation attached to RECOIL ticket #12 (prose, licence unstated).
