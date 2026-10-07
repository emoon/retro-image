# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow
[Semantic Versioning](https://semver.org/).

The minimum supported Rust version is 1.94 (the SIMD dispatch needs it). Raising it is a
minor-version change.

## [Unreleased]

### Added

- `Decoded`: `decode` and `decode_with` return it. `Decoded::image()`, `into_image()` and
  `format()` give the picture and the format that accepted it.
- `FormatId` and `Format::id()`, a stable identity for a format.
- `Format` accessors `platform()`, `name()` and `extensions()`.
- `Limits` and `DEFAULT_MAX_IMAGE_BYTES`: a process-wide cap on decoded picture size,
  64 MiB (4 bytes per pixel) by default. Change it with
  `Limits::default().with_max_image_bytes(n).install()`. Images over the cap fail with
  `DecodeError::TooLarge` before anything is allocated.
- `Image::flattened(background)`, which composites a picture with alpha onto a color.
- `Debug` for `Image`, and `Debug`, `Clone`, `Copy`, `Default`, `PartialEq` and `Eq` for
  `NoCompanions`.
- `#[must_use]` on the `Image` accessors and conversions.
- Runnable examples in the crate docs, and a written promise that decoding never panics on
  malformed input. `Image`, `Decoded`, `Format` and `DecodeError` are `Send + Sync`, and a
  compile-time check keeps them that way.
- Command line: `--max-image-mb MB` sets the size limit; `--help` and `--version` work; errors
  say which formats were tried and why they refused the file.

### Changed

- `DecodeError` is `#[non_exhaustive]` and no longer `Copy`. Its variants are now
  `UnknownFormat`, `Invalid`, `TooLarge` and `NoMatch { attempts }`; `NoMatch` lists every
  format that was tried and why it refused the data (the old `Unrecognized` variant is gone).
  `Attempt` gives access to each entry.
- The `Format` fields are private; use the accessors.
- `Companions` methods return `Option<Cow<'_, [u8]>>` and have defaults that find nothing, so
  the trait can gain methods without breaking implementations.
- FLIC, Utah RLE and KT4 decoding does work bounded by the input size, so crafted files can't
  make them run for seconds. TGA rejects a color-mapped image with an empty map.

[Unreleased]: https://github.com/emoon/retro-image/compare/main...release-1.0
