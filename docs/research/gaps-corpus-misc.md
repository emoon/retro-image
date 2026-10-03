# Corpus gaps: ICE single-set fonts, 32 KB `.SCR`, DeskMate `.PNT`

Fifteen corpus files that our decoders reject and `recoil2png` turns into a PNG. Each was
checked against the oracle. This note records what they are, the layout, a detection rule,
and what an implementation can reuse. No decoder was written. Nothing here was taken from
RECOIL or any GPL/LGPL code (CLEANROOM.md): layouts come from hex dumps, hand-made probe files
fed to `recoil2png`, public docs, and Deark (MIT) for DeskMate.

| Files | What they are | Oracle usable? | Verified |
|---|---|---|---|
| 7 `.ICE` (1027-1038 B) | Interlace Character Editor font sheets, one 128-glyph set, modes 0x1f-0x25 | yes | all 7 pixel-exact, plus ~170 random header/font probes |
| `VAN.SCR` | Apple IIGS Super Hi-Res screen dump (made by SHRConvert), not a CPC file | yes | pixel-exact |
| 5 AMSDOS-headed `.SCR` | Amstrad CPC overscan screens saved by iMPdraw (3 samples) and a loader-in-screen tool (2) | **no**: RECOIL shows SHR noise | visual only |
| `SHIP3.pnt` | DeskMate Paint, 1 lost 512-byte sector | **no**: RECOIL shows Paintworks noise | visual only |
| `ZERO.pnt` | not an image (4418 bytes of `F6`, the DOS format fill) | **no** | n/a |

Tools: scratch scripts in the session scratchpad (`model.py` is the ICE model, `ov3.py` the
CPC overscan renderer). Everything below was reproduced from those.

---

## 1. ICE: single-character-set fonts (7 files)

`docs/research/atari-8bit.md` section 9.1 already decodes the two-set ICE modes (0, 1, 3, 12;
2051-2055 bytes) and lists these seven as "not done". Public names for the modes come from the
ICE editor threads: ICE edits "Super IRG, Super IRG 2, Super 0, DIN" (IRG editor), "CIN 12,
MIN 12, PCIN 12" (CIN editor) and "Super 9, Super 10, Super 11, HIP 0, CHIP 0, APAC 0"
(GTIA editor); Super IRG 2 shifts colour registers every VBI as well as the character set.
That lines up with our files: IRG20 = Super IRG 2, SZAPS9 = Super 9, SZAPS11 = Super 11,
SZAPAPAC = APAC 0, HIP20 = HIP 0, CHIP20 = CHIP 0, SZAP20P = Super 10 (a guess from elimination;
the file name does not confirm it).

### 1.1 File layout

`mode byte`, a mode-dependent header, then exactly one 1024-byte character set (128 glyphs of 8
bytes). Total size = header length + 1024. The mode byte is also the header's byte 0. Accepted by
RECOIL (every mode byte 0-255 with header lengths 1-19 tried):

| Mode | Header | Size | File sample | Name (see above) |
|---|---|---|---|---|
| 0x1f | 8 | 1032 | IRG20 | Super IRG 2 |
| 0x20 | 14 | 1038 | SZAP20P | Super 10 (guess) |
| 0x21 | 3 | 1027 | SZAPS9 | Super 9 |
| 0x22 | 3 | 1027 | SZAPS11 | Super 11 |
| 0x23 | 8 | 1032 | HIP20 | HIP 0 |
| 0x24 | 8 | 1032 | CHIP20 | CHIP 0 |
| 0x25 | 3 | 1027 | SZAPAPAC | APAC 0 |

Every other mode byte (including 0x1e and 0x26) is rejected at these sizes.

### 1.2 Output: 256 x 288 sheet

Nine 256x32 blocks stacked. Block `b = 3*i + j` (`i`, `j` in 0-2). Each block is a 16 x 4 grid
of 16x8 cells (cell `n` = 0..63 at x = 16*(n%16), y = 8*(n/16) inside its block, row-major, no
ATASCII reordering unlike the two-set sheet).

Cell `n` shows two glyphs at once: glyph `A = font[n]` drawn with colour set `j`, and glyph
`B = font[n+64]` drawn with colour set `i`. Both are 4 pixels of 2 bits per row (MSB first),
each pixel 4 output pixels wide (so a glyph is 16 wide, 8 high). The output pixel is the
per-channel average of the A colour and the B colour, rounded down (`(a+b)>>1`, same as
`blend_rgb` in `atari8/ice.rs`).

