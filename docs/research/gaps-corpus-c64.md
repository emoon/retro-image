# Corpus gaps: C64 files rejected by extension

Fifteen files under `corpus/extra/commodore/sembiance/` are rejected with
"data does not match any format for this extension" although `recoil2png`
renders them. None needs a new image model. Each is either a known format
stored under the wrong extension or a size variant of one we already decode.

Method: header and tail hexdumps, mutation probing of `recoil2png`, a
throwaway disassembly of the self-running True Paint stub (6502 bytes in the
samples themselves), and pixel-for-pixel comparison against `recoil2png`
output. No RECOIL or other GPL source was read. Throwaway scripts lived in the
session scratchpad and are not committed.

Summary:

| Files | What they are | Fix |
|---|---|---|
| `koalaPaint/ggbikini.koa`, `ggblonde.koa`, `ggspazoz.koa` | Koala compressed (GG) saved as `.koa` | Add a packed fallback to the `koa`/`kla` entry |
| `koalaPaint/parallax.koala.koa` (10018) | Advanced Art Studio (OCP) saved as `.koa`; `recoil2png` renders it wrongly as Koala | Add an OCP fallback for `.koa`; exclude from the oracle comparison |
| `doodleC64/DD*.dd` (4 files, 9026) | Doodle with the bitmap trimmed to 8000 bytes | Accept 9026 bytes (zero-extend to 9218) |
| `truePaint/*.mcp.mci` (5 files) | True Paint, packed and self-running | New unpacker, then the existing MCI decoder |
| `printfox/baby.bs`, `chaot.bs`, `herz_1.bs` | Pagefox pages (`P`) saved as `.bs` | Add a Pagefox fallback to the `bs` entry |

The two paths already in the tree (`decode_koala_packed`, `decode_pg`) turn out
to decode these files unchanged. Running the CLI with `--ext gg` on the three
GG files, `--ext pg` on the three `.bs` files and `--ext ocp` on the parallax
file gives output byte-identical to `recoil2png` (parallax: identical to
`recoil2png` run on a copy renamed `.ocp`). So three of the five groups need
only extension routing.

## 1. Koala compressed (GG) under `.koa`

Files: `ggbikini.koa` (6291 bytes), `ggblonde.koa` (4828), `ggspazoz.koa` (7495).

Layout (verified):

- Bytes 0..1: load address `$6000` (`00 60`).
- Rest: `$FE value count` runs, every other byte literal. Count 0 means 256.
  Output is exactly 10001 bytes (the Koala body: bitmap 8000, screen 1000,
  colour 1000, background 1), with no load address.
- The stream is consumed exactly. After producing 10001 bytes the read pointer
  is at end of file in all three samples (no trailing bytes).
- The filename prefix `GG` is the C64 loader's convention (GoDot docs), which
  explains why people saved them as `.koa` on PC.

Reuse: `bitmap::decode_koala_packed` (`escape_rle(.., 0xfe, Run::ValueCount, 10001)`).
Pixel check: unpacked data given a `$6000` header and fed to `recoil2png`
matches the oracle PNG of the original file for all three.

Detection rule for the `koa`/`kla` entry: try plain Koala first (length 10003,
which already works). If the length is not a Koala size, load address is
`$6000`, and `escape_rle` yields 10001 bytes with 100 percent of the input
consumed, treat it as GG. The exact-consumption test is what keeps false
positives away.

Sources: Codebase64 "C64 Graphics File Format Specs"
(<http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>,
documentation); GoDot Koala page
(<https://www.godot64.de/german/l_koala.htm>, documentation, states
"RLE: $fe Byte Zähler"). Sembiance samples are copyrighted by their artists;
see `corpus/extra/commodore/MANIFEST.tsv`.

Open: how `recoil2png` chooses GG for a `.koa` is unknown (black box); our rule
is a superset on these samples. The fourth `GG`-style sample, `abydos.gg`,
already works.

## 2. parallax.koala.koa is Advanced Art Studio

File: `koalaPaint/parallax.koala.koa`, 10018 bytes, load `$2000`.

Findings:

- Rendered through `recoil2png` as `.koa`, the picture is sheared garbage: the
  bytes are read in Koala order though they are not in Koala order. Mutation
  probing shows bitmap bytes map linearly only up to about offset 2850.
  Then they drift, and bytes 5850..7050 change nothing.
- Renamed `.ocp`, `recoil2png` shows a clean multicolour picture (a spaceship
  with a "PARALLAX / Ocean" logo). Our `ocp` decoder (load `$2000`, 10018
  bytes, per `commodore.md`) output matches that byte for byte.
- Raw data is plain: bitmap at `$2000`, screen `$3F40` (starts with `eb eb ...`),
  colour block `$4328+`. Nothing is compressed (no escape byte reproduces a
  plausible size; the only `$FE` bytes are literal bitmap data).
