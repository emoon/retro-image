# Format documentation survey: Thomson MO5, MO6, TO7, TO7/70, TO8, TO9

Clean-room research notes. RECOIL does not support Thomson, so these formats were not in the
original survey. This file lists only public format descriptions, hardware manuals and
permissively licensed references. No GPL/LGPL decoder or emulator code was read. Surveyed 2026-10-02.

## Summary

| Docs quality  | Count | Formats |
|---------------|------:|---------|
| Spec          | 3 | MAP (BASIC `SAVEP`), Graffiti `.M16`/`.M04`/`.M02` + palette files, MAP TO-SNAP/PPM trailers |
| Partial       | 1 | PHO (two conflicting descriptions) |
| Hardware-only | 1 | Raw screen dumps (RAMA/RAMB, 8000 + 8000 bytes) |
| None          | 0 | |

Implemented (`crates/retro-image/src/platform/thomson*`): MAP (by extension and by content), Graffiti
`.M16`/`.M04`/`.M02` with `.D16`/`.D04`/`.D02` (and `.MAP` with `.DST`) palettes, PHO.
Not implemented: raw screen dumps (no distinctive size or header: video memory is two 8 KB banks
saved by `SAVEM` at arbitrary addresses).

## Sources

- **Prehisto, "Les fichiers graphiques Thomson"**, a ContacThoms bulletin article republished on
  Collection Thomson (Ghislain Fournier and François Mouret):
  <http://web.archive.org/web/20251005163132/http://collection.thomson.free.fr/code/articles/prehisto_bulletin/page.php?XI=0&XJ=13>.
  The full MAP format: binary file records, mode byte, size, column-wise RLE with a worked example,
  the reference 6809 unpacker (DECMAP), the TO-SNAP and PPM trailers, Graffiti extensions and
  palette files, and PHO. Also the pixel layout of all four screen modes.