`v` is the 2-bit pixel value. `h[k]` is header byte k. `reg(c) = rgb(c & 0xfe)`,
`rgb` = `atari8/palette.rs::rgb`. `SPREAD = [0,1,4,5]`; `nib(v,k) = SPREAD[v]*(k+1)` with
`k` the colour set 0-2 (A uses `k=j`, B uses `k=i`). `w` is 0 for A, 1 for B.

| Mode | Colour of pixel value `v` |
|---|---|
| 0x1f | A and B alike: v0 `reg(h1)`; v1 `reg(h[2+k])`; v2 `reg(h7)`; v3 `reg([h5,h7,h6][k])` |
| 0x20 | like 0x1f but A and B have their own colours: v0 `reg(h1)`; v1 `reg(h[2+2k+w])`; v2 `reg(h[12+w])`; v3 `reg([h[8+w], h[12+w], h[10+w]][k])` |
| 0x21 | GTIA 9: A `rgb((h1&0xfe) \| nib)`, B `rgb((h2&0xfe) \| nib)` (v0 gives nib 0) |
| 0x22 | GTIA 11: for v>0 `rgb(((h \| nib<<4) & 0xfe))`, for v0 `rgb(h & 0xf0)`; `h` is `h1` for A, `h2` for B |
| 0x25 | A as 0x21 using `h1`; B as 0x22 using `h2` |
| 0x23 | A as 0x21 using `h1`; B as 0x1f (including v0 `reg(h1)`); pixel stream shifted, see below |
| 0x24 | A as 0x22 using `h1`; B as 0x1f for v>0, v0 black; pixel stream shifted, see below |

The OR with the header (and the dropped luminance bit 0) is the GTIA behaviour: the header
byte is COLBK, the pixel value is ORed into the hue or luminance nibble. Notes from probing:
in 0x1f, the shared colour `h7` serves both v2 and set 1's v3 (so `h6` and `h5` are the v3
colours of sets 2 and 0); `h1` is the background of both glyphs.

**Shifted streams (0x23, 0x24).** For these two, a cell row is not 16 independent cells. Per
sheet row and glyph line, each glyph gets a continuous 64-pixel stream (16 cells x 4 pixels).
Pixel `k` of A covers output x = 4k-1 .. 4k+2; pixel `k` of B covers x = 4k+1 .. 4k+4 (so a
glyph spills one pixel into its neighbour cell, and the first/last output column of the row
has no A or no B pixel: it takes value 0). Without this the edge columns are wrong.

### 1.3 Verification

`recoil2png` black box only. Process: (1) hand-made files with one glyph byte set found the
cell geometry and that glyphs `n` and `n+64` share a cell; (2) single-colour glyphs (uniform
bytes 0x55/0xaa/0xff) per block found the colour sets; (3) one header byte changed at a time
mapped header bytes to roles; (4) a python port was compared to the oracle PNG.

- All 7 corpus files: 0 differing pixels.
- Random headers and random 1024-byte fonts: 0x1f, 0x20, 0x21, 0x22, 0x25 pixel-exact on
  36 probes (3 seeds, 12 each); 0x23 on 8, 0x24 on 30.

### 1.4 Detection

Extension `.ICE` and `len == header_len(mode) + 1024` for `mode` in 0x1f-0x25 (table above).
No overlap with the two-set modes (size header + 2048), so this can live next to
`decode_ice`. Random 1027-byte files cannot collide: only the first byte and the size are
checked, but those are the only ICE files of those sizes.

### 1.5 What an implementation reuses

`atari8/ice.rs` already has `rgb`/`register_rgb` (palette), `blend_rgb`, `Image::new`. It does not
fit the existing `Frame` model (that one blends two *frames* of the same screen; this blends
two glyphs of one set and has three colour sets per glyph). A second small function next to
`decode_ice` with a per-mode "colour of (glyph, set, v)" closure is enough. Include the source
note: "reverse engineered from `recoil2png` output and samples; mode names from the ICE
editor threads".

### 1.6 Open questions

- Which of the nine blocks is "the" intended view: unknown; RECOIL draws all nine, so we should
  match it.
