# Gap survey: Atari 8-bit corpus files RECOIL decodes and we do not (A4R PGR MPL SPC HPM BGP VSC PIC)

Clean-room notes under [CLEANROOM.md](../../CLEANROOM.md). No RECOIL source and no GPL/LGPL decoder (Altirra, Atari800, TRSE) was read. Every layout below was found by
hypothesising a decode in throwaway Python and comparing it pixel for pixel with `recoil2png`, or by mutating or synthesising files and watching what `recoil2png` changes.
Surveyed 2026-10-03. "Exact" means the scratch decoder matched `recoil2png` on every pixel of the named sample.

Other extensions from the task list (BG9, MIL, RIP, WIN, ZIM, FLI, P4I, NL3, ML1, ISH) are not Atari 8-bit or are covered in [gaps-corpus-other.md](gaps-corpus-other.md).

## Summary

| Format | Samples | Result | Fix size |
|---|---|---|---|
| A4R Anime 4ever | 8 | 7 exact; MOTOKO (two-segment file) unsolved | new decoder, about 80 lines |
| PGR PowerGraphics | 8 (7 distinct) | 6 samples exact (static colour events, GTIA 9); DALM and dragon need cycle-exact timing | new decoder, about 250 lines for the exact subset |
| MPL Mad Studio, 174 bytes | 6 | exact; we only handle the 9-byte-header variant | one more layout in `decode_mpl` |
| BGP Bugbiter | 2 | exact; our decoder misses a second 16-bit size word | two-line fix |
| VSC (+G2F) | 2 | exact; recipe confirmed | needs the `Companions` API change |
| PIC (Atari variants) | 5 files fail | OPIS (Koala text mode), BLASTER (4325), BARAHIR (7685), PAINTD (7680 APAC), SCHALT (7680 GR8): all exact | small |
| HPM Grass' Slideshow | 1 | RLE and 2bpp layout exact; colour source unresolved | partial |
| SPC Graphics Magician | 4 | not attempted (vector plus fill) | low value |

Suggested order: BGP, MPL174, PIC variants (cheap, exact) then A4R (8 files, exact) then VSC (API decision) then PGR exact subset, then HPM (colour heuristic), SPC last.

## 1. BGP Bugbiter APAC239i (2 samples: CH2016.BGP 19209 bytes, extra/atari8/dexvert/bgp/LUKE1.BGP 19181)

Our `decode_bgp` rejects both. Layout (exact on both): the 33-byte magic `BUGBITER_APAC239I_PICTURE_V1.0` `FF 50 EF`, four unread bytes, 16-bit title length, title,
then **two** planes, each introduced by its own 16-bit size `58 25` (9560 = 239 x 40): `size, plane A (9560), size, plane B (9560)`. Our code expects one size word followed by
2 x 9560 bytes. Removing the second `58 25` before calling our decoder gives pixel-identical output for both files. Fix: parse `size A` then `size B`, both 9560.

## 2. MPL Mad Studio multicolour player, 174-byte layout (6 samples: extra/atari8/madstudio/*.mpl)

Corpus `butterfly.mpl` (129 bytes, 9-byte header) already matches. The six 174-byte files fail. Layout (exact on all six):

- byte 0 = 40 (the only accepted value; height is fixed), bytes 1-4 X positions of players 0-3, bytes 5-8 colours, bytes 9-13 **ignored** (the sizes and the third-colour flag; mutating
  them changes nothing in `recoil2png`), then four 40-line player blocks (players 0-3, 1 byte per line, bit 7 left). Total 14 + 160 = 174.
- Rendering is as the existing variant: canvas from the leftmost to the rightmost player, pixels 2 wide, player 0 on top, colour register luminance bit 0 dropped. No colour OR on overlap
  (butterfly.mpl with flag 0x20 shows plain priority).
- Source: [Mad Studio file formats PDF](https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf) (MIT repo) describes the 4 sizes and flag; RECOIL ignores them.
- Detection: length 174 and byte0 == 40, or 9 + 4h with h = byte 0 (existing). They cannot collide (9 + 4h = 174 has no integer h).

## 3. VSC + G2F vertical scroll (katon.vsc, extra/atari8/g2f-full/vscroll/girl/girl.vsc)

Confirmed exact: output is the 336x240 renders of the listed G2F files stacked (katon 336x480 = katon_0 + katon_1, girl 336x720). The file is CR LF terminated names. Probed edge cases:
a name list with LF only is rejected, a name matched with different case is rejected, a single CR LF name works. Our existing G2F decoder output stacked equals `recoil2png` for both samples.
Needs `Companions::get_named(file_name)` as described in [atari-8bit.md](atari-8bit.md) section 9.5; nothing else is open.

## 4. PIC variants

Atari 8-bit `.PIC` files that RECOIL decodes and we reject, all exact:

