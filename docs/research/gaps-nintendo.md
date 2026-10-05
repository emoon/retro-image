# Nintendo consoles and handhelds: formats we do not decode yet (gap survey, clean-room)

Scope: NES/Famicom, SNES, Game Boy / Color, Game Boy Advance, Virtual Boy, Nintendo DS/DSi, 3DS, N64 and
GameCube/Wii. The question is which image or graphics formats on these platforms have a deterministic pixel
decode and are not in the registry yet. RECOIL's format list has no Nintendo console at all (checked
2026-10-05 against <https://recoil.sourceforge.net/formats.html>: the platform list jumps from FM Towns to
HP 48 to MSX, and the only console row is PlayStation TIM), so there is no oracle for anything below.
Each candidate says how it can be checked instead. Surveyed 2026-10-05. No RECOIL source and no GPL/LGPL
decoder source was opened. Prose docs, wiki pages and permissively licensed code were read; licences are
stated where checked.

## 0. Method, tags and limits

Confidence tags follow the other `gaps-*.md` files, plus one more:

- **[fetched]**: I read the page in this session. Raw text for GBATEK, YAGCD, mkwiiki, the Tilemap Studio help and the
  repository files; a model summary (the fetch tool) for the nesdev, SNESdev, Pan Docs, Flipnote wiki, Shonumi and
  Kaitai pages. Summary-level numbers were spot-checked against samples where I had one (the PPM thumbnail, the
  printer packets) and are otherwise as reliable as that summary.
- **[snippet]**: only a search-result summary or a summarising fetch was seen.
- **[memory]**: from my own recall, not checked. A lead, not a spec.
- **[verified]**: on top of [fetched], I decoded a real file in the scratchpad and looked at the picture or compared
  it with a reference image. These claims were tested, not just read.

Difficulty: S = a day or less from a spec, M = a few days or one risky detail, L = reverse engineering or an
emulator-style model. "Sig" = what content detection could use.

Limits that shaped the work:

- The shared WebSearch quota (200 calls) ran out about halfway, so the later sources were reached by direct
  fetch only. Some lookups I wanted (N64, Virtual Boy, Tile Molester, YY-CHR) stay [memory].
- The GitHub REST API (60 calls/hour per IP) was exhausted by the team. File counts for repositories come from
  `git clone --depth 1 --filter=blob:none` tree listings (no blobs downloaded) and `raw.githubusercontent.com`.
- GBATEK (<https://problemkaputt.de/gbatek.htm>, 5 MB) was downloaded once into the scratchpad and read
  locally. It has no licence notice (copyright Martin Korth), so it is used for facts only. It turned out to be the
  single best source here: it documents the DS banner, DS Nitro 2D and 3D files, the 3DS SMDH, CLIM, CTPK and
  CIA formats, and Flipnote PPM.
- `romhacking.net` returned 403, `shiru.untergrund.net` was unreachable, `wiibrew.org` has no TPL page worth
  reading. The Wayback Machine was not needed.
- Nothing was downloaded into the repository or `corpus/`. Sample files were fetched only to test claims.

### 0.1 What already exists

Registry (`docs/formats.md`): NES CHR, NAM, NSS; Game Boy Camera SAV, GBR, GBM; PlayStation TIM. Earlier notes:
[next-zx-misc.md](next-zx-misc.md) section 3 (the NES and Game Boy decoders) and
[gaps-others.md](gaps-others.md) section 2.6 (raw SNES, Mega Drive, PC Engine and Neo Geo tile dumps, ranked 18
and "low"). Nothing below repeats those, except B3 and V1-V6, which extend them. `Image` is RGB only (no alpha), and
`codec/` already has `inflate`, `packbits` and several packers, so a ZIP wrapper or a GBA BIOS decompressor are small
additions.

## 1. Candidate list

| ID | Candidate | Platform | Ext / magic | Diff. | Relevance | Verified here |
|---|---|---|---|---|---|---|
| A1 | ROM banner icon (and raw banner) | DS, DSi | `.nds .dsi .srl`; header logo CRC | S | high | yes, real ROM |
| A2 | SMDH icon in SMDH, 3DSX, CIA | 3DS | `SMDH`, `3DSX`; CIA by header | S (CIA M) | medium-high | yes, real 3DSX |
| A3 | Banner BNR and memory-card GCI | GameCube | `BNR1`/`BNR2`; `.gci` | S / M | low-medium | no samples |
| A4 | iNES / NES 2.0 ROM, CHR-ROM as tile sheet; UNIF | NES | `NES\x1A`; `UNIF` | S | high | header parsing only |
| B1 | rgbgfx outputs: `.2bpp .1bpp`, `.tilemap .attrmap .pal` | Game Boy | ext only | S / M | medium | fixtures fetched |
| B2 | GBA/DS headerless dumps and BIOS-compressed streams | GBA, DS | ext only | S-M | low-medium | spec fetched |
| B3 | SNES planar and Mode 7 dumps | SNES | ext only | S + options | low | spec fetched |
| C1 | Nitro 2D resources NCLR, NCGR, NSCR (NCBR, NBFx) | DS | `RLCN RGCN RCSN` | M | medium | spec + BSD-2 code |
| C2 | Nitro 3D textures BTX0 (`.nsbtx`, TEX0 in `.nsbmd`) | DS | `BTX0`, `BMD0` | M | low-medium | spec fetched |
| C3 | TPL texture palette library (also BTI, TEX0 later) | GameCube, Wii | `00 20 AF 30` | M | medium | spec fetched |
| C4 | CLIM/FLIM, CTPK, CTXB textures | 3DS | `CLIM` footer, `CTPK` | M (ETC1 +M) | low-medium | spec fetched |
| D1 | Flipnote Studio PPM (thumbnail, then frames) | DSi | `PARA` | S / M | medium | yes, real files |
| D2 | Game Boy Printer packet captures (text) | Game Boy | content only | S | low-medium | yes, 0 pixel diffs |
| D3 | Emulator save-state thumbnails (BizHawk) | any | ZIP + BMP | S | low | source read only |
| V1-V6 | Variants of existing formats | NES, GB | see section 6 | S | mixed | partly |

Ranking is in section 8.

## 2. Icons and banners inside ROMs and executables

These are the most useful for a thumbnailer. People have these files, they carry the picture Nintendo's own menus
show, and the signature is strong. Candidates are grouped here, not ranked; the ranking is
in section 8.

### A1. DS and DSi ROM banner icon

- What: every `.nds` has an "icon/title" banner whose offset is the u32 at header `0x68` (0 = none). The 32x32
  icon is what the DS menu and flashcart menus show. DSi-enhanced banners (version `0x0103`) add 8 animation
  frames. [fetched, GBATEK "DS Cartridge Icon/Title"] and [verified].
- Ext / Sig: `.nds`, `.dsi`, `.srl` ([memory] for `.srl`); raw banners are `.bnr` or `banner.bin` (no standard
  name). ROM signature: u16 at `0x15C` is `0xCF56` (CRC-16 of `0x0C0..0x15B`, the Nintendo logo) and u16 at `0x15E`
  equals the CRC-16 of `0x000..0x15D`. CRC-16 here is the BIOS one: init `0xFFFF`, reflected polynomial `0xA001`.
  Both held on two real ROMs ([verified]). Raw banner: version in {1, 2, 3, `0x0103`}, and u16 at `+2` equals the
  CRC-16 of `+0x20..+0x83F`. Strict enough for `.signature()`.
- Layout:
  - `+0x000` u16 version; `+0x002` CRC16 (`0x20..0x83F`); `+0x004` (`0x20..0x93F`, v2+); `+0x006` (`0x20..0xA3F`, v3+);
    `+0x008` (`0x1240..0x23BF`, v0103).
  - `+0x020` icon bitmap, 0x200 bytes: 32x32 pixels, 4bpp, 4x4 tiles of 8x8, 32 bytes per tile, tiles in raster
    order. Low nibble is the left pixel ([verified]; the doc does not say).
  - `+0x220` palette, 16 colours, u16 BGR555 (`bit 0-4` red); colour 0 is transparent.
  - `+0x240` titles, 8 languages x 0x100 bytes UTF-16 (metadata only).
  - DSi v0103 only: `+0x1240` 8 animation bitmaps (0x200 each), `+0x2240` 8 palettes (0x20 each), `+0x2340` sequence of
    u16 tokens (bit 15 flip V, bit 14 flip H, bits 13-11 palette, bits 10-8 bitmap, bits 7-0 duration in 1/60 s;
    first token 0 means "use the static icon"). Banner size is `0x23C0`, given at DSi header `0x208`. [fetched]
    only; no DSi-animated sample was tested.
- Checked on: `GodMode9i.nds` and `GodMode9i.dsi` (DS-Homebrew/GodMode9i v3.9.0 release, 994,304 bytes each).
  Header CRC, logo CRC and banner CRC all matched. The icon decodes to the recognisable GM9i badge (a face with
  "G M 9 i" corners), colour 0 magenta in my test render.
- Reference code: NitroPaint `NitroPaint/object/combo2d.c` (`combo2dIsValidBanner`, `combo2dReadBanner`), BSD-2-Clause
  (GitHub licence field and `LICENSE` file, [fetched]). Prose: GBATEK.
- Samples: see section 7. Homebrew `.nds` files are plentiful; commercial ROMs are copyrighted, so keep them out
  of anything but the git-ignored `corpus/`.
- RECOIL: no. Validation: by eye (recognisable icon), plus the original art. Homebrew repos build the banner
  from a source bitmap, so the source is a pixel oracle ([memory] for the path in each repo).
- Difficulty / relevance: S (about 120 lines with the CRC). High: every DS ROM on disk has one, and some desktop
  thumbnailers already show it [memory].
- Open decision: `Image` has no alpha. Colour 0 is transparent. Pick a neutral light grey rather than magenta.

### A2. 3DS icon: SMDH, 3DSX and CIA

- What: the SMDH is the 0x36C0-byte icon/title block the Home Menu shows. It sits standalone in `.smdh`, inside a
  `.3dsx` (extended header), inside a `.cia` (Meta section), and in a CXI's ExeFS (`icon`). [fetched, GBATEK
  "3DS Files - Video Icons (SMDH)", "3DSX", "CIA"] and [verified] on a 3DSX.