- Mode 0x20's public name (Super 10 is inferred).
- Whether real ICE files exist with other mode bytes (0x1e, 0x26 and up) that RECOIL rejects.
  None in our corpus.

---

## 2. `.SCR` of exactly 32768 bytes

Two unrelated things hide here, and neither is a plain CPC overscan screen that RECOIL decodes.

### 2.1 `VAN.SCR`: Apple IIGS Super Hi-Res screen dump

Pixel-exact against the oracle with the existing Super Hi-Res screen layout: 200 lines of 160
bytes (4 bits per pixel, high nibble first), 200 SCB bytes at 0x7d00 (all 0), 16 palettes of 16
little-endian `0RGB` words at 0x7e00. The 768 bytes after the pixels contain the text
"This picture generated by SHRConvert version 2.1 ]=" (SHRConvert is an Apple IIGS tool),
which is how it was recognised. Output is 320x200. The decoder already exists:
`apple/super_hires.rs::decode_screen` (File Type Note $C1/0000, CiderPress II notes).

So the only gap is registration: `.scr` is not in the extension list of the `decode_3200`
format in `apple.rs` (`"sh3","3200"` and `"shr"` only; `.pic` is handled elsewhere per
`riscos-ql.md`). RECOIL accepts the 32768-byte dump under `.scr`, `.shr`, `.pic`, `.3200`,
`.sh3` alike (probed with copies of one file; all identical).

**RECOIL ignores SCB bits 5 and 7 in this layout** (important for parity). Evidence:
- A copy of `VAN.SCR` with all 200 SCB bytes set to 0x80 still comes out 320x200 (also under
  `.shr`, `.pic`).
- `HARLEY.SCR` (84 lines with bit 7), `RESET#30.SCR` (170) and `PINUP2.SCR` (24), decoded as 320-mode
  with the low SCB nibble choosing the palette, match the oracle exactly.
- In `DRAGON.SCR` and `KDO2.SCR`, the 19 lines with SCB bit 5 set and a nonzero pixel after a
  zero one match only when fill mode is *not* applied (fill and no-fill disagree only there).
`super_hires.rs::render` currently doubles to 640 wide when any line has bit 7 and applies
fill mode. For `.scr`/`.shr` 32 KB dumps that disagrees with the oracle. Check whether the
existing `.shr` samples simply never exercise it (the four IIGS `.shr` and `test.shr` have no
bit 7 lines, so they would pass either way).

Detection rule for `.scr`: `len == 32768`, no valid AMSDOS header (`has_amsdos_header`
false), and for every palette number used by the 200 SCB bytes, all 16 colour words have
their high nibble of the high byte zero (`d[0x7e00 + 32*p + 2*c + 1] & 0xf0 == 0`). Run it
last among `.scr` decoders. Weakness: the rule does not tell a genuine SHR dump from a QL
screen renamed `.scr` whose pixels happen to pass (7 of the 41 QL/other 32 KB samples pass,
e.g. all-black ones); `riscos-ql.md` already registers QL raw screens only under `.qs4`/`.qs8`,
so this is acceptable. All other 32768-byte corpus files fail the rule except genuine SHR
and QL ones.

### 2.2 The other five: CPC overscan screens with a self-running loader

`DRAGON`, `KDO2`, `PINUP2` (iMPdraw v2, `impdraw/`), `HARLEY` (`impdraw/`) and `RESET#30`
(`scr/`). All five start with a *valid AMSDOS header* (checksum, user 0, `SCR`; our
`has_amsdos_header` accepts them), and the header length field is smaller than the file: the
rest is 0xE5 padding up to 32768.

RECOIL does not decode them as CPC: its output is the Apple SHR reading of the whole file,
header included (320x200 noise; solid green for HARLEY). Our `ocp::decode_amsdos_scr` correctly
rejects them (body is 32400 or 31936 bytes, not 16384). So the oracle test cannot score a
real decode of these, and "RECOIL decodes it" is a false positive: record a divergence entry
(RECOIL output is wrong, ours is right) if we add support.

Layout, verified by looking at the renders (no oracle):

| | iMPdraw v2 (KDO2, PINUP2, DRAGON) | HARLEY, RESET#30 |
|---|---|---|
| AMSDOS type | 0 (BASIC) | 2 (binary) |
| Load address (header 21-22) | 0x0170 | 0x0200 |
| Length (header 24-25) | 0x7e90 | 0x7cc0 |
| Entry (26-27) | 0 | 0x0811 |

