# Gap survey: corpus files RECOIL decodes and we reject (miscellaneous)

Clean-room notes under [CLEANROOM.md](../../CLEANROOM.md). Method: every corpus file that
`retro-image -i F -o x.png` rejects and `recoil2png` decodes, minus the extensions other agents own
(koa dd mci bs ice scr pnt a4r pgr mpl spc hpm bgp vsc pic). That left 26 files in 12 extension
groups. Layouts below were found by hypothesising a decode in throwaway Python and comparing it
pixel for pixel with `recoil2png` output, or by black-box probing `recoil2png` with mutated and
synthesised files. No RECOIL source and no GPL/LGPL decoder was read. Surveyed 2026-10-03.

"Verified" means a scratch decoder (or our own decoder on a patched copy) matched `recoil2png` on
every pixel of the listed samples. Colour values are as `recoil2png` shows them.

## Summary

| Group | Files | Status | Fix size |
|---|---|---|---|
| BG9 (Atari 8-bit) | 1 | solved: alias of G09 | one extension |
| MIL (C64 Micro Illustrator) | 1 | solved: magic is not required | one check |
| RIP (Atari 8-bit) | 1 | solved: data starts at the header-length field | one line |
| WIN (CPC OCP Art Studio) | 1 | solved: MJH-packed data may carry surplus bytes | one check |
| ZIM (PC-98) | 1 | solved: line stream ends with a zero word, not at EOF | one check |
| P4I 128x64 (Plus/4) | 1 | solved: new layout | new decoder, about 30 lines |
| NL3 (PC-98 Mapletown) | 2 | solved: new text format | new decoder, about 60 lines |
| FLI (C64 FLI Designer) | 1 | RECOIL is wrong here; we can do better | size rule |
| MX1 (PC-98 Mapletown) | 5 | transport and tile layout solved; pixels need ML1 | blocked |
| ML1 (PC-98 Mapletown) | 2 | header and palette solved; pixel coder not solved | blocked |
| Q4 (PC-98 XLD4) | 9 | container solved; codec not solved | blocked |
| ISH, 30738 bytes (C64) | 1 | partly probed; layout unknown | open |

The extension list in the task also mentioned `.zim`, `.win`, `.rip`, `.ish`, `.mil`, `.fli`, `.bg9`
and `.dctv`. `.dctv` has no rejected file in the corpus. Counts per file are in the sections below.

## 1. BG9 (Atari 8-bit, 1 sample)

`corpus/CAR.BG9`, 15360 bytes, `recoil2png` shows 640x192.

- Layout is exactly the 15360-byte G09 layout we already decode (`decode_g09`): two 7680-byte
  halves, the first holding the left 80 pixels of each row (40 bytes a row) and the second the
  right 80 pixels, two 4-bit grey pixels a byte, high nibble first. Level `n` shows as `n * 17`
  grey, each pixel 4 output pixels wide.
- Verified: copying the file to `x.g09` and running `retro-image` gives the same 640x192 pixels as
  `recoil2png` on the `.bg9` original.
- Detection: extension `bg9`, size exactly 15360. `recoil2png` rejects 15359, 15361 and 15362
  bytes. Zero bytes of 15360 draw a black picture.
- Action: add `bg9` to the G09 format's extension list. The earlier note in
  [atari-8bit.md](atari-8bit.md) ("BG9 layout unknown") can be dropped.
- Source: reverse engineered from the sample; the layout is the one `screen.rs` already documents.

## 2. MIL (C64 Micro Illustrator, 1 sample)

`corpus/extra/commodore/sembiance/mil/abydos.mil`, 10022 bytes, 320x200.

- Our `decode_micro_illustrator` demands the bytes `FF 80 69 67 14 00` at offsets 2-7. This
  sample has 20 zero bytes at offsets 2-21 instead and decodes with the same screen, colour and
  bitmap offsets (22, 1022, 2022) and background at byte 8 (here 0, black).
- Black-box mutation of the sample (set each of bytes 0-21 to 01, 55, FF): bytes 0-6 and 9-21 never
  change the output. Byte 7 must be 00 or 01 (01 changes the output, so it likely selects a packed
  variant; 55 and FF are rejected). Byte 8 changes the output (background). Load-address bytes 0-1
  are ignored. Size must be exactly 10022.
