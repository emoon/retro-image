# Sources

Nothing in retro-image comes from RECOIL's code, or from any GPL or LGPL code. Everything
was built from public documentation, permissively licensed code and sample files. The
rules are in [CLEANROOM.md](../CLEANROOM.md).

This page covers the main sources. For the exact ones, read the `//!` header of a
decoder's module under `crates/retro-image/src/platform/`. Those headers go down to the
level of "this palette order came from Deark, this background byte was found by editing
samples". The research notes per platform, including the projects we stayed away from,
are in [research/](research/README.md).

## Permissively licensed code

Where code or tables derive from one of these projects, the module keeps the original
copyright notice and license text.

| Project | License | Used for |
|---|---|---|
| [Deark](https://github.com/jsummers/deark) | MIT | Amiga icons (classic, NewIcons, GlowIcons), RISC OS sprites, PC text-mode fonts, DOS clip-art libraries, icons and self-displaying pictures, the Psion and Palm formats, and many smaller details |
| [GoDot](https://github.com/godot64/GoDot) | MIT | C64 loaders and GoDot's own 4-bit format |
| [Ancient](https://github.com/temisu/ancient) | BSD-2-Clause | Depackers: Pack-Ice, PowerPacker, XPK (RLEN, FAST, MASH, NUKE), RNC, Imploder, Crunch-Mania |
| [MAME](https://github.com/mamedev/mame) (Thomson driver) | BSD-3-Clause | Thomson palette and screen modes |
| [pynuvie](https://github.com/anarkiwi/pynuvie) | Apache-2.0 | C64 NUFLI tables |
| [libansilove](https://github.com/ansilove/libansilove) | BSD-2-Clause | PCBoard color codes |
| [monobit](https://github.com/robhagemans/monobit) | MIT | Daisy-Dot fonts (Atari 8-bit) |
| [CiderPress II](https://ciderpress2.com) | Apache-2.0 (docs CC BY-SA 4.0) | Apple II and IIGS formats, PackBytes |
| [libsixel](https://github.com/saitoha/libsixel) | MIT | Sixel: what viewers do where the DEC manual is silent (the color drawn before any selection, HLS rounding, picture size), read in `fromsixel.c`; its `sixel2png`, built from source and run as a black box, is the oracle for the sample files |
| [flipnote.js](https://github.com/jaames/flipnote.js) | MIT | The 16-color thumbnail palette of Flipnote Studio files. Its renderer also served as a black-box check on the decoded thumbnails |
| [RGBDS](https://github.com/gbdev/rgbds) | MIT | Game Boy rgbgfx files (tile data, maps, palettes) |
| [NitroPaint](https://github.com/Garhoogin/NitroPaint) | BSD-2-Clause | Nintendo DS NCLR, NCGR, NSCR and BTX0 files |
| [PuyoTools](https://github.com/nickworonekin/puyotools) | MIT | GameCube texture formats (GVR, TPL, banners), Dreamcast PVR and PVM layouts, PSP GIM |
| [TIC-80](https://github.com/nesbox/TIC-80) | MIT | TIC-80 cartridges: which palette a cartridge gets, and the DB16 colors, from `src/cart.c` |
| [X.Org](https://gitlab.freedesktop.org/xorg/proto/xorgproto) | MIT-style (The Open Group) | XWD header layout (`XWDFile.h`) and the X11 color names (`rgb.txt`) for XPM |

A few more projects were read for facts only. Nothing was copied from them, so the code
carries no notice for them: KallistiOS (the VMU file header and some sample files),
NeoDC-Icondata-Tool, NeoSpriteConv, picotool (the `.p8` layout and its test files), Damian
Yerrick's `pilbmp2nes.py` (the size prefix of packed NES tiles) and ImageMagick's HRZ
coder (picture size, channel order, samples scaled by four).

## Documentation

The [Just Solve the File Format Problem](http://fileformats.archiveteam.org) wiki comes up
on almost every platform. Per platform, the most used were:

- Atari ST, TT and Falcon: the [Atari Forum Wiki](https://temlib.org/AtariForumWiki/)
  file format pages and the [Atari Compendium](http://cd.textfiles.com/ataricompendium/).
- Atari 8-bit: [Atari Archives](https://www.atariarchives.org) and the Mad Team and
  Graph2Font documentation.
- Commodore: [Codebase64](http://codebase.c64.org), Peter Schepers' C64 format notes and
  the [GoDot](https://www.godot64.de) loader pages.
- Amiga: the IFF specifications on the [AmigaOS wiki](https://wiki.amigaos.net) and the
  ROM Kernel Reference Manuals, plus the SGX, IFF-RGFX, ILBM64 and YAFA texts on
  [Aminet](https://aminet.net).
- Apple: Apple's technical notes and the Apple IIGS Hardware Reference.
- MSX: the [MSX2 Technical Handbook](https://konamiman.github.io/MSX2-Technical-Handbook/)
  and the [MSX Assembly Page](https://map.grauw.nl).
- Amstrad CPC: the [CPCWiki](https://cpctech.cpcwiki.de) hardware pages.
- Acorn Archimedes: the RISC OS Programmer's Reference Manuals.
- Sinclair QL: [Dilwyn Jones' QL pages](https://www.sinclairql.net).
- Thomson: the [DCMOTO](http://dcmoto.free.fr) documentation.
- Nintendo handhelds and the NES: [GBATEK](https://problemkaputt.de/gbatek.htm),
  [3dbrew](https://www.3dbrew.org), the [nesdev wiki](https://www.nesdev.org/wiki/) and
  [Pan Docs](https://gbdev.io/pandocs/). They describe layouts and little else, so each
  decoder was checked against real files, and against an independent render where one
  existed.
- PC text mode: the [SAUCE](https://www.acid.org/info/sauce/sauce.htm) specification.
- Consoles: the web technology TIM2 specification (PlayStation 2), the 3DO Portfolio 2.5
  documentation, and YAGCD and the Custom Mario Kart wiki for GameCube and Wii files.

The other platforms rely on hardware manuals and old magazine articles.

For the ZX Spectrum, [SpectraLab](https://github.com/Bedazzle/SpectraLab) (MIT) and
[zx-image](https://github.com/moroz1999/zx-image) (CC0) describe a lot of formats. We
can't tell whether their authors looked at RECOIL, so we took facts from them, checked
each one against sample files, and didn't copy any code.

## Reverse engineering

About a fifth of the formats have no public description at all, and most of those are
Atari 8-bit. They were worked out from sample files. The `recoil2png` binary was run as a
black box, never read: the oracle test compares its pixels with ours, and for the harder
cases we fed it edited files to see what changed. NUFLI's sprite handling, for example,
came from probing it with modified copies of three sample pictures. Each module header
says which details were found this way.
