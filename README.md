# retro-image

retro-image turns pictures from old computers into RGB. It reads 461 formats from 49
platforms: Atari 8-bit and ST, Amiga, Commodore, ZX Spectrum, Amstrad CPC, MSX, PC-98 and
a long tail of rarer machines. The full list is in [docs/formats.md](https://github.com/emoon/retro-image/blob/main/docs/formats.md).

The project was inspired by [RECOIL](https://recoil.sourceforge.net), and the tests
compare our output with RECOIL's. None of RECOIL's code was used, though. The decoders
were written from public documentation, permissively licensed code, and sample files
taken apart by hand. [docs/sources.md](https://github.com/emoon/retro-image/blob/main/docs/sources.md) says where they came from.

## Library

The crate is `no_std` with `alloc`, has no dependencies and only ever sees byte slices.

```toml
[dependencies]
retro-image = "0.0.1"
```

```rust
let data = std::fs::read("PICTURE.PI1")?;
let image = retro_image::decode("PICTURE.PI1", &data)?;
let (width, height) = (image.width(), image.height());
let rgb: &[u8] = image.rgb(); // 3 bytes per pixel, row-major
```

The file name decides which formats to try. Formats with a reliable signature are also
recognised by content, so those still decode when the extension is wrong. Some formats
keep their colours in a second file, like a `.SCR` with a `.PAL`; pass it in with
`decode_with`.

## Command line

```sh
cargo install retro-image-cli
retro-image PICTURE.PI1                  # writes PICTURE.PI1.png
retro-image --list-formats
```

It can also make thumbnails for file managers that use freedesktop thumbnailers:

```sh
retro-image --mime-xml    > ~/.local/share/mime/packages/retro-image.xml
retro-image --thumbnailer > ~/.local/share/thumbnailers/retro-image.thumbnailer
update-mime-database ~/.local/share/mime
```

## Documentation

- [Supported formats](https://github.com/emoon/retro-image/blob/main/docs/formats.md)
- [Sources](https://github.com/emoon/retro-image/blob/main/docs/sources.md)
- [Development](https://github.com/emoon/retro-image/blob/main/docs/development.md): building, testing and adding a format

## License

MIT, see [LICENSE](https://github.com/emoon/retro-image/blob/main/LICENSE).