- Verified: scratch decoder using screen = bytes 22-1021, colour RAM = 1022-2021, bitmap = 2022-,
  background = byte 8, multicolour, matched `recoil2png` on all 64000 pixels.
- Detection rule that avoids false positives: extension `mil`, size 10022, byte 7 equal to 0. The
  magic is optional. Do not accept byte 7 = 1 until the packed variant is understood.
- Source: the Codebase64 / GoDot layout already cited in [commodore.md](commodore.md) plus the
  mutation results above.

## 3. RIP (Atari 8-bit Rocky Interlace Picture, 1 sample)

`corpus/TAQUART.RIP`, 7352 bytes, 320x200, mode byte `20` (HIP-like), packed.

- `rip.rs` starts the data right after the 9 colour bytes that follow `CM:`. This file has 29 more
  bytes there (`39 06 80 03 72 3a ff 36 13` + the title again + `02`) before the `PCK` packer
  signature. The 16-bit big-endian field at offset 10 ("header length, not read") is 81 and is
  exactly the offset of `PCK`. In the other seven samples that field equals the offset after the
  colour bytes (33, 36, 39, 43, 44).
- Verified two ways: `recoil2png` output is unchanged when the field is set to 81 and changes for
  any other value (52, 60, 82, 100, 200, 7352, 9000 tried); and our own decoder, given the file with
  bytes 52-80 deleted and the field set to 52, reproduces `recoil2png` exactly.
- Action: take the data start from the header-length field (checked against the file length),
  instead of from the end of `CM:` plus 9. The extra bytes look like a second title block; their
  meaning does not matter for pixels.
- Source: reverse engineered from samples and `recoil2png`, as in `rip.rs`.

## 4. WIN (Amstrad CPC Advanced OCP Art Studio window, 1 sample)

`corpus/extra/sinclair-cpc-misc/tosec-cpc/aasmain/CLOUD/CLOUD.WIN` (445 bytes) with `CLOUD.PAL`
(mode 0). `recoil2png` shows 92x38 (mode 0 pixels doubled), which is also what our decoder
produces from a repaired copy.

- The file is one MJH block (`MJH`, LE16 length 917). Unpacked, it is 874 pixel bytes
  (23 bytes a line x 38 lines), then 38 extra bytes, then the 5-byte trailer (`04 b8 00 26 00`:
  width 184 in mode-2 bits at +1, height 38 at +3). We require `line_bytes * height == pixels` and so
  reject it.
- Probing with a hand-written MJH packer (literal bytes, `01 01 01` for a literal 01):
  - `recoil2png` reads the trailer from the end of the unpacked data and takes pixels from the
    start; any number of surplus bytes between them (1, 37, 38, 39, 100, 500, 3000) is accepted and
    ignored; zero extra bytes is accepted; 30 bytes too few is rejected.
  - The same unpacked data stored raw (no `MJH` header) is accepted when exact and rejected with
    the surplus, so the tolerance applies to MJH data only.
  - The `.pal` companion is required to be next to the file for `recoil2png` to decode it.
- Verified: dropping the 38 surplus bytes and repacking, our decoder equals `recoil2png`.
- Action: for MJH-packed windows accept `unpacked_len >= pixels + 5` and read the trailer from the
  last 5 bytes. Keep the exact-size rule for raw files (our six other WIN samples).
- Open: what the 38 surplus bytes are (one per line; maybe a mask). The corpus has no second
  example. Last byte of raw data short by one gave a different picture in `recoil2png`; not pursued.