- The sibling `paralax.koala.koa` (10003 bytes, load `$4400`) is a different,
  real Koala-layout file and already decodes.

Fix: route `.koa` (and probably `.kla`) with length 10018 and load `$2000` to
the Advanced Art Studio decoder as a fallback. Do not compare it to the
`recoil2png` output for `.koa`: the oracle is wrong for this file. Compare to
the oracle run with the `.ocp` extension instead, or exclude it from the oracle
test with a note.

Sources: sample reverse engineering plus the layout already recorded in
`docs/research/commodore.md` (Codebase64 and GoDot OCP page,
<https://www.godot64.de/german/l_ocp.htm>).

Open: whether any real `.koa` of length 10018 exists. The load address `$2000`
is a safe discriminator since Koala is `$6000` (or `$4400`, `$4000`).

## 3. Doodle with a 8000-byte bitmap

Files: `doodleC64/DDC64_COMPUTER.dd`, `DDDIRTY_PAIR.dd`, `DDJAPANESE_GIRL.dd`,
`DDLIL_GAL.dd`, all 9026 bytes.

Layout (verified): load `$5C00`, screen RAM 1024 bytes, then the bitmap at
`$6000` for 8000 bytes. A normal Doodle file is 9218 bytes, because it keeps the
8192-byte bitmap area. These files stop at the 8000 bytes the picture needs
(2 + 1024 + 8000 = 9026). The `DD` filename prefix is the Doodle disk naming.

Verified by appending 192 zero bytes (to 9218) and running `recoil2png`: pixels
equal the oracle PNG of the original for all four.

Probing size tolerance with `DDLIL_GAL.dd` (zero padded or truncated, compared
to the oracle PNG of the original file):

| Length | Result |
|---|---|
| 9217 | identical |
| 9026 | identical |
| 9025 | differs (last bitmap byte missing) |
| 9024 and below | differs |

So the oracle zero-fills short files; the smallest length that still gives the
complete picture is 9026.

Reuse: `bitmap::decode_doodle` / `DOODLE`. A fix is to accept lengths 9026..9218
with the missing tail read as zero (the `JJ` unpacker already zero-extends the
same way, see `decode_doodle_packed`). Keep the load address check
(`$5C00`). Do not accept shorter files: they lose picture data.

Sources: Codebase64 spec and GoDot Doodle page
(<https://www.godot64.de/german/l_doodle.htm>); sizes from sample RE.

Open: `eldiva.dd` and `natalie.dd` (9218 bytes, load `$0000`) already decode, so
the load address is not strictly checked today. Keep that as is.

## 4. True Paint, packed and self-running

Files: `flowers.mcp.mci` (10996), `parriot.mcp.mci` (17574), `reloy.mcp.mci`
(7857), `starland.mcp.mci` (15259), `tete.mcp.mci` (14032). The plain `.mci`
files (19434 bytes, load `$9C00`) already decode.

The packed file is a C64 program that unpacks itself and shows the picture. I
disassembled the stub from the samples; all five share identical code and
differ only in a 9-entry flag table. Layout:

- File offset 0: load `$0801`. BASIC line `2059 SYS` at 2..0x0a:
  `01 08 0b 08 09 00 9e 32 30 35 39 00` (the first two bytes are the load
  address, then the BASIC line).
- Offset 0x0b: loader `a2 00 78 bd 1c 08 9d f5 00 e8 d0 f7 e6 01 4c 01 01`:
  copies 256 bytes from `$081C` to `$00F5`, sets `$01`, jumps to `$0101`.
  That block (file offset `0x1d`..`0x11c`) holds the depacker.
- Depacker: reads the packed stream backward from the end of the file
  (`$32F3` minus 1 for the 10996-byte file, i.e. file end), writes the output
  downward, ending at `$67E7`. The reader loads one byte and decrements its
  pointer first. The whole output block is therefore the data placed by the
  packer in reverse order, so read the file from the last byte down and
  reverse the output.
- Tables inside that block (file offsets):
  - `0x78`: 9 bytes `19 17 07 0b 11 00 00 00 00` (branch offsets, the same in
    all samples; not needed for decoding)
  - `0x81`: 9 flag bytes F0..F8 (differ per file: for example flowers
    `44 32 51 88 91 c4 d3 42 52`, parriot `48 52 61 84 e1 14 21 44 46`)
  - `0x85`: 9 value bytes V0..V8; only V5..V8 are used (flowers
    `00 ff 03 c0`, reloy `ff 00 aa 18`).
- Reading backward, take byte b. If b is not one of F0..F8, emit b. Otherwise
  by flag index (a flag that appears more than once matches the highest index
  in the stub's loop, which counts Y down from 8):
  - F0: read one byte v, emit v once (literal escape)
  - F1: read v, emit v three times
  - F2: read n, emit n+2 zero bytes
  - F3: emit three zero bytes
  - F4: read n; n == 0 ends the stream, else read v and emit v n+2 times
  - F5..F8: emit Vi twice
  In the file the order is value, count, flag; read backward the flag comes
  first.
- End marker: `00 F4` at file offsets 0x8e..0x8f. In all five samples the
  reader stops at 0x8e (count byte 0 right after F4), so the packed payload
  is `file[0x8e..]` (the end code is read first, so starting at 0x90 never sees it).
- Output is 19562 bytes: a 130-byte viewer (`78 a2 00 8e 20 d0 ...`, entered
  with `JMP $1B7E`) followed by the 19432-byte MCI image. The image block is
  the normal True Paint layout moved down by `$8000`: screen 1 `$1C00`,
  background `$1FE8`, bitmap 1 `$2000`, bitmap 2 `$4000`, screen 2 `$6000`,
  colour `$6400..$67E7`. The final `JMP` and stub registers are viewer-only.

Verification: all five unpacked with a Python implementation. With `00 9c`
prepended to the last 19432 bytes (the plain 19434-byte MCI layout),
`recoil2png` output equals the oracle PNG of the packed original for all five.
(`recoil2png` also accepts the full 19562 bytes with a `$9C00` header; the
130-byte prefix is ignored, so take the last 19432 bytes, not the first.)

Detection rule: extension `mci`; length below 19434; first 17 bytes from
offset 0 equal `01 08 0b 08 09 00 9e 32 30 35 39 00 a2 00 78 bd 1c`; the byte
pair at `0x8e` is `00` plus the 5th flag byte (`file[0x81+4]`); then unpack and
require the output to be at least 19432 bytes. Fixed-offset magic plus the
end-marker check is far stricter than the extension alone, so false positives
are not a concern. If the unpacked output is shorter than 19432 bytes, reject.

Reuse: `interlace::decode_true_paint` after an unpack step that returns the
19432-byte tail with a `$9C00` header (via `Prg::new`). A shared place for the
unpacker is `commodore/unpack.rs`, next to `backward_rle`. It is a different
scheme (flag table, count minus two) so it needs its own function.

Sources: sample reverse engineering and a disassembly of the stub in
`flowers.mcp.mci` (the code is in the file, no external source used). The
unpacked layout agrees with the memory map in `docs/research/commodore.md`
(Codebase64 and GoDot TruePaint page
<https://www.godot64.de/german/l_trupnt.htm>). The pack format itself is not in
any of the public docs I found. The Archive Team page
(<http://fileformats.archiveteam.org/wiki/True_Paint_I>) was unreachable from
this environment, so I could not check whether it says anything.

Open: the flag table is chosen by the packer per picture; other packers may use
other stubs. Samples from other True Paint packers would show whether the stub
and table offsets are constant.

## 5. Pagefox pages saved as .bs

Files: `printfox/baby.bs` (1266), `chaot.bs` (1200), `herz_1.bs` (1470).

These start `50 18 13 4b`, `50 16 12 4b`, `50 16 1c 4b`: type `P`, rows in
tiles, columns in tiles, `K`. That is the Pagefox page layout we already decode
for `.pg`, with `K` contour data up to `$00` followed by `$9B count value` RLE
(byte count). Dimensions: 152x192, 144x176, 224x176 pixels, which match the
oracle PNGs.

Verified: copied to `.pg`, `recoil2png` output equals the oracle PNG of the
`.bs` original; `retro-image --ext pg` on the `.bs` file gives identical pixels
to the oracle PNG for all three.

Detection rule: for the `bs` entry, if the first byte is `P` and byte 3 is `K`,
use the Pagefox decoder; if it is `B` use the Printfox screen decoder.
`decode_pg` already checks `P rows cols K` and the zero terminator.

Reuse: `printfox::decode_pg`. No new code.

Sources: GoDot PFoxSelect page (<https://www.godot64.de/german/l_pfoxs.htm>,
documentation); the layout comments already in `printfox.rs`.

Open: the corpus also has `.pg` and `.gb` samples that already decode, so the
mix-up is only in extension naming. Whether the `G` type is ever saved as `.bs`
is unknown; add the same sniff if it turns up.

## Suggested wiring

All five fixes are routing rather than decoding, so a decoder change can stay
small:

1. `koa`/`kla`: plain Koala, then GG unpack (exact-consumption test), then OCP
   (`$2000`, 10018 bytes).
2. `dd`: accept 9026..9218 bytes, zero-extend.
3. `mci`: plain True Paint, then packed True Paint (flag table unpacker).
4. `bs`: sniff the first byte; `B` is Printfox, `P` is Pagefox.
5. The oracle test should skip `parallax.koala.koa` or compare against the
   `.ocp` rendering, with the reason written next to it.
