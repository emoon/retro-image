# Gap survey: Sega, NEC, SNK, Atari, 3DO, CD-i, Sony and Xbox consoles and handhelds

Clean-room research notes on console image formats the registry does not decode yet. Nintendo
machines are covered elsewhere. RECOIL's own format list (fetched 2026-10-05) holds one console
format, PlayStation TIM, which we already register. None of the candidates below has a
`recoil2png` oracle, so each entry says how to check output instead. No RECOIL code and no
GPL/LGPL decoder code was read. Surveyed 2026-10-05, after reading `formats.md`, `coverage.md`,
`README.md`, `gaps-ranking.md`, `gaps-others.md` (section 2.6), `CLEANROOM.md` and
`adding-a-format.md`.

## How to read this

Facts carry a confidence tag:

- **[fetched]**: the primary page, spec or MIT-licensed source file was read in this session.
- **[observed]**: not from a document. I decoded a real sample file with a throwaway script
  (scratchpad only, nothing in the repo) and looked at the result. Counts as strong evidence
  for the specific fact, not for the whole format.
- **[snippet]**: only a search-result summary was seen.
- **[memory]**: from general knowledge, unverified.

Difficulty: S = a day or less from a spec, M = a few days or one risky detail, L = emulator-style
work or reverse engineering. "Sig" = whether content detection is safe.

## Environment limits

- The shared WebSearch quota (200 calls) ran out part-way through. Everything after that came
  from direct fetches of known URLs. Atari 2600/5200/7800/Jaguar, ColecoVision and Intellivision
  tool formats were therefore not searched properly; their entries in the rejected list rest
  on `[memory]` and say so.
- `segaretro.org` serves an Anubis "Access Denied" page to agents. `info.sonicretro.org` served the
  same wiki content (licence footer: CC BY 4.0) and was used instead.
- `archaicpixels.com` (the PC Engine wiki) is now a spam WordPress site; only 2014-2015 Wayback
  snapshots were usable. `xboxdevwiki.net` timed out live and was read through Wayback.
  `pico-8.fandom.com` sits behind a Cloudflare challenge and was not fetched. GitHub's unauthenticated
  API rate limit was hit, so repository trees were listed with blobless `git clone`.