- Source: [cpctech artstud](https://cpctech.cpcwiki.de/docs/artstud.html) for MJH and the trailer
  (already cited in `ocp.rs`); the rest reverse engineered.

## 5. ZIM (PC-98 Z's Staff Kid98, 1 sample)

`corpus/lockonstar.zim`, 130060 bytes. Our decoder (`nec_pc/zim.rs`) parses the header and all 400
line blocks correctly (the last block ends at byte 130054) and then fails `pos != data.len()`.

- The file ends with 6 zero bytes after the last line. `recoil2png` accepts the file when at least
  2 bytes remain after the last line and the first of them is a zero 16-bit word: a file cut
  right after the last line (0 or 1 bytes left) is rejected, a file with 2..7 or more zero bytes is
  accepted, and `FF FF`, `01 00` or `FF*6` there is rejected. Anything after the zero word is
  ignored (`00 00` followed by 50 bytes of `FF` is accepted).
- With only 100 lines followed by `00 00` `recoil2png` accepts the file and pads the picture, as
  already recorded in [msx-japanese.md](msx-japanese.md); without the terminator it rejects.
- Action: replace "stream must end at EOF" by "next LE16 word must exist and be 0", and ignore the
  rest of the file. Keep our stricter "all height lines present" rule if desired.
- Source: reverse engineered, black-box on `recoil2png`.

## 6. P4I, 128x64 variant (Commodore Plus/4, 1 sample)

`corpus/DCD.P4I`, 2050 bytes, `recoil2png` shows 256x64 (128x64, pixels doubled horizontally). Our
Botticelli decoder wants 10242 bytes, so this is a different thing under the same extension
(the old note in [commodore.md](commodore.md) says "justsolve says little is known").

- Layout (verified, 0 differing pixels): 2-byte load address (ignored; set to `00 60` here and any
  value gives the same picture), then 32 column strips of 64 bytes. Strip `c` holds the 4-pixel-wide
  column at x = 4c; each byte is one line, four 2-bit pixels, most significant pair first. Total
  2048 bytes, 128x64.
- Colours: the four values are TED colours with luminance steps on hue 1 (white), shown as
  `(3,3,3)`, `(86,85,90)`, `(178,172,179)`, `(249,249,249)` for 0, 1, 2, 3. In our TED palette
  (`ted.rs`) these are `0x030303`, `0x56555a`, `0xb2acb3`, `0xf9f9f9`, TED colours `0x00`, `0x31`,
  `0x51`, `0x71`.
- Detection: extension `p4i`, size exactly 2050. The 10242-byte Botticelli files are unaffected.
- Open: whether other sizes of this format exist (only one sample). A mirrored note: every
  strip is 64 lines tall, so a 128x64 is the only shape the data allows.
- Source: reverse engineered from the sample; palette observed from `recoil2png` output.

## 7. NL3 (PC-98 Mapletown Network, 2 samples)

`corpus/YOUKO.NL3` (3298 bytes, Shift-JIS) and `corpus/SAKI2_A_UTF-8.nl3` (3271 bytes, UTF-8, the
same format re-encoded). Both 160x100. Fully decoded, 0 differing pixels on both.

Text layer:
- Remove every CR and LF (a file without them, or with LF only, decodes identically). The text is
  then a sequence of 159 possible symbols: U+0020..U+007F map to 0..95, halfwidth katakana
  U+FF61..U+FF9F map to 96..158. In Shift-JIS files the katakana are the single bytes 0xA1..0xDF,
  in UTF-8 files they are the 3-byte sequences. DEL (0x7F, symbol 95) is rejected wherever tried.
  Other characters are rejected.
- Line breaks every 78 characters after a 128-character first line are cosmetic; none is needed.
- Trailing characters after the picture is full are ignored (space, `~~`, NUL, LF all accepted).

Header, 128 symbols:
- 64 palette entries of 2 symbols each. Entry value `v = s0 + 128 * s1` (both symbols as above);
  `v >= 729` is rejected. `v` is a base-9 colour: red `v / 81`, green `(v / 9) % 9`, blue `v % 9`,
  each level `L` shown as `L * 255 / 8` rounded down (1 gives 31, 8 gives 255). Unused entries are
  two spaces (black). Duplicate entries are allowed.

Pixels, 160x100 fixed, scanned **column by column** (x outer, y inner), runs may cross columns:
- Symbol `s < 64`: one pixel of palette colour `s`.
- Symbol `s >= 64`: a run of palette colour `s - 64` whose length is the next symbol plus 2
  (2..160). Colour symbols above 127 are rejected (palette has 64 entries).
- A run that overshoots the last pixel is clamped (the last run +1 is accepted); a stream that stops
  short is rejected.

Probing evidence: replacing the first data symbols one at a time showed each pixel pair's effect
(colour symbol: recolours that run; length symbol: shifts everything after it by one).
- Detection: extension `nl3`, then check that the first 128 symbols form 64 valid palette entries
  and the runs fill exactly 16000 pixels. There is no magic. The first four bytes are
  `20 20 78 25` in the YOUKO sample only because black comes first and white next.
- Sources: [Just Solve: Mapletown Network](http://fileformats.archiveteam.org/wiki/Mapletown_Network)
  (snippet only: NL3 starts `20 20 78 25`; the page was unreachable from the sandbox); layout is
  reverse engineered. Samples: dexvert `image/mapletownNetwork` (see msx-japanese.md).
- Note: msx-japanese.md calls NL3 "a different text format"; this is the format. It is also the
  clearest view of the ML1 palette and colour model (below).

## 8. MX1 and ML1 (PC-98 Mapletown Network)

Both are one pixel coder; MX1 is a text transport for ML1 files.

### 8.1 MX1 transport (solved, verified through `recoil2png`)

Samples: `kasmi.mx1` (4 tiles), `maru2001.mx1` (1), `gs_cats.mx1` (1, 320x200), `64-16.mx1` (16, a
UTF-8 file) and `EYECATCH.mx1` (20). Per tile, the file has:

```
@@@ name.ml1 by AUTHOR : TITLE (Nlines) @@@      header line (Shift-JIS or UTF-8 text)
<N text lines>                                    payload, CR LF ended
@@@ name.ml1 by AUTHOR : TITLE (end of data) @@@  footer line
```

then the next tile; after the last footer come blank lines and free text (the artist's comment).
The `(Nlines)` count is larger than the payload line count by 2 (it also counts header and footer).

Payload: each character carries 7 bits, most significant bit first, and the bits of the whole tile
concatenated give the ML1 file bytes exactly (a few padding bits at the end). Character to value:
- ASCII 0x21..0x7E excluding `"` `'` `,` `@` `\` `` ` `` gives values 0..87 in order (`!` is 0).
- 0xA1..0xC8 gives 88..127 (UTF-8 files carry these as U+FF61..U+FF88).
- Anything else (SJIS lead bytes, space, tab, 0x1A) is ignored.
Evidence: the first 28 bits of every payload decode to `"100"` 1A (the ML1 magic) and `!!!!!` is five
zero values.

Verified: every extracted tile (42 of them, from all five files) decodes in `recoil2png` as an ML1.
Tile layout: one tile is its own size (`gs_cats` is a single tile with rectangle 0,0 to 319,199 and
draws 320x200); several tiles are 160x100 each, in file order, and arranged as a 2x2 grid for 4 tiles,
a 4x4 grid for 16 tiles (row-major, verified against per-tile output), and a vertical stack of n tiles
for every other count tried (2, 3, 5-15, 17-20: 160 x 100n). Tile rectangles inside the ML1 headers
are all 0,0,159,99 in these files, so they do not place the tiles. 64 tiles may make an 8x8 grid;
untested.

### 8.2 ML1 header and palette (solved)

- Bytes 0-3 `31 30 30 1A`; 4-9 date (year - 1900, month, day, hour, minute, second; ignored); 10-17
  four big-endian words `x1 y1 x2 y2` (tile rectangle; changing `x2` and `y2` changes the picture
  size to `x2 + 1` by `y2 + 1`, tried for 80x50, 160x50, 40x100, 10x10 and 2x2 with x1 = y1 = 0);
  18-95 text (machine, software, name, title; ignored). The bit stream starts at 0x60, MSB first.
- Palette form 1 (first bit 1, used by every MX1 tile and GIRL): 8-bit count - 1, then per entry a
  7-bit index and a 10-bit colour (same base-9 colour as NL3). Palette form 0 (first bit 0,
  Win8Draw): one more bit, then 128 colours of 10 bits each (all 128 verified against the picture).
  The entries are in file order; the colour *position* in this list is what the pixel stream uses
  (below). The 7-bit index field does not appear to be used by the pixel stream; it is not understood.
  Duplicate colours are allowed.
- Stream end: all flips in the last bytes are rejected; the final seven bits of the file are free
  padding; appending bytes is accepted; removing even one byte is rejected. Every sample ends with
  the same 25 bits before the padding (`111111111111011110100000` plus a variable bit) and the single
  white tile has them twice, see 8.3.

### 8.3 ML1 pixel coder (NOT solved; what is known)

Tokens are Elias-gamma-like numbers written with ones: `n` one bits, a zero, then `n` bits; value =
`2^n + those bits` (so `0` is 1, `100` is 2, `101` is 3). Evidence: the decoded token streams line
up with the row-major run structure of the pictures:
- A uniform tile is `N/2`, `colour`, `1`, `N/2 + 1` where `N` is the pixel count and `colour` is
  palette position + 2. Checked by rebuilding the white tile (`EYECATCH_13`) and for sizes 2x1, 2x2,
  4x2, 10x10, 40x100, 80x50, 160x50, 160x100: only `(N/2, N/2 + 1)` is accepted.
- The first runs of the banded tiles `EYECATCH_04..12` are `(len/2, colour, 1)` triples in row-major
  order: run lengths 1760, 640, 424 (and so on) appear as tokens 880, 320, 212, with the colour token
  being palette position + 2. `EYECATCH_07` continues with a run of 778 pixels as 389, a run of 10 as
  `5, 2, 1`, and a run of 147 (odd) as `74, 1, 3`: odd lengths round up and the third token becomes 3.
- GIRL starts `7, 28, 2, 1, 1, ...`: the first row has 14 cyan pixels (7 pairs), colour 28 is
  palette position 26 plus 2, and the picture is not row runs after that: the cyan region continues
  down 13 more rows.
- Tokens of 1 (bit 0) are very common in busy pictures (Win8Draw starts `6, 1 x 11, 2, 4, ...`) and
  small values dominate; a colour literal is not simply `position + 2` there.
- Flipping bits in GIRL shows the coder is region or edge oriented: a flip near the start recolours
  or reshapes one blob, and blobs appear in increasing x of their first pixel. 66 percent of single-bit
  flips are rejected, so the coder checks consistency (likely that the pixel count comes out
  exactly).
- Unsolved: the meaning of the third token (1, 2, 3, 10...), the rule for runs after the first row,
  how the second pixel of a pair is chosen, and how prediction from the previous row works.

Research plan if someone picks this up: use the small valid streams enumerated for 2x1 and 2x2 images
(brute force of every bit string up to 16 or 17 bits behind a two-colour palette finds about 370 valid
streams; each takes 5 ms to test) and the all-bit-flip survey on a tiny tile such as `EYECATCH_13`
(136 bytes) before touching real pictures. The known 1-pair streams are:
`010001` (white pair), `010101` (red), `00000000101` (white then red), `00010000001` (red then
white); ignoring trailing zero padding, the last bit of each is the end marker.

- Sources: [Just Solve: Mapletown Network](http://justsolve.archiveteam.org/wiki/Mapletown_Network)
  (snippet only: ML1 `"100" 1A`, MX1 `"@@@ "`, NL3 `20 20 78 25`);
  [RECOIL news](https://recoil.sourceforge.net/news.html) (release notes only, entries of 2020-09-08 and
  2020-10-30: "NEC PC-98: Mapletown Network (ML1, MX1, NL3)", "Decode multiple tiles in a Mapletown Network MX1
  file"); a search snippet of the wakachan page
  (<http://www.ateliermw.com/cglib/software.html>) naming ML1 as made by NOZOMI of Mapletown-Network
  with the free editor nedi3. No decoder source was read. All layout information is reverse
  engineered.

## 9. Q4 / XLD4 (PC-98, 9 samples: TOM093.Q4 and 8 in `extra/msx-japanese/kawaii-dake-na-no/`)

All 640x400, 16 colours of 12 bits, each with a Shift-JIS `.q4d` text file that `recoil2png` does
not read. Container verified on all nine files, codec not solved.

Header (little-endian):

| Offset | Field |
|---|---|
| 0-7 | `1A 00`, then `11 01 01` or `12 01 01` and 3 more bytes; flipping any of them has no effect |
| 8 | file size, low 16 bits; changing it is rejected |
| 10 | no effect when flipped |
| 11-15 | `MAJYO`; changing it is rejected |
| 16 | preamble length `P`; changing it is rejected |
| 18, 20 | no effect when flipped (the first is `P` + 18 to 20, the second is 48 in all nine files) |
| 22 | start of the preamble |
| 22 + P | six band records |

Band records: the picture is cut into 6 horizontal bands, five of 75 rows and a last one of 25 rows
(400 rows). Each record is `LE16 L1`, `LE16 L2`, then `L1 + 2` bytes of data. The six records
exactly fill the file to its last byte in all nine samples; this is how the layout was checked.
`L2` is smaller than `L1` in 51 of 54 bands (about 0.7x) and its meaning is unknown (candidates: the
size of a second stage, or a count of symbols).

| File | Size | P | Band sizes `L1` |
|---|---:|---:|---|
| che_r06 | 7408 | 39 | 32, 574, 2828, 2269, 1259, 349 |
| TOM093 | 58352 | 46 | 8701, 10074, 13424, 12392, 9755, 3902 |
| iya | 19287 | 37 | 718, 2461, 5178, 5917, 4069, 849 |
| early | 26259 | 41 | 5813, 5269, 4729, 6287, 3580, 482 |

(the other five fit the same rule.)

Black-box findings (`che_r06.q4`):
- A flip in a band's data changes pixels from about the flip row to the end of that band and never
  past it: bands decode independently. A flip in the record header of band 1 changes rows 75-399.
- Bytes 22 to 61 (the preamble) hold the 12-bit palette and tables. A flip in some of them recolours
  one colour (536, 2665 or 285 pixels, equal to a colour's pixel count) and a flip in others changes
  about 240000 pixels. So the palette is not a plain 16 x 12 bit array: it is bit-packed or
  entropy-coded together with table data.
- Band 0 of `che_r06` is a flat white band of 48000 pixels and costs 34 bytes of random-looking
  bytes; band 1 starts with the same 18 bytes (it also starts with white rows) and then diverges.
  A flat area costs a small constant per row, which points at an adaptive arithmetic or
  context-model coder rather than run-length or LZ. This is a guess.
- The codec is described on the web only as "16-colour lossless, good ratio, fast decode, algorithm
  and loaders held by QLD and not published" (ja.wikipedia Q4, Just Solve XLD4 snippets).
- Public code exists but is not usable here: [rururutan/ifxld4](https://github.com/rururutan/ifxld4)
  (a Susie plugin, MIT per its README) says it was "based on analysis of xgload and q4toppm"; xgload
  (vector.co.jp) has no stated licence and `q4toppm` is attached to RECOIL's own bug tracker. Under
  CLEANROOM.md (no unverified licence, nothing derived from RECOIL material) it was not opened.
  The maintainer may decide whether an MIT file derived from an unlicensed tool counts as
  permissive.
- Recommendation: do not start until the licence question is settled. If it is, a clean second
  implementation would still need the context model, which black-box probing is unlikely to find.
- Sources: [Just Solve: XLD4](http://fileformats.archiveteam.org/wiki/XLD4) (snippet),
  [ja.wikipedia: Q4](https://ja.wikipedia.org/wiki/Q4) (snippet),
  [RECOIL ticket #14](https://sourceforge.net/p/recoil/bugs/14/) (description only; the attached
  `q4toppm.zip` was not opened); layout above reverse engineered.

## 10. FLI, `kingsd.fli` (C64, 1 sample)

`corpus/extra/commodore/sembiance/fliGraph/kingsd.fli`, 17280 bytes, load address `$3C00`.

- It is a FLI Designer picture (colour RAM at `$3C00`, eight screens from `$4000`, bitmap at `$6000`,
  17218 bytes) followed by 62 bytes of `0x1A`. 17280 is 135 CP/M records of 128 bytes, so the file
  was padded on a CP/M-style copy. Our FLI Designer decoder accepts only 17218, 17409 and 17410.
- The first 17218 bytes decode in our decoder to a recognisable picture (a rider on a horse, the
  title "King's Daughter" at the top) but `recoil2png` shows a 296x200 picture of 5 colours that is
  mostly black with green and grey streaks, which is a wrong decode. RECOIL accepts any length from
  about 1000 bytes up for this extension and falls through to some other layout; so this is RECOIL
  being wrong, not us. RECOIL is the baseline, not the ceiling: the oracle test will need a divergence
  entry for this file.
- Action: accept a size of 17218 + k where k < 128 and the trailing bytes are all `0x1A`. Record
  the divergence.
- Source: layout already in `fli.rs`; padding found by comparing sizes and looking at the tail.

## 11. ISH, `cybernoid_.ish` (C64, 30738 bytes, open)

`corpus/extra/commodore/sembiance/imageSystem/cybernoid_.ish`, load address `$4000`, 30736 bytes of
data; `abydos.ish` (9194 bytes) in the same folder is the Image System hires layout we already
decode. `recoil2png` shows a 320x200 picture whose real content is only the top-left 96x105
pixels (the rest is flat light blue), which looks like a partial decode, so the oracle may be wrong
here too.

What probing found (flipping bytes in `recoil2png` input and watching which pixels change):
- Offsets `$0000-$0FFF` of the data (load `$4000-$4FFF`) are a standard C64 cell-ordered hires
  bitmap, 40 cells a row, but only the left 24 cells (192 pixels) of 13 cell rows are used. Viewed
  alone it gives a clear 192x104 line drawing.
- From `$2000` (load `$6000`) the bytes are laid out in vertical strips of 3 bytes wide and about
  213 rows tall (640 bytes a strip, rows of 3 bytes); the first four strips form a 96x105 hires
  picture and the picture repeats at `$2A00` (second sheet) and again in a second 15 KB block starting
  at data offset `$4000` (`$8000` in load terms). Each sheet looks like one frame of an interlace pair.
- The file is made of two blocks of 15360 bytes (8192 + 7168) plus 16 trailing bytes; the second
  block repeats the first one's structure.
- Not found: where the colours (screen RAM) are, how the two frames are combined into the 10 shades
  `recoil2png` shows, or the real picture size. The note in [commodore.md](commodore.md) says "Interlace
  Super Hires Painter, no information found"; that matches.
- Recommendation: low priority. One sample, an oracle that may be wrong, and no documentation. Revisit
  only if a second `.ish` of this size turns up.
- Source: reverse engineered, partial.

## Suggested order

1. Trivial fixes, all pixel-verified: BG9 as a G09 alias; MIL without the magic; RIP data offset from
   the header-length field; ZIM terminator word; WIN surplus after MJH. About one hour in total
   including a test per fix.
2. FLI Designer with 0x1A padding, with a divergence entry (RECOIL is wrong on this file).
3. P4I 128x64 (about 30 lines, fixed layout, verified).
4. NL3 (about 60 lines, fully specified, 2 samples; decoder belongs in `nec_pc`).
5. Decide the xgload / ifxld4 licence question for Q4. Without it, leave Q4, ML1, MX1 and the 30738-byte
   ISH. MX1 would come for free once ML1 decodes (transport and tile layout above are done).

## Detection rules at a glance

| Format | Rule |
|---|---|
| BG9 | extension `bg9`, size 15360 |
| MIL | extension `mil`, size 10022, byte 7 = 0 |
| RIP | existing rule, data start = BE16 at offset 10 |
| WIN | existing rule; surplus bytes allowed only after an MJH header |
| ZIM | existing rule; the word after the last line must be 0, rest ignored |
| P4I (128x64) | extension `p4i`, size exactly 2050 |
| NL3 | extension `nl3`, 128 header symbols give 64 valid colours (< 729), runs fill 16000 pixels |
| MX1 | extension `mx1`, line starting `@@@ ` and ending ` @@@`, payload decodes to bytes beginning `"100" 1A` |
| ML1 | extension `ml1`, bytes 0-3 `31 30 30 1A` |
| Q4 | extension `q4`, `MAJYO` at 11, six band records filling the file |
| FLI | existing rules, size 17218 + k (k < 128) with 0x1A tail |

## Scratch work

All experiments ran in throwaway Python in the session scratchpad (a column-run NL3 decoder, an MX1
to ML1 extractor, a bit-string brute force driver and a flip survey harness for `recoil2png`). Nothing
from there is in the repository. The MX1 tiles it extracted decode in `recoil2png`, so they can be
rebuilt from the corpus whenever the ML1 work resumes.