- **OPIS.PIC (829 bytes): Koala text mode.** Same Koala container as `koala.rs` (`FF 80 C9 C7`, header length at 4, method at 7, window at 9-12, colours 708-712 at 13-17), but byte 8 = 2
  (ANTIC mode 2). Window is in character rows (`0, 40, 0, 24`). The body unpacks (method 2 = line order, packed entries as in `koala.rs`) to 960 screen codes. Render with the OS ROM font
  (`rom_font.rs`): bit 7 of the code inverts the glyph; set pixel = hue of COLPF2 (708+2 = byte 15) with luminance of COLPF1 (byte 14), clear pixel = COLPF2, 8x8 cells, 320x192.
  Header here: `28 CA 94 46 00`; ink 0x9A, paper 0x94 matched. Probing OPIS with modes 0-15 in byte 8: only 2 accepted.
- **BARAHIR.PIC (7685 bytes): raw Micro Illustrator.** 7680-byte GR15 screen then 5 tail bytes (playfield 0-2, background, one unused) = our existing `decode_mic` 5-byte tail. Only the `pic`
  extension alias is missing.
- **BLASTER.PIC (4325 bytes): GR15 with per-line colour tables.** Exact layout: bytes 0-4 = COLPF0, COLPF1, COLPF2, (ignored), COLBK for line 0; then 96 lines x 40 bytes (3840) of
  2bpp pixels (value 1 = PF0, 2 = PF1, 3 = PF2, 0 = BK); then 480 table bytes = 96 flags, 96 x PF0, 96 x PF1, 96 x PF2, 96 x BK. Line y>0 uses entry y-1 of the four colour tables
  **unless flag[y-1] has bit 7 set**, in which case the registers are unchanged (flags tested by mutation: bit 7 only; bit 7 of flag 0 changes lines 1-2). Picture 160x96, every pixel 2x2 =
  320x192. Exactly 4325 bytes required (4324 and 4326 rejected). Known as the Magic Painter family: our 3845-byte `decode_mgp_pic` is the same 5+3840 head without tables, but the header
  colour order differs (there BK is byte 3, here byte 4).
- **PAINTD.PIC (7680 bytes): APAC 80x96, byte per pixel.** Pixel-identical to our `256`/`ap2` decoder fed the same bytes. **RECOIL chooses between this and GR8 by file name**: the
  same content named `PAINT?.PIC` (stem = `PAINT` plus exactly one character, case-insensitive: PAINTD, PAINT1, PAINT_, PAINTZ all APAC; PAINT, PAINT12, PAINTZZ, p.PIC not) is APAC,
  any other name is read as GR8. Not content-detectable that I could find.
- **SCHALT.PIC (extra/atari8/pigwa-forever/Design_Master, 7680 bytes): plain GR8 monochrome**, matches our `gr8` decoder. Default for any 7680-byte `.PIC` not named `PAINT?`.
  Open question: what to do about the name rule. The decoder API has no file name; either add an extension-gated "PIC 7680" that decodes GR8 and accept the PAINTD divergence, or add the stem to the context.
- Rejected by RECOIL (ours also fails): hostile/atari8 SKANT.PIC and UNIVER.PIC (7682). hostile/crumble.pic (3845): we accept, RECOIL rejects (header byte 4 != 0 required by us, so something else
  is wrong with that file; leave).

## 5. A4R Anime 4ever (8 samples)