- **Prehisto, "La piste 20"** (same site):
  <http://web.archive.org/web/2020/http://collection.thomson.free.fr/code/programmation/prehisto_documentation/page.php?XI=1&XJ=4>.
  Thomson DOS: FAT at track 20 sector 2, directory in sectors 3-16, 32-byte entries, blocks of
  8 sectors. Used only for extracting samples from `.fd` disk images. Files hold 255 data bytes per
  256-byte sector (not in the article; every extracted MAP file's length field agrees only then).
- **MAME** Thomson driver, `src/mame/thomson/` (every file `license:BSD-3-Clause`,
  `copyright-holders:Antoine Mine`; licence headers checked before reading):
  `to_video.cpp` (pixel decoding per video mode: the TO7/70 colour byte with inverted pastel bits,
  bitmap 4, bitmap 16 with RAMA then RAMB across the line, 80 columns; the MO5 and TO9 variants of
  the 40-column colour byte) and `thomson_m.cpp` (the 16 power-on palette values, the 12-bit `0BGR`
  palette format and the EF9369 gamma of 2.8). `palette.rs` and `video.rs` carry MAME's licence text.
- **dcmoto.free.fr** (Daniel Coulom): software archives (`.fd` disk images) and DCMOTO screenshots of
  each program, used as reference renders. Technical manuals are there as DjVu scans
  (<http://dcmoto.free.fr/documentation/index.html>); DjVu tools were not available, so they were not read.
- **logicielsmoto.com** forum: Samuel Devulder's posts in "Convertion images & photos" (topic 383,
  read via the Wayback Machine because the live topic fails with an SQL error) give his empirical
  TO8 levels for the 16 channel values (0, 100, 127, 142, 163, 179, 191, 203, 215, 223, 231, 239,
  243, 247, 251, 255). We use MAME's gamma curve instead; DCMOTO's levels (seen in its screenshots:
  122, 183, 219, 249 for values 2, 6, 10, 14) lie between the two.

## Formats

| Ext | Name | Docs | Source | Notes |
|---|---|---|---|---|
| MAP | BASIC 128/512 `SAVEP` picture | **Spec** | Prehisto article | `00 len len 00 00`, mode (`00` 40 col / bitmap 4, `40` bitmap 16, `80` 80 col), columns-1, rows-1, column-wise RLE (`n v` run, `00 n` literal; runs cross columns), banks closed by zeros, optional trailer, `FF 00 00 00 00`. An odd pad byte may follow the closing zeros (`SAVEP` saves an integer array). |
| MAP trailer | TO-SNAP | **Spec** (unverified) | Prehisto article | 40 bytes before the closing record: `SCRMOD`, border, `CONSOLE` mode, 16 palette words, `A55A`. The article's BASIC reader reads it in the opposite order; we follow its offset table. No sample found. |
| MAP trailer | PPM | **Spec** (unverified) | Prehisto article | 36 bytes: 16 palette words (colour 15 first), `CONSOLE` mode, `HL`. No sample found. |
| M16, M04, M02 (+ D16, D04, D02); MAP + DST | Graffiti (Interlude / Free Game Blot) | **Spec** | Prehisto article, `GARDEN.D16` | MAP files named by mode; text palette: width-1, height-1, then 16 x (B, step, G, step, R, step) with channels pre-shifted; entry = B+G+R. |
| PHO | Digitised photo | **Partial** | Teo-Drive n°3 samples + screenshots; Prehisto article | Teo-Drive's PHO files are MAP files with bitmap 4 banks shown in greys `EEE AAA 666 222` (pixel-exact against DCMOTO screenshots). Prehisto describes TO-PHOTO's PHO as bitmap 16 with a black-to-white palette; none found. |
| (none) | Raw RAMA/RAMB dumps | **Hardware-only** | MAME `to_video.cpp` | 2 x 8000 bytes; saved with `SAVEM` at any address, no fixed size or header. Not implemented. |

### Display

- 40 columns (TO7/70, TO8, TO9 in TO7/70 mode): 320x200, RAMA "forme" bits (MSB left), RAMB colour
  byte per 8 pixels: foreground `((b >> 3) & 15) ^ 8`, background `((b & 7) | ((b >> 4) & 8)) ^ 8`.
  The MO5/MO6 use `b >> 4` / `b & 15` and the TO9's own mode doesn't invert the pastel bit; a MAP
  file doesn't say which machine made it, so all 40-column MAP files are shown with the TO7/70
  encoding, as the article states ("codage BVR TO7").
- Bitmap 4: 320x200, colour = 2 x RAMA bit + RAMB bit.
- Bitmap 16: 160x200, two pixels per byte (high nibble left), RAMA byte then RAMB byte; shown 2x1.
- 80 columns: 640x200, RAMA byte then RAMB byte, set bit = colour 1; shown 1x2.
- Palette: 12-bit `0BGR`, power-on values `000 00F 0F0 0FF F00 F0F FF0 FFF 777 33A 3A3 3AA A33 A3A EE7 07B`,
  level = `255 * (v / 15) ^ (1 / 2.8)` (EF9369; MAME).

### Known limits

- Mode byte `00` is shared by 40 columns and bitmap 4, and one Teo-Drive n°2 viewer even saves
  bitmap 16 pictures with it (`IMAGE1.MAP`, `IMAGE2.MAP`: RAMA plane then RAMB plane). Without a
  TO-SNAP trailer or a telling extension (`.M04`, `.PHO`) we show 40 columns. Statistics (RAMB
  diversity, nibble smoothness) did not separate the cases on the samples, so there is no guessing.
- Most bitmap 16 pictures in the samples have no palette: their viewer program sets it. They are
  shown with the power-on palette.

## Samples

`corpus/extra/thomson/dcmoto/` (38 files, `MANIFEST.tsv`): Graffiti, Table à Dessin, and the
Teo-Drive n°1-6 disk magazines (A.S.C.I., 1991), extracted from dcmoto's `.fd` images with a
throwaway script. 18 of them match a DCMOTO screenshot pixel for pixel up to palette.

## Blocked or unavailable

- pulkomandy.tk (PulkoMandy's Thomson wiki, which has MAP notes): robots.txt disallows AI agents
  (`ClaudeBot`, `anthropic-ai`, `Content-Signal: ai-input=no`). Not read, not even via the Wayback Machine.
- Collection Thomson's current host (ct.ghislain.fr) has no Wayback snapshots; the older
  collection.thomson.free.fr pages are archived.
- cjoint.com links to Devulder's converted MAP files (`borderlands.zip`, `xthomsonmap.zip`) are
  dead and not archived.

## To avoid (GPL code: do not read)

| Project | License | Relevance |
|---|---|---|
| RECOIL | GPL | No Thomson support anyway. |
| [GrafX2](http://grafx2.chez.com/) | GPL-2 | Reads and writes Thomson MAP (with TO-SNAP), has Thomson palettes. Its prose docs are fine. |
| [Theodore (libretro)](https://github.com/Zlika/theodore) | GPL-3 | Thomson emulator. |
| [Teo / teo-ng](https://github.com/sam-itt/teo-ng) | GPL-2 | Thomson TO8 emulator: video modes and palette. |
| Samuel Devulder's Lua converters (logicielsmoto.com attachments, GrafX2 scripts) | No licence found / bundled with GrafX2 | MAP writers; read only his forum prose. |
| DCMOTO | Freeware, no source | Run-only; its screenshots are used as reference renders. |

**Safe to read:** MAME's `src/mame/thomson/*` (each file BSD-3-Clause; check the header of any other
file first). Keep MAME's licence text in files derived from it.