The file body is a memory image starting at the load address. The picture is a 32 KB overscan
screen starting at that same address (`S`), mode 0, **96 bytes per line, 34 character rows =
272 lines**, 192x272 mode-0 pixels, drawn 384x272 (mode 0 doubles horizontally like
`hardware.rs::Mode::Zero.scale()`).

Address of byte `x` of line `l` (0-7) in character row `r` (0-33), as a memory address:

```
p    = S + r*96 + x
addr = (p & 0x7ff) + l*0x800 + (p >> 11) * 0x4000
body index = addr - S
```

So each 0x800 block holds 21.3 rows; rows after that continue in the next 16 KB page. This
is the CPC's 32 KB CRTC screen addressing for a 96-byte line (not `screen_line_offset`, which
assumes lines that fit in 0x800). The bytes of each 0x800 block before `S` are not shown:
they hold the loader, tables and palette. Strong consistency checks: for HARLEY and RESET#30
the highest screen byte is 0x7ebf, which is exactly the end of the saved range (0x0200 +
0x7cc0); the pen table of the iMPdraw files sits at 0x7f00, outside the 0x7e30 end of the
picture. In the iMPdraw files the loader (BASIC line `10 ' iMP v2`, `20 CALL &01AD`, then
Z80) lives in the first screen bytes, so the top few lines of the picture are loader noise
there; in HARLEY and RESET#30 it sits in the hidden bytes.

Palette and mode, from disassembling the loaders (read as data only):