Sources: [Just Solve: Anime 4ever slideshow](http://fileformats.archiveteam.org/wiki/Anime_4ever_slideshow), [pouet 71261](https://www.pouet.net/prod.php?which=71261) (no layout published).
Layout is entirely reverse engineered; the previous note's "unsolved packer" is solved for 7 files.

Output: 80 x 256 pixels, 4 bits each (GTIA-9 style 16 greys, value x 17), 40 bytes per line, high nibble left, each pixel 4 output pixels wide = 320x256. The 10240 picture bytes
are `decoded[(0x4F - byte4) * 256 ..]`, zero-filled where the offset is negative (byte4 4D..50 accepted, 4C and 51 rejected; mutation test on MIYU).

Compressed stream (LZSS family, byte oriented):

- Header 5 bytes `b0 b1 00 90 4F`. `b0` and `b1` need bit 7 set. `b0 & 0x7F` is the first **marker**, `b1 & 0x7F` is the flag byte of the first group.
- A **marker** byte covers 8 groups, MSB first. A set bit means the group is preceded by a flag byte in the stream, a clear bit means the group has no flag byte and is 8 literals.
  After every 8 groups the next stream byte is a new marker.
- A **group** is 8 items; the flag byte is read MSB first, 0 = literal (copy 1 byte), 1 = match token.
- Match token, first byte `v`: if `v & 0xFE == 0` it is an extended run: next byte `n`, repeat the previous output byte `n + 2` times. Otherwise distance = `(256 - (v & 0xFE)) / 2` (1..127),
  length = `2 + (v & 1)`. Copies are byte by byte (overlap allowed); a source before the start of the buffer reads as 0.
- `00 90 4F` also appears in the data after the picture (a 146-byte run, then text): the credits text of the slideshow follows the picture in the same stream, which RECOIL requires to decode to the
  end of the file (min accepted truncation = whole file).

Verified exact (picture bytes == `recoil2png`): CATTY (ff 98), ANIME_B.007.SHIZUKU (ff d8), ANIME_B.003.PUMA (ef 80), DEUNAN (dd 80), IRIA (9d c0), MIYU (ff 80), PLASTIC (ff 9a). The header
bytes were not constant, so `b0` and `b1` matter: first marker 0x7F/0x6F/0x5F/0x1D and flags 00/18/58/1A/40 all decoded.

**MOTOKO.A4R** (header `cf 80 00 90 4d`): the header's `00 90 PP` is a segment start (page `PP`, first byte follows as the first literal), and the same token with a 4th byte `VV` occurs in the stream
(`00 90 4F FF` at file offset 289) where the credits text segment (page 4D) ends and the picture (page 4F) begins: the output pointer moves to `(PP - first page) * 256`, zero-filled, and `VV` is
the first byte there. It takes one item slot, which is why the group/marker bookkeeping seemed off by a few items. Picture exact against `recoil2png`.

Detection rule: `d[2] == 0 && d[3] == 0x90 && 0x4D..=0x50 contains d[4] && d[0] & 0x80 && d[1] & 0x80`, then require a clean decode of at least the picture bytes. `.a4r` is unique enough that
extension gating is also fine.

## 6. PGR PowerGraphics (8 samples, 7 distinct: JOYRIDE x2, PLAZMA, DALM, extra/atari8/g2f-full/pgr/dragon.pgr, LOTUS, LOOK, ENDRM)

Source: [Just Solve: Atari graphics formats](http://fileformats.archiveteam.org/wiki/Atari_graphics_formats) lists it, no layout published. Everything below is reverse engineered.

The file is an Atari binary-load record: `FF FF`, start (always 0x8206), end; length must equal end - start + 1 + 6. The body is a memory image:

- `$8206`: 16-bit address of the event stream; `PowerGFX` at `$8208`.
- `$8210`: ANTIC display list. 0x00 = 1 blank line (more with high nibble), `4E/4F lo hi` LMS, `0E`/`0F` further lines, `41 10 82` JVB. Output row = scanline = display-list line;
  lines after the list are blank lines in the background colour. Screen rows are `LMS` addresses (`$8800`, `$9000`, `$A000`) advancing 32/40/48 bytes per line by DMACTL width.
- Register image (all other bytes of those pages are 0): `$83F8`: HPOSP0-3, HPOSM0-3; `$8400`: SIZEP0-3, SIZEM, `$8405` GRAFM; `$84F8`: GRAFP0-3, COLPM0-3; `$8500`: COLPF0-3, COLBK, PRIOR, DMACTL
  (bits 0-1 width: 1 = 32 bytes, 2 = 40, 3 = 48; bits 2/3 enable missile/player DMA, bit 4 single-line). These live in the unused rows of the P/M memory (PMBASE `$8000`, single line:
  missiles `$8300`, P0-P3 `$8400/$8500/$8600/$8700`, row y reads byte y + 8). Mode E 2bpp: value 1/2/3 = COLPF0/1/2. Mode F hires: ink = hue of COLPF2 with luminance of COLPF1 (bit 0
  dropped), paper = COLPF2 & 0xFE; with PRIOR bit 6 set (0x40) mode F is GTIA 9 (4-bit luminance over COLBK hue; the 8 pixels left and right of the 320-wide window are black, including blank
  lines). Narrow width: 256-wide picture centred in 336.
- **Event stream** (one entry list per scanline, at least 240 lines, extra ignored). Byte `b`: `reg = b & 0x1F`, bit 5 (0x20) = a value byte follows, bit 7 = this event ends the scanline,
  except that reg 0x1C-0x1F never ends a line (exception 0x1C and 0x3C do end it: `1C` alone is an idle line). Bit 6 has no visible effect. reg 0..0x1B map to GTIA write registers: 0-3
  HPOSP, 4-7 HPOSM, 8-0xB SIZEP, 0xC SIZEM, 0xD-0x10 GRAFP0-3, 0x11 GRAFM, 0x12-0x15 COLPM, 0x16-0x19 COLPF0-3, 0x1A COLBK, 0x1B PRIOR. An event without a value byte writes the **latch**
  (the last value byte seen in any event, carried across lines; verified by writing COLPF0 then using a one-byte COLBK event). This classification was found by feeding `recoil2png`
  synthetic event streams (acceptance probes over all 256 first bytes, and 240-line counts).
- Events normally take effect at the start of their scanline, last write wins.

Exact (scratch decoder == `recoil2png`): JOYRIDE (both copies), LOTUS, LOOK (static), PLAZMA and ENDRM (GTIA 9, COLBK events). For DALM (players, 0x11 priority, fifth player) the same
model with the players/missiles code from `gtia.rs` leaves 1140 of 80640 pixels different (single pixels at player edges and some 0xA0 vs 0xA6 shades).
**dragon.pgr, solved.** Events on registers 0x1C-0x1F are waits that move later writes into the scanline, and its DMACTL (3E) turns on player/missile DMA. Pixel-exact against `recoil2png` after
modelling the display kernel and ANTIC (all by probing `recoil2png` with COLBK write chains in blank and mode E rows; the simulated write positions matched every probe):

- Events start at CPU cycle -9 of a scanline (x = 4 * cycle - 86; writes landing left of x = 0 take effect at 0). Costs in work cycles: write 4, +2 with a value byte; an event on register 0x1C + n:
  8n + 2 (bit 5) + 2 (bit 6) + 4 (bit 7). Only 1C and 3C and writes with bit 7 end a scanline; 1D-1F with bit 7 do not.
- The CPU progresses only on cycles ANTIC leaves free. Stolen: DL fetch at 9 (10-11 more for an LMS line), refresh at 25, 29 .. 57, player DMA 0-4, missile-only DMA 0, playfield fetch on every
  second cycle from 20 (40 bytes) or 28 (32 bytes) for as many cycles as bytes. The 48-byte start was not observed.
- Player/missile memory at 8000, single-line: missiles +300, players +400, +500, +600, +700; picture line y reads byte y + 8. DMACTL bit 3 loads the players, bit 2 the missiles; the load
  takes effect at x = 0, so objects starting left of the picture show the previous line's byte.
- An object starts when the beam reaches HPOS (x = 2 * HPOS - 88) and keeps the graphics it started with; an HPOS write after that does not move it, and one landing after the new position was
  passed does not start it. A size write stretches the remaining bits from x + 2.

Detection: `FF FF 06 82`, length consistent, `PowerGFX` at offset 8.

## 7. HPM Grass' Slideshow (1 sample: JORDAN.HPM, 3494 bytes)

`extra/atari8/demozoo/180880/DRAGON.HPM` (19203 bytes) is HiRes Player Missile, a different program; RECOIL renders noise from it, ignore.
JORDAN.HPM, exact structure: stream of tokens, `00 v n` = run of byte `v` repeated `n` times (n = 0 untested, probably 256), any other byte `k` = `k` literal bytes follow. It unpacks to
7681 bytes: 7680 of GR15 2bpp (160x192, 40 bytes per line, pixels 2 wide) and a trailing byte (here 0x34). Pixel value 0 = black, 1/2/3 = 0x34, 0x38, 0x3C for this file.
**Colour source unresolved**: changing the last byte to any of 249 values gives the grey default (0, 04, 08, 0C) and only 0x34/0x35, 0x30, 0x04, 0x51, 0x74, 0xE4 give other palettes
(0x74: 00 74 58 7E; 0xE4: 00 E4 C8 BE; 0x30: 0E 30 C6 7A) and extra bytes beyond change them again. It looks like a table keyed by the trailer rather than a rule. With one sample, implement
"trailer 0x34/0x35 -> (0, 34, 38, 3C), else grey (0, 04, 08, 0C)" at most, marked as a guess.

## 8. SPC Graphics Magician Picture Painter (COIN1 111, COIN2 153, TEST 255, ROCKETOR 574 bytes; the other `.SPC` in corpus are ST Spectrum 512)

Observations only. Files start with a 16-bit length (file size - 3) and end with `00`. Commands seen: `80 x y` move, `A0 x y` line to, `60 n`, `E0 x y` fill; coordinates in a 160x192 space
(pixels doubled to 320x192). COIN1 and COIN2 render as filled coin outlines in one colour; TEST and ROCKETOR look like random vectors because RECOIL's interpretation of colour/pattern codes
is not recoverable from these samples. A pixel-exact decoder needs the line and flood-fill rules; skipped as low value (4 samples, 2 of them test files). Reference for the Apple II original:
[Graphics Magician disassembly](https://6502disassembly.com/a2-graphics-magician/) (prose only; the Atari port differs).

## Method note and tooling

All probes used the scratchpad scripts (palette from `palette.rs` fitted to `recoil2png`; `recoil2png` as black box). Mutation probing (change one byte, diff the decoded raw) was the key
technique for A4R (it exposed the copy-token format and the page offset in `b4`) and PGR (event grammar from acceptance of synthetic streams).