- `mc.pp.se` (Marcus Comstedt's Dreamcast pages) has an expired TLS certificate; the pages were read
  with `curl -k`. Plain text, no scripts.
- Sample files fetched for verification (about 40 files, all small) sit under the session scratchpad.
  Nothing was added to `corpus/`.

## 1. What is already there

- `platform/playstation.rs`: TIM only. psx-spx lists PXL and CLT as separate formats; its own text calls
  them "very rare" (three games), so they are not worth a decoder.
- `platform/nes/`, `platform/game_boy/`: Nintendo, someone else's area.
- `gaps-others.md` section 2.6 mentions "SNES / MD / PCE / Neo Geo raw tiles" as low priority with a
  `[search]` tag. This survey replaces that line with fetched layouts (section 3.9) and adds the
  container formats that line did not know about.
- RECOIL supports no Sega, NEC, SNK, Atari-console, 3DO, CD-i or Dreamcast format
  (<https://recoil.sourceforge.net/formats.html> [fetched]; PlayStation TIM is the only console row).

## 2. Summary

| # | Format | Platform | Docs | Common | Diff | Samples | Sig |
|---|---|---|---|---|---|---|---|
| 1 | PVR / PVP / PVM (GVR / GVM siblings) | Dreamcast | Spec [fetched], MIT reference code | High (game modding) | M | In hand (12) | yes |
| 2 | TIM2 (`.tm2`) | PlayStation 2 | Official spec [fetched] | High (PS2 modding) | S-M | In hand (17) | yes |
| 3 | 3DO IMAG / CEL / ANIM | 3DO | Official docs [fetched] | Low-medium | M | In hand (365 in one repo) | yes |
| 4 | VMS / ICONDATA_VMS icons and eyecatch | Dreamcast VMU | Spec [fetched] | Low-medium | S | In hand (1) | weak (CRC) |
| 5 | GIM (PSP, PS3) | Sony | Partial [snippet], MIT reference code | Medium | M | In hand (12) | yes |
| 6 | Neo Geo C-ROM pair, CD `.SPR`, `.FIX` | SNK | Spec [fetched] | Low-medium | S-M | In hand (4) | no |
| 7 | Nemesis / Kosinski tile art | Mega Drive | Spec [fetched] | Low-medium (ROM hacking) | M | Easy (100+) | no |
| 8 | PS1 memory-card save icons | PlayStation | Spec [fetched] | Low | S | none yet | partial |
| 9 | Raw VRAM tile dumps: SMS/GG, MD, PCE, WonderSwan, NGPC | various | Spec [fetched] | Low (emulator dumps) | S each | Easy | none |
| 10 | CD-i IFF image (DYUV, CLUT4, RL7) | Philips CD-i | Partial [fetched] | Low | M-L | In hand (11) | yes |
| 11 | Lynx `.spr` + `.pal` (sprpck) | Atari Lynx | Partial [snippet] | Low | M | none | no |
| 12 | TIC-80 `.tic`, PICO-8 `.p8` | fantasy consoles | Spec [fetched] / [memory] | Medium (modern) | S | Easy | yes / no |

## 3. Candidates

### 3.1 Dreamcast PVR, PVP, PVM, and the GVR / GVM siblings

- Files: `.pvr` texture, `.pvp` external palette, `.pvm` archive of PVR textures. The same Sega
  toolchain produced `.gvr`, `.gvp`, `.gvm` for GameCube and `.svr` for PS2 (PuyoTools lists all
  of them as supported textures [fetched README]).
- Magic: optional `GBIX` chunk (GVR also `GCIX`), then `PVRT` (GVR: `GVRT`). PVM starts `PVMH`,
  PVP starts `PVPL` [fetched: PuyoTools wiki; Ikaruga guide for PVPL].
- PVRT header [fetched, <https://github.com/nickworonekin/puyotools/wiki/PVR-Texture>]: `PVRT`,
  u32 LE chunk length, u8 pixel format, u8 data format, 2 zero bytes, u16 LE width, u16 LE height. A
  `GBIX` chunk is `GBIX`, u32 length (8), u32 global index, 4 pad bytes.
- Pixel formats [fetched, MIT source `Textures/Pvr/PvrPixelFormat.cs`]: 0 ARGB1555, 1 RGB565,
  2 ARGB4444, 3 YUV422, 4 bump, 6 ARGB8888. For the palette data formats this byte presumably names the
  palette's colour format [memory; no indexed sample was seen].
- Data formats [fetched, `PvrDataFormat.cs`]: 1 square twiddled, 2 +mipmaps, 3 VQ, 4 VQ +mipmaps,
  5 index4 (external palette), 6 index4 +mipmaps, 7 index8, 8 index8 +mipmaps, 9 rectangle, 0xB stride,
  0xD rectangle twiddled, 0x10 small VQ, 0x11 small VQ +mipmaps, 0x12 square twiddled +mipmaps (alt).
- Conflicting docs: the 2006 "PVR Graphic Format" by SmokesGrass
  ([Shenmue-Mods/Knowledgebase](https://github.com/Shenmue-Mods/Knowledgebase/blob/master/Dreamcast_Information/PVR_Graphic_Format.md),
  licence not found) lists pixel formats 5 and 6 as 4/8-bit and numbers data types 5-8 differently. PuyoTools,
  the Ikaruga guide and the real samples agree with each other, so treat that document as wrong there.
- Twiddle order [observed + fetched]: index = (spread(x) << 1) | spread(y), where `spread` moves bit i
  to bit 2i. So y is the fast axis; x sits on the odd bits. The Ikaruga guide's formula
  (<https://ikaruga.dashgl.com/guides/pvr/>) has the axes the other way round, which gives a transposed
  picture. Confirmed on `0GDTEX.PVR` (upright "DREAM SNES" disc label with this order, rotated with the other).
- VQ [observed]: 256-entry codebook of 2x2 blocks (8 bytes each at 16 bits per pixel), one index byte
  per block, indices in the same twiddle order, pixels inside a block in column order (top-left, bottom-left,
  top-right, bottom-right). Verified on `Font.pvr`: the ASCII table reads upright only with this combination.
  Small VQ shrinks the codebook with texture size: 16 entries up to 16x16, 32 at 32x32, 128 at 64x64,
  256 above, per the Ikaruga guide [fetched, not seen in a sample].
- Mipmaps: levels run smallest to largest, base last [fetched wiki]. For twiddled mipmaps the wiki
  says two zero bytes precede the 1x1 level. For VQ +mipmaps [observed]: after the 2048-byte codebook come
  the index grids of the smaller levels (the 1x1 and 2x2-pixel levels one byte each), then the base level.
  Base offset 2048 + 1366 for 128x128 and 2048 + 5462 for 256x256; both samples carry 10 more bytes after
  the base. Found by scoring neighbour smoothness over every offset, then checked by eye.
- Do not trust the length fields [observed]: KallistiOS's own `star.pvr` has a `GBIX` length of 0
  and a `PVRT` length of 0, with `PVRT` at offset 16. Find `PVRT` after the 8 + n header and size the data
  from width, height and format.
- PVM [fetched wiki]: `PVMH`, u32 offset of first texture minus 8, u16 flags (1 global index,
  2 dimensions, 4 pixel and data format, 8 file names), u16 count; each entry is a u16 number followed by
  whichever of a 28-byte name, 2-byte format, 2-byte size and 4-byte global index the flags announce; each
  texture is 16-byte aligned and starts at `PVRT` with no `GBIX`.
- GVR header [fetched, MIT source `Textures/Gvr/GvrTextureDecoder.cs`]: optional `GBIX`/`GCIX`
  (length LE + 8, index BE), then `GVRT`; 10 bytes after the tag come a byte with palette format
  (high nibble) and flags (low nibble), a data-format byte, then width and height big-endian. Data
  formats [`GvrDataFormat.cs`]: 0 I4, 1 I8, 2 IA4, 3 IA8, 4 RGB565, 5 RGB5A3, 6 ARGB8888, 8 index4,
  9 index8, 0xE DXT1/CMPR. These are the standard GameCube formats; YAGCD chapter 17
  (<https://hitmen.c02.at/files/yagcd/yagcd/chap17.html> [snippet]) describes the tiling. If the
  Nintendo survey covers TPL/BTI, the pixel codecs are shared.
- Permissive reference code: PuyoTools.Core, MIT (<https://github.com/nickworonekin/puyotools>):
  `Textures/Pvr` (34 files), `Textures/Gvr` (28), `Textures/Svr` (19), `Textures/Gim` (24),
  `Archives/Formats/Pvm` and `Gvm`. Also MIT: [Exortile/gvrtex](https://github.com/Exortile/gvrtex) (Rust, GVR),
  [Venomalia/DolphinTextureExtraction-tool](https://github.com/Venomalia/DolphinTextureExtraction-tool)
  (GC/Wii textures; it credits Puyo Tools for GVR). KallistiOS (BSD-like "KOS License", attribution
  required) documents the hardware side in `kernel/arch/dreamcast/hardware/pvr/`.
- Samples: verified downloads. KallistiOS `examples/dreamcast/gldc/nehe/nehe06/romdisk/glass.pvr`
  (RGB565 rectangle 128x128, with GBIX) and `.../nehe09/romdisk/star.pvr` (RGB565 VQ 128x128, zero length
  fields), under the KOS licence. `https://sembiance.com/fileFormatSamples/image/pvrTexture/` holds 10 real
  files (`0GDTEX.PVR` twiddled ARGB4444 256x256, `BACKWALL.PVR` rectangle ARGB4444 128x256, `Font.pvr` VQ,
  `stunt.pvr` and `008.__` VQ with mipmaps, and five more). I downloaded and decoded the five named ones
  with my scratch decoder; the other five are listed but untried. archive.org items named "Dreamcast
  Texture Dump Archive" and "Dreamcast Original Texture PNG" exist [snippet]; their contents were not
  checked.
- RECOIL: no. Check by: PuyoTools' CLI as a black box (MIT, .NET) writes PNGs; KallistiOS
  `utils/pvrtex` can write a decoded preview (`-p`), and its `approvaltest/tests/approved/` tree holds
  fixed outputs for several formats (run it, do not read its source, see 7). Failing both, by eye.
- Difficulty: PVR M, PVP/PVM S, GVR M (RGB5A3, IA8, CMPR and the 4x4/8x4/8x8 tilings). Relevance:
  high in Sonic Adventure, Puyo Puyo and Dreamcast modding circles [memory].
- Registry notes: ARGB formats carry alpha, which `Image` drops (see section 6); fonts stored as
  alpha-only glyphs will show as solid squares. Choose the largest mipmap. Index4/index8 files need the
  `.pvp`: register `PVR` with a PVP companion and fall back to a grey ramp when it is missing.

### 3.2 PlayStation 2 TIM2

- Files: `.tm2`, `.tim2`. Magic `TIM2`.
- Spec [fetched]: "TIM2 format specification ver.4", web technology Corp., 1999-12-02, in
  `tim2v4b_e.zip` inside <https://github.com/GirianSeed/tim2> (the repo archives the spec and samples; no
  licence file). File header 16 bytes: `TIM2`, u8 version, u8 format id (0 = 16-byte alignment, 1 = 128),
  u16 picture count, 8 zero bytes. Each picture has a 48-byte header: u32 total, clut and image sizes, u16
  header size, u16 clut colours, u8 picture format (0), u8 mipmap count, u8 ClutType, u8 ImageType,
  u16 width, u16 height, then GS TEX0/TEX1/TEXA/TEXCLUT words. A mipmap header follows if there are
  two or more levels, then user data, image data, CLUT data. Little endian.
- ImageType: 1 = 16 bit, 2 = 24 bit, 3 = 32 bit, 4 = 4-bit index, 5 = 8-bit index. ClutType: bits
  5..0 give the CLUT depth (1 = 16, 2 = 24, 3 = 32 bit), bit 6 = compound order, bit 7 = CSM2.
- Pixels [fetched + observed]: 16-bit pixels have R in bits 0-4, G 5-9, B 10-14, A bit 15. 32-bit is
  R,G,B,A bytes, and the opaque alpha is 0x80, not 0xFF (every pixel of `i32.tm2` and every CLUT entry of
  `i8c32.tm2` has alpha 128). 4-bit pixels pack the low nibble first.
- CLUT order [fetched + observed]: CSM1 8-bit palettes are stored "compound": entry i of the file
  belongs to index i with bits 3 and 4 swapped (spec 4.5). Without that swap `i8c16.tm2`, `i8c32.tm2` and
  three real game files decode with scrambled colours. CSM2 palettes are sequential. For 16-colour CSM1
  with bit 6 set, the compound order applies across 32 entries.
- Real files are linear [observed]: three 8-bit game files (`TF6_PHOENIX.TM2` and
  `interlaced_DMC3_0140_01.tm2` with PSM 0x13 in their TEX0 word, `BaLitDu.tm2` with PSM 0) decode correctly as
  plain row-major with the CLUT swap; a GS-swizzle pass made them worse. The 24-bit `P01_001_unpacked.tm2`
  decodes plainly too. [PS2HomeDeveloper/ps2-tim2-tool](https://github.com/PS2HomeDeveloper/ps2-tim2-tool)
  (MIT, 2026) offers optional swizzle for 32- and 16-bit data only, which fits. The Rainbow wiki says
  "image data may be swizzled" [snippet]; treat as a variant to handle if samples appear.
- Odd files [observed]: `canntviewunk_4bpp.tm2` is 512x256 4-bit with a 320-colour CLUT; the first 16
  entries give noise. Decode what makes sense and fail cleanly on this one.
- Spec provenance: the spec text says "[CONFIDENTIAL]" and limits disclosure to authorised developers.
  It has been public for years (the GitHub archive above, the VG Resource wiki, Just Solve). It is
  prose, so CLEANROOM allows it, but the maintainer should decide whether the banner matters. Its
  sample C code (`tim2sample_e.zip`, `sample/` in the repo) has no licence: do not read.
- Samples: `tim2img_e.zip` in the same repo has 11 files of one picture (a sleeping cat, 256x256):
  16/24/32-bit direct, 4/8-bit with 16/24/32-bit CLUTs, 128-byte alignment, CSM2. They decode to the same
  image with my scratch script, which makes them a self-consistency oracle. Real files:
  `https://sembiance.com/fileFormatSamples/image/tim2/` (6 files, verified downloads). The `tim2TXC/`
  folder next to it is Yuke's `RTX38` containers, a different undocumented format.
- RECOIL: no (TIM only). Check by: the 11 variants must agree; compare with any independent viewer.
- Difficulty: S-M. Relevance: high for PS2 game modding. Sig: strong (`TIM2` + version
  and alignment byte checks).

### 3.3 3DO IMAG / CEL / ANIM

- Files: `.imag`, `.cel`, `.anim` (also `.img` in some SDK examples). Big-endian chunk files from the
  3DO Company's Macintosh/PC tools. Chunk = 4-byte tag, u32 size including the 8-byte header, body, padded
  to 4 bytes. Tags: `3DO ` (optional wrapper), `IMAG`, `CCB `, `PDAT`, `PLUT`, `ANIM`, `VDL `, `CPYR`,
  `DESC`, `KWRD`, `CRDT`.
- Spec [fetched]: <https://3dodev.com/documentation/file_formats/media/container/3do> (original:
  `ppgfldr/smmfldr/cdmfldr/08CDM001.html` of the Portfolio 2.5 docs, also in
  [trapexit/3do-devkit](https://github.com/trapexit/3do-devkit) `docs/3dosdk/`). Licence of the docs: not
  stated, prose use only.
- IMAG (28 bytes) [fetched]: i32 width, height, bytes per row; u8 bits per pixel (8/16/24), components
  (3 RGB, 1 indexed), planes, colour space (0 RGB, 1 YCrCb), compression (0 none, 1 "cel bit packed"),
  hvformat, pixel order, version. `PDAT` holds the pixels; `PLUT` (u32 count + RGB555) holds the palette
  of indexed images.
- Pixel order 1 interleaves row pairs [observed]: with pixel order 1 ("LRform"), rows 2p and 2p+1 are
  stored interleaved pixel by pixel. A plain row-major read of `seafloor.imag` shows scanline banding; the
  interleaved read is clean. 16-bit pixels are big-endian 0RRRRRGGGGGBBBBB.
- CCB / cel data [fetched, Graphics Programmer's Guide `5gpgd.html` in the same repo]: a `CCB ` chunk (80
  bytes in all samples) carries the 3DO cel control block, including PRE0/PRE1 and width/height. Cels are 1, 2,
  4, 6, 8 or 16 bits per pixel, coded (through a PLUT) or not, and either unpacked or packed. Packed rows start
  with an 8- or 10-bit offset to the next row, then bit-packed packets: a 2-bit type (literal 01, repeat 11,
  transparent 10, end-of-line 00) and a 6-bit count minus one. Not yet decoded by me; the doc is enough to
  start.
- ANIM files hold one `CCB `/`PDAT` pair per frame (12 frames in `fish.anim`, all 128x76) [observed].
  Show frame 0.
- Samples: `trapexit/3do-devkit` (verified; `git ls-tree` plus raw downloads of five files) has about 171
  `.cel`, 134 `.imag`, 60 `.anim`, 54 `.img`, 15 `.vdl` files, mostly under `examples/reworked/*/data/`,
  plus `art/`. They are 3DO SDK assets; their redistribution terms are unclear, which is fine for a git-ignored
  corpus. Examples: `animsample/data/seafloor.imag` (320x240, 16 bit), `bounce/data/BounceFolder/Art/ball.cel`
  (50x50 cel), `animsample/data/fish.anim`.
- Reference code: none permissive found or read. `3dodev.com` and the devkit repo carry SDK sources of
  unchecked licence.
- RECOIL: no. Check by: eye (the samples are recognisable pictures); no independent decoder identified.
- Difficulty: M (IMAG with pixel orders and bit depths is S; packed cels, 6-bit pixels and RGB555 PLUT lookups
  are the M part). Relevance: low-medium; 3DO preservation is active. Sig: strong (tags at offset 0).
  `.cel` collides with the registered Cyber Paint and Animator CEL formats, so detection must go by tag.

### 3.4 Dreamcast VMU: VMS, ICONDATA_VMS, DCI

- Spec [fetched, Marcus Comstedt, 2000, no licence statement]:
  <https://mc.pp.se/dc/vms/fileheader.html> and <https://mc.pp.se/dc/vms/icondata.html>.
- VMS header (at offset 0 for data files, `$200` for game files): 16-byte menu description, 32-byte
  file-manager description, 16-byte creator, u16 icon count (1-3), u16 animation speed, u16 eyecatch type, u16 CRC,
  u32 data size, 20 reserved; then a 32-byte palette (16 x ARGB4444, little endian) at `$60`, icon bitmaps at `$80`
  (32x32, 4 bit, high nibble left, 512 bytes each), then the eyecatch (72x56): type 1 = 16-bit true colour
  (8064 bytes), type 2 = 256-colour (512-byte palette + 4032 bytes), type 3 = 16-colour (32-byte palette + 2016 bytes).
- ICONDATA_VMS: 16-byte description, u32 offset of a 32x32 monochrome icon (128 bytes, 1 = black), u32 offset of an
  optional colour icon (palette + 512 bytes) or 0.
- CRC (CRC-16 over the file with the CRC field zeroed, polynomial 0x1021, given as C code in the spec) is the
  only strong signature for data files. It is ignored in game files.
- Verified [observed]: KallistiOS `examples/dreamcast/vmu/vmu_game/romdisk/TETRIS.VMS` (3584 bytes, game file,
  header at `$200`, 2 icons) decodes to a two-frame animated Tetris piece.
- Not verified: `.dci` (Nexus/DreamExplorer dumps: the same data with each 4-byte group byte-reversed, plus a
  directory entry in front) and `.vmi` (metadata only, no image) [memory]. Full 128 KiB VMU images need a
  filesystem walk; skip.
- Reference code: [mrneo240/NeoDC-Icondata-Tool](https://github.com/mrneo240/NeoDC-Icondata-Tool) (BSD-style
  2018), [RobertDaleSmith/vmu-icon-maker](https://github.com/RobertDaleSmith/vmu-icon-maker) (MIT). Avoid
  [bucanero/dcvmu-tool](https://github.com/bucanero/dcvmu-tool) (GPL-3.0).
- RECOIL: no. Check by: eye; Dreamcast emulators show the same icons. Difficulty: S. Relevance:
  low-medium (VMU save managers). Output: icon frames and eyecatch on one sheet, or frame 0 only.

### 3.5 Sony GIM (PSP and PS3)

- Magic: `MIG.00.1PSP` + NUL (little endian, PSP) or `.GIM1.00PSP` (big endian, PS3) [snippet + observed
  for the PSP form in `Checkpoint.gim`]. Block structure: id 2 root, 3 picture, 4 image, 5 palette, each with
  size and offsets [snippet, matches the first 0x60 bytes of the sample]. Pixel order 0 normal or 1 swizzled
  (16x8-byte blocks) [snippet]. Pixel formats include RGB565, RGBA5551, RGBA4444, RGBA8888, 4/8/16/32-bit
  index and DXT1/3/5; palettes in the RGB formats [snippet via PuyoTools codec file names].
- Docs: [psdevwiki PS3 GIM page](https://www.psdevwiki.com/ps3/Graphic_Image_Map_(GIM)) and the Reverse
  Engineering Wiki page [snippet only; neither was read in full].
- Reference code: PuyoTools.Core `Textures/Gim/` (MIT; I read only the magic-code lines, which handle both
  endiannesses); [adeyblue/GIMSplit](https://github.com/adeyblue/GIMSplit) (MIT, splits multi-picture files).
  Avoid Kuriimu2 (GPL-3.0).
- Samples: `https://sembiance.com/fileFormatSamples/image/psxGIM/` (12 verified listings; one downloaded,
  66768 bytes = 256x256 8-bit + 1 KiB palette plus headers).
- RECOIL: no. Difficulty: M. Relevance: medium (PSP and Vita-era game modding).

### 3.6 SNK Neo Geo: cartridge sprites, CD `.SPR`, `.FIX`

- Spec [fetched, <https://wiki.neogeodev.org/>, content "Public Domain"]:
  - Sprites: 16x16 pixels, 4 bits, 128 bytes per tile. Four 8x8 blocks in the order top-right, bottom-right,
    top-left, bottom-left. Each row of a block is stored in four bit planes, one byte per plane, bit 7 the
    leftmost pixel. On cartridge, planes 0 and 1 live in the odd C ROM (`c1`, `c3`...) as byte pairs, planes
    2 and 3 in the even one (`c2`...), so decoding needs the pair. On CD it is one `.SPR` file (up to 4 MiB),
    four bytes per row in plane order 1, 0, 3, 2 (page "Sprite graphics format").
  - Fix layer (8x8, 32 bytes, `.s1` on cartridge, `.FIX` on CD, up to 128 KiB): 4 bits per pixel, linear, stored by
    column top to bottom, pixels in a byte swapped (left pixel in bits 0-3), column order A..., with the half and
    column bits in the address (page "Fix graphics format").
  - Colours: 16-bit words, `D R0 G0 B0 R4 R3 R2 R1 G4..G1 B4..B1` from the top bit (page "Colors"): five
    bits per channel with the "dark" bit as a shared least significant bit; palettes are 16 colours with entry 0
    transparent.
- Verified [observed]: freem's tutorial pack
  <https://www.ajworld.net/neogeodev/beginner/media/helloworld_tutorial.zip> (50 KB) contains `hello-c1.c1` +
  `hello-c2.c2` (128 KiB each), `HELLO.SPR` (256 KiB), `hello.fix` (128 KiB) and the source tiles `HELLO.sms`
  (SMS-format 4 bpp). The glyph data populates plane 0 only, so the CD file has its nonzero bytes at row offset 1
  of each 4-byte group, which matches "1, 0, 3, 2". Cartridge and CD files are the same art, so they check each
  other. Plane 0 only means the other three planes and the quadrant order remain unverified.
- Palette files: Neo Geo CD loads `.PAL` files (type 5) [snippet]; the extension and layout (one bank of
  16-bit words) are not confirmed. The page for the IPL file would say; not read.
- Reference code: [freem/NeoSpriteConv](https://github.com/freem/NeoSpriteConv) (MIT) converts the SMS
  4-bit format to cartridge and CD sprites. `city41/neospriteviewer` has no licence file: docs only.
- Registry notes: needs `Companions` for the `c1`/`c2` pair (the naming varies: `*-c1.c1`, `*_c1.bin`,
  MAME-style `NNN-c1.c1`). `.spr` is already claimed by five formats (RISC OS, Apple II, two Atari 8-bit, Vector-06C); a CD sprite
  file is headerless and only size-checkable (multiple of 128 bytes, at most 4 MiB), so it would sit behind
  extension gating and rank last among claimants.
- RECOIL: no. Check by: the tutorial pack's three representations of the same art; MAME's graphics
  viewer by eye. Difficulty: S-M. Relevance: low-medium (homebrew and ROM-hack work).

### 3.7 Mega Drive tile art: Nemesis and Kosinski

- What: graphics in Sonic-era Mega Drive ROMs are LZ- or entropy-compressed 4-bit tile data; hacking
  projects keep them as separate files (`.nem`, `.kos`, `.bin`). Enigma (`.eni`) compresses tilemaps, which
  needs a tile set to mean anything: skip.
- Nemesis [fetched, <https://info.sonicretro.org/Nemesis_compression>, CC BY 4.0]: first word = number
  of 8x8 patterns, sign bit set for XOR mode; then a code table (palette index nibble, run length - 1 and code
  length in the second byte, the code in the third; `FF` ends the table); then a bit stream where `111111`
  introduces inline data (`XXXYYYY`: run - 1, nibble) and every other code is looked up. Rows are 4 bytes; XOR
  mode applies each row to the previous. The page also gives the Mega Drive tile layout it expands to.
- Kosinski: a LZ77 variant used for Sonic 2 and 3 level art; a page exists on the same wiki
  (<https://info.sonicretro.org/Kosinski_compression>); only its title was seen.
- Plain Mega Drive tile [fetched, plutiedev.com `tiles-and-palettes`]: 8x8, 32 bytes, 4 bytes a row, one nibble
  per pixel, high nibble on the left. Palette: 32 bytes per palette, 16 big-endian words `0000 BBB0 GGG0 RRR0`
  (only even values are used), four palettes in CRAM (128 bytes). Tilemap word: bit 15 priority, 14-13
  palette, 12 vflip, 11 hflip, 10-0 tile (`tile-id` page).
- DAC output levels [fetched, plutiedev `vdp-color-ramp`]: for CRAM values 0, 2, 4, 6, 8, A, C, E the
  voltage steps are about 0, 52, 87, 116, 144, 172, 206, 255 on a 0-255 scale (the usual `v * 17` is not what
  the hardware does). That is a palette-choice decision like the NES master palette: record it.
- Samples: <https://github.com/sonicretro/s1disasm> `artnem/` (verified by directory listing: 100+ `.nem`
  files such as `Rings.nem`, `HUD.nem`, `Boss - Main.nem`), plus other disassemblies for Kosinski. The repo has
  no licence file; the data is Sega's, which a git-ignored corpus tolerates. Palettes are separate raw files
  (`palette/*.bin`) that could be offered as companions.
- Reference code: not read. mdcomp and clownnemesis exist but their licences were not checked; the prose
  page is enough for Nemesis.
- RECOIL: no. Check by: eye (Sonic sprites are recognisable); an independent Nemesis tool as a black box.
- Difficulty: M (bit-level decoder, tile-sheet layout, palette). Relevance: low-medium. Without a
  palette the sheet is grey, which tells little; the format earns its place only with a palette companion.

### 3.8 PlayStation memory-card save icons

- Spec [fetched, psx-spx "Memory Card Data Format",
  <https://problemkaputt.de/psxspx-memory-card-data-format.htm>]: the first 128-byte frame of a save is a title
  frame: `SC`, icon display flag (`11`/`12`/`13` = 1, 2, 3 frames), block number, 64-byte Shift-JIS title, a 16-entry
  CLUT of 15-bit colours at `$60` (0000h transparent, 8000h solid black), then 1-3 icon frames of 128 bytes
  (16x16, 4 bit).
- Containers [memory]: raw 128 KiB card images (`.mcr`, `.mcd`, `.gme`, `.bin`: the directory is in
  block 0, saves follow), single-save wrappers `.mcs`, `.psv`, `.mcb`, `.psx`. Wrapper headers differ and were not
  checked. A decoder would show the first icon of the first save in a card image, or one sheet of every icon.
- Samples: none located (saves are copyrighted; PS1 save collections are everywhere, none verified).
- RECOIL: no. Difficulty: S for the icon, M once wrappers are counted. Relevance: low. The 16x16
  result is small enough to be a poor thumbnail.

### 3.9 Raw VRAM / tile dumps by console

None of these has a header, so a decoder would be extension-gated and, like NES `.chr`, show a tile sheet.
They matter for emulator debugger dumps and ROM-hacking folders. The extension conventions are not
documented anywhere I could find (`.chr`, `.bin`, `.vram`, `.til` all appear); a survey of real folders is needed
before registering any. Layouts:

| Machine | Tile | Palette | Source |
|---|---|---|---|
| Master System, Game Gear | 8x8, 32 bytes, 4 bytes per row, one plane per byte (plane 0 first) | SMS: 32 bytes `--BBGGRR`; GG: 64 bytes, LE words `----BBBBGGGGRRRR`; two 16-colour palettes, sprites use the second | [SMS Power Tiles](https://www.smspower.org/Development/Tiles) [fetched]; Charles MacDonald's `msvdp-20021112.txt` section 5 [fetched, "unpublished work Copyright"; facts only] |
| Mega Drive | 8x8, 32 bytes, nibble per pixel | 4 x 16 words `0BBB0GGG0RRR0` | plutiedev [fetched] |
| PC Engine / TurboGrafx | BG 8x8, 32 bytes: 8 words with plane 0 in the low byte and plane 1 in the high byte, then 8 words for planes 2 and 3; sprite 16x16, 128 bytes (4 planes of 16 words) | 9 bits, GRB: bits 0-2 blue, 3-5 red, 6-8 green, 512 entries (32 palettes of 16) | HuC6270 and HuC6260 pages on the old Archaic Pixels wiki, Wayback 2014-15 [fetched for VCE bits and the CG0/CG1 word split; exact byte order from [snippet] and [memory]] |
| WonderSwan | 2 bpp planar (Game Boy format, 16 bytes), 4 bpp planar (SMS format, 32 bytes) or 4 bpp packed | colour: 12-bit `rrrr gggg bbbb` words, 16 palettes of 4 colours (or 16-colour mode); mono: 3-bit entries into a shade table | [WSdev wiki](https://ws.nesdev.org/wiki/Display/Tile_Data) [fetched], CC0 |
| Neo Geo Pocket Color | 8x8, 2 bpp, 16 bytes | 12-bit 0BGR words | [memory]; `devrs.com/ngp/files/ngpctech.txt` was reachable but had no tile section |

Palette traps [fetched]: the SMS's 2-bit channels have no single RGB conversion. BMP2Tile's README says it
accepts Meka's two palettes and eSMS's and prefers 0, 85, 170, 255; plutiedev measures 0, 90, 173, 255 (early
SMS), 0, 89, 174, 255 (later) and 0, 99, 162, 255 (Mega Drive in mode 4). Pick one and record it, as for the NES.
BMP2Tile itself (<https://github.com/maxim-zhao/bmp2tile>, MIT) turns BMP/PNG/GIF into these tiles, so its
output makes test files; its compressor plugins are a separate repo of mixed licences (unchecked).

- Samples: homebrew and emulator-debugger dumps; none verified. Real folders would settle the extension question.
- RECOIL: no. Check by: compare with an emulator's VRAM viewer (BlastEm, Mesen-style) by eye.
- Difficulty: S each, and one parameterised sheet decoder could cover all of them. Relevance: low; the
  risk is false positives, since nothing in these files identifies them.

### 3.10 Philips CD-i IFF images

- What I found [observed]: `FORM` + u32 BE length + `IMAG`, then `IHDR` (14-byte body starting with width,
  line size in bytes, height), then `PLTE` (palette) and/or `IDAT` (pixels). Eleven samples at
  `https://sembiance.com/fileFormatSamples/image/cdiIFFImage/` use extensions that encode the coding method:
  `.6dy`/`.4dy` (DYUV), `.6r7` (RL7), `.4c4` (CLUT4), plus `.iff`. `INTRO_N.6DY` is 99882 bytes with a 0x18600-byte
  `IDAT`.
- Spec [fetched]: Philips Technical Note 22 refers to the "CD-I IFF specification version .99" and
  describes padding via the `IHDR` line size and a planar RGB555 layout, but the spec itself is not on
  icdia.co.uk. DYUV is fully described in Technical Note 86 (<http://www.icdia.co.uk/notes/technote086.pdf>,
  Philips copyright, "not to be duplicated" banner; prose use only): a 16-entry table maps 4-bit codes to deltas
  (0, 1, 4, 9, 16, 27, 44, 79, 128, 177, 212, 229, 240, 247, 252, 255), added modulo 256 to the previous pixel's
  Y, U and V, with start values per line. RL7 and CLUT7 are only named in the technical summary
  (<http://icdia.co.uk/brochures/developer_info/devinfo_techsummary.pdf>) and the YUV-to-RGB matrix would need the
  Green Book (not located).
- RECOIL: no. Difficulty: M for CLUT4 and DYUV once the YUV matrix is chosen; L for RL7 without a doc.
  Relevance: low. Check by: eye.

### 3.11 Atari Lynx `.spr` + `.pal` (sprpck)

- Tool: sprpck, the standard Lynx sprite packer ([42Bastian/sprpck](https://github.com/42Bastian/sprpck),
  Apache-2.0, checked): reads PCX, BMP, PI1 and raw data and writes `.spr` (sprite data) plus a palette file in
  C, ASM or "LYXASS" text form. Sprites are 1-4 bpp, packed or literal.
- Format [snippet, AtariAge threads and chibiakumas]: each line starts with a byte giving the offset to the next
  line (0 ends the sprite); then 5-bit-count blocks, literal or run, with a flag bit; bit depth and literal-or-packed
  live in the SCB, not in the `.spr` file.
- Why it ranks low: the file carries no width, height or bit depth, and the palette is text. Width could be
  recovered by decoding a row and depth guessed, but that is a heuristic. A `.lnx` ROM header holds no image.
- RECOIL: no. Difficulty: M. Relevance: low.

### 3.12 Adjacent finds: fantasy consoles

These are outside the brief but turned up and have documentation; the maintainer can decide.

- TIC-80 `.tic` [fetched, <https://github.com/nesbox/TIC-80/wiki/.tic-File-Format>, MIT]: chunks of a
  4-byte header (bank:3 + type:5, u16 LE size, reserved) and data. Type 1 tiles and 2 sprites (256 each, 8x8, two
  pixels per byte), 12 palette (16 x RGB), 18 screen (240x136, 4 bit; the cover image). Default palette is
  "Sweetie 16" when no palette chunk exists [memory]. A cartridge-cover or sprite-sheet render is S, samples are
  everywhere on tic80.com.
- PICO-8 `.p8` [memory; its wiki is behind a Cloudflare challenge]: text file with `__gfx__` (128 rows of
  128 hex digits, one 4-bit pixel each), `__label__` (128x128) and a fixed 16-colour palette. `.p8.png` cartridges
  are 160x205 PNGs with data hidden in the low bits; they need a PNG decoder, which the registry lacks.

## 4. Rejected, and why

| Candidate | Reason |
|---|---|
| Atari 2600 / 7800 graphics tools | No standalone image format found. 7800basic converts PNG at build time and PlayerPal-style editors save assembler text [memory]; not searched because the quota ran out. |
| Atari Jaguar | CRY and RGB16 framebuffers exist, but no file container was identified [memory]. |
| Atari 5200 | Same GTIA/ANTIC output as the 8-bit family, already covered there; no 5200-specific file format known. |
| ColecoVision, SG-1000, SC-3000 | TMS9918 screen-2 dumps are what `msx/screen.rs` already decodes; no console-specific container found [memory; not searched]. |
| Intellivision, Odyssey2, Channel F, Vectrex | GRAM cards or vectors in ROM; no bitmap file formats. |
| Pokemon Mini, Watara Supervision, Game.com | Raw tiles inside ROMs, no documentation reached. |
| Sega Saturn | VDP1/VDP2 cell and bitmap formats are hardware formats; no standalone container found [memory]. |
| Sega CD, 32X | Same as above; the 32X has a 15-bit bitmap mode, nothing file-shaped. |
| PC-FX, PCE-CD | No image formats found. |
| Xbox XPR / XBX | The XPR page on xboxdevwiki (via Wayback) has `[FIXME]` fields for the texture entry; needs D3D formats, Morton swizzle and DXT. 2001 hardware and no demand evidence. |
| PS1 PXL / CLT | psx-spx: "very rare", three games; also has swapped ids in practice. |
| PS1 Ape Escape TIM-RLE, BS/MDEC stills | Documented in psx-spx but game-specific or a video codec; wait for demand. |
| Yuke's TXC (`RTX38`) | Undocumented. |
| Emulator savestates (Gens `.gs0`, Genecyst, Fusion, Mednafen) | Contain VRAM and registers, so a render needs a VDP model: L, and the file formats are tied to GPL emulators. Not pursued. |
| BizHawk `.State` | A ZIP whose `Framebuffer` entry is a BMP written by `QuickBmpFile` [fetched, MIT `SavestateFile.cs`]. It would need a ZIP reader and BMP support, and says nothing about the console. |
| KallistiOS `.dt`/`.kmg`/`.tex` | KOS `pvrtex` output formats, tool-specific, no demand. |

## 5. Ranked top 10

1. PVR / PVP / PVM (and GVR / GVM). Strong signature, high demand, MIT reference code for four texture
   families, free samples (KOS) and a dozen more in hand. The twiddle and VQ pitfalls are now written down.
2. TIM2. Official spec, 11 variant samples that check each other, six real files, and the one gotcha (CSM1
   CLUT swap) found. Nothing else in the console area is this ready.
3. 3DO IMAG / CEL / ANIM. Official docs, strong tags, 365 files in one repo. The row-pair interleave and the packed
   cel decoder are the work.
4. Dreamcast VMS / ICONDATA_VMS. Small, fully specified, one verified sample. A good filler between
   bigger jobs.
5. GIM. MIT reference code and 12 samples; the spec is the weak part.
6. Neo Geo CD `.SPR`/`.FIX` and cartridge C pairs. Public-domain wiki, a three-way self-checking sample
   pack; held back by missing detection and a collision-prone extension.
7. Mega Drive Nemesis tile art. Clear CC BY spec and many samples, but only useful with a palette.
8. PS1 memory-card icons. Small and certain, small payoff.
9. Raw VRAM tile sheets (SMS/GG, MD, PCE, WonderSwan). Layouts known, nothing to detect them with, extension
   practice unknown.
10. CD-i IFF images. Eleven samples but the spec is not public in what I reached; CLUT4 and DYUV only.

Then: Lynx `.spr`/`.pal`, TIC-80 `.tic` (S, easy, outside the brief).

## 6. Registry and API notes

- Alpha: `Image` is RGB only and every existing decoder ignores alpha (GIF, ICO, BMP, TGA do). PVR, GVR,
  TIM2, VMS and GIM all carry it. Ignoring it is consistent but loses alpha-only fonts (`Font.pvr` would be
  solid). Composite onto a grey (as `fm_towns/icn.rs` does) or accept the loss; decide once.
- Several pictures per file: PVM/GVM archives, multi-picture TIM2, 3DO ANIM, VMS icon frames, memory-card
  icons. Existing precedent: first frame (`fm_towns/hel.rs`) and a sheet (`fm_towns/icn.rs`). Pick per format:
  first picture for archives of unrelated textures, a sheet for icon frames.
- Companions fit PVR+PVP, Neo Geo `c1`+`c2`, and palette files for tile sheets. Every one must still
  decode alone (grey ramp or grey sheet), as `adding-a-format.md` requires.
- Shared helpers: a Morton/twiddle index (PVR; GVR uses GameCube block tiling instead, and a GS swizzle
  could join it if swizzled TIM2 files appear) and the GameCube tile unpackers are worth one module each
  rather than copies. The Nintendo survey may want the latter.
- Extension collisions: `.cel` (Cyber Paint, Animator), `.spr` (five formats), `.chr` (Blazing Paddles font,
  NES). Use tags for 3DO and PVR; gate headerless Neo Geo and tile dumps by extension and size, last.
- Palette decisions to record (like the NES master palette): SMS 2-bit expansion, Mega Drive DAC ramp, PCE
  9-bit expansion, Neo Geo dark bit, TIM2 alpha (0x80 = opaque), VMU ARGB4444 (v * 17).
- Provenance question for the maintainer: the TIM2 spec carries a "confidential" banner and the CD-i
  technical notes a "not to be duplicated" one. CLEANROOM allows public prose, so this is a judgement call,
  not a rule.

## 7. To avoid (adds to the existing lists)

- RECOIL and any port or fork, as always.
- bucanero/dcvmu-tool (GPL-3.0), marco-calautti/Rainbow (GPL-2.0), FanTranslatorsInternational/Kuriimu2
  (GPL-3.0): licences read from their `LICENSE` files. Rainbow's wiki TIM2 page is a stub.
- KallistiOS `utils/pvrtex/`: it bundles FFmpeg's `elbg` files (`elbg.c`, `libavcodec/elbg.h`), which are LGPL
  [memory]. Running the binary is fine; reading its source is not. The rest of KOS is BSD-like and readable.
- GirianSeed/tim2 `sample/` and `tim2sample_e.zip`: no licence, treat as all rights reserved. The spec text and
  the sample images are the useful parts.
- city41/neospriteviewer: no licence file.
- Ikaruga guides (ikaruga.dashgl.com), SmokesGrass PVR doc: licence not stated, prose facts only, and both
  contain errors (section 3.1).
- Licences not checked: mdcomp, clownnemesis, bmp2tilecompressors, the Sonic disassemblies, the 3DO devkit
  sources. Treat as unreadable until checked.

## 8. Sample source index

| Source | Format | What | Verified |
|---|---|---|---|
| KallistiOS `examples/dreamcast/` | PVR, VMS | `glass.pvr`, `star.pvr`, `TETRIS.VMS` | downloaded and decoded |
| `sembiance.com/fileFormatSamples/image/pvrTexture/` | PVR | 10 files, 7 KB-175 KB | 5 downloaded, 5 decoded |
| `sembiance.com/.../tim2/` | TIM2 | 6 game files | 5 downloaded, 4 decoded, 1 odd |
| `github.com/GirianSeed/tim2` `webtech/tim2img_e.zip` | TIM2 | 11 spec sample files | downloaded, all decoded |
| `sembiance.com/.../psxGIM/` | GIM | 12 files | listed, 1 downloaded (header only) |
| `github.com/trapexit/3do-devkit` `examples/reworked/` | 3DO | 171 CEL, 134 IMAG, 60 ANIM, 54 IMG | tree listed, 5 downloaded, 1 decoded |
| `ajworld.net/neogeodev/beginner/media/helloworld_tutorial.zip` | Neo Geo | c1/c2, SPR, FIX, SMS source | downloaded, inspected |
| `sembiance.com/.../cdiIFFImage/` | CD-i | 11 files | 3 downloaded, headers read |
| `github.com/sonicretro/s1disasm` `artnem/` | Nemesis | 100+ `.nem` | tree listed |

All sample files belong to their artists and publishers; they would live in the git-ignored `corpus/` with a
`MANIFEST.tsv`, as before.

## 9. Gaps in this survey

- Atari 2600/5200/7800/Jaguar, ColecoVision, Intellivision and Saturn tool formats need a proper search once a
  quota is available.
- Kosinski, Enigma and Saxman were not read in detail.
- PCE tile byte order and NGPC tile layout are `[memory]`; confirm against the Hudson VDC manual or an emulator
  before coding.
- The 3DO packed-cel decoder, the GIM block layout, `.dci` byte order and the PS1 memory-card wrappers have not
  been tried against samples.