- **HARLEY, RESET#30** (loader at 0x80b): mode byte at memory 0x800 (0 in both), then 16
  firmware ink numbers (0-26, `9*green + 3*red + blue`) at 0x801-0x810. Same inks as the CPC
  `INK` command, so `hardware.rs::firmware_color` applies. A CRTC table at 0x847 holds
  R1=0x30, R2=0x32, R6=0x22, R7=0x23, R12=0x0d, R13=0x00 (RESET#30 adds R3=0x89): width 48
  words = 96 bytes, height 34 rows, start `0x0d00` -> word offset 0x100 -> byte 0x200, which equals
  the load address. That is the only independent confirmation of `S`.
- **iMPdraw v2**: 16 pen values `0x40 | hardware colour` at memory 0x7f00-0x7f0f (Gate Array
  ink values, our `hardware_color`), the loader sets mode 0 via `RMR = 0xb8`. If the byte at
  memory 0x1ac is 1 (KDO2, DRAGON), the loader also writes the ASIC unlock sequence and copies 32 bytes
  from 0x801 to 0x6400: a CPC Plus palette, 16 words, low byte `R<<4 | B`, high byte `G`
  (cpcwiki Plus palette format). For KDO2 this is a coherent 12-bit palette and the picture
  looks right with it; with the 0x7f00 inks the same picture is garish. PINUP2 (flag 0) uses
  the 0x7f00 inks and looks right. DRAGON has flag 1 but no palette at 0x801 (the bytes
  there look like pixel data) and appears dithered under the 0x7f00 inks; it was not resolved
  (open question below).
- The iMPdraw CRTC table at memory 0x18d reads `R12=0x0d, ... d0` and would suggest start
  0x3a0 or 0x1a0, which contradicts every render (only start = load address gives an aligned
  picture). Probably the table is in a different pair order than I read; not resolved.

Detection: valid AMSDOS header, name extension `SCR`, body length 31936 or 32400 (only these
two sizes seen), and (for the iMPdraw family) type 0 with `10 ' iMP` at the start of the body
(bytes `0e 00 0a 00 01 c0 20 69 4d 50`). For the type 2 family there is no signature beyond type 2,
load 0x0200 and entry 0x0811; the two samples share them. A decoder keyed on these exact values
will not collide with other files; broader acceptance is unjustified by two samples. The
`.scr` decoders must try AMSDOS first so that these five never reach the SHR rule (the SHR
rule also rejects them: they fail the palette test).

Reuse: `amsdos.rs` (`amsdos_body`, `amsdos_extension`, `has_amsdos_header`), `hardware.rs`
(`Mode::Zero`, `render(mode, width, height, line, pens)` takes a `line(y)` closure, so only the
new addressing is needed; pens from `hardware_color` or `firmware_color`; a Plus palette
needs a 12-bit entry converter, nothing existing). Output 384x272.

### 2.3 Sources and open questions

- AMSDOS header: <https://cpctech.cpcwiki.de/docs/allhead.html> (already cited in `amsdos.rs`).
- Screen/CRTC addressing and Gate Array/RMR: <https://cpctech.cpcwiki.de/docs/screen.html>, <https://cpctech.cpcwiki.de/docs/garray.html> (cited elsewhere). cpcwiki "Programming:Overscan" was not reachable (403 earlier per `gaps-amiga-8bit.md`), so the page-continuation rule above is empirical.
- Apple SHR dump: <https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>, FTN $C1/0000 (already cited in `super_hires.rs`).
- ASIC palette word format (`GGGG RRRR BBBB`): cpcwiki Plus palette page, `https://www.cpcwiki.eu/index.php/Gate_Array` (not fetched here; recalled and confirmed by the colours on KDO2).
- Open: DRAGON palette and whether more iMPdraw variants exist; the iMPdraw CRTC table;
  whether a bare 32 KB overscan `.scr` without AMSDOS header (OCP/ConvImgCPC style) exists in the wild:
  none in the corpus, and it would be indistinguishable from SHR/QL data.
- No 640-mode or fill-mode `.scr` samples decode under RECOIL beyond what is above.

---

## 3. DeskMate Paint `.PNT`

Existing decoder: `tandy1000.rs` (Deark `misc2.c`, MIT, copyright 2016-2021 Jason Summers;
credit it in anything derived): signature `13 "PNT"`, pixels at offset 22, 312x176, 4 bpp high
nibble first, raw if the file is exactly 22 + 27456 bytes, otherwise (value, count) byte pairs.
Deark has no zero-count guard.

### 3.1 `SHIP3.pnt` (26240 bytes): valid, but damaged

The signature and the pair stream are fine. The decoder rejects because 256 consecutive pairs
have count 0: file offset 0x1c00-0x1dff (exactly one 512-byte disk sector, aligned to the
file) is all zeros, a lost sector. Without it the pairs expand to 27049 of the 27456 pixel
bytes; the missing 407 bytes is what the lost pairs would have held (2.6 rows).

Decode rule that works: expand pairs normally; treat a run of zero-count pairs as the lost
data; after the stream, the shortfall `27456 - sum(counts)` is exactly the size of the hole.
Placing the hole (filled with any colour) where the zero pairs were gives a picture that is
coherent before and after it (a jet over a field and clouds; the colour streaks continue across
the gap with no horizontal shear, so the hole size is right). Rendered with the existing
palette in `tandy1000.rs`. A single faulty region only; with several zero runs the sizes would
be ambiguous, so require one run, 512-byte-sized/aligned (or just one run).

RECOIL's output (320x396) is its Apple IIGS Paintworks reading of this file: noise. The oracle
cannot score it; treat as a "ours is better than RECOIL" case (divergence entry).

### 3.2 `ZERO.pnt` (4418 bytes): not an image

No signature. Every byte is 0xF6 (the fill DOS writes when formatting a disk), 2209 pairs of
(0xF6, 0xF6). A freshly formatted, never written file. Nothing to decode; keep rejecting.
RECOIL again reads it as Paintworks noise.

### 3.3 Sources

- Deark `misc2.c` DeskMate section (MIT-style): <https://github.com/jsummers/deark/blob/master/modules/misc2.c>.
- File format notes already in `amiga-apple-misc.md`: Just Solve "DeskMate Paint", Tvdog DeskMate page.
- The damaged-sector handling is from sample reverse engineering.

---

## 4. Summary of changes a fix would make

1. ICE: new single-set decode path (section 1) next to `decode_ice`; size + mode byte rule.
2. `.scr`: register the Apple IIGS 32768-byte screen dump under `scr` (last in order, with the
   palette check); consider aligning fill/640 handling with RECOIL for dumps.
3. CPC overscan: new `overscan` decoder in `amstrad_cpc/` keyed on the AMSDOS header facts
   above, mode 0, 96x34x8 layout, firmware inks (type 2) or Gate Array/Plus palette (iMPdraw).
   Oracle test must skip these five or expect a recorded divergence.
4. DeskMate: tolerate one zero-count run (damaged sector) in `unpack_runs`; keep rejecting
   `ZERO.pnt`.