- Sig: `SMDH` at 0 (`.smdh`); `3DSX` at 0 (`.3dsx`, u16 at 4 = header size, `0x2C` means the extended header is
  present). CIA has no magic: u32 `0x2020` at 0, then u16 type 0 and version 0. Validate section sizes against the file
  length and gate on `.cia`.
- Layout:
  - SMDH: `+0x0` `SMDH`; `+0x8` 16 title blocks of 0x200 bytes (UTF-16: 0x80 short, 0x100 long, 0x80 publisher); `+0x2040`
    small icon 24x24 (0x480 bytes); `+0x24C0` large icon 48x48 (0x1200 bytes).
  - Icon pixels: RGB565 little endian. 8x8 tiles in plain raster order, no padding to power of two. Inside a tile,
    pixels are in Z-order (Morton): pixel index bits 0, 2, 4 form x and bits 1, 3, 5 form y. [fetched]
    [verified]: the GBATEK swizzle table and my decode agree and the 48x48 picture is clean.
  - 3DSX extended header: u32 SMDH offset at `0x20`, u32 size at `0x24` (`0x36C0`), u32 RomFS offset at `0x28`.
  - CIA: u32 header size at 0, then cert size `+8`, ticket `+0xC`, TMD `+0x10`, meta `+0x14`, app size (u64) `+0x18`.
    Sections follow the 0x20-byte-padded header (`0x2040` start) each rounded up to 0x40. Meta is `0x3AC0` bytes
    with the SMDH at `Meta+0x400`. A meta size of 0, 8 or 0x200 means no usable icon, and the icon is a dummy in
    some retail CIAs. [fetched]
  - Transparency: GBATEK says it is unknown whether any exists. Treat the icon as opaque.
- Checked on: `Universal-Updater.3dsx` (Universal-Team v3.4.1 release, 3,002,452 bytes): SMDH at `0x26D598`, size
  `0x36C0`, English title "Universal-Updater", icon rendered as the blue downward-arrow logo.
- Reference code: none read. GBATEK is enough. 3dbrew's SMDH page exists but I did not fetch it.
- Samples / RECOIL / validation: see section 7; RECOIL no; homebrew repos carry the 48x48 `icon.png` the SMDH was
  built from, which gives a pixel oracle ([memory]).
- Difficulty / relevance: S for `.smdh` and `.3dsx` (about 80 lines), M for CIA. Medium-high: the 3DS homebrew scene
  has a large library, and the RGB565 Morton tile code is shared with C4.
- Out: retail `.3ds`/`.cci`/`.cxi` need NCSD/NCCH parsing and the ExeFS is encrypted in retail dumps, so skip.

### A3. GameCube banner BNR and memory-card GCI

- What: `opening.bnr` is the 96x32 banner the cube menu shows; `.gci` is a GameCube save as exported by
  emulators and adapters, with a banner and 1-8 icon frames. [fetched, YAGCD chapters 12 and 14]
- BNR: magic `BNR1` (US/JP, always 6,496 bytes) or `BNR2` (EU, text blocks repeat per language every `0x140`).
  `+0x20` pixels, 96x32, 0x1800 bytes, RGB5A3 (see C3 for the pixel format, 4x4 blocks). YAGCD calls the pixel format
  "RGB5A1"; its own footnote and the mkwiiki definition describe RGB5A3, which is what to implement.
