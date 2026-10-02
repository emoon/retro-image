# retro-image

Decodes picture formats of retro computers (Atari 8-bit and ST, Amiga, Commodore, ZX
Spectrum, Amstrad CPC, MSX, PC-98 and more) to RGB. It is an independent, MIT-licensed
reimplementation of what [RECOIL](https://recoil.sourceforge.net) decodes. See
[docs/coverage.md](docs/coverage.md) for which of RECOIL's formats are covered.

- `retro-image`: the library. `#![no_std]` + `alloc`, no dependencies, works on byte
  slices only.
- `retro-image-cli`: a `retro-image` command that converts to PNG and works as a
  freedesktop thumbnailer.

## Library

```rust
let data = std::fs::read("PICTURE.PI1")?;
let image = retro_image::decode("PICTURE.PI1", &data)?;
let (width, height) = (image.width(), image.height());
let rgb: &[u8] = image.rgb(); // 3 bytes per pixel, row-major
```

The file name picks candidate formats by extension. Formats with reliable magic bytes are
also tried for any file name (content detection). `retro_image::candidates(name)` lists the
formats that would be tried, in order.

Some formats keep colours or other data in a second file (e.g. `.SCR` + `.PAL`). Pass
those through the `Companions` trait with `decode_with`. Without them, such a picture is
still decoded where the format allows it, e.g. with the default palette.

## Command line

```sh
cargo install --path crates/retro-image-cli

retro-image PICTURE.PI1                  # writes PICTURE.PI1.png
retro-image PICTURE.PI1 -o out.png
retro-image --list-formats               # platform, name, extensions, detection
```

### Thumbnails in file managers

```sh
retro-image -i INPUT -o OUTPUT -s SIZE [--ext EXT]   # thumbnailer mode
retro-image --mime-xml     > ~/.local/share/mime/packages/retro-image.xml
retro-image --thumbnailer  > ~/.local/share/thumbnailers/retro-image.thumbnailer
update-mime-database ~/.local/share/mime
```

The MIME package registers every supported extension as `image/x-retro-image`. On
failure the command writes nothing and exits non-zero, as thumbnailers require.

## Clean-room development

RECOIL is GPL. retro-image is written without reading its source or any other GPL
decoder: decoders come from public documentation, permissively licensed code, and
reverse engineering of sample files. The `recoil2png` binary is used only as a black box
to compare output. The rules are in [CLEANROOM.md](CLEANROOM.md); each decoder module
names its sources.

## Development

```sh
cargo clippy --workspace --all-targets
cargo test --workspace
```

The test suite includes two corpus tests. The sample corpus is not in the repository
(the pictures are copyrighted); see [CLEANROOM.md](CLEANROOM.md) to fetch RECOIL's
example set into `corpus/`. Both tests skip the corpus when it is missing.

- `tests/oracle.rs` compares every decodable corpus file with `recoil2png`. RECOIL is a
  baseline, not the definition of correct: deliberate differences, each with its
  evidence, are listed in `crates/retro-image/tests/divergences/`.
- `tests/robustness.rs` feeds truncated and mutated files to every decoder and fails on
  any panic. For fuzzing, see [fuzz/README.md](fuzz/README.md).

To add a format, see [docs/adding-a-format.md](docs/adding-a-format.md). Per-platform
format research is in [docs/formats/](docs/formats/README.md). Regenerate the coverage
report with:

```sh
curl -sLo /tmp/recoil-formats.html https://recoil.sourceforge.net/formats.html
scripts/coverage.py /tmp/recoil-formats.html > docs/coverage.md
```

## License

MIT, see [LICENSE](LICENSE).