- GCI: a 64-byte directory entry, then the file data. `+0x07` image key (banner present, banner colour format,
  icon animation mode), `+0x2C` u32 image data offset, `+0x30` u16 icon formats (2 bits per icon: 0 none, 1 CI8 with a
  shared palette after the last frame, 2 RGB5A3, 3 CI8 with its own palette), `+0x32` u16 icon speeds. Banner then
  icons then palettes (256 x RGB5A3 = 0x200). Conflict to resolve with a real file: YAGCD's bit table for `+0x07`
  is hard to reconcile with Dolphin-style "0 none, 1 CI8, 2 RGB5A3" banner formats [memory]. Whether the image offset
  counts from the file data start or the file start also needs a check.
- Samples: none free that I could verify. GCIs come from commercial game saves; BNRs from discs. Mark unverified.
- Difficulty / relevance: BNR S, GCI M; low-medium. Do it together with C3, which supplies the RGB5A3 and CI8 code.

### A4. NES ROM as a CHR tile sheet (iNES, NES 2.0, UNIF)

- What: most `.nes` files carry their pattern tables as CHR-ROM. The existing `.chr` decoder already draws a
  4 or 8 KiB table as a sheet; this extracts the same from the ROM. Games that use CHR-RAM have no tiles in the file
  and must stay `Unrecognized`. [fetched, nesdev "INES", "NES 2.0", "UNIF"; no licence stated]
- Sig: `4E 45 53 1A`. NES 2.0 when `byte7 & 0x0C == 0x08`.
- Layout: 16-byte header, optional 512-byte trainer (`byte6` bit 2), PRG-ROM, CHR-ROM. iNES: byte 4 is PRG in 16 KiB
  units, byte 5 is CHR in 8 KiB units, 0 means CHR-RAM. NES 2.0: CHR size is byte 5 plus the high nibble of byte 9 as the
  MSB; if that nibble is `$F` the size is `2^E * (MM*2+1)` with E in bits 2-7 and MM in bits 0-1 of byte 5. UNIF:
  `UNIF`, u32 version, 24 zero bytes, then `id[4] u32-length data` chunks; CHR-ROM is in `CHR0..CHRF`, any order.
  UNIF is deprecated, so it is a bonus. FDS images (`FDS\x1A`, CHR file blocks) are rare; skip.
- Samples: `christopherpow/nes-test-roms` has 263 `.nes` files, 13.5 MB. I fetched 10 at random: all were valid
  iNES 1.0, 5 had CHR-ROM (8-64 KiB) and 5 were CHR-RAM, so expect about half to decode. No licence file in the repo;
  corpus use only. Homebrew ROMs (nesdev compo entries, itch.io) are better; none checked.
- Reference code: Kaitai `ines.ksy`, WTFPL (<https://formats.kaitai.io/ines/>, [fetched]): iNES 1 only, "TODO: Support
  NES 2.0". The header facts are small enough that no code is needed.
- RECOIL: no. Validation: by eye against the project's own `.chr` of the same game (famidash and hxlnt are
  already in the corpus), or an emulator tile viewer.
- Difficulty / relevance: S, reusing `nes::tile_pixel`. High volume. Preview value is modest (a sprite sheet) but it
  is the only image a ROM can give.
- Open decision: CHR up to 1 MiB at 16 tiles wide is a 128x32768 image. Widen above some size or cap it.

## 3. Raw tile dumps with a file-name convention

These have no header, so they are extension-gated and need options. Value is highest where the toolchains name
their outputs predictably.

### B1. rgbgfx outputs (Game Boy)

- What: RGBDS's `rgbgfx` turns a PNG into `.2bpp` tile data (or `.1bpp`), `.tilemap`, `.attrmap` and `.pal`.
  pret's Pokemon disassemblies build the same files, and many Game Boy projects keep them. [snippet, rgbgfx(1) man
  page via a search summary] and [fetched] fixtures.
- Layout: tile data is "a binary dump of VRAM, no padding": 16 bytes per tile, 2 per row, low plane first, bit 7
  leftmost. With bit depth 1 the all-zero second plane is not written (8 bytes per tile) [snippet]. Pan Docs
  (<https://gbdev.io/pandocs/Tile_Data.html>, CC0) states the same. `.tilemap` is one tile-ID byte per tile, `.attrmap`
  one attribute byte per tile (mirror bits only with `-m`, bank bit, palette index) [snippet], `.pal` is 4 colours x u16
  little-endian RGB555 per palette. In the fixtures, a four-colour `.out.pal` is `00 80 ff 7f 10 42 00 00`, the 0x8000
  being a transparent-colour marker (inferred from the fixture name `alpha_grayscale`; the rule itself is [memory]). The GBC BG attribute bit layout is [memory] (Pan Docs has it).
- Problem: the `.tilemap` has no width. Heuristics (20x18 = 360 bytes, 32x32 = 1024) or a companion-based choice
  are needed; Tilemap Studio asks the user. Treat a bare `.2bpp` as a 16-wide sheet like `.chr`.
- Oracle: the RGBDS repo (MIT) ships `test/gfx/`: PNG inputs and the expected `.out.2bpp`, `.out.tilemap`,
  `.out.attrmap`, `.out.pal` for each flag set. I fetched `alpha_grayscale.png` (16x8) with its 32-byte `.2bpp`, 2-byte
  tilemap and 8-byte palette. A decoder can be checked pixel-exact against the PNG, which is rare in this survey.
  Counts from the tree: about 60 `.2bpp` (39 `.out.2bpp`, 16 `.2bpp`, 5 `.in.2bpp`), 36 `.tilemap`, 32 `.attrmap`, 42 `.pal`,
  72 PNGs across all test directories.
- Reference code: RGBDS, MIT (`LICENSE`, [fetched]). The `src/gfx/` encoder is readable. SuperFamiconv (MIT, Rust,
  see section 12) covers the same ground for several consoles.
- Conflicts: `.pal` is also the NES emulator palette (192 bytes), JASC-PAL text, and the 16-byte NESST palette.
  Tell them apart by size and first line. Registry `.pal` would be a companion extension, not a main one.
- Difficulty / relevance: S for the sheet, M for map + attributes + palette as `Format::with_companions`. Medium
  within Game Boy development, little elsewhere. RECOIL: no.

### B2. GBA and DS headerless dumps; BIOS compression

- Layout [fetched, GBATEK "LCD VRAM Character Data" and "BG Screen Data Format"]: 4bpp tiles are 32 bytes, 4 bytes per
  row, low nibble is the left pixel; 8bpp tiles are 64 bytes, one byte per pixel. Palettes are u16 BGR555 (bits 0-4 red).
  Text BG map entries are u16: tile bits 0-9, hflip bit 10, vflip bit 11, palette bits 12-15 (4bpp only). Rotation/scaling
  maps are one byte per entry (16, 32, 64 or 128 tiles square).
- Naming: pret builds `.4bpp`, `.8bpp` and `.lz` ([fetched]: Tilemap Studio's help mentions "built `.1bpp`,
  `.2bpp`, `.4bpp`, or `.8bpp` tileset" and "compressed `.lz` files") and `.gbapal` [memory]; grit writes `.img.bin`, `.pal.bin`, `.map.bin` [memory];
  libnds examples ship bare `texture.bin` (32,768 bytes), `drunkenlogo.bin` (65,536) and `palette.bin` (512) (file list
  [fetched]). A double extension like `.img.bin` can only be matched if the registry can see the whole file name.
- Conflict: `.4bpp` and `.8bpp` mean linear nibbles/bytes for GBA but planar for SNES (B3). Pick one meaning per
  platform name or require the extension plus a platform hint.
- BIOS compression [fetched, GBATEK "BIOS Decompression Functions"]: GBA/DS streams start `10h` (LZ77) or `11h` (LZ11,
  DS/3DS), commonly in `.lz`/`.lz77` files, sometimes behind a `LZ77` or `CMPR` 4-byte prefix. The RLE (`30h`), Huffman
  (`20h`) and diff (`80h`) types are [memory]. DS "LZrev" (`.blz`) runs backwards with a footer. NitroPaint
  (BSD-2) auto-detects all of these, which is a reference. An `inflate`-style `codec/` module would be S (LZ77 and RLE
  about 60 lines each).
- Oracle: SuperFamiconv in `gba` and `gba_affine` mode (MIT) or `grit` as a black box (GPL; running it is fine). Round
  trip a PNG: encode, decode, compare.
- Difficulty / relevance: S-M. Low-medium: ROM hackers and homebrew developers. RECOIL: no. No signature.

### B3. SNES planar and Mode 7 dumps

- Layout [fetched, snes.nesdev.org "Tiles", "Palettes", "Tilemaps", all CC0]:
  2bpp: each VRAM word is one row (low byte plane 0, high byte plane 1), 16 bytes per tile, same as Game Boy.
  4bpp: 32 bytes, planes 0-1 interleaved for 8 rows, then planes 2-3 in the next 16 bytes. 8bpp: 64 bytes in four such
  2bpp blocks. Mode 7: tile data in the high bytes of VRAM words only, one byte per pixel, left to right, top to bottom;
  the 128x128 tile map sits in the low bytes. CGRAM is 256 x u16 `.BBBBBGGGGGRRRRR`. Tilemap entries are u16: tile bits 0-9,
  palette 10-12, priority 13, vflip 14, hflip 15, in 32x32-tile screens combined 1x1, 2x1, 1x2 or 2x2. 3bpp (24 bytes: 2bpp
  rows, then plane 2 as 8 bytes) is used by SMW-hacking tools [memory].
- Naming: none standard. SuperFamiconv writes `tiles.bin`, `palette.bin`, `map.bin`; PVSnesLib's gfx tools use `.pic`,
  `.pal`, `.map` [memory].
- Oracle: SuperFamiconv in `snes` and `snes_mode7` mode (MIT, Rust): PNG to native data and back. PVSnesLib is MIT
  too (the repository has 57 BMP and 72 PNG files, mostly example art, and no pre-converted tile files in the tree).
- Difficulty / relevance: S plus options, low (ROM-hack scene). I would only do this as part of a shared planar codec
  (section 9), not as a format on its own.

## 4. Nintendo resource formats

### C1. Nitro 2D resources: NCLR, NCGR, NSCR

- What: the NitroSDK's palette, character (tile) and screen (tile map) files, found in DS games, extracted by
  Tinke, NitroPaint, ndspy and similar. They combine like NES CHR + palette + nametable, so they fit
  `Format::with_companions`: NCGR alone is a grey sheet, with NCLR coloured, and an NSCR shows the arranged screen.
- Magic: `RLCN` (NCLR), `RGCN` (NCGR), `RCSN` (NSCR), all little-endian with byte-order mark `FEFF`, version
  `0100`/`0101`, total size, header size `0x10`, chunk count.
- Layout:
  - NCLR chunk `TTLP`: `+8` depth (3 = 4bpp, 4 = 8bpp), `+0x10` data size, `+0x14` data offset (0x10), then u16 BGR555 colours. [fetched, GBATEK]
  - NCGR chunk `RAHC`, per NitroPaint `NitroCharacter.c` (BSD-2): at chunk body `+0` u16 tilesY, `+2` u16 tilesX (`0xFFFF` =
    unknown), `+4` u32 depth (3/4), `+8` u32 mapping mode, `+0xC` u32 type (1 = bitmap/scanline, otherwise 8x8 tiles), `+0x10` u32
    data size, `+0x14` u32 data offset (`0x18`). [fetched] GBATEK reads the first two u16 as "size in KB, always 0x20"; the
    two readings differ. Trust the BSD-2 code and check samples. A trailing `SOPC` chunk is optional.
  - NSCR chunk `NRCS`: u16 width and u16 height in pixels at `+8`/`+0xA` (GBATEK lists the width as 4 bytes at `+8` and the height at `+0xA`, which
    overlap, so I read it as a typo; check samples), data size at `+0x10`, then u16 map entries with the GBA text BG layout (tile 0-9, hflip 10,
    vflip 11, palette 12-15). Tilemap Studio's help says "after a $24-byte header", which matches. [fetched]
  - NCBR is the same container with bitmap-mode tiles (NitroPaint supports it). NBFC/NBFP/NBFS are headerless tile, palette
    and screen files [memory, unverified].
  - Many files are wrapped in BIOS LZ/RLE/Huffman or sit in NARC archives; see B2.
- Reference code: NitroPaint, BSD-2-Clause (`NitroPaint/object/NitroCharacter.c`, `NitroPalette.c`, `NitroScreen.c`,
  `NitroFont.c`, `combo2d.c`). It also reads NCER cells and NANR animations, which I would skip.
- Samples: none free found. The NitroPaint repo has no sample files; nitrogfx-style repositories (pret/pokeheartgold,
  pokeplatinum) carry PNG sources with no licence file. So samples come from the user's own cartridges. Unverified.
- RECOIL: no. Validation: open the same files in NitroPaint (Windows, BSD-2) or compare with a known game screen.
- Difficulty / relevance: M. Medium within DS ROM hacking and ripping.

### C2. Nitro 3D textures: BTX0 and TEX0

- What: `.nsbtx` (`BTX0`) and the TEX0 chunk inside `.nsbmd` (`BMD0`). Textures are named; palettes are separate and
  matched by name (a name guess is needed). [fetched, GBATEK "DS Files - 3D Video BTX0"]
- Layout: `BTX0` header (BOM, version 1, size, chunk count, offset to `TEX0`), `TEX0` with data/dict offsets for normal
  textures, a separate compressed-4x4 block pair, and a palette block. Dict entries are 8 bytes: TEXIMAGE_PARAM (bits 20-22
  S size as `8 << W`, 23-25 T size, 26-28 format 0-7, bit 29 colour 0 transparent) and an 11-bit width and height. DS formats: 1
  A3I5, 2 4-colour, 3 16-colour, 4 256-colour, 5 compressed 4x4, 6 A5I3, 7 direct colour. [fetched]; the per-format pixel layouts are
  in GBATEK's DS 3D texture pages, which I did not read.
- Output: a contact sheet of the textures (as the multi-frame decoders do).
- Samples: extracted from commercial games; none free. Reference: NitroPaint `NitroTexArc.c` and `texture.c` (BSD-2).
- Difficulty / relevance: M, low-medium (Pokemon DS and Mario Kart DS hacking).

### C3. TPL texture palette library (GameCube, Wii)

- What: the SDK texture container used all over GC and Wii games, Mario Kart Wii custom tracks and the Wii menu.
  Related single-image formats BTI (Wind Waker) and BRRES TEX0 use the same pixel formats. [fetched, YAGCD 15.35; mkwiiki]
- Magic: `00 20 AF 30`, then u32 texture count, u32 table offset (`0x0C`). Big endian throughout. Each table entry is
  (image header offset, palette header offset or 0). Image header: u16 height, u16 width, u32 format, u32 data offset, then
  wrap, filter, LOD fields. Palette header: u16 entry count, u8 unpacked, pad, u32 format (0 IA8, 1 RGB565, 2 RGB5A3), u32
  data offset. A "TPLx" variant has the table at `0x1C` [fetched, mkwiiki]. YAGCD says other TPL layouts exist.
- Pixel formats [fetched, mkwiiki "Image Formats"]: all pixels are stored as blocks, blocks in raster order, rows in a block
  top to bottom. I4 8x8 (4bpp, grey `*0x11`), I8 8x4, IA4 8x4 (high nibble alpha), IA8 4x4, RGB565 4x4, RGB5A3 4x4 (top bit 1: RGB555,
  else 3-bit alpha and RGB444), RGBA32 4x4 (a block is 64 bytes: sixteen AR pairs, then sixteen GB pairs), C4 8x8, C8 8x4, C14X2 4x4
  (top 2 bits ignored), CMPR 8x8 (four 4x4 DXT1/BC1 sub-blocks in order, big-endian RGB565 pairs, 2-bit indices, 3-colour plus
  transparent when colour 0 is not greater than colour 1; the equal case is [memory]). Mipmaps follow the base image; ignore them.
- Overlap: [gaps-consoles.md](gaps-consoles.md) section 3.1 covers Sega's GVR/GVM (`GBIX`/`GCIX` + `GVRT`), which are GameCube
  textures with the same GX pixel formats (I4, I8, IA4, IA8, RGB565, RGB5A3, ARGB8888, index4/8, DXT1). Build one shared GX
  block codec and use it for both TPL and GVR. That survey names MIT references for it: `Exortile/gvrtex` (Rust),
  `Venomalia/DolphinTextureExtraction-tool` and PuyoTools' `Textures/Gvr` (`nickworonekin/puyotools`). I have not checked
  these myself; take their licences from that file.
- Reference code: none permissive confirmed by me beyond the GVR references above. libogc's loader and Wiimm's SZS tools (`wimgt`) are [memory]: licence
  not checked, do not read. Wiimm's `wimgt` as a black box (decode TPL to PNG) is the likely oracle; unverified.
- Samples: none free verified. The `devkitPro/wii-examples` tree has 9 `.scf` scripts and 8 BMPs, no TPL. TPLs are
  built by `gxtexconv`; commercial ones come from game files. Unverified.
- Difficulty / relevance: M (about 250 lines including CMPR and the palette types). Medium in Wii/GC modding.

### C4. 3DS CLIM/FLIM, CTPK, CTXB

- CLIM [fetched, GBATEK "Video Layout Images (CLIM/FLIM)" and "3DS GPU Texture Formats"]: texture data first (width and height
  padded up to a power of two, at least 8), then a footer at `filesize - 0x28`: `CLIM`, BOM, footer size `0x14`, version,
  size, block count, `imag`, chunk size `0x10`, u16 width, u16 height, u8 format. CLIM format numbers: 0 L8, 1 A8, 2 LA4, 3 LA8, 4 HILO8,
  5 RGB565, 6 RGB8, 7 RGBA5551, 8 RGBA4, 9 RGBA8, 0xA ETC1, 0xB ETC1A4, 0xC L4, 0xD A4; GBATEK gives the table to the GPU numbering.
  Pixels use the 8x8 Z-order tiles from A2. ETC1 is 64 bits per 4x4 with its own column-major pixel order (GBATEK documents it).
  FLIM (Wii U and eShop) is mirrored/rotated and GBATEK doubts its own format table; skip FLIM.
- CTPK [fetched]: `CTPK`, u16 version 1, u16 count, texture section offset/size, per texture 0x20-byte info (name offset,
  size, data offset, format, u16 width and height, mip count, type). Names are `.tga` strings but data is raw. CTXB has
  OpenTK format codes. Both documented only in outline.
- Samples: none free verified (themes and game files). Reference code: none read; ETC1 has a public algorithm and
  `rg_etc1` is public domain [memory].
- Difficulty / relevance: M, plus M for ETC1. Low-medium.

## 5. Applications with their own picture files

### D1. Flipnote Studio PPM (DSi)

- What: the animation file of Flipnote Studio. A 64x48 4-bit thumbnail sits at a fixed offset, so a first version is
  tiny; the full frames (256x192, two 1bpp layers) are a second step. [fetched, GBATEK "DSi SD/MMC Flipnote Files";
  Flipnote-Collective wiki "PPM format"] and [verified] for the thumbnail.
- Sig: `PARA` at 0 (strong). Note `.ppm` also means Netpbm; the magic separates them.
- Layout: `+0` `PARA`, `+4` u32 animation data size, `+8` u32 sound size, `+0xC` u16 frame count minus 1, `+0xE` u16 version
  `0x24`. Thumbnail at `+0xA0`, 1536 bytes: 4bpp, 64x48, in 8x8 tiles in raster order, 4 bytes per row, low nibble first.
  Fixed 16-colour palette (white, greys, reds, blues, greens; entries 0-3 are white, dark grey, white, light grey). The
  thumbnail rendered correctly on both samples tried ([verified]: a Mario sprite and a fox with red text).
  Animation header at `+0x6A0`: u16 frame-offset table size, flags, then u32 offsets per frame. Frame: header byte (bit 7
  keyframe, bits 5-6 translate, bits 3-4 and 1-2 pen colours of the layers, bit 0 paper), 48 bytes of 2-bit line types per
  layer (192 lines), then line data in 8-pixel chunks. Pen colours: 0 and 1 inverse of paper, 2 red `#ff2a2a`, 3 blue
  `#0a39ff`. Sound is IMA ADPCM; skip it.
- Licence note: the wiki repo (`Flipnote-Collective/flipnote-studio-docs`) is GPL-2.0. It is prose documentation, which
  the project rules allow, but note it. GBATEK covers the same ground. `jaames/flipnote.js` (MIT, TypeScript) is readable
  code for cross-checks and is a working oracle: it renders the same files to bitmaps.
- Samples: `jaames/flipnote.js` `test/samples/` has 7 `.ppm` and 8 `.kwz` (MIT repo); `jaames/flipnote-player`
  `public/static/` lists 49 `.ppm` and 56 `.kwz`. I downloaded `juntso.ppm` (46,496 bytes) and `keke.ppm` (148,434). The
  works are by their Flipnote authors, so corpus use only.
- RECOIL: no. Difficulty: S for the thumbnail, M for frames (keyframe then diffs, translate, layer composite).
  Relevance: medium: a distinct, well-documented DSi format with an active community.
- KWZ (Flipnote Studio 3D) is rejected for now: its thumbnail is JPEG and its frames use a tile compression that I did
  not find a permissive spec for.

### D2. Game Boy Printer packet captures

- What: text logs of what a Game Boy sent to the printer, produced by the Arduino/WebUSB printer emulators. The
  image is rebuilt from the packets. [fetched, Pan Docs "Game Boy Printer" (CC0); Shonumi "In Depth: The Game Boy
  Printer"; emulator README] and [verified] pixel-exact.
- Packet: `88 33`, command (1 init, 2 print, 4 data, `0F` inquiry), compression flag, u16 length (little endian), data,
  u16 checksum (sum of all bytes after the magic, before the checksum), then two response bytes (ack `81` and status).
  Data command fills the buffer with up to `0x280` bytes: standard 2bpp tiles, 20 tiles (160 pixels) per row, two tile rows
  per packet. Print command data: sheets, margins, palette byte (works like BGP, often `E4`), exposure. RLE when the
  flag is set: a control byte with bit 7 set repeats the next byte `(n & 0x7F) + 2` times, with bit 7 clear copies
  `n + 1` literal bytes. [fetched]
- Dialects seen: (a) one packet per line, space-separated hex, `//` comments, `// 0 : INIT` markers; (b) a C-array
  form, `0x88, 0x33, ...,`, with `/* 3 : DATA */` markers and packets that wrap across lines, so the stream must be
  re-framed by sync bytes and length. (c) The README also mentions a "tile mode" with one tile per line. Parse the stream, not the lines.
- Checked: `test1.txt` (1 print, 160x144) and `2020-08-10_Pokemon_trading_card_compressiontest.txt` (26 packets, 13 RLE,
  3 prints, 160x208) both decode to a 0-pixel-difference match with the PNGs next to them in the emulator repo, using
  grey levels 255/170/85/0 through the palette byte, strips stacked in order and margins ignored.
- Reference: SameBoy `Core/printer.c` and `printer.h`, Expat/MIT (LICENSE read; I only opened the header). The Arduino emulator's
  decoders (C, Python, JS) are in a GPL-3 repo; do not read them. The capture files and expected PNGs are data in that repo.
- Samples: `mofosyne/arduino-gameboy-printer-emulator`: about 10 distinct captures plus 4-5 expected PNG/BMP files; Raphael
  Boichot's captures in the same repo.
- Sig: content only: the first data line is `88 33` or `0x88, 0x33`, and checksums verify. No usable extension.
- Difficulty: S (about 150 lines). Relevance: low-medium (printer-mod and Game Boy Camera scene). RECOIL: no.

### D3. Emulator save-state thumbnails

- BizHawk `.State` is a ZIP whose `Framebuffer` entry is a BMP written by `QuickBmpFile`; the source also names
  `Branches/FrameBuffer`. [fetched, `BinaryStateLump.cs`, `SavestateFile.cs`; BizHawk's own code is MIT, but its `LICENSE`
  calls the repository "a minefield", so read only the frontend C# and nothing from the embedded cores.] I did not obtain a
  `.State` file to check the entry name (it may carry a `.bmp` suffix) or compression.
- Works for every BizHawk core (NES, SNES, GB, GBA, N64, PS1, Genesis and more). Needs a ZIP reader plus the existing BMP
  decoder and `codec::inflate`. S, low relevance (TAS community).
- Other emulators are rejected, see section 10.

## 6. Variants of formats we already decode

| ID | Variant | Fix | Notes |
|---|---|---|---|
| V1 | NES `.chr` of other sizes | S | Only 4096 and 8192 bytes are accepted. Real CHR files, and CHR-ROM banks pulled from `.nes`, run 1-256 KiB. Accept any multiple of 16 up to a cap. Extension-gated, so false positives are limited. |
| V2 | NES palette companions | S | `.pal` as a 192-byte (or 1536-byte emphasis) RGB palette replaces the built-in master palette [fetched, nesdev ".pal"]; NES Screen Tool's `.pal` holds NES colour numbers (16 or 32 bytes) and would colour a `.nam` [memory; NESST docs not obtained]. Sources conflict, so tell them apart by size. |
| V3 | NES `.rle` nametables | S-M | Written by NES Screen Tool [fetched, nesdev forum thread: tool outputs RLE]. The byte layout is [memory]; NESST is described as public domain with source [snippet, forum] but the source URL was unreachable. NEXXT (a successor) is "free and public domain" [snippet, itch.io page]. |
| V4 | NES `.pb53`, `.pkb` | S | Pin Eight tile compressions [memory]. `pilbmp2nes.py` writes `.pkb`. Unverified. |
| V5 | Game Boy Camera: `.srm` as an alias, contact sheet of all 30 slots, 32x32 thumbnails at slot `+0xDF0` | S | The existing decoder picks one photo. Slot geometry is already known. |
| V6 | NES Screen Tool binary session | ? | Only the text variant `NSTssTXT` is handled. I found no sample of any binary variant. |

## 7. Sample sources

| Source | Content | Count / size | Licence / provenance | Checked |
|---|---|---|---|---|
| `DS-Homebrew/GodMode9i` release v3.9.0 | `.nds`, `.dsi` | 994,304 B each | GPL code (assumed, [memory]); art by authors | downloaded, decoded |
| `DS-Homebrew/TWiLightMenu` v27.24.1, `nds-bootstrap` v2.16.0 | `.7z`/`.zip` bundles with `.nds` inside | 12 assets / 2 | GPL (assumed) | asset names listed only |
| `Universal-Team/Universal-Updater` v3.4.1 | `.3dsx`, `.cia` | 3,002,452 B (3dsx) | GPL code (assumed) | downloaded, decoded |
| `christopherpow/nes-test-roms` | `.nes` test ROMs | 263 files, 13.5 MB | no licence file; per-ROM terms | 10 fetched, headers parsed |
| `gbdev/rgbds` `test/gfx/` | PNG + rgbgfx outputs | about 60 `.2bpp`, 36 `.tilemap`, 32 `.attrmap`, 42 `.pal` | MIT | one set fetched |
| `mofosyne/arduino-gameboy-printer-emulator` | printer captures + expected images | about 10 captures | GPL-3 repo | 3 files fetched, decoded |
| `jaames/flipnote.js`, `jaames/flipnote-player` | PPM, KWZ | 7 + 49 PPM, 8 + 56 KWZ | MIT repos; works by their authors | 2 PPM fetched, decoded |
| `Rangi42/tilemap-studio` `example/` | tilemaps (`.bin`, `.rle`, `.map`) + PNGs | 21 files | LGPL-3 repo | listing only |
| `devkitPro/nds-examples` | headerless `.bin` raw tiles/textures, `.pcx` | 14 `.bin`, 11 `.pcx` | no licence file | listing only |
| `alekmaul/pvsneslib` | example art | 57 BMP, 72 PNG in the whole repo | MIT | listing only |
| `pinobatch/little-things-nes` | NES `.nam` + PNG screenshots | 28 `.nam`, 203 PNG | no licence file | listing only |

Direct URLs that worked on 2026-10-05:

- <https://github.com/DS-Homebrew/GodMode9i/releases/download/v3.9.0/GodMode9i.nds> (and `.dsi` in the same release)
- <https://github.com/Universal-Team/Universal-Updater/releases/download/v3.4.1/Universal-Updater.3dsx>
- <https://raw.githubusercontent.com/jaames/flipnote.js/master/test/samples/juntso.ppm> (also `keke.ppm`, and the
  folder <https://github.com/jaames/flipnote.js/tree/master/test/samples>)
- <https://github.com/jaames/flipnote-player/tree/master/public/static> (49 PPM, 56 KWZ, listing only)
- <https://github.com/mofosyne/arduino-gameboy-printer-emulator/tree/master/GameboyPrinterDecoderPython/testdata>
  (`test1.txt` + `test1.png`, the compression test + PNG); more captures under `GameBoyPrinterEmulator/test/`
- <https://github.com/gbdev/rgbds/tree/master/test/gfx> (raw files at `raw.githubusercontent.com/gbdev/rgbds/master/test/gfx/`)
- <https://github.com/christopherpow/nes-test-roms> (raw files under `master/<dir>/<name>.nes`)
- Release asset lists of other homebrew can be read without the API at
  `https://github.com/<owner>/<repo>/releases/expanded_assets/<tag>`.

Not found: free GameCube BNR/GCI/TPL, DS NCGR/NCLR/NSCR/NSBTX, 3DS CLIM/CTPK, and a binary NSS. Sembiance's file
format samples (<https://sembiance.com/fileFormatSamples/>) have no Nintendo console graphics; the TIM and other console
entries belong to other areas. All sample artwork belongs to its authors; use the git-ignored `corpus/` only.

## 8. Ranking

1. A1 DS/DSi ROM banner icon. The most common file here, a strong two-CRC signature, a clean 120-line decoder, and
   already tested end to end on two real ROMs.
2. A2 3DS SMDH, 3DSX, CIA icon. Same payoff, tested on a real `.3dsx`, and it builds the RGB565 Morton tile code that C4
   reuses.
3. A4 `.nes` CHR sheet (and UNIF). About 80 lines on top of `nes::tile_pixel`, a magic number, and the biggest file population
   in the survey. Half of the test ROMs are CHR-RAM and will not decode.
4. D1 Flipnote PPM. A distinct, well-documented format with a fixed thumbnail we already decode correctly, 56 free samples
   and an MIT oracle. Frames come later.
5. B1 rgbgfx family. Needs no spec work, and the RGBDS fixtures give a pixel-exact PNG oracle. Held back by the missing
   tilemap width and the `.pal` name clashes.
6. D2 Game Boy Printer captures. Small, fully understood, and already pixel-identical to the reference PNGs on both
   dialects. Niche audience.
7. C3 TPL (plus BNR, GCI, BTI later). Well documented, strong Wii/GC modding audience; no free samples and about 250 lines.
8. C1 Nitro NCLR/NCGR/NSCR. BSD-2 reference code exists and the companion model fits, but no free sample set and the NCGR
   header has two readings.
9. C4 3DS CLIM/CTPK. GBATEK specifies it, but it needs ETC1 and I found no samples.
10. B2/B3 GBA, DS and SNES raw tiles with a BIOS decompressor. A shared codec once, then cheap per format, but
    extension conflicts and no signature keep it last.

Runners-up: C2 BTX0 textures (M, GBATEK complete, no free samples), D3 BizHawk save states (S, tiny audience), V1 and V2
(S each, but they improve existing formats, so do them whenever A4 is built), A3 GCI (conflicting bit table).

## 9. Design notes

- One planar-tile codec before more formats. `nes::tile_pixel` and `game_boy/camera.rs` already have separate planar tile
  loops, and A4, B1, B2, B3 and the Game Boy tile sheets all need "bpp, plane order, row interleave, nibble order, tile size".
  Per the "refactor first" rule, pull a `tiles` helper out before adding the decoders. Pin Eight's `pilbmp2nes.py`
  (all-permissive licence, see section 12) and SuperFamiconv list these layouts compactly.
- Transparency. `Image` is RGB only. DS icons (colour 0), GBA sprites and BG layers all treat colour 0 as transparent.
  Choose one fill (I suggest light grey) and record it once.
- Platform names. `Format::platform` is meant to follow RECOIL's list, which has none of these. Proposed:
  "Nintendo DS", "Nintendo 3DS", "GameCube", "Wii", "Super Nintendo", "Game Boy Advance"; NES and Game Boy exist.
  `$RETRO_IMAGE_PLATFORMS` filtering depends on the names.
- Signatures. Strong enough for `.signature()`: A1 (CRC pair), A2 (`SMDH`/`3DSX`), A4 (`NES\x1A`), D1 (`PARA`), D2 (framed
  packets with valid checksums), C1 and C2 (chunk magics), C3 (`00 20 AF 30`). Not for B1 to B3.
- Output size. A4 and C2 can produce very tall sheets. `check_size` should cap them, or the sheet should widen.

## 10. Rejected candidates

| Candidate | Why |
|---|---|
| Mesen `.mss`, FCEUX `.fc0`, Snes9x save states, bsnes `.bst`, DeSmuME `.ds#`, VBA `.sgm`, Dolphin `.sNN` | Formats are undocumented and the only readable code is GPL, MPL or non-commercial. Reverse engineering from samples is allowed but the payoff is a screenshot thumbnail for one small audience. mGBA states (`.ss#`) are PNG files with an extra chunk [memory], so a PNG decoder would cover them, and PNG is outside our retro scope. |
| N64 | No standalone image file with a header. Textures are raw RGBA16/32, IA, I and CI blocks inside ROMs. The same pixel formats appear in TPL (C3). [memory] |
| Virtual Boy | I found no container or tool format. The 2bpp 8x8 character layout is hardware-only and in the "Sacred Tech Scroll" (the page returned only site chrome). [memory] |
| Wii U and Switch (BFLIM from Wii U, GTX, BNTX, NUTEXB) | Modern GPU-swizzled BCn/ASTC data and ASTC decoders, not retro scope. |
| 3DS MPO (3D photos) | Two JPEGs with a header. JPEG decoding is out of scope. [fetched, GBATEK] |
| Flipnote Studio 3D KWZ | JPEG thumbnail and a tile-based compression with no permissive spec found. |
| Pokemon Gen 1/2 sprite compression (`.pic`, `.2bpp.lz`) | Samples exist only as ROM data or as pret build outputs, and pret repositories show no licence. [memory] |
| Game Boy, DS and GBA ROM headers | They carry the Nintendo logo bitmap, identical in every ROM, so it tells nothing about the game. (A1 uses the DS icon, which is different.) |
| e-Reader dot codes | Needs a dot-code reader, and the payload is game data. |
| FDS disk images with CHR blocks | The block layout is [memory] and the images are rare. |
| Tile Studio `.tsp`, Lunar Magic ExGFX, YY-CHR, Tile Molester | No fixed pixel format of their own: Tile Studio is a general tool (MIT source, <https://github.com/Wiering/Tile-Studio>, Delphi) for another survey; the rest work on raw files and `.pal`. |
| NFTR, BCFNT fonts | Bitmap fonts with glyph metrics, not pictures. NitroPaint (BSD-2) documents NFTR if wanted. |
| NCER/NANR (DS cells and animations) | Compose sprites from OBJ attributes; needs a renderer, not a decoder. |

## 11. To avoid (licence status)

| Project | Licence | Status |
|---|---|---|
| Tilemap Studio (Rangi42) | LGPL-3.0 (`LICENSE.md`, [fetched]) | code: avoid. I read only `res/help.html`, its user guide, for the tilemap format list. |
| Flipnote-Collective/flipnote-studio-docs | GPL-2.0 (GitHub licence field) | prose wiki only |
| mofosyne/arduino-gameboy-printer-emulator decoders | GPL-3.0 ([fetched]) | code: avoid; captures and PNGs are data |
| grit (devkitPro) | GPL-2.0 (GitHub licence field) | code: avoid; running the binary is fine |
| xoreos (`Graphics::NCGR`) | GPL-3 [snippet] | avoid |
| ndspy, Tinke, Kiwi.DS | GPL [memory] | avoid; Tinke wiki prose only |
| Mesen, Mesen2 | GPL-3 [memory] | avoid |
| FCEUX, Nestopia, Snes9x (non-commercial), bsnes/higan, mGBA (MPL-2.0), VBA/VBA-M, Gambatte, DeSmuME, melonDS, Citra, Dolphin | GPL/MPL/custom [memory] | avoid |
| pret/pokered, pokecrystal, pokeemerald, pokeheartgold (and their `gbagfx`, `nitrogfx`) | no licence file found (GitHub licence field empty) | all rights reserved: avoid code and data |
| devkitPro libnds, libctru, libogc, `3ds-examples`, `wii-examples`, `nds-examples` | libnds: GitHub does not recognise the licence; the rest show none | avoid until read |
| christopherpow/nes-test-roms, little-things-nes, nrom-template (repo) | no licence file | samples only |
| NES Screen Tool / NEXXT source | public domain per [snippet]; source not located | treat as docs only until a URL and licence are checked |
| Wiimm's SZS tools (`wimgt`), BrawlCrate, libogc TPL loader | unchecked [memory] | black-box use only |
| YY-CHR | closed freeware [memory] | black box |

## 12. Permissive references (may be read)

| Reference | Licence | Where | For |
|---|---|---|---|
| NitroPaint (Garhoogin) | BSD-2-Clause | `NitroPaint/object/`: `NitroCharacter.c`, `NitroPalette.c`, `NitroScreen.c`, `NitroTexArc.c`, `NitroFont.c`, `combo2d.c` (banner); `texture.c`; `compression/` | A1, C1, C2, B2 |
| RGBDS | MIT | `src/gfx/`, `test/gfx/`, man pages | B1 |
| SuperFamiconv v0.12 (Rust) | MIT | github.com/Optiroc/SuperFamiconv | B1, B2, B3 oracle, planar codec |
| PVSnesLib | MIT | github.com/alekmaul/pvsneslib | B3 |
| SameBoy | Expat | `Core/printer.c`, `printer.h` | D2 |
| jaames/flipnote.js | MIT | `src/parsers/` | D1 oracle |
| BizHawk frontend C# only | MIT with a warning in `LICENSE` | `src/BizHawk.Client.Common/savestates/` | D3 |
| Kaitai `ines.ksy` | WTFPL | formats.kaitai.io/ines | A4 |
| Pan Docs | CC0 | gbdev.io/pandocs | B1, D2 |
| SNESdev wiki | CC0 | snes.nesdev.org | B3 |
| `pilbmp2nes.py` (Pin Eight) | GNU all-permissive (copy and distribute with the notice kept) | `pinobatch/nrom-template/tools/` | planar plane maps for NES, GB, SMS, SNES/TG16, Mega Drive, GBA (little-endian row option). Not on the project's MIT/BSD/zlib/Apache/CC0 list; the project rule to carry a permissive licence's notice into the file would apply. Confirm before reading. |
| Tile Studio | MIT (Delphi) | github.com/Wiering/Tile-Studio | not examined |
| GVR texture decoders (`gvrtex`, PuyoTools `Textures/Gvr`, DolphinTextureExtraction-tool) | MIT per [gaps-consoles.md](gaps-consoles.md) section 3.1; not checked by me | see that file | C3 (shared GX pixel codec) |

Facts only (no licence or restricted): nesdev wiki, GBATEK, YAGCD, mkwiiki (Custom Mario Kart wiki), 3dbrew,
Shonumi's printer article.

## 13. Open questions for the team lead

1. Is "GNU all-permissive" (`pilbmp2nes.py`) acceptable as a readable source, or does the list stay MIT/BSD/zlib/Apache/CC0?
2. Should ROM and executable containers (A1, A2, A4) count as "image formats" for the registry, given that the picture is
   an icon or the pattern table rather than a standalone image? They behave like the Game Boy Camera save.
3. New platform names (section 9), and the single transparent-colour fill.
4. A shared planar `tiles` helper first, then the decoders (recommended), or per-decoder code as today?
